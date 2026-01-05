// src/init.rs

use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

const MARK_BEGIN: &str = "# >>> apogee >>>";
const MARK_END: &str = "# <<< apogee <<<";

fn home_dir() -> PathBuf {
    // Minimal + cross-platform-ish without extra deps.
    // Good enough for mac/linux. Windows: USERPROFILE.
    if let Ok(h) = env::var("HOME") {
        return PathBuf::from(h);
    }
    if let Ok(up) = env::var("USERPROFILE") {
        return PathBuf::from(up);
    }
    PathBuf::from(".")
}

fn xdg_config_home() -> PathBuf {
    if let Ok(xdg) = env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg);
    }
    home_dir().join(".config")
}

fn apogee_config_dir() -> PathBuf {
    xdg_config_home().join("apogee")
}

fn detect_shell_family() -> String {
    // Priority:
    // 1) APOGEE_SHELL (explicit)
    // 2) SHELL basename (unix)
    // 3) heuristics for pwsh
    if let Ok(s) = env::var("APOGEE_SHELL") {
        return s;
    }

    if env::var("PSModulePath").is_ok() || env::var("PROMPT").is_ok() && env::var("SHELL").is_err() {
        // very rough; ok for "ran from pwsh" scenarios
        return "pwsh".to_string();
    }

    if let Ok(shell) = env::var("SHELL") {
        let name = shell.rsplit('/').next().unwrap_or(shell.as_str());
        return name.to_string(); // "zsh", "bash", "fish", ...
    }

    "zsh".to_string()
}

fn rc_file_for_shell(shell: &str) -> Option<PathBuf> {
    let home = home_dir();
    let xdg = xdg_config_home();

    match shell {
        "zsh" => Some(home.join(".zshrc")),
        "bash" => Some(home.join(".bashrc")),
        "fish" => Some(xdg.join("fish").join("config.fish")),
        // PowerShell profile is not a single fixed location.
        // We'll try the common path for pwsh on mac/linux:
        "pwsh" | "powershell" => {
            // ~/.config/powershell/Microsoft.PowerShell_profile.ps1 (pwsh)
            // also used on linux; on mac it’s common too.
            Some(xdg.join("powershell").join("Microsoft.PowerShell_profile.ps1"))
        }
        _ => None,
    }
}

fn hook_block(shell: &str) -> String {
    match shell {
        "zsh" | "bash" => format!(
            r#"{begin}
if command -v apogee >/dev/null 2>&1; then
  eval "$(APOGEE_SHELL={shell} apogee)"
fi
{end}
"#,
            begin = MARK_BEGIN,
            end = MARK_END,
            shell = shell
        ),

        "fish" => format!(
            r#"{begin}
if type -q apogee
  env APOGEE_SHELL=fish apogee | source
end
{end}
"#,
            begin = MARK_BEGIN,
            end = MARK_END
        ),

        "pwsh" | "powershell" => format!(
            r#"{begin}
if (Get-Command apogee -ErrorAction SilentlyContinue) {{
  $env:APOGEE_SHELL = "pwsh"
  (& apogee) | Out-String | Invoke-Expression
}}
{end}
"#,
            begin = MARK_BEGIN,
            end = MARK_END
        ),

        _ => format!(
            r#"{begin}
# Unknown shell. You can manually add:
#   eval "$(APOGEE_SHELL=<shell> apogee)"
{end}
"#,
            begin = MARK_BEGIN,
            end = MARK_END
        ),
    }
}

fn file_contains_markers(s: &str) -> bool {
    s.contains(MARK_BEGIN) && s.contains(MARK_END)
}

fn append_hook_if_missing(rc_path: &Path, block: &str) -> std::io::Result<()> {
    let existing = fs::read_to_string(rc_path).unwrap_or_default();
    if file_contains_markers(&existing) {
        return Ok(());
    }

    // Ensure parent dir exists (fish/pwsh profiles)
    if let Some(parent) = rc_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(rc_path)?;

    if !existing.ends_with('\n') && !existing.is_empty() {
        writeln!(f)?;
    }
    write!(f, "{block}")?;
    Ok(())
}

pub fn run() -> anyhow::Result<()> {
    let shell = detect_shell_family();
    let cfg_dir = apogee_config_dir();
    fs::create_dir_all(&cfg_dir)?;

    // Optional directories (nice starter structure)
    fs::create_dir_all(cfg_dir.join("functions"))?;
    fs::create_dir_all(cfg_dir.join("hooks"))?;
    fs::create_dir_all(cfg_dir.join("templates"))?;

    // Write starter config only if missing
    let cfg_path = cfg_dir.join("config.toml");
    if !cfg_path.exists() {
        let tmpl = include_str!("../assets/default_config.toml");
        fs::write(&cfg_path, tmpl)?;
        eprintln!("Created {}", cfg_path.display());
    } else {
        eprintln!("Config already exists: {}", cfg_path.display());
    }

    // Append shell hook
    if let Some(rc_path) = rc_file_for_shell(&shell) {
        let block = hook_block(&shell);
        append_hook_if_missing(&rc_path, &block)?;
        eprintln!("Updated {}", rc_path.display());
    } else {
        eprintln!("Could not determine rc file for shell '{shell}'.");
        eprintln!("Manually add: eval \"$(APOGEE_SHELL=<shell> apogee)\"");
    }

    eprintln!("Done. Restart your shell (or source your rc file).");
    Ok(())
}
