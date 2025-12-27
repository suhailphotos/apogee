use crate::{
    config::{Config, SecretsStrategy, Shell},
    context::ContextEnv,
    emit::Emitter,
    resolve::Resolver,
};
use anyhow::{Context as _, Result};
use std::{collections::BTreeMap, fs, path::Path};

#[derive(Debug, Clone)]
pub struct RuntimeEnv {
    pub vars: BTreeMap<String, String>,
}

impl RuntimeEnv {
    pub fn build(ctx: &ContextEnv, cfg: &Config) -> Result<Self> {
        let mut vars = ctx.vars.clone();

        // Apply bootstrap defaults (always fill-missing)
        if let Some(bootstrap) = cfg.apogee.bootstrap.as_ref() {
            for (k, v) in bootstrap.defaults.env.iter() {
                let missing =
                    !vars.contains_key(k) || vars.get(k).map(|s| s.is_empty()).unwrap_or(true);
                if !missing {
                    continue;
                }

                let resolved = {
                    let r = Resolver::new(ctx, &vars);
                    r.resolve(v)
                        .with_context(|| format!("failed to resolve bootstrap env value for {k}"))?
                };

                vars.insert(k.clone(), resolved);
            }
        }

        // Strategy for env file merges
        let strategy = cfg
            .apogee
            .bootstrap
            .as_ref()
            .map(|b| b.secrets.strategy)
            .unwrap_or(SecretsStrategy::FillMissing);

        // env_file default
        let env_file_raw = cfg
            .apogee
            .env_file
            .as_deref()
            .unwrap_or("{config_dir}/.env");

        let r = Resolver::new(ctx, &vars);
        let env_file = r
            .resolve(env_file_raw)
            .with_context(|| format!("failed to resolve apogee.env_file: {env_file_raw}"))?;

        self::merge_env_file(ctx, &mut vars, Path::new(&env_file), strategy)?;

        // secrets_file (optional)
        if let Some(secrets_raw) = cfg.apogee.secrets_file.as_deref() {
            let r2 = Resolver::new(ctx, &vars);
            let secrets_path = r2
                .resolve(secrets_raw)
                .with_context(|| format!("failed to resolve apogee.secrets_file: {secrets_raw}"))?;
            self::merge_env_file(ctx, &mut vars, Path::new(&secrets_path), strategy)?;
        }

        // Apply global env (resolved) into vars so downstream token resolution works.
        for (k, v_raw) in cfg.global.env.iter() {
            let r = Resolver::new(ctx, &vars);
            let v = r
                .resolve(v_raw)
                .with_context(|| format!("failed to resolve global env value for {k}"))?;
            vars.insert(k.clone(), v);
        }

        Ok(Self { vars })
    }
}

fn merge_env_file(
    ctx: &ContextEnv,
    vars: &mut BTreeMap<String, String>,
    path: &Path,
    strategy: SecretsStrategy,
) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    let text = fs::read_to_string(path)
        .with_context(|| format!("failed to read env file: {}", path.display()))?;

    let incoming = parse_env_text(&text)?;

    // Resolve tokens in incoming values (against current vars)
    let mut resolved_incoming = BTreeMap::new();
    for (k, v) in incoming {
        let r = Resolver::new(ctx, vars);
        let val = r.resolve(&v).with_context(|| {
            format!(
                "failed to resolve token(s) in env file {} for key {}",
                path.display(),
                k
            )
        })?;
        resolved_incoming.insert(k, val);
    }

    apply_strategy(vars, resolved_incoming, strategy);
    Ok(())
}

fn apply_strategy(
    dst: &mut BTreeMap<String, String>,
    src: BTreeMap<String, String>,
    strategy: SecretsStrategy,
) {
    match strategy {
        SecretsStrategy::FillMissing => {
            for (k, v) in src {
                if !dst.contains_key(&k) || dst.get(&k).map(|s| s.is_empty()).unwrap_or(true) {
                    dst.insert(k, v);
                }
            }
        }
        SecretsStrategy::Override => {
            for (k, v) in src {
                dst.insert(k, v);
            }
        }
    }
}

fn parse_env_text(text: &str) -> Result<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();

    for (idx, line) in text.lines().enumerate() {
        let mut s = line.trim();
        if s.is_empty() || s.starts_with('#') {
            continue;
        }

        if let Some(rest) = s.strip_prefix("export ") {
            s = rest.trim();
        }

        let (k, v) = s.split_once('=').with_context(|| {
            format!(
                "invalid env line {} (expected KEY=VALUE): {}",
                idx + 1,
                line
            )
        })?;

        let key = k.trim().to_string();
        if key.is_empty() {
            continue;
        }

        let mut val = v.trim().to_string();

        // Remove surrounding quotes (simple)
        if val.len() >= 2 {
            let bytes = val.as_bytes();
            let first = bytes[0];
            let last = bytes[bytes.len() - 1];
            if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
                val = val[1..val.len() - 1].to_string();
            }
        }
        out.insert(key, val);
    }

    Ok(out)
}

pub fn emit_env_delta(
    shell: Shell,
    before: &BTreeMap<String, String>,
    after: &BTreeMap<String, String>,
) -> String {
    let em = Emitter::new(shell);
    let mut out = String::new();
    em.header(&mut out, "apogee (dotenv)");

    let mut emitted_any = false;

    for (k, v_after) in after.iter() {
        let v_before = before.get(k);

        // emit if missing OR different
        if v_before.map(|s| s.as_str()) != Some(v_after.as_str()) {
            emitted_any = true;
            em.set_env(&mut out, k, v_after);
        }
    }

    if emitted_any {
        out
    } else {
        String::new()
    }
}
