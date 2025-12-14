use crate::{
  config::{AliasMap, Shell},
  context::Context,
};

use super::builder::ScriptBuilder;

pub fn emit_aliases(ctx: &Context, aliases: &AliasMap, out: &mut ScriptBuilder) {
  match ctx.shell {
    Shell::Zsh | Shell::Bash => {
      for (name, value) in aliases {
        let v = escape_single_quotes(value);
        out.push_line(&format!("alias {name}='{v}'"));
      }
    }
    Shell::Fish => {
      // Fish: use functions for reliability (works for complex commands).
      for (name, value) in aliases {
        // Note: not doing heavy quoting yet; keep aliases simple for now.
        out.push_line(&format!("function {name}; {value}; end"));
      }
    }
    Shell::Pwsh => {
      for (name, value) in aliases {
        out.push_line(&format!("function {name} {{ {value} }}"));
      }
    }
  }
}

fn escape_single_quotes(input: &str) -> String {
  input.replace('\'', r#"'"'"'"#)
}
