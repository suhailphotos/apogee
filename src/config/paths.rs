// src/config/paths.rs
use std::collections::HashMap;

#[derive(Debug, Default, serde::Deserialize)]
pub struct PathsConfig {
	#[serde(default)]
	pub global: PathScope,

	#[serde(default)]
	pub platform: PlatformPathsConfig,

	/// Per-host PATH overrides.
	#[serde(default)]
	pub host: HashMap<String, PathScope>,
}

#[derive(Debug, Default, serde::Deserialize)]
pub struct PathScope {
	#[serde(default)]
	pub prepend_if_exists: Vec<String>,

	#[serde(default)]
	pub append_if_exists: Vec<String>,
}

#[derive(Debug, Default, serde::Deserialize)]
pub struct PlatformPathsConfig {
	#[serde(default)]
	pub mac: PathScope,

	#[serde(default)]
	pub linux: PathScope,

	#[serde(default)]
	pub wsl: PathScope,

	#[serde(default)]
	pub other: PathScope,
}
