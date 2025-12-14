mod config;
mod context;
mod emit;

use std::{env, path::PathBuf, process::Command};

use anyhow::{Context as AnyhowContext, Result};
use dirs::config_dir;

use context::Context;
use emit::builder::ScriptBuilder;

const SUPPORTED_SCHEMA_VERSION: u32 = 2;

fn main() -> Result<()> {
  let config_path = default_config_path();

  eprintln!(
    "apogee {}: looking for config at {}",
    env!("CARGO_PKG_VERSION"),
    config_path.display()
  );

  if !config_path.exists() {
    eprintln!(
      "apogee: no config found yet.\n\
       Create this file and re-run:\n  {}",
      config_path.display()
    );
    return Ok(());
  }

  let cfg = config::Config::load_from_path(&config_path)
    .with_context(|| format!("failed to load config at {}", config_path.display()))?;

  if cfg.apogee.schema_version != SUPPORTED_SCHEMA_VERSION {
    anyhow::bail!(
      "unsupported schema_version={} (supported={})",
      cfg.apogee.schema_version,
      SUPPORTED_SCHEMA_VERSION
    );
  }

  let ctx = Context {
    platform: detect_platform(),
    shell: cfg.apogee.default_shell,
    hostname: detect_hostname(),
  };

  let aliases = cfg.aliases.resolved(ctx.platform, ctx.hostname_str());

  let mut out = ScriptBuilder::new();

  // You can add a header if you want
  // out.push_line("# apogee generated");

  emit::prelude::emit_prelude(&ctx, &mut out);
  emit::aliases::emit_aliases(&ctx, &aliases, &mut out);

  print!("{}", out.finish());

  Ok(())
}

fn detect_platform() -> config::Platform {
  let is_wsl = env::var_os("WSL_DISTRO_NAME").is_some() || env::var_os("WSL_INTEROP").is_some();
  if is_wsl {
    return config::Platform::Wsl;
  }
  match env::consts::OS {
    "macos" => config::Platform::Mac,
    "linux" => config::Platform::Linux,
    _ => config::Platform::Other,
  }
}

fn detect_hostname() -> Option<String> {
  if let Ok(h) = env::var("HOSTNAME") {
    let h = h.trim().to_string();
    if !h.is_empty() {
      return Some(short_hostname(&h));
    }
  }

  if let Ok(h) = env::var("COMPUTERNAME") {
    let h = h.trim().to_string();
    if !h.is_empty() {
      return Some(short_hostname(&h));
    }
  }

  try_hostname_cmd(&["-s"])
    .or_else(|| try_hostname_cmd(&[]))
    .map(|h| short_hostname(&h))
}

fn try_hostname_cmd(args: &[&str]) -> Option<String> {
  let out = Command::new("hostname").args(args).output().ok()?;
  if !out.status.success() {
    return None;
  }
  let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
  if s.is_empty() { None } else { Some(s) }
}

fn short_hostname(h: &str) -> String {
  h.split('.').next().unwrap_or(h).to_string()
}

fn default_config_path() -> PathBuf {
  if let Ok(xdg) = env::var("XDG_CONFIG_HOME") {
    if !xdg.is_empty() {
      return PathBuf::from(xdg).join("apogee").join("config.toml");
    }
  }
  if let Some(home) = env::var_os("HOME") {
    return PathBuf::from(home).join(".config/apogee/config.toml");
  }
  if let Some(dir) = config_dir() {
    return dir.join("apogee").join("config.toml");
  }
  PathBuf::from("apogee/config.toml")
}
