use anyhow::Result;

fn main() -> Result<()> {
    let mut ctx = apogee::ContextEnv::new()?;
    let cfg = ctx.load_config()?;
    let rt = apogee::RuntimeEnv::build(&ctx, &cfg)?;

    let shell = ctx.shell_type.unwrap_or(cfg.apogee.default_shell);

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
