use anyhow::Result;

fn main() -> Result<()> {
  let mut ctx = apogee::ContextEnv::new()?;
  let cfg = ctx.load_config()?;
  let rt = apogee::RuntimeEnv::build(&ctx, &cfg)?;

  let shell = ctx.shell_type.unwrap_or(cfg.apogee.default_shell);

  let cloud_script = apogee::emit_cloud(&ctx, &rt, &cfg, shell)?;
  println!("{cloud_script}");

  Ok(())
}
