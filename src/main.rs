use anyhow::Result;

fn main() -> Result<()> {
  let mut ctx = apogee::ContextEnv::new()?;

  // Sets config_path/config_dir and loads the file.
  let cfg = ctx.load_config()?;

  // Build runtime env (bootstrap defaults + .env + secrets_file)
  // let runtime = apogee::RuntimeEnv::build(&ctx, &cfg)?;
  let _runtime = apogee::RuntimeEnv::build(&ctx, &cfg)?;
  println!("{ctx}");
  println!();
  println!("{:#?}", cfg);

  // Optional: quick sanity check
  // println!("\nruntime env keys: {}", runtime.vars.len());

  Ok(())
}
