use serde::Deserialize;
use std::collections::HashMap;
use std::fmt::Display;
use std::ops::Deref;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;
use thiserror::Error;

use crate::ENV_PREFIX;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_executable")]
    executable: String,
    #[serde(default)]
    options: Options,
    repos: HashMap<Name, Repo>,
    locations: HashMap<Name, Location>,
}

fn default_executable() -> String {
    "restic".to_string()
}

impl Config {
    pub fn new(config_path: &Path) -> Result<Self, config::ConfigError> {
        let s = config::Config::builder()
            .add_source(config::File::with_name(
                config_path.to_string_lossy().deref(),
            ))
            .add_source(config::Environment::with_prefix(ENV_PREFIX).separator("_"))
            .build()?;
        s.try_deserialize()
    }

    pub fn executable(&self) -> &str {
        &self.executable
    }
    pub fn options(&self) -> &Options {
        &self.options
    }
    pub fn repos(&self) -> &HashMap<Name, Repo> {
        &self.repos
    }
    pub fn locations(&self) -> &HashMap<Name, Location> {
        &self.locations
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct Options {
    backup: Option<BackupOptions>,
    forget: Option<ForgetOptions>,
}

impl Options {
    pub fn backup(&self) -> Option<&BackupOptions> {
        self.backup.as_ref()
    }
    pub fn forget(&self) -> Option<&ForgetOptions> {
        self.forget.as_ref()
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct BackupOptions {
    #[serde(default)]
    forget: bool,
    #[serde(default)]
    exclude: Vec<String>,
    #[serde(default)]
    iexclude: Vec<String>,
    #[serde(default)]
    exclude_file: Vec<PathBuf>,
    #[serde(default)]
    iexclude_file: Vec<PathBuf>,
    #[serde(default)]
    exclude_caches: bool,
    #[serde(default)]
    exclude_if_present: Vec<String>,
    #[serde(default)]
    exclude_larger_than: Option<String>,
    #[serde(default)]
    ignore_ctime: bool,
    #[serde(default)]
    ignore_inode: bool,
    #[serde(default)]
    no_scan: bool,
    #[serde(default)]
    one_file_system: bool,
    #[serde(default)]
    skip_if_unchanged: bool,
    #[serde(default)]
    use_fs_snapshot: bool,
    #[serde(default)]
    with_atime: bool,
    #[serde(default)]
    hooks: HookOptions,
}

impl BackupOptions {
    pub fn forget(&self) -> bool {
        self.forget
    }
    pub fn exclude(&self) -> &Vec<String> {
        &self.exclude
    }
    pub fn iexclude(&self) -> &Vec<String> {
        &self.iexclude
    }
    pub fn exclude_file(&self) -> &Vec<PathBuf> {
        &self.exclude_file
    }
    pub fn iexclude_file(&self) -> &Vec<PathBuf> {
        &self.iexclude_file
    }
    pub fn exclude_caches(&self) -> bool {
        self.exclude_caches
    }
    pub fn exclude_if_present(&self) -> &Vec<String> {
        &self.exclude_if_present
    }
    pub fn exclude_larger_than(&self) -> Option<&str> {
        self.exclude_larger_than.as_deref()
    }
    pub fn ignore_ctime(&self) -> bool {
        self.ignore_ctime
    }
    pub fn ignore_inode(&self) -> bool {
        self.ignore_inode
    }
    pub fn no_scan(&self) -> bool {
        self.no_scan
    }
    pub fn one_file_system(&self) -> bool {
        self.one_file_system
    }
    pub fn skip_if_unchanged(&self) -> bool {
        self.skip_if_unchanged
    }
    pub fn use_fs_snapshot(&self) -> bool {
        self.use_fs_snapshot
    }
    pub fn with_atime(&self) -> bool {
        self.with_atime
    }
    pub fn hooks(&self) -> &HookOptions {
        &self.hooks
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct HookOptions {
    r#if: Vec<CommandSeq>,
}

impl HookOptions {
    pub fn r#if(&self) -> &[CommandSeq] {
        &self.r#if
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ForgetOptions {
    #[serde(default)]
    prune: bool,
    keep_last: Option<u32>,
    keep_hourly: Option<u32>,
    keep_daily: Option<u32>,
    keep_weekly: Option<u32>,
    keep_monthly: Option<u32>,
    keep_yearly: Option<u32>,
    keep_within: Option<String>,
    keep_within_hourly: Option<String>,
    keep_within_daily: Option<String>,
    keep_within_weekly: Option<String>,
    keep_within_monthly: Option<String>,
    keep_within_yearly: Option<String>,
    #[serde(default)]
    keep_tag: Vec<String>,
    #[serde(default)]
    hooks: HookOptions,
}

impl ForgetOptions {
    pub fn prune(&self) -> bool {
        self.prune
    }
    pub fn keep_last(&self) -> Option<u32> {
        self.keep_last
    }
    pub fn keep_hourly(&self) -> Option<u32> {
        self.keep_hourly
    }
    pub fn keep_daily(&self) -> Option<u32> {
        self.keep_daily
    }
    pub fn keep_weekly(&self) -> Option<u32> {
        self.keep_weekly
    }
    pub fn keep_monthly(&self) -> Option<u32> {
        self.keep_monthly
    }
    pub fn keep_yearly(&self) -> Option<u32> {
        self.keep_yearly
    }
    pub fn keep_within(&self) -> Option<&str> {
        self.keep_within.as_deref()
    }
    pub fn keep_within_hourly(&self) -> Option<&str> {
        self.keep_within_hourly.as_deref()
    }
    pub fn keep_within_daily(&self) -> Option<&str> {
        self.keep_within_daily.as_deref()
    }
    pub fn keep_within_weekly(&self) -> Option<&str> {
        self.keep_within_weekly.as_deref()
    }
    pub fn keep_within_monthly(&self) -> Option<&str> {
        self.keep_within_monthly.as_deref()
    }
    pub fn keep_within_yearly(&self) -> Option<&str> {
        self.keep_within_yearly.as_deref()
    }
    pub fn keep_tag(&self) -> &Vec<String> {
        &self.keep_tag
    }
    pub fn hooks(&self) -> &HookOptions {
        &self.hooks
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Repo {
    #[serde(default)]
    path: String,
    #[serde(default)]
    key: String,
    #[serde(default)]
    retry_lock: String,
    #[serde(default)]
    options: Vec<String>,
}

impl Repo {
    pub fn path(&self) -> &str {
        &self.path
    }
    pub fn key(&self) -> &str {
        &self.key
    }
    pub fn retry_lock(&self) -> &str {
        &self.retry_lock
    }
    pub fn options(&self) -> &Vec<String> {
        &self.options
    }
}

#[derive(Debug, Deserialize)]
pub struct Location {
    #[serde(alias = "from")]
    paths: Vec<PathBuf>,
    #[serde(alias = "to")]
    repos: Vec<Name>,
    #[serde(default)]
    options: Options,
}

impl Location {
    pub fn paths(&self) -> &Vec<PathBuf> {
        &self.paths
    }
    pub fn repos(&self) -> &Vec<Name> {
        &self.repos
    }
    pub fn options(&self) -> &Options {
        &self.options
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Name(String);

impl Name {
    pub fn parse(s: &str) -> Result<Self, NameParseError> {
        if s.chars().all(Self::is_valid_char) {
            Ok(Self(s.to_string()))
        } else {
            Err(NameParseError(
                "Invalid name (only [A-Za-z0-9_-] are allowed).".to_string(),
            ))
        }
    }

    fn is_valid_char(c: char) -> bool {
        c.is_ascii_alphanumeric() || c == '_' || c == '-'
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for Name {
    type Err = NameParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Name::parse(s)
    }
}

impl<'de> Deserialize<'de> for Name {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::*;

        struct NameVisitor;

        impl<'de> de::Visitor<'de> for NameVisitor {
            type Value = Name;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a string consisting of characters [A-Za-z0-9_-]")
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_str(&v)
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Name::parse(v).map_err(de::Error::custom)
            }
        }

        deserializer.deserialize_seq(NameVisitor)
    }
}

#[derive(Debug, Error)]
#[error("{0}")]
pub struct NameParseError(String);

#[derive(Clone, Debug)]
pub struct LocationRepo(Name, Option<Name>);

impl LocationRepo {
    pub fn parse(s: &str) -> Result<Self, LocationRepoParseError> {
        Ok(match s.find('@') {
            Some(i) => {
                let loc =
                    Name::parse(&s[..i]).map_err(|e| LocationRepoParseError(e.to_string()))?;
                let repo =
                    Name::parse(&s[i + 1..]).map_err(|e| LocationRepoParseError(e.to_string()))?;
                LocationRepo(loc, Some(repo))
            }
            None => {
                let loc = Name::parse(s).map_err(|e| LocationRepoParseError(e.to_string()))?;
                LocationRepo(loc, None)
            }
        })
    }

    pub fn location(&self) -> &Name {
        &self.0
    }

    pub fn repo(&self) -> Option<&Name> {
        self.1.as_ref()
    }
}

impl FromStr for LocationRepo {
    type Err = LocationRepoParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        LocationRepo::parse(s)
    }
}

impl<'de> Deserialize<'de> for LocationRepo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::*;

        struct RepoNameVisitor;

        impl<'de> de::Visitor<'de> for RepoNameVisitor {
            type Value = LocationRepo;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a program name followed by any number of arguments either as a string or a sequence of string")
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_str(&v)
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                LocationRepo::parse(v).map_err(de::Error::custom)
            }
        }

        deserializer.deserialize_seq(RepoNameVisitor)
    }
}

#[derive(Debug, Error)]
#[error("{0}")]
pub struct LocationRepoParseError(String);

#[derive(Clone, Debug)]
pub struct CommandSeq(Vec<String>);

impl CommandSeq {
    pub fn from_vec(v: Vec<String>) -> Result<Self, CommandSeqParseError> {
        if v.is_empty() {
            return Err(CommandSeqParseError(
                "At least one element required.".to_string(),
            ));
        }
        Ok(Self(v))
    }

    pub fn parse_shell_words(str: &str) -> Result<Self, CommandSeqParseError> {
        let v = shell_words::split(str).map_err(|e| CommandSeqParseError(e.to_string()))?;
        Ok(Self(v))
    }

    pub fn program(&self) -> &String {
        self.0.first().unwrap()
    }

    pub fn args(&self) -> &[String] {
        &self.0[1..]
    }

    pub fn to_command(&self) -> std::process::Command {
        let mut cmd = std::process::Command::new(self.program());
        cmd.args(self.args());
        cmd
    }
}

impl<'de> Deserialize<'de> for CommandSeq {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::*;

        struct CommandSeqVisitor;

        impl<'de> de::Visitor<'de> for CommandSeqVisitor {
            type Value = CommandSeq;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a program name followed by any number of arguments either as a string or a sequence of string")
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_str(&v)
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                CommandSeq::parse_shell_words(v).map_err(de::Error::custom)
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let capacity = seq.size_hint().unwrap_or(0);
                let mut values = Vec::<String>::with_capacity(capacity);

                while let Some(value) = seq.next_element()? {
                    values.push(value);
                }

                if values.is_empty() {
                    return Err(de::Error::invalid_length(0, &"at least one element"));
                }

                CommandSeq::from_vec(values).map_err(de::Error::custom)
            }
        }

        deserializer.deserialize_seq(CommandSeqVisitor)
    }
}

#[derive(Debug, Error)]
#[error("{0}")]
pub struct CommandSeqParseError(String);
