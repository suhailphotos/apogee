use anyhow::{bail, Context as _, Result};
use std::{
    collections::BTreeMap,
    fmt,
    path::{Path, PathBuf},
    process::Command,
};

use crate::config::{Platform, Shell};

#[derive(Debug, Clone)]
pub struct ContextEnv {
    pub vars: BTreeMap<String, String>,
    pub home: PathBuf,
    pub xdg_config_home: PathBuf,
    pub platform: Platform,
    pub shell_type: Option<Shell>,
    pub host: String,

    pub config_path: Option<PathBuf>,
    pub config_dir: Option<PathBuf>,
}

impl ContextEnv {
    /// Build a minimal runtime context:
    /// - env vars snapshot
    /// - home + XDG_CONFIG_HOME (fallback to ~/.config)
    /// - platform (incl WSL)
    /// - shell_type (best-effort)
    /// - host (best-effort)
    pub fn new() -> Result<Self> {
        let mut vars: BTreeMap<String, String> = std::env::vars().collect();

        let home = detect_home(&vars).context("could not determine home directory")?;
        let home_str = home.to_string_lossy().to_string();

        // Normalize HOME / USERPROFILE (handy for cross-platform config expansion later)
        vars.entry("HOME".to_string()).or_insert(home_str.clone());
        vars.entry("USERPROFILE".to_string())
            .or_insert(home_str.clone());

        // XDG_CONFIG_HOME fallback
        let xdg_config_home = vars
            .get("XDG_CONFIG_HOME")
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config"));

        vars.insert(
            "XDG_CONFIG_HOME".to_string(),
            xdg_config_home.to_string_lossy().to_string(),
        );

        let platform = detect_platform(&vars);
        let shell_type = detect_shell(&vars);
        let host = detect_hostname(&vars).unwrap_or_else(|| "unknown".to_string());

        // Helpful computed vars (small + harmless)
        vars.insert(
            "APOGEE_PLATFORM".to_string(),
            platform_to_str(platform).to_string(),
        );
        if let Some(sh) = shell_type {
            vars.insert("APOGEE_SHELL".to_string(), shell_to_str(sh).to_string());
        }
        vars.insert("APOGEE_HOST".to_string(), host.clone());

        Ok(Self {
            vars,
            home,
            xdg_config_home,
            platform,
            shell_type,
            host,
            config_path: None,
            config_dir: None,
        })
    }

    pub fn default_config_path(&self) -> PathBuf {
        self.xdg_config_home.join("apogee").join("config.toml")
    }

    /// Config path precedence:
    /// 1) APOGEE_CONFIG (must exist)
    /// 2) default: $XDG_CONFIG_HOME/apogee/config.toml (must exist)
    ///
    /// No auto-creation: this is intentionally side-effect free now.
    pub fn locate_config(&mut self) -> Result<PathBuf> {
        let p = if let Some(p) = env_path(&self.vars, "APOGEE_CONFIG") {
            p
        } else {
            self.default_config_path()
        };

        if !p.exists() {
            bail!(
                "config.toml not found: {} (set APOGEE_CONFIG to override)",
                p.display()
            );
        }

        self.set_config_path(p.clone());
        Ok(p)
    }

    pub fn load_config(&mut self) -> Result<crate::config::Config> {
        let path = self.locate_config()?;
        crate::config::Config::load_from_path(&path)
            .with_context(|| format!("failed to load config at {}", path.display()))
    }

    fn set_config_path(&mut self, path: PathBuf) {
        let dir = path.parent().map(Path::to_path_buf);

        self.config_path = Some(path.clone());
        self.config_dir = dir.clone();

        self.vars.insert(
            "APOGEE_CONFIG".to_string(),
            path.to_string_lossy().to_string(),
        );

        if let Some(d) = dir {
            self.vars.insert(
                "APOGEE_CONFIG_DIR".to_string(),
                d.to_string_lossy().to_string(),
            );
        }
    }

    // Small ergonomic getters (optional)
    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn config_dir(&self) -> Option<&Path> {
        self.config_dir.as_deref()
    }

    pub fn config_path(&self) -> Option<&Path> {
        self.config_path.as_deref()
    }
}

impl fmt::Display for ContextEnv {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let shell = self
            .shell_type
            .map(|s| s.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let cfg_path = self
            .config_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "<unset>".to_string());

        let cfg_dir = self
            .config_dir
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "<unset>".to_string());

        write!(
            f,
            "apogee context\n\
       - host: {}\n\
       - platform: {}\n\
       - shell: {}\n\
       - home: {}\n\
       - xdg_config_home: {}\n\
       - config_path: {}\n\
       - config_dir: {}",
            self.host,
            self.platform,
            shell,
            self.home.display(),
            self.xdg_config_home.display(),
            cfg_path,
            cfg_dir
        )
    }
}

// -------------------- helpers --------------------

fn env_path(vars: &BTreeMap<String, String>, key: &str) -> Option<PathBuf> {
    vars.get(key)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
}

fn detect_home(vars: &BTreeMap<String, String>) -> Option<PathBuf> {
    dirs::home_dir()
        .or_else(|| vars.get("HOME").map(PathBuf::from))
        .or_else(|| vars.get("USERPROFILE").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
}

fn detect_platform(vars: &BTreeMap<String, String>) -> Platform {
    let is_wsl = vars.contains_key("WSL_DISTRO_NAME") || vars.contains_key("WSL_INTEROP");
    if is_wsl {
        return Platform::Wsl;
    }

    if cfg!(target_os = "macos") {
        Platform::Mac
    } else if cfg!(target_os = "windows") {
        Platform::Windows
    } else if cfg!(target_os = "linux") {
        Platform::Linux
    } else {
        Platform::Other
    }
}

fn detect_shell(vars: &BTreeMap<String, String>) -> Option<Shell> {
    // Prefer pwsh signal first (important on mac/linux where SHELL may still be zsh)
    if vars.contains_key("PSModulePath") || vars.contains_key("POWERSHELL_DISTRIBUTION_CHANNEL") {
        return Some(Shell::Pwsh);
    }

    if let Some(sh) = vars.get("SHELL") {
        let sh = sh.to_ascii_lowercase();
        if sh.contains("zsh") {
            return Some(Shell::Zsh);
        }
        if sh.contains("bash") {
            return Some(Shell::Bash);
        }
        if sh.contains("fish") {
            return Some(Shell::Fish);
        }
    }

    None
}

fn detect_hostname(vars: &BTreeMap<String, String>) -> Option<String> {
    if let Some(h) = vars.get("HOSTNAME") {
        let h = h.trim();
        if !h.is_empty() {
            return Some(short_hostname(h));
        }
    }

    if let Some(h) = vars.get("COMPUTERNAME") {
        let h = h.trim();
        if !h.is_empty() {
            return Some(short_hostname(h));
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
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

fn short_hostname(h: &str) -> String {
    h.split('.').next().unwrap_or(h).to_string()
}

fn platform_to_str(p: Platform) -> &'static str {
    match p {
        Platform::Mac => "mac",
        Platform::Linux => "linux",
        Platform::Windows => "windows",
        Platform::Wsl => "wsl",
        Platform::Other => "other",
    }
}

fn shell_to_str(s: Shell) -> &'static str {
    match s {
        Shell::Zsh => "zsh",
        Shell::Bash => "bash",
        Shell::Fish => "fish",
        Shell::Pwsh => "pwsh",
    }
}
