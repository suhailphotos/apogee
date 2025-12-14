// src/config/mod.rs

pub mod aliases;

pub use aliases::{AliasMap, AliasesConfig, PlatformAliasConfig};

#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Shell {
  Zsh,
  Bash,
  Fish,
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

#[derive(Debug, serde::Deserialize)]
pub struct Config {
  pub apogee: ApogeeMeta,

  /// Only this section (for now)
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
  pub fn load_from_path<P>(path: P) -> anyhow::Result<Self>
  where
    P: AsRef<std::path::Path>,
  {
    let text = std::fs::read_to_string(path.as_ref())?;
    let cfg: Config = toml::from_str(&text)?;
    Ok(cfg)
  }
}

impl AliasesConfig {
  /// Merge order: global -> platform -> host (host wins last)
  pub fn resolved(&self, platform: Platform, hostname: Option<&str>) -> AliasMap {
    let mut out = AliasMap::new();

    for (k, v) in &self.global {
      out.insert(k.clone(), v.clone());
    }

    let plat_map = match platform {
      Platform::Mac => &self.platform.mac,
      Platform::Linux => &self.platform.linux,
      Platform::Wsl => &self.platform.wsl,
      Platform::Other => &self.platform.other,
    };

    for (k, v) in plat_map {
      out.insert(k.clone(), v.clone());
    }

    if let Some(host) = hostname {
      if let Some(host_map) = self.host.get(host) {
        for (k, v) in host_map {
          out.insert(k.clone(), v.clone());
        }
      }
    }

    out
  }
}
