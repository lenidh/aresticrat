use std::path::{Path, PathBuf};

use clap::{Args as ClapArgs, Parser as ClapParser, Subcommand as ClapSubcommand};

#[derive(ClapParser, Debug)]
#[command(version, about)]
pub struct Args {
    /// Set configuration file
    #[arg(short, long = "config", default_value = "aresticrat.toml")]
    config_file: PathBuf,
    /// Set working directory
    #[arg(long = "wd")]
    working_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

impl Args {
    pub fn config_file(&self) -> &Path {
        &self.config_file
    }
    pub fn working_dir(&self) -> Option<&Path> {
        self.working_dir.as_deref()
    }
    pub fn command(&self) -> &Command {
        &self.command
    }
}

#[derive(ClapSubcommand, Debug)]
pub enum Command {
    /// Create a new backup of configured locations.
    Backup(BackupArgs),
    /// Run a native restic command for a configured repository.
    Exec(ExecArgs),
    /// Remove snapshots of configured locations from their repositories.
    Forget(ForgetArgs),
    /// Validate the configuration file and test access to configured
    /// repositories.
    Verify(VerifyArgs),
}

#[derive(ClapArgs, Debug)]
pub struct BackupArgs {
    /// Do not upload or write any data, just show what would be done.
    #[arg(long)]
    dry_run: bool,
}

impl BackupArgs {
    pub fn dry_run(&self) -> bool {
        self.dry_run
    }
}

#[derive(ClapArgs, Debug)]
pub struct ExecArgs {
    /// Only run the command for this repository (can be specified multiple times).
    #[arg(short, long = "repo", value_name = "REPO")]
    repos: Vec<String>,
    /// One or more arguments passed to the restic executable.
    #[arg(required = true, raw = true, value_name = "ARG")]
    args: Vec<String>,
}

impl ExecArgs {
    pub fn repos(&self) -> &[String] {
        &self.repos
    }
    pub fn args(&self) -> &[String] {
        &self.args
    }
}

#[derive(ClapArgs, Debug)]
pub struct ForgetArgs {
    /// Only remove snapshots of this location (can be specified multiple times).
    #[arg(short, long)]
    locations: Vec<String>,
    /// Do not delete any data, just show what would be done.
    #[arg(long)]
    dry_run: bool,
}

impl ForgetArgs {
    pub fn locations(&self) -> &Vec<String> {
        &self.locations
    }
    pub fn dry_run(&self) -> bool {
        self.dry_run
    }
}

#[derive(ClapArgs, Debug)]
pub struct VerifyArgs {
    /// Create missing repositories.
    #[arg(long)]
    init: bool,
}

impl VerifyArgs {
    pub fn init(&self) -> bool {
        self.init
    }
}
