// src/config/mod.rs

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

#[derive(Debug, serde::Deserialize)]
pub struct Config {
  pub apogee: ApogeeMeta,
}

#[derive(Debug, serde::Deserialize)]
pub struct ApogeeMeta {
  /// Config schema version (yours, not Cargo crate version)
  pub schema_version: u32,

  /// Which shell to emit for by default
  pub default_shell: Shell,

  /// Which platforms you want this config to support
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
