use anyhow::Result;

#[cfg_attr(debug_assertions, allow(unused_variables))]
fn main() -> Result<()> {
  let mut ctx = apogee::ContextEnv::new()?;

  let cfg = ctx.load_config()?;

  let runtime = apogee::RuntimeEnv::build(&ctx, &cfg)?;

  println!("{ctx}");
  println!();

  #[cfg(debug_assertions)]
  println!("{:#?}", cfg);

  // Optional: quick sanity check
  #[cfg(debug_assertions)]
  println!("\nruntime env keys: {}", runtime.vars.len());

  Ok(())
}
