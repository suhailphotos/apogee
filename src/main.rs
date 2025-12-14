mod config;

use std::{env, path::PathBuf};

use anyhow::{Context, Result};
use dirs::config_dir;

const SUPPORTED_SCHEMA_VERSION: u32 = 1;

fn main() -> Result<()> {
  let config_path = default_config_path();

  println!(
    "apogee {}: looking for config at {}",
    env!("CARGO_PKG_VERSION"),
    config_path.display()
  );

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

  // simple schema guard
  if cfg.apogee.schema_version != SUPPORTED_SCHEMA_VERSION {
    anyhow::bail!(
      "unsupported schema_version={} (supported={})",
      cfg.apogee.schema_version,
      SUPPORTED_SCHEMA_VERSION
    );
  }

  println!("apogee: schema_version={}", cfg.apogee.schema_version);
  println!("apogee: default_shell={:?}", cfg.apogee.default_shell);
  println!("apogee: platforms={:?}", cfg.apogee.platforms);

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
