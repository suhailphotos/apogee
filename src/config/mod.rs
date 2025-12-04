// src/config/mod.rs

/// Small helper for serde defaults.
pub(crate) fn bool_true() -> bool {
	true
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Shell {
	Zsh,
	Bash,
	Pwsh,
}

#[derive(Debug, Clone, Copy, serde::Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
	Mac,
	Linux,
	Wsl,
	Other,
}

pub mod modules;
pub mod env;
pub mod paths;
pub mod projects;
pub mod aliases;

pub use modules::{
	ModulesConfig,
	CloudModulesConfig,
	CloudDetectorConfig,
	AppModulesConfig,
	AppModuleConfig,
	CustomModuleConfig,
};
pub use env::{EnvConfig, EnvMap, PlatformEnvConfig};
pub use paths::{PathsConfig, PathScope, PlatformPathsConfig};
pub use projects::ProjectsConfig;
pub use aliases::{AliasesConfig, AliasMap};

#[derive(Debug, serde::Deserialize)]
pub struct Config {
	pub apogee: ApogeeMeta,

	#[serde(default)]
	pub modules: ModulesConfig,

	#[serde(default)]
	pub env: EnvConfig,

	#[serde(default)]
	pub paths: PathsConfig,

	#[serde(default)]
	pub projects: ProjectsConfig,

	#[serde(default)]
	pub aliases: AliasesConfig,
}

#[derive(Debug, serde::Deserialize)]
pub struct ApogeeMeta {
	pub schema_version: u32,
	pub default_shell: Shell,
	pub platforms: Vec<Platform>,
}

impl Config {
	pub fn load_from_path<P: AsRef<std::path::Path>>(path: P) -> anyhow::Result<Self> {
		let text = std::fs::read_to_string(&path)?;
		let cfg: Config = toml::from_str(&text)?;
		Ok(cfg)
	}
}
