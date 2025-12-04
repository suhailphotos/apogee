mod config;

#[cfg(debug_assertions)]
mod dev_dump;

use std::{env, path::PathBuf};

use anyhow::{Context, Result};
use dirs::config_dir;

fn main() -> Result<()> {
	let config_path = default_config_path();

	println!("apogee: looking for config at {}", config_path.display());

	if !config_path.exists() {
		println!(
			"apogee: no config found yet.\n\
			 Create this file and re-run:\n  {}",
			config_path.display()
		);
		return Ok(());
	}

	let cfg = config::Config::load_from_path(&config_path)
		.with_context(|| format!("failed to load config at {}", config_path.display()))?;

	println!(
		"apogee: loaded schema_version={} default_shell={:?}",
		cfg.apogee.schema_version,
		cfg.apogee.default_shell
	);

	// Dev-only dump. Only compiled in debug builds.
	#[cfg(debug_assertions)]
	{
		dev_dump::dump_full_config(&cfg);
	}

	Ok(())
}

/// Prefer XDG: $XDG_CONFIG_HOME/apogee/config.toml
/// Fallback: ~/.config/apogee/config.toml
/// Last resort: platform config dir (e.g. ~/Library/Application Support on macOS)
fn default_config_path() -> PathBuf {
	// 1) $XDG_CONFIG_HOME
	if let Ok(xdg) = env::var("XDG_CONFIG_HOME") {
		if !xdg.is_empty() {
			return PathBuf::from(xdg).join("apogee").join("config.toml");
		}
	}
	// 2) ~/.config
	if let Some(home) = env::var_os("HOME") {
		return PathBuf::from(home).join(".config/apogee/config.toml");
	}
	// 3) platform default
	if let Some(dir) = config_dir() {
		return dir.join("apogee").join("config.toml");
	}
	// 4) absolute last fallback (relative)
	PathBuf::from("apogee/config.toml")
}
