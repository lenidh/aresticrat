use std::path::{Path, PathBuf};

use clap::{Args as ClapArgs, Parser as ClapParser, Subcommand as ClapSubcommand};

#[derive(ClapParser, Debug)]
#[command(version, about)]
pub struct Args {
    /// Set configuration file.
    #[arg(short, long = "config", default_value = "aresticrat.toml")]
    config_file: PathBuf,
    /// Set working directory.
    #[arg(long = "wd")]
    working_dir: Option<PathBuf>,
    /// Do not output to stdout/stderr (doesn't affect logging).
    #[arg(short, long)]
    quiet: bool,
    /// Print more information (doesn't affect logging) (repeatable).
    ///
    /// Specify multiple times to increase verbosity step by step:
    /// off (default with -q) -> error -> warn -> info (default without -q) ->
    /// debug -> trace.
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
    /// Additionally read environment variables from the specified file
    /// (repeatable).
    ///
    /// Files are processed in the specified order, so values in later files
    /// overwrite those in earlier files.
    #[arg(long = "env", value_name = "ENV_FILE")]
    env_files: Vec<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

impl Args {
    pub fn config_file(&self) -> &Path {
        &self.config_file
    }
    pub fn quiet(&self) -> bool {
        self.quiet
    }
    pub fn verbose(&self) -> u8 {
        self.verbose
    }
    pub fn working_dir(&self) -> Option<&Path> {
        self.working_dir.as_deref()
    }
    pub fn env_files(&self) -> &[PathBuf] {
        &self.env_files
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
    /// Show copyright and license information.
    About,
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
    /// Only run the command for this repository (repeatable).
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
    /// Only remove snapshots of this location (repeatable).
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
