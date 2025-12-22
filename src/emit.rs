use crate::config::Shell;

#[derive(Debug, Clone, Copy)]
pub struct Emitter {
    shell: Shell,
}

impl Emitter {
    pub fn new(shell: Shell) -> Self {
        Self { shell }
    }

    pub fn header(&self, out: &mut String, title: &str) {
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
                out.push_str("alias ");
                out.push_str(name);
                out.push(' ');
                out.push_str(&quote_fish_single(&cmd));
                out.push('\n');
            }
            Shell::Pwsh => {
                out.push_str("function ");
                out.push_str(name);
                out.push_str(" { ");
                out.push_str(&cmd);
                out.push_str(" }\n");
            }
        }
    }

    pub fn init_eval_if_exists(
        &self,
        out: &mut String,
        cmd: &str,
        args: &[String],
        pwsh_out_string: bool,
    ) {
        let c = self.rewrite_value_for_shell(cmd);
        let args: Vec<String> = args
            .iter()
            .map(|a| self.rewrite_value_for_shell(a))
            .collect();

        let is_path = c.contains('/') || c.contains('\\');

        match self.shell {
            Shell::Zsh | Shell::Bash => {
                let words = posix_words(&c, &args);
                if is_path {
                    out.push_str("if [ -x ");
                    out.push_str(&quote_posix(&c));
                    out.push_str(" ]; then eval \"$(");
                    out.push_str(&words);
                    out.push_str(")\"; fi\n");
                } else {
                    out.push_str("if command -v ");
                    out.push_str(&c);
                    out.push_str(" >/dev/null 2>&1; then eval \"$(");
                    out.push_str(&words);
                    out.push_str(")\"; fi\n");
                }
            }

            Shell::Fish => {
                let words = fish_words(&c, &args);
                if is_path {
                    out.push_str("if test -x ");
                    out.push_str(&quote_fish(&c));
                    out.push_str("; ");
                    out.push_str(&words);
                    out.push_str(" | source; end\n");
                } else {
                    out.push_str("if type -q ");
                    out.push_str(&c);
                    out.push_str("; ");
                    out.push_str(&words);
                    out.push_str(" | source; end\n");
                }
            }

            Shell::Pwsh => {
                let words = pwsh_words(&c, &args);

                if is_path {
                    out.push_str("if (Test-Path -Path ");
                    out.push_str(&quote_pwsh(&c));
                    out.push_str(" -PathType Leaf) { ");
                } else {
                    out.push_str("if (Get-Command ");
                    out.push_str(&quote_pwsh(&c));
                    out.push_str(" -ErrorAction SilentlyContinue) { ");
                }

                if pwsh_out_string {
                    out.push_str("Invoke-Expression (& { (");
                    out.push_str(&words);
                    out.push_str(" | Out-String) })");
                } else {
                    out.push_str("Invoke-Expression (");
                    out.push_str(&words);
                    out.push(')');
                }

                out.push_str(" }\n");
            }
        }
    }

    pub fn path_append_if_exists(&self, out: &mut String, dir: &str) {
        let d = self.rewrite_value_for_shell(dir);

        match self.shell {
            Shell::Zsh | Shell::Bash => {
                out.push_str("if [ -d ");
                out.push_str(&quote_posix(&d));
                out.push_str(" ]; then __apogee_dir=");
                out.push_str(&quote_posix(&d));
                out.push_str("; case \":$PATH:\" in *\":$__apogee_dir:\"*) ;; *) export PATH=");
                out.push_str(&quote_posix("$PATH:$__apogee_dir"));
                out.push_str(" ;; esac; unset __apogee_dir; fi\n");
            }
            Shell::Fish => {
                out.push_str("if test -d ");
                out.push_str(&quote_fish(&d));
                out.push_str("; fish_add_path -g -a ");
                out.push_str(&quote_fish(&d));
                out.push_str("; end\n");
            }
            Shell::Pwsh => {
                out.push_str("if (Test-Path -Path ");
                out.push_str(&quote_pwsh(&d));
                out.push_str(" -PathType Container) { ");
                out.push_str("$sep = [IO.Path]::PathSeparator; ");
                out.push_str("$parts = $env:PATH -split [regex]::Escape($sep); ");
                out.push_str("if ($parts -notcontains ");
                out.push_str(&quote_pwsh(&d));
                out.push_str(") { $env:PATH = (@($env:PATH, ");
                out.push_str(&quote_pwsh(&d));
                out.push_str(") | Where-Object { $_ }) -join $sep } }\n");
            }
        }
    }

    pub fn path_prepend_if_exists(&self, out: &mut String, dir: &str) {
        let d = self.rewrite_value_for_shell(dir);

        match self.shell {
            Shell::Zsh | Shell::Bash => {
                out.push_str("if [ -d ");
                out.push_str(&quote_posix(&d));
                out.push_str(" ]; then __apogee_dir=");
                out.push_str(&quote_posix(&d));
                out.push_str("; case \":$PATH:\" in *\":$__apogee_dir:\"*) ;; *) export PATH=");
                out.push_str(&quote_posix("$__apogee_dir:$PATH"));
                out.push_str(" ;; esac; unset __apogee_dir; fi\n");
            }
            Shell::Fish => {
                out.push_str("if test -d ");
                out.push_str(&quote_fish(&d));
                out.push_str("; fish_add_path -g -p ");
                out.push_str(&quote_fish(&d));
                out.push_str("; end\n");
            }
            Shell::Pwsh => {
                out.push_str("if (Test-Path -Path ");
                out.push_str(&quote_pwsh(&d));
                out.push_str(" -PathType Container) { ");
                out.push_str("$sep = [IO.Path]::PathSeparator; ");
                out.push_str("$parts = $env:PATH -split [regex]::Escape($sep); ");
                out.push_str("if ($parts -notcontains ");
                out.push_str(&quote_pwsh(&d));
                out.push_str(") { $env:PATH = (@(");
                out.push_str(&quote_pwsh(&d));
                out.push_str(", $env:PATH) | Where-Object { $_ }) -join $sep } }\n");
            }
        }
    }

    pub fn source_if_exists(&self, out: &mut String, path: &str) {
        let p = self.rewrite_value_for_shell(path);

        match self.shell {
            Shell::Zsh | Shell::Bash => {
                out.push_str("if [ -r ");
                out.push_str(&quote_posix(&p));
                out.push_str(" ]; then source ");
                out.push_str(&quote_posix(&p));
                out.push_str("; fi\n");
            }
            Shell::Fish => {
                out.push_str("if test -r ");
                out.push_str(&quote_fish(&p));
                out.push_str("; source ");
                out.push_str(&quote_fish(&p));
                out.push_str("; end\n");
            }
            Shell::Pwsh => {
                out.push_str("if (Test-Path -Path ");
                out.push_str(&quote_pwsh(&p));
                out.push_str(" -PathType Leaf) { . ");
                out.push_str(&quote_pwsh(&p));
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
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '\\' | '"' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

fn quote_posix_single(s: &str) -> String {
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
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '\\' | '"' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

fn quote_fish_single(s: &str) -> String {
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
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '`' => out.push_str("``"),
            '"' => out.push_str("`\""),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

// -------------------- pwsh env rewrite --------------------

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

        if input[i..].starts_with("$env:") {
            out.push_str("$env:");
            i += 5;
            continue;
        }

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
            out.push('$');
            i += 1;
            continue;
        }

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

        out.push('$');
        i += 1;
    }

    out
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn is_valid_name(s: &str) -> bool {
    let mut it = s.bytes();
    match it.next() {
        Some(b) if b.is_ascii_alphabetic() || b == b'_' => {}
        _ => return false,
    }
    it.all(is_ident_char)
}

fn posix_words(cmd: &str, args: &[String]) -> String {
    let mut out = String::new();
    out.push_str(&quote_posix(cmd));
    for a in args {
        out.push(' ');
        out.push_str(&quote_posix(a));
    }
    out
}

fn fish_words(cmd: &str, args: &[String]) -> String {
    let mut out = String::new();
    out.push_str(&quote_fish(cmd));
    for a in args {
        out.push(' ');
        out.push_str(&quote_fish(a));
    }
    out
}

// returns a PowerShell invocation expression, using call operator &
// e.g. & "starship" "init" "powershell"
fn pwsh_words(cmd: &str, args: &[String]) -> String {
    let mut out = String::new();
    out.push_str("& ");
    out.push_str(&quote_pwsh(cmd));
    for a in args {
        out.push(' ');
        out.push_str(&quote_pwsh(a));
    }
    out
}
