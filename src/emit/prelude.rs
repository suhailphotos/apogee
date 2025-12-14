use crate::{config::Shell, context::Context};
use super::builder::ScriptBuilder;

pub fn emit_prelude(ctx: &Context, out: &mut ScriptBuilder) {
  match ctx.shell {
    Shell::Zsh | Shell::Bash => prelude_posix(ctx, out),
    Shell::Fish => prelude_fish(ctx, out),
    Shell::Pwsh => prelude_pwsh(ctx, out),
  }
}

fn prelude_posix(ctx: &Context, out: &mut ScriptBuilder) {
  out.push_line("# apogee generated (eval)");
  out.push_line(&format!("# shell={:?} platform={:?} host={:?}", ctx.shell, ctx.platform, ctx.hostname));
  out.push_line("");

  // Loaded-once guard (safe for eval/sourcing)
  out.push_line(r#"if [ -n "${APOGEE_LOADED-}" ]; then"#);
  out.push_line(r#"  return 0 2>/dev/null || exit 0"#);
  out.push_line(r#"fi"#);
  out.push_line(r#"export APOGEE_LOADED=1"#);
  out.push_line("");
}

fn prelude_fish(ctx: &Context, out: &mut ScriptBuilder) {
  out.push_line("# apogee generated (source)");
  out.push_line(&format!("# shell={:?} platform={:?} host={:?}", ctx.shell, ctx.platform, ctx.hostname));
  out.push_line("");

  out.push_line("if set -q APOGEE_LOADED");
  out.push_line("  return");
  out.push_line("end");
  out.push_line("set -gx APOGEE_LOADED 1");
  out.push_line("");
}

fn prelude_pwsh(ctx: &Context, out: &mut ScriptBuilder) {
  out.push_line("# apogee generated (iex)");
  out.push_line(&format!("# shell={:?} platform={:?} host={:?}", ctx.shell, ctx.platform, ctx.hostname));
  out.push_line("");

  out.push_line(r#"if ($env:APOGEE_LOADED) { return }"#);
  out.push_line(r#"$env:APOGEE_LOADED = "1""#);
  out.push_line("");
}
