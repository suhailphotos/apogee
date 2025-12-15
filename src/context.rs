use anyhow::{bail, Context as _, Result};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use crate::config::{Platform, Shell};

#[derive(Debug, Clone)]
pub struct ContextEnv {
    vars: BTreeMap<String, String>,
    secret_keys: BTreeSet<String>,
    home: PathBuf,
    xdg_config_home: PathBuf,
    platform: Platform,
    shell_guess: Option<Shell>,
    host: String,

    config_path: Option<PathBuf>,
    config_dir: Option<PathBuf>,
    secrets_path: Option<PathBuf>,
}

impl ContextEnv {
    pub fn new() -> Result<Self> {
        let mut vars: BTreeMap<String, String> = std::env::vars().collect();

        let home = dirs::home_dir()
            .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
            .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
            .context("could not determine home directory")?;

        let home_str = home.to_string_lossy().to_string();
        vars.entry("HOME".to_string()).or_insert(home_str.clone());
        vars.entry("USERPROFILE".to_string()).or_insert(home_str.clone());

        // XDG_CONFIG_HOME: honor if present, else fallback to ~/.config
        let xdg_config_home = match vars.get("XDG_CONFIG_HOME").map(|s| s.trim()).filter(|s| !s.is_empty()) {
            Some(s) => PathBuf::from(s),
            None => home.join(".config"),
        };
        vars.insert(
            "XDG_CONFIG_HOME".to_string(),
            xdg_config_home.to_string_lossy().to_string(),
        );

        // (Optional nice-to-have) Fill other XDG defaults if missing
        set_if_missing(&mut vars, "XDG_CACHE_HOME", format!("{}/.cache", home_str));
        set_if_missing(&mut vars, "XDG_DATA_HOME", format!("{}/.local/share", home_str));
        set_if_missing(&mut vars, "XDG_STATE_HOME", format!("{}/.local/state", home_str));

        let platform = detect_platform(&vars);
        let shell_guess = detect_shell(&vars);
        let host = detect_hostname(&vars).unwrap_or_else(|| "unknown".to_string());

        vars.insert("APOGEE_PLATFORM".to_string(), platform_to_str(platform).to_string());
        if let Some(sh) = shell_guess {
            vars.insert("APOGEE_SHELL".to_string(), shell_to_str(sh).to_string());
        }
        vars.insert("APOGEE_HOST".to_string(), host.clone());

        Ok(Self {
            vars,
            secret_keys: BTreeSet::new(),
            home,
            xdg_config_home,
            platform,
            shell_guess,
            host,
            config_path: None,
            config_dir: None,
            secrets_path: None,
        })
    }

    // ---------- public getters ----------

    pub fn platform(&self) -> Platform {
        self.platform
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn default_config_path(&self) -> PathBuf {
        self.xdg_config_home.join("apogee").join("config.toml")
    }

    pub fn default_secrets_path(&self) -> PathBuf {
        self.xdg_config_home.join("apogee").join("secrets.env")
    }

    // ---------- locating paths (simple contract) ----------

    /// Config path precedence:
    /// 1) CLI --config (must exist)
    /// 2) APOGEE_CONFIG (must exist)
    /// 3) default XDG_CONFIG_HOME/apogee/config.toml (created if missing)
    pub fn locate_config(&mut self, cli_config: Option<&PathBuf>) -> Result<PathBuf> {
        if let Some(p) = cli_config {
            if !p.exists() {
                bail!("--config was provided but file does not exist: {}", p.display());
            }
            self.set_config_path(p.clone());
            return Ok(p.clone());
        }

        if let Some(p) = self.get_env_path("APOGEE_CONFIG") {
            if !p.exists() {
                bail!(
                    "APOGEE_CONFIG is set but file does not exist: {}",
                    p.display()
                );
            }
            self.set_config_path(p.clone());
            return Ok(p);
        }

        let p = self.default_config_path();
        ensure_parent_dir(&p)?;
        if !p.exists() {
            write_default_config_stub(&p)?;
        }
        self.set_config_path(p.clone());
        Ok(p)
    }

    /// Secrets path precedence:
    /// 1) APOGEE_SECRETS (must exist)
    /// 2) default XDG_CONFIG_HOME/apogee/secrets.env (created if missing)
    pub fn locate_secrets(&mut self) -> Result<PathBuf> {
        if let Some(p) = self.get_env_path("APOGEE_SECRETS") {
            if !p.exists() {
                bail!(
                    "APOGEE_SECRETS is set but file does not exist: {}",
                    p.display()
                );
            }
            self.set_secrets_path(p.clone());
            return Ok(p);
        }

        let p = self.default_secrets_path();
        ensure_parent_dir(&p)?;
        if !p.exists() {
            write_default_secrets_stub(&p)?;
        }
        self.set_secrets_path(p.clone());
        Ok(p)
    }

    fn set_config_path(&mut self, path: PathBuf) {
        let dir = path.parent().map(Path::to_path_buf);

        self.config_path = Some(path.clone());
        self.config_dir = dir.clone();

        self.vars
            .insert("APOGEE_CONFIG".to_string(), path.to_string_lossy().to_string());

        if let Some(d) = dir {
            self.vars.insert(
                "APOGEE_CONFIG_DIR".to_string(),
                d.to_string_lossy().to_string(),
            );
        }
    }

    fn set_secrets_path(&mut self, path: PathBuf) {
        self.secrets_path = Some(path.clone());
        self.vars.insert(
            "APOGEE_SECRETS".to_string(),
            path.to_string_lossy().to_string(),
        );
    }

    fn get_env_path(&self, key: &str) -> Option<PathBuf> {
        self.vars
            .get(key)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
    }

    // ---------- secrets loading + dump ----------

    pub fn merge_secrets_fill_missing(&mut self, secrets: &BTreeMap<String, String>) {
        for (k, v) in secrets {
            self.secret_keys.insert(k.clone());
            if !self.vars.contains_key(k) {
                self.vars.insert(k.clone(), v.clone());
            }
        }
    }

    pub fn debug_dump(&self, redact: bool) -> String {
        let mut out = String::new();

        out.push_str("apogee runtime context (debug)\n");
        out.push_str("============================\n");

        out.push_str(&format!("platform: {}\n", platform_to_str(self.platform)));
        out.push_str(&format!(
            "shell_guess: {}\n",
            self.shell_guess.map(shell_to_str).unwrap_or("unknown")
        ));
        out.push_str(&format!("host: {}\n", self.host));
        out.push_str(&format!("home: {}\n", self.home.to_string_lossy()));

        out.push_str(&format!(
            "config_path: {}\n",
            self.config_path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| "<unset>".to_string())
        ));
        out.push_str(&format!(
            "secrets_path: {}\n",
            self.secrets_path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| "<unset>".to_string())
        ));

        if let Some(d) = &self.config_dir {
            out.push_str(&format!("config_dir: {}\n", d.to_string_lossy()));
        }

        out.push_str("\nvars:\n");
        for (k, v) in &self.vars {
            let should_redact = redact && (looks_sensitive_key(k) || self.secret_keys.contains(k));
            if should_redact {
                out.push_str(&format!("  {} = <redacted>\n", k));
            } else {
                out.push_str(&format!("  {} = {}\n", k, v));
            }
        }

        out
    }
}

pub fn load_secrets_env(path: &Path) -> Result<BTreeMap<String, String>> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("failed to read secrets file at {}", path.display()))?;

    let mut out = BTreeMap::new();
    for (idx, line) in text.lines().enumerate() {
        let raw = line.trim();
        if raw.is_empty() || raw.starts_with('#') {
            continue;
        }

        let Some((k, v)) = raw.split_once('=') else {
            bail!("invalid secrets line {}: {}", idx + 1, raw);
        };

        let key = k.trim().to_string();
        if key.is_empty() {
            continue;
        }

        let mut val = v.trim().to_string();
        if (val.starts_with('"') && val.ends_with('"')) || (val.starts_with('\'') && val.ends_with('\'')) {
            val = val[1..val.len() - 1].to_string();
        }

        out.insert(key, val);
    }

    Ok(out)
}

// -------------------- helpers --------------------

fn ensure_parent_dir(p: &Path) -> Result<()> {
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory {}", parent.display()))?;
    }
    Ok(())
}

fn write_default_config_stub(p: &Path) -> Result<()> {
    let stub = r#"# apogee config.toml (created by apogee)
#
# Fill this in with your modules + rules.
# If you want config somewhere else, set APOGEE_CONFIG to that file path.

[apogee]
schema_version = 2
default_shell  = "zsh"
platforms      = ["mac", "linux", "windows", "wsl", "other"]

[modules]
enable_cloud = true
enable_apps  = true
enable_hooks = true
"#;
    fs::write(p, stub).with_context(|| format!("failed to write {}", p.display()))?;
    Ok(())
}

fn write_default_secrets_stub(p: &Path) -> Result<()> {
    let stub = r#"# apogee secrets.env (created by apogee)
# Format: KEY=value
# This file should NOT be committed.
"#;
    fs::write(p, stub).with_context(|| format!("failed to write {}", p.display()))?;
    Ok(())
}

fn set_if_missing(vars: &mut BTreeMap<String, String>, key: &str, val: String) {
    match vars.get(key) {
        Some(existing) if !existing.trim().is_empty() => {}
        _ => {
            vars.insert(key.to_string(), val);
        }
    }
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
    if let Some(sh) = vars.get("SHELL") {
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

    if vars.contains_key("PSModulePath") {
        return Some(Shell::Pwsh);
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
    if s.is_empty() { None } else { Some(s) }
}

fn short_hostname(h: &str) -> String {
    h.split('.').next().unwrap_or(h).to_string()
}

fn looks_sensitive_key(k: &str) -> bool {
    let u = k.to_ascii_uppercase();
    u.contains("TOKEN")
        || u.contains("SECRET")
        || u.contains("PASSWORD")
        || u.contains("PRIVATE")
        || u == "OPENAI_API_KEY"
        || u == "NOTION_API_KEY"
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
