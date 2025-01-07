use std::collections::HashMap;
use std::ops::Deref;
use std::path::Path;
use std::path::PathBuf;

use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_executable")]
    executable: String,
    #[serde(default)]
    options: Options,
    repos: HashMap<String, Repo>,
    locations: HashMap<String, Location>,
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
            .build()?;
        s.try_deserialize()
    }

    pub fn executable(&self) -> &str {
        &self.executable
    }
    pub fn options(&self) -> &Options {
        &self.options
    }
    pub fn repos(&self) -> &HashMap<String, Repo> {
        &self.repos
    }
    pub fn locations(&self) -> &HashMap<String, Location> {
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
    // TODO: larger-than
    // TODO: if-present
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
    // TODO: durations
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
    pub fn hooks(&self) -> &HookOptions {
        &self.hooks
    }
}

#[derive(Debug, Deserialize)]
pub struct Repo {
    #[serde(default)]
    path: String,
    #[serde(default)]
    key: String,
}

impl Repo {
    pub fn path(&self) -> &str {
        &self.path
    }
    pub fn key(&self) -> &str {
        &self.key
    }
}

#[derive(Debug, Deserialize)]
pub struct Location {
    #[serde(alias = "from")]
    paths: Vec<PathBuf>,
    #[serde(alias = "to")]
    repos: Vec<String>,
    #[serde(default)]
    options: Options,
}

impl Location {
    pub fn paths(&self) -> &Vec<PathBuf> {
        &self.paths
    }
    pub fn repos(&self) -> &Vec<String> {
        &self.repos
    }
    pub fn options(&self) -> &Options {
        &self.options
    }
}

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
