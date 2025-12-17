use anyhow::Result;

fn main() -> Result<()> {
    let mut ctx = apogee::ContextEnv::new()?;
    let cfg = ctx.load_config()?;
    // Determine the effective shell we are emitting for.
    // Precedence:
    // 1) APOGEE_SHELL (env/config override)
    // 2) detected ctx.shell_type
    // 3) config default
    let shell = ctx
        .vars
        .get("APOGEE_SHELL")
        .and_then(|s| apogee::Shell::parse(s))
        .or(ctx.shell_type)
        .unwrap_or(cfg.apogee.default_shell);

    // Make sure runtime build + token resolution can see the effective shell.
    ctx.shell_type = Some(shell);
    ctx.vars.insert("APOGEE_SHELL".to_string(), shell.to_string());

    let rt = apogee::RuntimeEnv::build(&ctx, &cfg)?;

    let cloud_script = apogee::emit_cloud(&ctx, &rt, &cfg, shell)?;
    let apps_script = apogee::emit_apps(&ctx, &rt, &cfg, shell)?;

    let mut out = String::new();
    if !cloud_script.trim().is_empty() {
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

    print!("{out}");
    Ok(())
}
