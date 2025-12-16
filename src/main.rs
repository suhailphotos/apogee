use anyhow::Result;

fn main() -> Result<()> {
  let mut ctx = apogee::ContextEnv::new()?;

  // This sets config_path/config_dir and loads the file.
  let cfg = ctx.load_config()?;

  println!("{ctx}");
  println!();
  println!("{:#?}", cfg);

  Ok(())
}
