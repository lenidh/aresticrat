use crate::config::BackupOptions;
use crate::config::ForgetOptions;
use crate::config::Name;
use crate::run;
use crate::ENV_PREFIX;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::ExitStatus;
use thiserror::Error;

const BACKUP_READ_ERROR_CODE: i32 = 3;

pub struct Api {
    exe: String,
    verbosity: usize,
}

impl Api {
    pub fn new(exe: String, verbosity: usize) -> Self {
        Api { exe, verbosity }
    }

    pub fn backup<I, P, S>(
        &self,
        repo: &Repository,
        paths: I,
        tag: S,
        options: &BackupOptions,
        dry_run: bool,
    ) -> Result<()>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
        S: AsRef<str>,
    {
        let mut cmd = self.command(repo);
        cmd.arg("backup");
        if dry_run {
            cmd.arg("--dry-run");
        }
        for pattern in options.exclude() {
            cmd.arg("--exclude");
            cmd.arg(pattern);
        }
        for pattern in options.iexclude() {
            cmd.arg("--iexclude");
            cmd.arg(pattern);
        }
        for file in options.exclude_file() {
            cmd.arg("--exclude-file");
            cmd.arg(file);
        }
        for file in options.iexclude_file() {
            cmd.arg("--iexclude-file");
            cmd.arg(file);
        }
        for file in options.exclude_if_present() {
            cmd.arg("--exclude-if-present");
            cmd.arg(file);
        }
        if let Some(size) = options.exclude_larger_than() {
            cmd.arg("--exclude-larger-than");
            cmd.arg(size);
        }
        if options.exclude_caches() {
            cmd.arg("--exclude-caches");
        }
        if options.ignore_ctime() {
            cmd.arg("--ignore-ctime");
        }
        if options.ignore_inode() {
            cmd.arg("--ignore-inode");
        }
        if options.no_scan() {
            cmd.arg("--no-scan");
        }
        if options.one_file_system() {
            cmd.arg("--one-file-system");
        }
        if options.skip_if_unchanged() {
            cmd.arg("--skip-if-unchanged");
        }
        if options.use_fs_snapshot() {
            cmd.arg("--use-fs-snapshot");
        }
        if options.with_atime() {
            cmd.arg("--with-atime");
        }
        cmd.arg("--tag");
        cmd.arg(tag.as_ref());
        for path in paths.into_iter().collect::<Vec<_>>() {
            cmd.arg(OsStr::new(path.as_ref()));
        }
        match run(&mut cmd) {
            Err(Error::CmdFailure { status, .. }) if is_backup_read_error(status) => Ok(()),
            result => result,
        }
    }

    pub fn forget<S>(
        &self,
        repo: &Repository,
        tag: S,
        options: &ForgetOptions,
        dry_run: bool,
    ) -> Result<()>
    where
        S: AsRef<str>,
    {
        let mut cmd = self.command(repo);
        cmd.arg("forget");
        if dry_run {
            cmd.arg("--dry-run");
        }
        if options.prune() {
            cmd.arg("--prune");
        }
        if let Some(n) = options.keep_last() {
            cmd.arg("--keep-last");
            cmd.arg(format!("{n}"));
        }
        if let Some(n) = options.keep_hourly() {
            cmd.arg("--keep-hourly");
            cmd.arg(format!("{n}"));
        }
        if let Some(n) = options.keep_daily() {
            cmd.arg("--keep-daily");
            cmd.arg(format!("{n}"));
        }
        if let Some(n) = options.keep_weekly() {
            cmd.arg("--keep-weekly");
            cmd.arg(format!("{n}"));
        }
        if let Some(n) = options.keep_monthly() {
            cmd.arg("--keep-monthly");
            cmd.arg(format!("{n}"));
        }
        if let Some(n) = options.keep_yearly() {
            cmd.arg("--keep-yearly");
            cmd.arg(format!("{n}"));
        }
        if let Some(duration) = options.keep_within() {
            cmd.arg("--keep-within");
            cmd.arg(duration);
        }
        if let Some(duration) = options.keep_within_hourly() {
            cmd.arg("--keep-within-hourly");
            cmd.arg(duration);
        }
        if let Some(duration) = options.keep_within_daily() {
            cmd.arg("--keep-within-daily");
            cmd.arg(duration);
        }
        if let Some(duration) = options.keep_within_weekly() {
            cmd.arg("--keep-within-weekly");
            cmd.arg(duration);
        }
        if let Some(duration) = options.keep_within_monthly() {
            cmd.arg("--keep-within-monthly");
            cmd.arg(duration);
        }
        if let Some(duration) = options.keep_within_yearly() {
            cmd.arg("--keep-within-yearly");
            cmd.arg(duration);
        }
        for tag in options.keep_tag() {
            cmd.arg("--keep-tag");
            cmd.arg(tag);
        }
        cmd.arg("--tag");
        cmd.arg(tag.as_ref());
        run(&mut cmd)
    }

    pub fn status(&self, repo: &Repository) -> Result<RepoStatus> {
        let mut cmd = self.command(repo);
        cmd.arg("cat");
        cmd.arg("config");

        let status = run::run(&mut cmd, true)?;
        match status.code() {
            Some(0) => Ok(RepoStatus::Ok),
            Some(10) => Ok(RepoStatus::NoRepository),
            Some(11) => Ok(RepoStatus::Locked),
            Some(12) => Ok(RepoStatus::InvalidKey),
            _ => Err(Error::CmdFailure {
                program: cmd.get_program().to_owned(),
                status,
            }),
        }
    }

    pub fn init(&self, repo: &Repository) -> Result<()> {
        let mut cmd = self.command(repo);
        cmd.arg("init");
        run(&mut cmd)
    }

    pub fn exec<I, S>(&self, repo: &Repository, args: I) -> Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut cmd = self.command(repo);
        args.into_iter().for_each(|arg| {
            cmd.arg(arg.as_ref());
        });
        run(&mut cmd)
    }

    fn command(&self, repo: &Repository) -> Command {
        let env_prefix = format!("{ENV_PREFIX}_R_");
        let repo_env_prefix = format!("{}{}_", env_prefix, repo.name.as_str().to_uppercase());

        let vars = std::env::vars()
            .map(|(mut k, v)| {
                remove_prefix(&mut k, &repo_env_prefix);
                (k, v)
            })
            .filter(|(k, _)| !k.starts_with(&env_prefix));

        let mut cmd = std::process::Command::new(&self.exe);
        cmd.env_clear();
        cmd.envs(vars);
        cmd.envs(&repo.environment);
        if !repo.path.is_empty() {
            cmd.env("RESTIC_REPOSITORY", &repo.path);
        }
        if !repo.password.is_empty() {
            cmd.env("RESTIC_PASSWORD", &repo.password);
        }
        if let Some(path) = &repo.password_file {
            cmd.env("RESTIC_PASSWORD_FILE", path);
        }
        if !repo.password_command.is_empty() {
            cmd.env("RESTIC_PASSWORD_COMMAND", &repo.password_command);
        }
        cmd.env("RESTIC_PROGRESS_FPS", "0.016666");
        if self.verbosity > 0 {
            cmd.arg(format!("--verbose={}", self.verbosity));
        }
        if !repo.retry_lock.is_empty() {
            cmd.arg("--retry-lock");
            cmd.arg(&repo.retry_lock);
        }
        for option in &repo.options {
            cmd.arg("--option");
            cmd.arg(option);
        }
        cmd
    }
}

fn run(cmd: &mut Command) -> Result<()> {
    let status = run::run(cmd, false)?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::CmdFailure {
            program: cmd.get_program().to_os_string(),
            status,
        })
    }
}

fn is_backup_read_error(status: ExitStatus) -> bool {
    status.code() == Some(BACKUP_READ_ERROR_CODE)
}

fn remove_prefix(str: &mut String, prefix: &str) -> bool {
    if str.starts_with(prefix) {
        str.replace_range(..prefix.len(), "");
        return true;
    }
    false
}

pub enum RepoStatus {
    Ok,
    NoRepository,
    Locked,
    InvalidKey,
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Execution of {program:?} failed ({status}).")]
    CmdFailure {
        program: OsString,
        status: ExitStatus,
    },
    #[error("{0}")]
    IoError(#[from] std::io::Error),
}

pub struct Repository {
    pub name: Name,
    pub path: String,
    pub password: String,
    pub password_file: Option<PathBuf>,
    pub password_command: String,
    pub retry_lock: String,
    pub options: Vec<String>,
    pub environment: HashMap<String, String>,
}
