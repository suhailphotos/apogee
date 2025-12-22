use anyhow::{bail, Result};
use std::collections::BTreeMap;

use crate::config::{Platform, Shell};
use crate::context::ContextEnv;

pub type DetectVars = BTreeMap<String, String>;

pub struct Resolver<'a> {
    pub ctx: &'a ContextEnv,
    pub env: &'a BTreeMap<String, String>,
    pub detect: Option<&'a DetectVars>,
}

impl<'a> Resolver<'a> {
    pub fn new(ctx: &'a ContextEnv, env: &'a BTreeMap<String, String>) -> Self {
        Self {
            ctx,
            env,
            detect: None,
        }
    }

    pub fn with_detect(mut self, detect: &'a DetectVars) -> Self {
        self.detect = Some(detect);
        self
    }

    pub fn resolve(&self, input: &str) -> Result<String> {
        // Fast path: no braces at all
        if !input.contains('{') && !input.contains('}') {
            return Ok(input.to_string());
        }

        // UTF-8 safe resolver:
        // - supports tokens: {name}
        // - supports escaping: "{{" -> "{", "}}" -> "}"
        // - leaves lone "}" untouched
        let bytes = input.as_bytes();
        let mut out = String::with_capacity(input.len() + 8);
        let mut i = 0usize;

        while i < bytes.len() {
            // Find next brace of either kind.
            let mut j = i;
            while j < bytes.len() && bytes[j] != b'{' && bytes[j] != b'}' {
                j += 1;
            }
            // Copy the intervening slice (UTF-8 safe because we only stop on ASCII braces).
            if j > i {
                out.push_str(&input[i..j]);
                i = j;
            }
            if i >= bytes.len() {
                break;
            }

            // Handle braces at bytes[i]
            if bytes[i] == b'{' {
                // Escaped literal "{"
                if i + 1 < bytes.len() && bytes[i + 1] == b'{' {
                    out.push('{');
                    i += 2;
                    continue;
                }

                // IMPORTANT: treat ${VAR} as literal shell syntax (do not token-expand)
                if i > 0 && bytes[i - 1] == b'$' {
                    // copy through the matching }
                    let start = i; // at '{'
                    let mut end = i + 1;
                    while end < bytes.len() && bytes[end] != b'}' {
                        end += 1;
                    }
                    if end < bytes.len() && bytes[end] == b'}' {
                        out.push_str(&input[start..=end]); // includes { ... }
                        i = end + 1;
                        continue;
                    }
                    // If it's "${" but unclosed, just treat "{" literally.
                    out.push('{');
                    i += 1;
                    continue;
                }

                // Token start: find closing "}"
                let start = i + 1;
                let mut end = start;
                while end < bytes.len() && bytes[end] != b'}' {
                    end += 1;
                }
                if end >= bytes.len() {
                    bail!("unclosed token in string: {input}");
                }

                let token = &input[start..end];
                if token.is_empty() {
                    bail!("empty token in string: {input}");
                }

                let repl = self
                    .token_value(token)
                    .ok_or_else(|| anyhow::anyhow!("unknown token: {{{token}}} in: {input}"))?;

                out.push_str(&repl);
                i = end + 1;
                continue;
            }

            // bytes[i] == b'}'
            // Escaped literal "}"
            if i + 1 < bytes.len() && bytes[i + 1] == b'}' {
                out.push('}');
                i += 2;
                continue;
            }

            // Lone "}" is literal
            out.push('}');
            i += 1;
        }

        Ok(out)
    }

    fn token_value(&self, token: &str) -> Option<String> {
        // detect.*
        if let Some(rest) = token.strip_prefix("detect.") {
            let det = self.detect?;
            return det.get(rest).cloned();
        }
        // Determine effective shell from runtime env first, then context fallback.
        let eff_shell: Option<Shell> = self
            .env
            .get("APOGEE_SHELL")
            .and_then(|s| Shell::parse(s))
            .or(self.ctx.shell_type);

        match token {
            "home" => Some(self.ctx.home.to_string_lossy().to_string()),
            "config_dir" => self
                .ctx
                .config_dir()
                .map(|p| p.to_string_lossy().to_string()),
            "config_path" => self
                .ctx
                .config_path()
                .map(|p| p.to_string_lossy().to_string()),
            "host" => Some(self.ctx.host().to_string()),
            "platform" => Some(self.ctx.platform.to_string()),
            "shell" => Some(
                eff_shell
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
            ),

            // Shell-specific extension (zsh|bash|fish|ps1)
            "shell_ext" => Some(shell_ext(eff_shell).to_string()),

            // Shell family (posix|fish|pwsh)
            "shell_family" => Some(shell_family(eff_shell).to_string()),

            // Shell-family extension (sh|fish|ps1)
            "shell_family_ext" => Some(shell_family_ext(eff_shell).to_string()),

            "xdg_config_home" => self
                .env
                .get("XDG_CONFIG_HOME")
                .cloned()
                .or_else(|| Some(self.ctx.xdg_config_home.to_string_lossy().to_string())),
            // init-arg for tools that want shell name (starship, zoxide, etc)
            // zsh|bash|fish|powershell
            "shell_init" => Some(match eff_shell {
                Some(Shell::Zsh) => "zsh".to_string(),
                Some(Shell::Bash) => "bash".to_string(),
                Some(Shell::Fish) => "fish".to_string(),
                Some(Shell::Pwsh) => "powershell".to_string(),
                None => "sh".to_string(),
            }),
            "xdg_cache_home" => self
                .env
                .get("XDG_CACHE_HOME")
                .cloned()
                .or_else(|| Some(default_xdg_cache_home(self.ctx.platform, &self.ctx.home))),
            "xdg_data_home" => self
                .env
                .get("XDG_DATA_HOME")
                .cloned()
                .or_else(|| Some(default_xdg_data_home(self.ctx.platform, &self.ctx.home))),
            "xdg_state_home" => self
                .env
                .get("XDG_STATE_HOME")
                .cloned()
                .or_else(|| Some(default_xdg_state_home(self.ctx.platform, &self.ctx.home))),

            "userprofile" => self
                .env
                .get("USERPROFILE")
                .cloned()
                .or_else(|| self.env.get("HOME").cloned()),
            "username" => self
                .env
                .get("USERNAME")
                .cloned()
                .or_else(|| self.env.get("USER").cloned()),

            _ => None,
        }
    }
}

fn shell_ext(sh: Option<Shell>) -> &'static str {
    match sh {
        Some(Shell::Zsh) => "zsh",
        Some(Shell::Bash) => "bash",
        Some(Shell::Fish) => "fish",
        Some(Shell::Pwsh) => "ps1",
        None => "sh",
    }
}

fn shell_family(sh: Option<Shell>) -> &'static str {
    match sh {
        Some(Shell::Fish) => "fish",
        Some(Shell::Pwsh) => "pwsh",
        Some(Shell::Zsh) | Some(Shell::Bash) | None => "posix",
    }
}

fn shell_family_ext(sh: Option<Shell>) -> &'static str {
    match sh {
        Some(Shell::Fish) => "fish",
        Some(Shell::Pwsh) => "ps1",
        Some(Shell::Zsh) | Some(Shell::Bash) | None => "sh",
    }
}

fn default_xdg_cache_home(_p: Platform, home: &std::path::Path) -> String {
    // Keep it simple and useful on mac/linux; Windows users typically set XDG_* explicitly.
    home.join(".cache").to_string_lossy().to_string()
}

fn default_xdg_data_home(_p: Platform, home: &std::path::Path) -> String {
    home.join(".local")
        .join("share")
        .to_string_lossy()
        .to_string()
}

fn default_xdg_state_home(_p: Platform, home: &std::path::Path) -> String {
    home.join(".local")
        .join("state")
        .to_string_lossy()
        .to_string()
}
