use anyhow::{Context, Result};
use cli::{Args, BackupArgs, Command, ExecArgs, ForgetArgs, VerifyArgs};
use std::{
    env, error::Error, io::ErrorKind, path::{Path, PathBuf}
};

use config::{BackupOptions, Config, ForgetOptions};

use clap::Parser as ClapParser;
use tracing::{debug, error, info, level_filters::LevelFilter, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

mod cli;
mod config;
mod restic_api;

const ENV_PREFIX: &str = "ARESTICRAT_";

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    if let Some(wd) = args.working_dir() {
        env::set_current_dir(wd)?;
    }

    load_env_files(args.env_files())?;

    setup_logger();

    let config = config::Config::new(args.config_file())?;

    match args.command() {
        Command::Backup(backup_args) => backup(&config, backup_args)?,
        Command::Exec(exec_args) => exec(&config, exec_args)?,
        Command::Forget(forget_args) => forget(&config, forget_args)?,
        Command::Verify(verify_args) => verify(&config, verify_args)?,
    }

    Ok(())
}

fn backup(config: &Config, args: &BackupArgs) -> Result<()> {
    let api = restic_api::Api::new(config.executable().to_string());

    for (location_name, location) in config.locations() {
        let _span = tracing::info_span!("Backup", location = location_name).entered();

        let tag = get_tag(location_name);
        let backup_opts = get_backup_options(location_name, config);
        let forget_opts = get_forget_options(location_name, config);

        for repo_name in location.repos() {
            if let Some(repo) = config.repos().get(repo_name) {
                info!("Backup ...");
                let output = api.backup(
                    repo_name,
                    repo,
                    location.paths(),
                    &tag,
                    &backup_opts,
                    args.dry_run(),
                )?;
                info!("Backup done:\n\n{output}");

                if !args.dry_run() && backup_opts.forget() {
                    info!("Forget ...");
                    let output = api.forget(repo_name, repo, &tag, &forget_opts, false)?;
                    info!("Forget done:\n\n{output}");
                }
            } else {
                warn!("Location {location_name} refers to an undefined repository {repo_name}.")
            }
        }
    }
    Ok(())
}

fn exec(config: &Config, args: &ExecArgs) -> Result<()> {
    let api = restic_api::Api::new(config.executable().to_string());
    let mut repo_names = args.repos().to_vec();
    if (*repo_names).as_ref().is_empty() {
        repo_names = config.repos().keys().cloned().collect();
    }

    for repo_name in (*repo_names).as_ref() {
        let output = api.exec(
            repo_name,
            config.repos().get(repo_name).unwrap(),
            args.args(),
        )?;
        info!("\n{output}");
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
        let tag = get_tag(location_name);
        let forget_opts = get_forget_options(location_name, config);

        for r in location.repos() {
            if let Some(repo) = config.repos().get(r) {
                let output = api.forget(r, repo, &tag, &forget_opts, args.dry_run())?;
                info!("{output}")
            }
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
                info!("Repository {repo_name}: OK")
            }
            NoRepository if args.init() => {
                debug!("Repository {repo_name} not found. Initialize ...");
                let output = api.init(repo_name, repo.path(), repo.key())?;
                info!("Repository {repo_name}: INITIALIZED\n\n{output}")
            }
            NoRepository => error!("Repository {repo_name}: NOT FOUND"),
            Locked => error!("Repository {repo_name}: LOCKED"),
            InvalidKey => error!("Repository {repo_name}: INVALID KEY."),
        }
    }

    Ok(())
}

fn load_env_files(additional_files: &[PathBuf]) -> Result<(), Box<dyn Error>> {
    load_env_file(".env")?;
    load_env_file(".aresticrat.env")?;
    load_env_file("aresticrat.env")?;
    for env_file in additional_files {
        load_env_file(env_file)?;
    }
    Ok(())
}

fn load_env_file<P>(file: P) -> Result<(), Box<dyn Error>>
where
    P: AsRef<Path>,
{
    match dotenvy::from_path_override(file) {
        Ok(_) => Ok(()),
        Err(dotenvy::Error::Io(e)) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(e) => Err(Box::new(e)),
    }
}

fn setup_logger() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer().with_filter(
                EnvFilter::builder()
                    .with_default_directive(LevelFilter::INFO.into())
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
