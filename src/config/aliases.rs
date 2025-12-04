// src/config/aliases.rs
use std::collections::{BTreeMap, HashMap};

pub type AliasMap = BTreeMap<String, String>;

#[derive(Debug, Default, serde::Deserialize)]
pub struct AliasesConfig {
	#[serde(default)]
	pub global: AliasMap,

	#[serde(default)]
	pub platform: PlatformAliasConfig,

	/// Per-host alias overrides (hostname → alias map).
	#[serde(default)]
	pub host: HashMap<String, AliasMap>,
}

#[derive(Debug, Default, serde::Deserialize)]
pub struct PlatformAliasConfig {
	#[serde(default)]
	pub mac: AliasMap,

	#[serde(default)]
	pub linux: AliasMap,

	#[serde(default)]
	pub wsl: AliasMap,

	#[serde(default)]
	pub other: AliasMap,
}
