#![allow(dead_code)]

use anyhow::Result;
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

pub fn default_config_path() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg).join("apogee").join("config.toml");
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home)
            .join(".config")
            .join("apogee")
            .join("config.toml");
    }
    PathBuf::from("apogee/config.toml")
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub apogee: ApogeeMeta,

    #[serde(default)]
    pub modules: ModulesRoot,

    #[serde(default)]
    pub global: GlobalConfig,
}

impl Config {
    pub fn load_from_path(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)?;
        let cfg: Self = toml::from_str(&text)?;
        Ok(cfg)
    }
}

#[derive(Debug, Deserialize)]
pub struct ApogeeMeta {
    pub schema_version: u32,

    #[serde(default = "default_shell")]
    pub default_shell: Shell,

    #[serde(default)]
    pub platforms: Vec<Platform>,

    #[serde(default)]
    pub secrets_file: Option<String>,

    #[serde(default)]
    pub bootstrap: Option<BootstrapConfig>,
}

#[derive(Debug, Default, Deserialize)]
pub struct BootstrapConfig {
    #[serde(default)]
    pub defaults: BootstrapDefaults,

    #[serde(default)]
    pub secrets: BootstrapSecrets,
}

#[derive(Debug, Default, Deserialize)]
pub struct BootstrapDefaults {
    #[serde(default)]
    pub env: EnvMap,
}

#[derive(Debug, Deserialize)]
pub struct BootstrapSecrets {
    #[serde(default = "default_secrets_strategy")]
    pub strategy: SecretsStrategy,
}

impl Default for BootstrapSecrets {
    fn default() -> Self {
        Self {
            strategy: default_secrets_strategy(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecretsStrategy {
    FillMissing,
    Override,
}

fn default_secrets_strategy() -> SecretsStrategy {
    SecretsStrategy::FillMissing
}

fn default_shell() -> Shell {
    Shell::Zsh
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Shell {
    Zsh,
    Bash,
    Fish,
    Pwsh,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Mac,
    Linux,
    Windows,
    Wsl,
    Other,
}

#[derive(Debug, Default, Deserialize)]
pub struct ModulesRoot {
    #[serde(default = "default_true")]
    pub enable_cloud: bool,
    #[serde(default = "default_true")]
    pub enable_apps: bool,
    #[serde(default = "default_true")]
    pub enable_hooks: bool,

    #[serde(default)]
    pub cloud: CloudModules,

    #[serde(default)]
    pub apps: AppModules,

    #[serde(default)]
    pub hooks: HooksModules,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Default, Deserialize)]
pub struct GlobalConfig {
    #[serde(default)]
    pub aliases: GlobalAliases,
}

#[derive(Debug, Default, Deserialize)]
pub struct GlobalAliases {
    #[serde(default)]
    pub platform: PlatformAliasMaps,

    #[serde(default)]
    pub shell: ShellAliasMaps,
}

#[derive(Debug, Default, Deserialize)]
pub struct PlatformAliasMaps {
    #[serde(default)]
    pub mac: AliasMap,
    #[serde(default)]
    pub linux: AliasMap,
    #[serde(default)]
    pub windows: AliasMap,
    #[serde(default)]
    pub wsl: AliasMap,
    #[serde(default)]
    pub other: AliasMap,
}

#[derive(Debug, Default, Deserialize)]
pub struct ShellAliasMaps {
    #[serde(default)]
    pub zsh: AliasMap,
    #[serde(default)]
    pub bash: AliasMap,
    #[serde(default)]
    pub fish: AliasMap,
    #[serde(default)]
    pub pwsh: AliasMap,
}

pub type AliasMap = BTreeMap<String, String>;
pub type EnvMap = BTreeMap<String, String>;

#[derive(Debug, Default, Deserialize)]
pub struct CloudModules {
    #[serde(default = "default_true")]
    pub enabled: bool,

    #[serde(flatten, default)]
    pub items: BTreeMap<String, CloudModule>,
}

#[derive(Debug, Deserialize)]
pub struct CloudModule {
    pub enabled: bool,

    #[serde(default)]
    pub kind: Option<CloudKind>,

    #[serde(default)]
    pub platforms: Vec<Platform>,

    #[serde(default)]
    pub detect: DetectBlock,

    #[serde(default)]
    pub emit: EmitBlock,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CloudKind {
    Storage,
    Service,
}

#[derive(Debug, Default, Deserialize)]
pub struct AppModules {
    #[serde(default = "default_true")]
    pub enabled: bool,

    #[serde(flatten, default)]
    pub items: BTreeMap<String, AppModule>,
}

#[derive(Debug, Deserialize)]
pub struct AppModule {
    pub enabled: bool,

    #[serde(default)]
    pub kind: Option<AppKind>,

    #[serde(default)]
    pub platforms: Vec<Platform>,

    #[serde(default)]
    pub detect: DetectBlock,

    #[serde(default)]
    pub emit: EmitBlock,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AppKind {
    Cli,
    Desktop,
}

#[derive(Debug, Default, Deserialize)]
pub struct HooksModules {
    #[serde(default = "default_true")]
    pub enabled: bool,

    #[serde(default)]
    pub items: Vec<HookItem>,
}

#[derive(Debug, Deserialize)]
pub struct HookItem {
    pub name: String,
    pub enabled: bool,

    #[serde(default)]
    pub platforms: Vec<Platform>,

    #[serde(default)]
    pub hosts: Vec<String>,

    #[serde(default)]
    pub shells: Vec<Shell>,

    pub script: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct DetectBlock {
    #[serde(default)]
    pub paths: PlatformAnyOf,

    #[serde(default)]
    pub files: PlatformAnyOf,

    #[serde(default)]
    pub commands: AnyOf,

    #[serde(default)]
    pub env: AnyOf,

    #[serde(default)]
    pub version: Option<VersionDetectSpec>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum VersionDetectSpec {
    Single(VersionDetect),
    PerPlatform(PlatformVersionDetect),
}

#[derive(Debug, Default, Deserialize)]
pub struct PlatformVersionDetect {
    #[serde(default)]
    pub mac: Option<VersionDetect>,
    #[serde(default)]
    pub linux: Option<VersionDetect>,
    #[serde(default)]
    pub windows: Option<VersionDetect>,
    #[serde(default)]
    pub wsl: Option<VersionDetect>,
    #[serde(default)]
    pub other: Option<VersionDetect>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum VersionDetect {
    Tagged(VersionDetectTagged),

    Command {
        command: String,
        #[serde(default)]
        args: Vec<String>,

        #[serde(default)]
        regex: Option<String>,
        #[serde(default = "default_version_capture")]
        capture: String,
    },

    PathRegex {
        regex: String,
        #[serde(default = "default_version_capture")]
        capture: String,
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum VersionDetectTagged {
    Command {
        command: String,
        #[serde(default)]
        args: Vec<String>,

        #[serde(default)]
        regex: Option<String>,
        #[serde(default = "default_version_capture")]
        capture: String,
    },
    PathRegex {
        regex: String,
        #[serde(default = "default_version_capture")]
        capture: String,
    },
}

fn default_version_capture() -> String {
    "version".to_string()
}

#[derive(Debug, Default, Deserialize)]
pub struct PlatformAnyOf {
    #[serde(default)]
    pub mac: AnyOf,
    #[serde(default)]
    pub linux: AnyOf,
    #[serde(default)]
    pub windows: AnyOf,
    #[serde(default)]
    pub wsl: AnyOf,
    #[serde(default)]
    pub other: AnyOf,
}

#[derive(Debug, Default, Deserialize)]
pub struct AnyOf {
    #[serde(default)]
    pub any_of: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct EmitBlock {
    #[serde(default)]
    pub env: EnvMap,

    #[serde(default)]
    pub env_derived: EnvMap,

    #[serde(default)]
    pub aliases: AliasMap,

    #[serde(default)]
    pub functions: FunctionsEmit,

    #[serde(default)]
    pub paths: PathsEmit,
}

#[derive(Debug, Default, Deserialize)]
pub struct PathsEmit {
    #[serde(default)]
    pub prepend_if_exists: Vec<String>,

    #[serde(default)]
    pub append_if_exists: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct FunctionsEmit {
    #[serde(default)]
    pub files: Vec<String>,
}
