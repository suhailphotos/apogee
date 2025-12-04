// src/config/env.rs
use std::collections::{BTreeMap, HashMap};

pub type EnvMap = BTreeMap<String, String>;

#[derive(Debug, Default, serde::Deserialize)]
pub struct EnvConfig {
	/// Global env vars (all platforms/hosts).
	#[serde(default)]
	pub global: EnvMap,

	/// Per-platform env.
	#[serde(default)]
	pub platform: PlatformEnvConfig,

	/// Nebula-derived env.
	#[serde(default)]
	pub nebula: EnvMap,

	/// Per-host env overrides (hostname → env map).
	#[serde(default)]
	pub host: HashMap<String, EnvMap>,
}

#[derive(Debug, Default, serde::Deserialize)]
pub struct PlatformEnvConfig {
	#[serde(default)]
	pub mac: EnvMap,

	#[serde(default)]
	pub linux: EnvMap,

	#[serde(default)]
	pub wsl: EnvMap,

	#[serde(default)]
	pub other: EnvMap,
}
