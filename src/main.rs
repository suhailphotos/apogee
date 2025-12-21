use anyhow::Result;
use std::collections::BTreeSet;

fn main() -> Result<()> {
    let mut ctx = apogee::ContextEnv::new()?;
    let cfg = ctx.load_config()?;

    let shell = ctx
        .vars
        .get("APOGEE_SHELL")
        .and_then(|s| apogee::Shell::parse(s))
        .or(ctx.shell_type)
        .unwrap_or(cfg.apogee.default_shell);

    ctx.shell_type = Some(shell);
    ctx.vars
        .insert("APOGEE_SHELL".to_string(), shell.to_string());

    let rt0 = apogee::RuntimeEnv::build(&ctx, &cfg)?;

    let mut work = rt0.clone();
    let mut active: BTreeSet<String> = BTreeSet::new();

    let global_script = apogee::emit_global(&ctx, &work, &cfg, shell)?;

    // 1) CLOUD first
    let cloud_script = apogee::emit_cloud_seq(&ctx, &mut work, &cfg, shell, &mut active)?;

    // 2) APPS second
    let apps_script = apogee::emit_apps_seq(&ctx, &mut work, &cfg, shell, &mut active)?;

    // 3) HOOKS after apps
    let hooks_script = apogee::emit_hooks(&ctx, &work, &cfg, shell)?;

    // 4) TEMPLATES last
    let templates_script =
        apogee::emit_templates_with_active(&ctx, &work, &cfg, shell, &mut active)?;

    // Stitch output with clean spacing
    let mut out = String::new();

    if !global_script.trim().is_empty() {
        out.push_str(&global_script);
    }

    if !cloud_script.trim().is_empty() {
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&cloud_script);
    }

    if !apps_script.trim().is_empty() {
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&apps_script);
    }

    if !hooks_script.trim().is_empty() {
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&hooks_script);
    }

    if !templates_script.trim().is_empty() {
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&templates_script);
    }

    print!("{out}");
    Ok(())
}
