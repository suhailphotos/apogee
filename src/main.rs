use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    env,
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Parser)]
#[command(name = "apogee", version, about = "Emit shell config from TOML")]
struct Args {
    /// Path to config file (or set APOGEE_CONFIG)
    #[arg(short, long, env = "APOGEE_CONFIG", value_hint = clap::ValueHint::FilePath)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    cmd: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Aliases,
}

#[derive(Debug, Deserialize)]
struct Config {
    #[serde(default)]
    aliases: BTreeMap<String, String>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let cfg_path = args.config.unwrap_or_else(default_config_path);
    let cfg = read_config(&cfg_path)
        .with_context(|| format!("failed to read config at {}", cfg_path.display()))?;

    match args.cmd {
        Command::Aliases => emit_aliases(&cfg),
    }
    Ok(())
}

fn read_config(path: &Path) -> Result<Config> {
    if !path.exists() {
        bail!("config file not found: {}", path.display());
    }
    let raw = fs::read_to_string(path)
        .with_context(|| format!("unable to read {}", path.display()))?;
    let cfg: Config = toml::from_str(&raw)
        .with_context(|| format!("invalid TOML in {}", path.display()))?;
    Ok(cfg)
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
    if let Some(dir) = dirs::config_dir() {
        return dir.join("apogee").join("config.toml");
    }
    // 4) absolute last fallback (relative)
    PathBuf::from("apogee/config.toml")
}

fn emit_aliases(cfg: &Config) {
    for (name, value) in &cfg.aliases {
        println!("alias {}={}", name, sh_single_quote(value));
    }
}

fn sh_single_quote(s: &str) -> String {
    if s.contains('\'') {
        let mut out = String::with_capacity(s.len() + 2);
        out.push('\'');
        for ch in s.chars() {
            if ch == '\'' {
                out.push_str("'\"'\"'");
            } else {
                out.push(ch);
            }
        }
        out.push('\'');
        out
    } else {
        format!("'{}'", s)
    }
}
