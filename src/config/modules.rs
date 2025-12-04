// src/config/modules.rs
use std::collections::BTreeMap;

use super::{bool_true, Platform, Shell};

#[derive(Debug, Default, serde::Deserialize)]
pub struct ModulesConfig {
	#[serde(default)]
	pub enable_cloud: bool,

	#[serde(default)]
	pub enable_apps: bool,

	#[serde(default)]
	pub enable_hooks: bool,

	#[serde(default)]
	pub cloud: CloudModulesConfig,

	#[serde(default)]
	pub apps: AppModulesConfig,

	#[serde(default)]
	pub custom: Vec<CustomModuleConfig>,
}

#[derive(Debug, Default, serde::Deserialize)]
pub struct CloudModulesConfig {
	#[serde(default = "bool_true")]
	pub enabled: bool,

	/// e.g. "dropbox", "synology_datalib", "nebula_ai"
	#[serde(flatten)]
	#[serde(default)]
	pub detectors: BTreeMap<String, CloudDetectorConfig>,
}

#[derive(Debug, serde::Deserialize)]
pub struct CloudDetectorConfig {
	#[serde(default = "bool_true")]
	pub enabled: bool,

	#[serde(default)]
	pub platforms: Vec<Platform>,

	#[serde(default)]
	pub candidate_paths: Vec<String>,

	pub set_env: String,
}

#[derive(Debug, Default, serde::Deserialize)]
pub struct AppModulesConfig {
	#[serde(default = "bool_true")]
	pub enabled: bool,

	/// e.g. "eza", "zoxide", "ripgrep"
	#[serde(flatten)]
	#[serde(default)]
	pub apps: BTreeMap<String, AppModuleConfig>,
}

#[derive(Debug, serde::Deserialize)]
pub struct AppModuleConfig {
	#[serde(default = "bool_true")]
	pub enabled: bool,

	/// Command used to detect presence (e.g. "eza", "zoxide").
	pub command: String,

	/// Env vars to set if the command is available.
	#[serde(default)]
	pub env: BTreeMap<String, String>,

	/// Aliases to define if the command is available.
	#[serde(default)]
	pub aliases: BTreeMap<String, String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct CustomModuleConfig {
	pub name: String,

	#[serde(default = "bool_true")]
	pub enabled: bool,

	#[serde(default)]
	pub platforms: Vec<Platform>,

	#[serde(default)]
	pub hosts: Vec<String>,

	#[serde(default)]
	pub shells: Vec<Shell>,

	pub script: String,
}
