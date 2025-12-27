use anyhow::{Context as _, Result};

use crate::{
    config::{Config, Platform, Shell},
    context::ContextEnv,
    emit::Emitter,
    resolve::Resolver,
    runtime::RuntimeEnv,
};

pub fn emit_global(ctx: &ContextEnv, rt: &RuntimeEnv, cfg: &Config, shell: Shell) -> Result<String> {
    let em = Emitter::new(shell);
    let mut out = String::new();
    em.header(&mut out, "apogee (global)");

    // IMPORTANT: resolve against *runtime* vars (rt.vars), not ctx.vars
    let r = Resolver::new(ctx, &rt.vars);

    // -----------------------
    // global env (optional)
    // -----------------------
    for (k, v_raw) in cfg.global.env.iter() {
        let v = r
            .resolve(v_raw)
            .with_context(|| format!("failed to resolve global env {k}"))?;
        em.set_env(&mut out, k, &v);
    }

    // -----------------------
    // aliases
    // -----------------------

    // platform aliases
    let platform_aliases = match ctx.platform {
        Platform::Mac => &cfg.global.aliases.platform.mac,
        Platform::Linux => &cfg.global.aliases.platform.linux,
        Platform::Windows => &cfg.global.aliases.platform.windows,
        Platform::Wsl => &cfg.global.aliases.platform.wsl,
        Platform::Other => &cfg.global.aliases.platform.other,
    };

    // shell aliases
    let shell_aliases = match shell {
        Shell::Zsh => &cfg.global.aliases.shell.zsh,
        Shell::Bash => &cfg.global.aliases.shell.bash,
        Shell::Fish => &cfg.global.aliases.shell.fish,
        Shell::Pwsh => &cfg.global.aliases.shell.pwsh,
    };

    // If nothing emitted, return empty (so main.rs doesn't print the header)
    if cfg.global.env.is_empty() && platform_aliases.is_empty() && shell_aliases.is_empty() {
        return Ok(String::new());
    }

    for (k, v_raw) in platform_aliases {
        let v = r
            .resolve(v_raw)
            .with_context(|| format!("failed to resolve global alias {k}"))?;
        em.alias(&mut out, k, &v);
    }

    for (k, v_raw) in shell_aliases {
        let v = r
            .resolve(v_raw)
            .with_context(|| format!("failed to resolve global alias {k}"))?;
        em.alias(&mut out, k, &v);
    }

    Ok(out)
}
