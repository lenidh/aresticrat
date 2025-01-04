use std::ffi::OsStr;
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;
use std::process::ExitStatus;

use thiserror::Error;
use tracing::debug;

use crate::config::BackupOptions;
use crate::config::ForgetOptions;
use crate::config::Repo;
use crate::ENV_PREFIX;

pub struct Api {
    exe: String,
}

impl Api {
    pub fn new(exe: String) -> Self {
        Api { exe }
    }

    pub fn backup<I, P, S>(
        &self,
        repo_name: &str,
        repo: &Repo,
        paths: I,
        tag: S,
        backup_options: &BackupOptions,
        dry_run: bool,
    ) -> Result<String>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
        S: AsRef<str>,
    {
        let mut cmd = self.command(repo_name, repo.path(), repo.key());
        cmd.arg("backup");
        if dry_run {
            cmd.arg("--dry-run");
        }
        for pattern in backup_options.exclude() {
            cmd.arg("--exclude");
            cmd.arg(pattern);
        }
        for pattern in backup_options.iexclude() {
            cmd.arg("--iexclude");
            cmd.arg(pattern);
        }
        for file in backup_options.exclude_file() {
            cmd.arg("--exclude-file");
            cmd.arg(file);
        }
        for file in backup_options.iexclude_file() {
            cmd.arg("--iexclude-file");
            cmd.arg(file);
        }
        if backup_options.exclude_caches() {
            cmd.arg("--exclude-caches");
        }
        cmd.arg("--tag");
        cmd.arg(tag.as_ref());
        for path in paths.into_iter().collect::<Vec<_>>() {
            cmd.arg(OsStr::new(path.as_ref()));
        }
        exec(&mut cmd)
    }

    pub fn forget<S>(
        &self,
        repo_name: &str,
        repo: &Repo,
        tag: S,
        options: &ForgetOptions,
        dry_run: bool,
    ) -> Result<String>
    where
        S: AsRef<str>,
    {
        let mut forget_cmd = self.command(repo_name, repo.path(), repo.key());
        forget_cmd.arg("forget");
        if dry_run {
            forget_cmd.arg("--dry-run");
        }
        if options.prune() {
            forget_cmd.arg("--prune");
        }
        if let Some(n) = options.keep_last() {
            forget_cmd.arg("--keep-last");
            forget_cmd.arg(format!("{n}"));
        }
        if let Some(n) = options.keep_hourly() {
            forget_cmd.arg("--keep-hourly");
            forget_cmd.arg(format!("{n}"));
        }
        if let Some(n) = options.keep_daily() {
            forget_cmd.arg("--keep-daily");
            forget_cmd.arg(format!("{n}"));
        }
        if let Some(n) = options.keep_weekly() {
            forget_cmd.arg("--keep-weekly");
            forget_cmd.arg(format!("{n}"));
        }
        if let Some(n) = options.keep_monthly() {
            forget_cmd.arg("--keep-monthly");
            forget_cmd.arg(format!("{n}"));
        }
        if let Some(n) = options.keep_yearly() {
            forget_cmd.arg("--keep-yearly");
            forget_cmd.arg(format!("{n}"));
        }
        forget_cmd.arg("--tag");
        forget_cmd.arg(tag.as_ref());
        exec(&mut forget_cmd)
    }

    pub fn status(&self, repo_name: &str, repo_path: &str, key: &str) -> Result<RepoStatus> {
        let mut cmd = self.command(repo_name, repo_path, key);
        cmd.arg("cat");
        cmd.arg("config");

        match exec(&mut cmd) {
            Ok(_) => Ok(RepoStatus::Ok),
            Err(Error::CmdFailure {
                program,
                status,
                stdout,
                stderr,
            }) => match status.code() {
                Some(10) => Ok(RepoStatus::NoRepository),
                Some(11) => Ok(RepoStatus::Locked),
                Some(12) => Ok(RepoStatus::InvalidKey),
                _ => Err(Error::CmdFailure {
                    program,
                    status,
                    stdout,
                    stderr,
                }),
            },
            Err(e) => Err(e),
        }
    }

    pub fn init(&self, repo_name: &str, repo_path: &str, key: &str) -> Result<String> {
        let mut cmd = self.command(repo_name, repo_path, key);
        cmd.arg("init");
        exec(&mut cmd)
    }

    pub fn exec<I, S>(&self, repo_name: &str, repo: &Repo, args: I) -> Result<String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut cmd = self.command(repo_name, repo.path(), repo.key());
        args.into_iter().for_each(|arg| {
            cmd.arg(arg.as_ref());
        });
        exec(&mut cmd)
    }

    fn command(&self, repo_name: &str, path: &str, key: &str) -> Command {
        let repo_env_prefix = format!("{}{}_", ENV_PREFIX, repo_name.to_uppercase());

        let vars = std::env::vars()
            .map(|(mut k, v)| {
                remove_prefix(&mut k, &repo_env_prefix);
                (k, v)
            })
            .filter(|(k, _)| !k.starts_with(ENV_PREFIX));

        let mut cmd = std::process::Command::new(&self.exe);
        cmd.env_clear();
        cmd.envs(vars);
        if !path.is_empty() {
            cmd.env("RESTIC_REPOSITORY", path);
        }
        if !key.is_empty() {
            cmd.env("RESTIC_PASSWORD", key);
        }
        cmd
    }
}

fn exec(cmd: &mut Command) -> Result<String> {
    debug!("Execute command: {cmd:?}");
    let result = cmd.output()?;
    if result.status.success() {
        let out = String::from_utf8(result.stdout)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
        Ok(out)
    } else {
        Err(Error::CmdFailure {
            program: cmd.get_program().to_os_string(),
            status: result.status,
            stdout: String::from_utf8_lossy(&result.stdout).to_string(),
            stderr: String::from_utf8_lossy(&result.stderr).to_string(),
        })
    }
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
    #[error("Execution of {program:?} failed ({status}).\n\n{stderr}")]
    CmdFailure {
        program: OsString,
        status: ExitStatus,
        stdout: String,
        stderr: String,
    },
    #[error("{0}")]
    IoError(#[from] std::io::Error),
}
