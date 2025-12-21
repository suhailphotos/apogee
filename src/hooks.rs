use anyhow::{Context as _, Result};

use crate::{
    config::{Config, Shell},
    context::ContextEnv,
    emit::Emitter,
    resolve::Resolver,
    runtime::RuntimeEnv,
};

pub fn emit_hooks(ctx: &ContextEnv, rt: &RuntimeEnv, cfg: &Config, shell: Shell) -> Result<String> {
    if !cfg.modules.enable_hooks || !cfg.modules.hooks.enabled {
        return Ok(String::new());
    }

    let em = Emitter::new(shell);
    let mut out = String::new();
    em.header(&mut out, "apogee (hooks)");

    let mut emitted_any = false;

    for h in cfg.modules.hooks.items.iter() {
        if !h.enabled {
            continue;
        }

        if !h.platforms.is_empty() && !h.platforms.contains(&ctx.platform) {
            continue;
        }

        if !h.hosts.is_empty() && !h.hosts.iter().any(|x| x == ctx.host()) {
            continue;
        }

        if !h.shells.is_empty() && !h.shells.contains(&shell) {
            continue;
        }

        let r = Resolver::new(ctx, &rt.vars);
        let script = r
            .resolve(&h.script)
            .with_context(|| format!("hooks.{}: failed to resolve script path", h.name))?;

        em.comment(&mut out, &format!("--- hook: {} ---", h.name));
        em.source_if_exists(&mut out, &script);
        em.blank(&mut out);

        emitted_any = true;
    }

    if !emitted_any {
        return Ok(String::new());
    }

    Ok(out)
}
