use anyhow::{Context, Result};
use cli::{Args, BackupArgs, Command, ExecArgs, ForgetArgs, VerifyArgs};
use std::{
    env,
    io::{ErrorKind, IsTerminal},
    path::{Path, PathBuf},
    sync::OnceLock,
};

use config::{BackupOptions, CommandSeq, Config, ForgetOptions, Location};

use clap::Parser as ClapParser;
use tracing::{level_filters::LevelFilter, Level};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

mod cli;
mod config;
mod restic_api;
mod run;

const ENV_PREFIX: &str = "ARESTICRAT";

const DEFAULT_VERBOSITY: usize = 3;
static VERBOSITY: OnceLock<usize> = OnceLock::new();

fn verbosity() -> usize {
    *VERBOSITY.get().expect("Verbosity state not initialized.")
}

fn init_verbosity(quiet: bool, inc: usize) {
    let mut verbosity: usize = DEFAULT_VERBOSITY;
    if quiet {
        verbosity = 0;
    }
    verbosity += inc;
    VERBOSITY.get_or_init(|| verbosity);
}

fn main() -> Result<()> {
    let args = Args::parse();
    if let Command::License = args.command() {
        return about();
    }

    init_verbosity(args.quiet(), args.verbose() as usize);

    if let Some(wd) = args.working_dir() {
        env::set_current_dir(wd)?;
    }

    load_env_files(args.env_files())?;

    setup_logger();

    if let Err(err) = handle_command(args) {
        print_log!(Level::ERROR, "{err}");
        std::process::exit(1);
    }

    Ok(())
}

fn handle_command(args: Args) -> Result<()> {
    let config = config::Config::new(args.config_file())?;

    match args.command() {
        Command::Backup(backup_args) => backup(&config, backup_args)?,
        Command::Exec(exec_args) => exec(&config, exec_args)?,
        Command::Forget(forget_args) => forget(&config, forget_args)?,
        Command::Verify(verify_args) => verify(&config, verify_args)?,
        Command::License => panic!("Command must be handled earlier."),
    }

    Ok(())
}

fn backup(config: &Config, args: &BackupArgs) -> Result<()> {
    let api = restic_api::Api::new(config.executable().to_string());

    for (location_name, location) in config.locations() {
        let _span = tracing::info_span!("Backup", location = location_name).entered();

        let tag = get_tag(location_name);
        let backup_opts = get_backup_options(location_name, config);

        print_log!(Level::INFO, "Backup location {location_name} ...");

        let if_status = run_hooks("IF", backup_opts.hooks().r#if())?;
        if !if_status.success() {
            print_log!(Level::INFO, "IF hook failed. Skip location.");
            continue;
        }

        for repo_name in location.repos() {
            if let Some(repo) = config.repos().get(repo_name) {
                print_log!(Level::INFO, "Backup to repository {repo_name} ...");
                api.backup(
                    repo_name,
                    repo,
                    location.paths(),
                    &tag,
                    &backup_opts,
                    args.dry_run(),
                )?;
                print_log!(Level::INFO, "Backup to repository {repo_name} done.");
            } else {
                print_log!(
                    Level::WARN,
                    "Location {location_name} refers to an undefined repository {repo_name}."
                )
            }
        }

        if !args.dry_run() && backup_opts.forget() {
            forget_location(&api, location_name, location, config, args.dry_run())?;
        }
    }
    Ok(())
}

fn run_hooks(name: &str, hooks: &[CommandSeq]) -> Result<std::process::ExitStatus, std::io::Error> {
    if hooks.is_empty() {
        return Ok(Default::default());
    }

    print_log!(Level::INFO, "Running {name} hooks ...");
    run::run_sequential(hooks.iter().map(|c| c.to_command()), false)
}

fn exec(config: &Config, args: &ExecArgs) -> Result<()> {
    let api = restic_api::Api::new(config.executable().to_string());
    let mut repo_names = args.repos().to_vec();
    if (*repo_names).as_ref().is_empty() {
        repo_names = config.repos().keys().cloned().collect();
    }

    for repo_name in (*repo_names).as_ref() {
        api.exec(
            repo_name,
            config.repos().get(repo_name).unwrap(),
            args.args(),
        )?;
    }

    Ok(())
}

fn forget(config: &Config, args: &ForgetArgs) -> Result<()> {
    let api = restic_api::Api::new(config.executable().to_string());

    let locations = args
        .locations()
        .iter()
        .map(|name| {
            config
                .locations()
                .get(name)
                .map(|l| (name, l))
                .context(format!("Location {name} is undefined."))
        })
        .collect::<Result<Vec<_>>>()?;

    for (location_name, location) in locations {
        forget_location(&api, location_name, location, config, args.dry_run())?;
    }

    Ok(())
}

fn forget_location(
    api: &restic_api::Api,
    location_name: &str,
    location: &Location,
    config: &Config,
    dry_run: bool,
) -> Result<()> {
    print_log!(Level::INFO, "Forget for location {location_name} ...");

    let tag = get_tag(location_name);
    let forget_opts = get_forget_options(location_name, config);

    let if_status = run_hooks("IF", forget_opts.hooks().r#if())?;
    if !if_status.success() {
        print_log!(Level::INFO, "IF hook failed. Skip location.");
        return Ok(());
    }

    for repo_name in location.repos() {
        if let Some(repo) = config.repos().get(repo_name) {
            print_log!(Level::INFO, "Forget from repository {repo_name} ...");
            api.forget(repo_name, repo, &tag, &forget_opts, dry_run)?;
            print_log!(Level::INFO, "Forget from repository {repo_name} done.");
        }
    }

    Ok(())
}

fn verify(config: &Config, args: &VerifyArgs) -> Result<()> {
    let api = restic_api::Api::new(config.executable().to_string());

    for (repo_name, repo) in config.repos() {
        let status = api.status(repo_name, repo.path(), repo.key())?;

        use restic_api::RepoStatus::*;
        match status {
            Ok => {
                print_log!(Level::INFO, "Repository {repo_name}: OK")
            }
            NoRepository if args.init() => {
                print_log!(
                    Level::DEBUG,
                    "Repository {repo_name} not found. Initialize ..."
                );
                api.init(repo_name, repo.path(), repo.key())?;
                print_log!(Level::INFO, "Repository {repo_name}: INITIALIZED")
            }
            NoRepository => print_log!(Level::ERROR, "Repository {repo_name}: NOT FOUND"),
            Locked => print_log!(Level::ERROR, "Repository {repo_name}: LOCKED"),
            InvalidKey => print_log!(Level::ERROR, "Repository {repo_name}: INVALID KEY."),
        }
    }

    Ok(())
}

fn about() -> Result<()> {
    let about_html = include_bytes!(env!("ABOUT_HTML_PATH"));
    let about_path = std::env::temp_dir()
        .canonicalize()?
        .join("about-aresticrat.html");
    std::fs::write(&about_path, about_html)?;
    open::that(&about_path)?;

    println!(
        concat!(
            "The information should automatically appear in your default web",
            "browser. If it doesn't, you can find it here: {}"
        ),
        about_path.display()
    );

    Ok(())
}

fn load_env_files(additional_files: &[PathBuf]) -> Result<()> {
    load_env_file(".env")?;
    load_env_file(".aresticrat.env")?;
    load_env_file("aresticrat.env")?;
    for env_file in additional_files {
        load_env_file(env_file)?;
    }
    Ok(())
}

fn load_env_file<P>(file: P) -> Result<()>
where
    P: AsRef<Path>,
{
    match dotenvy::from_path_override(file) {
        Ok(_) => Ok(()),
        Err(dotenvy::Error::Io(e)) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

fn setup_logger() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(std::io::stdout().is_terminal())
                .with_filter(
                    EnvFilter::builder()
                        .with_default_directive(LevelFilter::OFF.into())
                        .with_env_var("ARESTICRAT_LOG")
                        .from_env_lossy(),
                ),
        )
        .init();
}

fn get_tag(location_name: &str) -> String {
    format!("_aresticrat_{location_name}")
}

fn get_backup_options(location_name: &str, config: &Config) -> BackupOptions {
    config
        .locations()
        .get(location_name)
        .and_then(|l| l.options().backup())
        .or_else(|| config.options().backup())
        .cloned()
        .unwrap_or_default()
}

fn get_forget_options(location_name: &str, config: &Config) -> ForgetOptions {
    config
        .locations()
        .get(location_name)
        .and_then(|l| l.options().forget())
        .or_else(|| config.options().forget())
        .cloned()
        .unwrap_or_default()
}

macro_rules! print_log {
    ($lvl:expr, $($arg:tt)*) => {
        {
            match ($lvl, crate::verbosity()) {
                (tracing::Level::TRACE, v) if v > 4 => println!($($arg)*),
                (tracing::Level::DEBUG, v) if v > 3 => println!($($arg)*),
                (tracing::Level::INFO, v) if v > 2 => println!($($arg)*),
                (tracing::Level::WARN, v) if v > 1 => eprintln!($($arg)*),
                (tracing::Level::ERROR, v) if v > 0 => eprintln!($($arg)*),
                _ => {},
            };
            tracing::event!($lvl, $($arg)*)
        }
    };
}

pub(crate) use print_log;
