use std::collections::HashMap;
use std::ops::Deref;
use std::path::Path;
use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_executable")]
    executable: String,
    options: Options,
    repos: HashMap<String, Repo>,
    locations: HashMap<String, Location>,
}

fn default_executable() -> String {
    "restic".to_string()
}

impl Config {
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
    hooks: BackupHooks,
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
    pub fn hooks(&self) -> &BackupHooks {
        &self.hooks
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct BackupHooks {
    r#if: Vec<Vec<String>>
}

impl BackupHooks {
    pub fn r#if(&self) -> &[Vec<String>] {
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

impl Config {
    pub fn new(config_path: &Path) -> Result<Self, config::ConfigError> {
        let s = config::Config::builder()
            .add_source(config::File::with_name(
                config_path.to_string_lossy().deref(),
            ))
            .build()?;
        s.try_deserialize()
    }
}
