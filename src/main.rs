use anyhow::Result;
use clap::Parser as ClapParser;
use cli::{Args, BackupArgs, Command, ExecArgs, ForgetArgs, VerifyArgs};
use config::{BackupOptions, CommandSeq, Config, ForgetOptions, LocationRepo, Name};
use std::{
    collections::{HashMap, HashSet},
    env,
    fs::File,
    io::{BufRead, ErrorKind, IsTerminal},
    path::{Path, PathBuf},
    sync::OnceLock,
};
use tracing::{level_filters::LevelFilter, Level};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

use crate::{config::Environment, restic_api::Repository};

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

fn restic_verbosity() -> usize {
    (verbosity() - DEFAULT_VERBOSITY).max(0)
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
    let api = restic_api::Api::new(config.executable().to_string(), restic_verbosity());

    let m = resolve_selection(args.selected_locations(), config)?;

    for (location_name, repo_names) in &m {
        let location = &config.locations()[location_name];
        let _span = tracing::info_span!("Backup", location = location_name.as_str()).entered();

        let tag = get_tag(location_name);
        let backup_opts = get_backup_options(location_name, config);

        print_log!(Level::INFO, "Backup location {location_name} ...");

        let if_status = run_hooks("IF", backup_opts.hooks().r#if())?;
        if !if_status.success() {
            print_log!(Level::INFO, "IF hook failed. Skip location.");
            continue;
        }

        for repo_name in repo_names {
            if let Some(repo) = resolve_repository(repo_name, config) {
                print_log!(Level::INFO, "Backup to repository {repo_name} ...");
                api.backup(&repo, location.paths(), &tag, &backup_opts, args.dry_run())?;
                print_log!(Level::INFO, "Backup to repository {repo_name} done.");
            } else {
                print_log!(
                    Level::WARN,
                    "Location {location_name} refers to an undefined repository {repo_name}."
                )
            }
        }

        if !args.dry_run() && backup_opts.forget() {
            forget_location(&api, location_name, repo_names, config, args.dry_run())?;
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
    let api = restic_api::Api::new(config.executable().to_string(), restic_verbosity());
    let mut repo_names = args.repos().to_vec();
    if (*repo_names).as_ref().is_empty() {
        repo_names = config.repos().keys().cloned().collect();
    }

    for repo_name in (*repo_names).as_ref() {
        if let Some(repo) = resolve_repository(repo_name, config) {
            api.exec(&repo, args.args())?;
        } else {
            print_log!(
                Level::WARN,
                "Argument refers to an undefined repository {repo_name}."
            )
        }
    }

    Ok(())
}

fn forget(config: &Config, args: &ForgetArgs) -> Result<()> {
    let api = restic_api::Api::new(config.executable().to_string(), restic_verbosity());

    let m = resolve_selection(args.selected_locations(), config)?;

    for (location_name, repo_names) in &m {
        forget_location(&api, location_name, repo_names, config, args.dry_run())?;
    }

    Ok(())
}

fn forget_location(
    api: &restic_api::Api,
    location_name: &Name,
    repo_names: &HashSet<Name>,
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

    for repo_name in repo_names {
        if let Some(repo) = resolve_repository(repo_name, config) {
            print_log!(Level::INFO, "Forget from repository {repo_name} ...");
            api.forget(&repo, &tag, &forget_opts, dry_run)?;
            print_log!(Level::INFO, "Forget from repository {repo_name} done.");
        } else {
            print_log!(
                Level::WARN,
                "Location {location_name} refers to an undefined repository {repo_name}."
            )
        }
    }

    Ok(())
}

fn verify(config: &Config, args: &VerifyArgs) -> Result<()> {
    let api = restic_api::Api::new(config.executable().to_string(), restic_verbosity());

    for repo_name in config.repos().keys() {
        if let Some(repo) = resolve_repository(repo_name, config) {
            let status = api.status(&repo)?;

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
                    api.init(&repo)?;
                    print_log!(Level::INFO, "Repository {repo_name}: INITIALIZED")
                }
                NoRepository => print_log!(Level::ERROR, "Repository {repo_name}: NOT FOUND"),
                Locked => print_log!(Level::ERROR, "Repository {repo_name}: LOCKED"),
                InvalidKey => print_log!(Level::ERROR, "Repository {repo_name}: INVALID KEY."),
            }
        }
        // No else required here, because we resolve the repository from the
        // definied repository configurations.
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

fn get_tag(location_name: &Name) -> String {
    format!("_aresticrat_{location_name}")
}

fn get_backup_options(location_name: &Name, config: &Config) -> BackupOptions {
    config
        .locations()
        .get(location_name)
        .and_then(|l| l.options().backup())
        .or_else(|| config.options().backup())
        .cloned()
        .unwrap_or_default()
}

fn get_forget_options(location_name: &Name, config: &Config) -> ForgetOptions {
    config
        .locations()
        .get(location_name)
        .and_then(|l| l.options().forget())
        .or_else(|| config.options().forget())
        .cloned()
        .unwrap_or_default()
}

fn get_repo_env_vars(repo_name: &Name, config: &Config) -> HashMap<String, String> {
    let mut vars = HashMap::new();
    append_env(config.environment(), &mut vars);
    if let Some(repo) = config.repos().get(repo_name) {
        append_env(repo.environment(), &mut vars);
    }
    vars
}

fn append_env(env: &Environment, vars: &mut HashMap<String, String>) {
    for path in env.env_files() {
        read_env_file_to(path, vars);
    }
    for (k, v) in env.vars() {
        vars.insert(k.clone(), v.clone());
    }
}

fn read_env_file_to<P: AsRef<Path>>(path: P, vars: &mut HashMap<String, String>) {
    let file = match File::open(&path) {
        Ok(file) => file,
        Err(err) => {
            print_log!(
                Level::WARN,
                "Failed to read environment file {}:\n{}",
                path.as_ref().to_string_lossy(),
                err
            );
            return;
        }
    };
    for (i, line) in std::io::BufReader::new(file).lines().enumerate() {
        if let Ok(Some((k, v))) = line.map(parse_env_var) {
            vars.insert(k, v);
        } else {
            print_log!(
                Level::WARN,
                "Invalid environment variable in {} at line {}.",
                path.as_ref().to_string_lossy(),
                i
            );
        }
    }
}

fn parse_env_var<S: AsRef<str>>(str: S) -> Option<(String, String)> {
    str.as_ref()
        .split_once('=')
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
}

fn resolve_selection(
    selection: &[LocationRepo],
    config: &Config,
) -> Result<HashMap<Name, HashSet<Name>>> {
    let mut m: HashMap<Name, HashSet<Name>> = HashMap::new();
    if selection.is_empty() {
        for (location_name, location) in config.locations() {
            m.insert(
                location_name.clone(),
                location.repos().iter().cloned().collect(),
            );
        }
    } else {
        for t in selection {
            let assigned_repos = config.locations()[t.location()].repos();
            let set = m.entry(t.location().clone()).or_default();
            if let Some(r) = t.repo() {
                if assigned_repos.contains(r) {
                    set.insert(r.clone());
                } else {
                    print_log!(Level::WARN, "Combination {r} is invalid.")
                }
            } else {
                let a = config.locations()[t.location()].repos();
                a.iter().for_each(|x| {
                    set.insert(x.clone());
                });
            }
        }
    }
    m.retain(|_k, v| !v.is_empty());
    Ok(m)
}

/// Turns the repository configuration into the format that ist expected by the
/// API.
fn resolve_repository(repo_name: &Name, config: &Config) -> Option<Repository> {
    if let Some(repo_config) = config.repos().get(repo_name) {
        let env_vars = get_repo_env_vars(repo_name, config);
        Some(Repository {
            name: repo_name.clone(),
            path: repo_config.path().to_string(),
            password: repo_config.password().to_string(),
            password_file: repo_config.password_file().map(Path::to_path_buf),
            password_command: repo_config.password_command().to_string(),
            retry_lock: repo_config.retry_lock().to_string(),
            options: repo_config.options().clone(),
            environment: env_vars,
        })
    } else {
        None
    }
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
