use crate::config::{Platform, Shell};

#[derive(Debug, Clone, Copy)]
pub struct Emitter {
  shell: Shell,
  platform: Platform,
}

impl Emitter {
  pub fn new(shell: Shell, platform: Platform) -> Self {
    Self { shell, platform }
  }

  pub fn header(&self, out: &mut String, title: &str) {
    // `#` works for posix, fish, and pwsh
    out.push_str("# ");
    out.push_str(title);
    out.push('\n');
    out.push('\n');
  }

  pub fn comment(&self, out: &mut String, text: &str) {
    out.push_str("# ");
    out.push_str(text);
    out.push('\n');
  }

  pub fn blank(&self, out: &mut String) {
    out.push('\n');
  }

  pub fn set_env(&self, out: &mut String, key: &str, value: &str) {
    let v = self.rewrite_value_for_shell(value);

    match self.shell {
      Shell::Zsh | Shell::Bash => {
        out.push_str("export ");
        out.push_str(key);
        out.push('=');
        out.push_str(&quote_posix(&v));
        out.push('\n');
      }
      Shell::Fish => {
        out.push_str("set -gx ");
        out.push_str(key);
        out.push(' ');
        out.push_str(&quote_fish(&v));
        out.push('\n');
      }
      Shell::Pwsh => {
        out.push_str("$env:");
        out.push_str(key);
        out.push_str(" = ");
        out.push_str(&quote_pwsh(&v));
        out.push('\n');
      }
    }
  }

  pub fn alias(&self, out: &mut String, name: &str, command: &str) {
    let cmd = self.rewrite_value_for_shell(command);

    match self.shell {
      Shell::Zsh | Shell::Bash => {
        out.push_str("alias ");
        out.push_str(name);
        out.push('=');
        out.push_str(&quote_posix_single(&cmd));
        out.push('\n');
      }
      Shell::Fish => {
        // fish supports: alias ll 'eza -l'
        out.push_str("alias ");
        out.push_str(name);
        out.push(' ');
        out.push_str(&quote_fish_single(&cmd));
        out.push('\n');
      }
      Shell::Pwsh => {
        // Prefer a function (works for `cd`, complex pipelines, etc.)
        out.push_str("function ");
        out.push_str(name);
        out.push_str(" { ");
        out.push_str(&cmd);
        out.push_str(" }\n");
      }
    }
  }

  pub fn path_prepend_if_exists(&self, out: &mut String, dir: &str) {
    let d = self.rewrite_value_for_shell(dir);

    match self.shell {
      Shell::Zsh | Shell::Bash => {
        out.push_str("if [ -d ");
        out.push_str(&quote_posix(&d));
        out.push_str(" ]; then export PATH=");
        out.push_str(&quote_posix(&format!("{d}:$PATH")));
        out.push_str("; fi\n");
      }
      Shell::Fish => {
        // fish_add_path handles duplicates nicely
        out.push_str("if test -d ");
        out.push_str(&quote_fish(&d));
        out.push_str("; fish_add_path -g -p ");
        out.push_str(&quote_fish(&d));
        out.push_str("; end\n");
      }
      Shell::Pwsh => {
        let sep = path_sep(self.platform);
        out.push_str("if (Test-Path -Path ");
        out.push_str(&quote_pwsh(&d));
        out.push_str(" -PathType Container) { $env:Path = ");
        out.push_str(&quote_pwsh(&format!("{d}{sep}$env:Path")));
        out.push_str(" }\n");
      }
    }
  }

  fn rewrite_value_for_shell(&self, s: &str) -> String {
    match self.shell {
      Shell::Pwsh => rewrite_env_refs_for_pwsh(s),
      _ => s.to_string(),
    }
  }
}

// -------------------- quoting helpers --------------------

fn quote_posix(s: &str) -> String {
  // double-quote, escape backslash + double quote + $
  let mut out = String::with_capacity(s.len() + 2);
  out.push('"');
  for ch in s.chars() {
    match ch {
      '\\' | '"' | '$' => { out.push('\\'); out.push(ch); }
      _ => out.push(ch),
    }
  }
  out.push('"');
  out
}

fn quote_posix_single(s: &str) -> String {
  // single-quote, escape single quotes with: '\'' sequence
  let mut out = String::from("'");
  for ch in s.chars() {
    if ch == '\'' {
      out.push_str("'\\''");
    } else {
      out.push(ch);
    }
  }
  out.push('\'');
  out
}

fn quote_fish(s: &str) -> String {
  // double quotes are fine; escape backslash + double quote
  let mut out = String::with_capacity(s.len() + 2);
  out.push('"');
  for ch in s.chars() {
    match ch {
      '\\' | '"' => { out.push('\\'); out.push(ch); }
      _ => out.push(ch),
    }
  }
  out.push('"');
  out
}

fn quote_fish_single(s: &str) -> String {
  // fish: single quotes are literal; escape single quote by backslash
  let mut out = String::from("'");
  for ch in s.chars() {
    if ch == '\'' {
      out.push_str("\\'");
    } else {
      out.push(ch);
    }
  }
  out.push('\'');
  out
}

fn quote_pwsh(s: &str) -> String {
  // single quotes are literal in pwsh; escape ' as ''
  let mut out = String::from("'");
  for ch in s.chars() {
    if ch == '\'' {
      out.push_str("''");
    } else {
      out.push(ch);
    }
  }
  out.push('\'');
  out
}

fn path_sep(p: Platform) -> &'static str {
  match p {
    Platform::Windows => ";",
    _ => ":",
  }
}

// -------------------- pwsh env rewrite --------------------
// Turn $NAME and ${NAME} into $env:NAME.
// (We intentionally do NOT touch already-$env:... forms.)

fn rewrite_env_refs_for_pwsh(input: &str) -> String {
  let bytes = input.as_bytes();
  let mut out = String::with_capacity(input.len() + 8);
  let mut i = 0;

  while i < bytes.len() {
    if bytes[i] != b'$' {
      out.push(bytes[i] as char);
      i += 1;
      continue;
    }

    // already $env:...
    if input[i..].starts_with("$env:") {
      out.push_str("$env:");
      i += 5;
      continue;
    }

    // ${NAME}
    if i + 1 < bytes.len() && bytes[i + 1] == b'{' {
      let mut j = i + 2;
      while j < bytes.len() && bytes[j] != b'}' {
        j += 1;
      }
      if j < bytes.len() {
        let name = &input[i + 2..j];
        if is_valid_name(name) {
          out.push_str("$env:");
          out.push_str(name);
          i = j + 1;
          continue;
        }
      }
      // fallback: keep '$' as-is
      out.push('$');
      i += 1;
      continue;
    }

    // $NAME
    let mut j = i + 1;
    while j < bytes.len() && is_ident_char(bytes[j]) {
      j += 1;
    }
    let name = &input[i + 1..j];
    if is_valid_name(name) {
      out.push_str("$env:");
      out.push_str(name);
      i = j;
      continue;
    }

    // not a var ref
    out.push('$');
    i += 1;
  }

  out
}

fn is_ident_char(b: u8) -> bool {
  (b'A'..=b'Z').contains(&b)
    || (b'a'..=b'z').contains(&b)
    || (b'0'..=b'9').contains(&b)
    || b == b'_'
}

fn is_valid_name(s: &str) -> bool {
  let mut it = s.bytes();
  match it.next() {
    Some(b) if (b'A'..=b'Z').contains(&b) || (b'a'..=b'z').contains(&b) || b == b'_' => {}
    _ => return false,
  }
  it.all(is_ident_char)
}
