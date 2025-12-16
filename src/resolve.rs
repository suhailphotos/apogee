use anyhow::{bail, Result};
use std::collections::BTreeMap;

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
        // Fast path
        if !input.contains('{') {
            return Ok(input.to_string());
        }

        let mut out = String::with_capacity(input.len());
        let bytes = input.as_bytes();
        let mut i = 0;

        while i < bytes.len() {
            if bytes[i] == b'{' {
                let start = i + 1;
                let mut end = start;
                while end < bytes.len() && bytes[end] != b'}' {
                    end += 1;
                }
                if end >= bytes.len() {
                    bail!("unclosed token in string: {input}");
                }

                let token = &input[start..end];
                let repl = self
                    .token_value(token)
                    .ok_or_else(|| anyhow::anyhow!("unknown token: {{{token}}} in: {input}"))?;

                out.push_str(&repl);
                i = end + 1;
            } else {
                out.push(bytes[i] as char);
                i += 1;
            }
        }

        Ok(out)
    }

    fn token_value(&self, token: &str) -> Option<String> {
        // detect.*
        if let Some(rest) = token.strip_prefix("detect.") {
            let det = self.detect?;
            return det.get(rest).cloned();
        }

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
                self.ctx
                    .shell_type
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
            ),

            // Prefer runtime env for xdg so .env can override, but fall back to ctx
            "xdg_config_home" => self
                .env
                .get("XDG_CONFIG_HOME")
                .cloned()
                .or_else(|| Some(self.ctx.xdg_config_home.to_string_lossy().to_string())),
            "xdg_cache_home" => self.env.get("XDG_CACHE_HOME").cloned(),
            "xdg_data_home" => self.env.get("XDG_DATA_HOME").cloned(),
            "xdg_state_home" => self.env.get("XDG_STATE_HOME").cloned(),

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
