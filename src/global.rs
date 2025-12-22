use anyhow::Result;

use crate::{
    config::{Config, Platform, Shell},
    context::ContextEnv,
    emit::Emitter,
    runtime::RuntimeEnv,
};

pub fn emit_global(
    ctx: &ContextEnv,
    _rt: &RuntimeEnv,
    cfg: &Config,
    shell: Shell,
) -> Result<String> {
    let em = Emitter::new(shell);
    let mut out = String::new();
    em.header(&mut out, "apogee (global)");

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

    if platform_aliases.is_empty() && shell_aliases.is_empty() {
        return Ok(String::new());
    }

    for (k, v) in platform_aliases {
        em.alias(&mut out, k, v);
    }

    for (k, v) in shell_aliases {
        em.alias(&mut out, k, v);
    }

    Ok(out)
}
