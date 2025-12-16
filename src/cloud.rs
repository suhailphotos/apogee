use anyhow::{Context as _, Result};
use regex::Regex;
use std::{
  collections::{BTreeMap, BTreeSet, VecDeque},
  fs,
  path::{Path, PathBuf},
};

use crate::{
  config::{CloudModule, Config, EmitBlock, Platform, Shell},
  context::ContextEnv,
  resolve::{DetectVars, Resolver},
  runtime::RuntimeEnv,
};

#[derive(Debug, Clone)]
pub struct DetectedCloud {
  pub name: String,
  pub detect: DetectVars, // e.g. { "path": "/Users/..../Dropbox" }
  pub module: CloudModule,
}

pub fn detect_cloud_modules(ctx: &ContextEnv, rt: &RuntimeEnv, cfg: &Config) -> Result<Vec<DetectedCloud>> {
  if !cfg.modules.enable_cloud || !cfg.modules.cloud.enabled {
    return Ok(vec![]);
  }

  let mut out = Vec::new();
  for (name, m) in cfg.modules.cloud.items.iter() {
    if !m.enabled {
      continue;
    }
    if !module_supports_platform(m, ctx.platform) {
      continue;
    }

    if let Some(det) = detect_one_cloud(ctx, rt, name, m)? {
      out.push(det);
    }
  }

  Ok(out)
}

fn module_supports_platform(m: &CloudModule, p: Platform) -> bool {
  m.platforms.is_empty() || m.platforms.iter().any(|x| *x == p)
}

fn detect_one_cloud(ctx: &ContextEnv, rt: &RuntimeEnv, name: &str, m: &CloudModule) -> Result<Option<DetectedCloud>> {
  // 1) env detection: if any env var listed is present, treat as active and use its value as detect.path
  if let Some((_, val)) = first_present_env(&rt.vars, &m.detect.env.any_of) {
    let mut detect = DetectVars::new();
    detect.insert("path".to_string(), val);
    return Ok(Some(DetectedCloud {
      name: name.to_string(),
      detect,
      module: m.clone(),
    }));
  }

  // 2) path detection: first existing match wins
  let candidates = platform_any_of(&m.detect.paths, ctx.platform);
  for raw in candidates {
    let r = Resolver::new(ctx, &rt.vars);
    let resolved = r
      .resolve(raw)
      .with_context(|| format!("cloud.{name}: failed to resolve detect path pattern: {raw}"))?;

    if let Some(found) = first_path_match(&resolved)? {
      let mut detect = DetectVars::new();
      detect.insert("path".to_string(), found);
      return Ok(Some(DetectedCloud {
        name: name.to_string(),
        detect,
        module: m.clone(),
      }));
    }
  }

  // no match => inactive
  Ok(None)
}

fn platform_any_of(block: &crate::config::PlatformAnyOf, p: Platform) -> &Vec<String> {
  match p {
    Platform::Mac => &block.mac.any_of,
    Platform::Linux => &block.linux.any_of,
    Platform::Windows => &block.windows.any_of,
    Platform::Wsl => &block.wsl.any_of,
    Platform::Other => &block.other.any_of,
  }
}

/// Supports plain paths and simple globs like "/Applications/Houdini*.app" or "/opt/hfs*".
/// Returns the FIRST match (full path string).
fn first_path_match(pattern: &str) -> Result<Option<String>> {
  // No glob characters => just exists()
  if !pattern.contains('*') && !pattern.contains('?') {
    return Ok(Path::new(pattern).exists().then(|| pattern.to_string()));
  }

  // glob on last segment: <dir>/<name_glob>
  let (dir, glob) = split_dir_and_glob(pattern);
  let dir_path = Path::new(&dir);
  if !dir_path.exists() || !dir_path.is_dir() {
    return Ok(None);
  }

  let re = glob_to_regex(&glob)?;
  let mut entries = fs::read_dir(dir_path)
    .with_context(|| format!("failed to read_dir for glob: {pattern}"))?
    .filter_map(|e| e.ok())
    .collect::<Vec<_>>();

  // deterministic
  entries.sort_by_key(|e| e.file_name());

  for e in entries {
    let fname = e.file_name().to_string_lossy().to_string();
    if re.is_match(&fname) {
      let full = e.path().to_string_lossy().to_string();
      return Ok(Some(full));
    }
  }

  Ok(None)
}

fn split_dir_and_glob(p: &str) -> (String, String) {
  let pb = PathBuf::from(p);
  let dir = pb.parent().map(|x| x.to_string_lossy().to_string()).unwrap_or_else(|| ".".to_string());
  let glob = pb.file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or_else(|| p.to_string());
  (dir, glob)
}

fn glob_to_regex(glob: &str) -> Result<Regex> {
  // escape regex meta, then replace glob tokens
  let mut s = String::new();
  s.push('^');
  for ch in glob.chars() {
    match ch {
      '*' => s.push_str(".*"),
      '?' => s.push('.'),
      '.' | '+' | '(' | ')' | '|' | '^' | '$' | '{' | '}' | '[' | ']' | '\\' => {
        s.push('\\');
        s.push(ch);
      }
      _ => s.push(ch),
    }
  }
  s.push('$');
  Ok(Regex::new(&s)?)
}

fn first_present_env(vars: &BTreeMap<String, String>, keys: &[String]) -> Option<(String, String)> {
  for k in keys {
    if let Some(v) = vars.get(k).map(|s| s.trim()).filter(|s| !s.is_empty()) {
      return Some((k.clone(), v.to_string()));
    }
  }
  None
}

// --------------------- EMIT (cloud only) ---------------------

pub fn emit_cloud(ctx: &ContextEnv, rt: &RuntimeEnv, cfg: &Config, shell: Shell) -> Result<String> {
  let detected = detect_cloud_modules(ctx, rt, cfg)?;

  let mut out = String::new();
  if detected.is_empty() {
    return Ok(out);
  }

  out.push_str(&emit_prelude(shell));

  for d in detected {
    out.push_str(&format!("\n# --- cloud: {} ---\n", d.name));
    out.push_str(&emit_cloud_module(ctx, rt, shell, &d.detect, &d.module.emit)?);
  }

  Ok(out)
}

fn emit_prelude(shell: Shell) -> String {
  match shell {
    Shell::Zsh | Shell::Bash => "# apogee (cloud)\n".to_string(),
    Shell::Fish => "# apogee (cloud)\n".to_string(),
    Shell::Pwsh => "# apogee (cloud)\n".to_string(),
  }
}

fn emit_cloud_module(ctx: &ContextEnv, rt: &RuntimeEnv, shell: Shell, detect: &DetectVars, emit: &EmitBlock) -> Result<String> {
  let mut out = String::new();

  let r = Resolver::new(ctx, &rt.vars).with_detect(detect);

  // Combine env + env_derived into one assignment map (tokens resolved, $VARS preserved)
  let mut assigns: BTreeMap<String, String> = BTreeMap::new();

  for (k, v) in emit.env.iter() {
    let resolved = r.resolve(v)?;
    assigns.insert(k.clone(), adapt_value_for_shell(shell, &resolved));
  }
  for (k, v) in emit.env_derived.iter() {
    let resolved = r.resolve(v)?;
    assigns.insert(k.clone(), adapt_value_for_shell(shell, &resolved));
  }

  // Emit env exports in dependency order (based on $VAR refs)
  let ordered = order_env_assignments(&assigns, shell);

  for (k, v) in ordered {
    out.push_str(&emit_set_env(shell, &k, &v));
    out.push('\n');
  }

  // Aliases (posix + fish for now; pwsh TODO)
  if !emit.aliases.is_empty() {
    out.push('\n');
    for (name, raw) in emit.aliases.iter() {
      let val = r.resolve(raw)?;
      out.push_str(&emit_alias(shell, name, &val));
      out.push('\n');
    }
  }

  // PATH mods (emit runtime checks, not Rust-side expansion)
  if !emit.paths.prepend_if_exists.is_empty() || !emit.paths.append_if_exists.is_empty() {
    out.push('\n');
    for p in emit.paths.prepend_if_exists.iter() {
      let s = adapt_value_for_shell(shell, &r.resolve(p)?);
      out.push_str(&emit_path_prepend(shell, &s));
      out.push('\n');
    }
    for p in emit.paths.append_if_exists.iter() {
      let s = adapt_value_for_shell(shell, &r.resolve(p)?);
      out.push_str(&emit_path_append(shell, &s));
      out.push('\n');
    }
  }

  Ok(out)
}

fn adapt_value_for_shell(shell: Shell, v: &str) -> String {
  match shell {
    Shell::Pwsh => adapt_env_refs_for_pwsh(v),
    _ => v.to_string(),
  }
}

/// Convert "$FOO" or "${FOO}" into "$env:FOO" for PowerShell.
fn adapt_env_refs_for_pwsh(v: &str) -> String {
  // very small, predictable rewrite
  let re = Regex::new(r"\$\{([A-Za-z_][A-Za-z0-9_]*)\}|\$([A-Za-z_][A-Za-z0-9_]*)").unwrap();
  re.replace_all(v, |caps: &regex::Captures| {
    let name = caps.get(1).or_else(|| caps.get(2)).unwrap().as_str();
    format!("$env:{name}")
  })
  .to_string()
}

fn emit_set_env(shell: Shell, k: &str, v: &str) -> String {
  match shell {
    Shell::Zsh | Shell::Bash => format!("export {}=\"{}\"", k, sh_escape_double(v)),
    Shell::Fish => format!("set -gx {} \"{}\"", k, sh_escape_double(v)),
    Shell::Pwsh => format!("$env:{} = \"{}\"", k, sh_escape_double(v)),
  }
}

fn emit_alias(shell: Shell, name: &str, body: &str) -> String {
  match shell {
    Shell::Zsh | Shell::Bash => format!("alias {}={}", name, sh_quote_single(body)),
    Shell::Fish => format!("alias {} {}", name, sh_quote_single(body)),
    Shell::Pwsh => {
      // TODO: real pwsh alias handling (Set-Alias can't take args; you'd want a function)
      format!("# TODO(pwsh): alias {name} -> {body}")
    }
  }
}

fn emit_path_prepend(shell: Shell, p: &str) -> String {
  match shell {
    Shell::Zsh | Shell::Bash => {
      let q = sh_escape_double(p);
      format!("if [ -d \"{q}\" ]; then export PATH=\"{q}:$PATH\"; fi")
    }
    Shell::Fish => {
      let q = sh_escape_double(p);
      format!("if test -d \"{q}\"; fish_add_path -g -p \"{q}\"; end")
    }
    Shell::Pwsh => {
      let q = sh_escape_double(p);
      // simplistic PATH prepend
      format!("if (Test-Path \"{q}\") {{ $env:PATH = \"{q};\" + $env:PATH }}")
    }
  }
}

fn emit_path_append(shell: Shell, p: &str) -> String {
  match shell {
    Shell::Zsh | Shell::Bash => {
      let q = sh_escape_double(p);
      format!("if [ -d \"{q}\" ]; then export PATH=\"$PATH:{q}\"; fi")
    }
    Shell::Fish => {
      let q = sh_escape_double(p);
      format!("if test -d \"{q}\"; fish_add_path -g \"{q}\"; end")
    }
    Shell::Pwsh => {
      let q = sh_escape_double(p);
      format!("if (Test-Path \"{q}\") {{ $env:PATH = $env:PATH + \";{q}\" }}")
    }
  }
}

// ---------------- ordering ----------------

fn order_env_assignments(assigns: &BTreeMap<String, String>, shell: Shell) -> Vec<(String, String)> {
  let keys: BTreeSet<String> = assigns.keys().cloned().collect();

  // deps[k] = vars this k depends on (within this assigns set)
  let mut deps: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
  let mut indeg: BTreeMap<String, usize> = BTreeMap::new();

  for (k, v) in assigns.iter() {
    let ds = extract_deps(v, shell)
      .into_iter()
      .filter(|d| keys.contains(d))
      .collect::<BTreeSet<_>>();

    deps.insert(k.clone(), ds);
    indeg.insert(k.clone(), 0);
  }

  // compute indegree
  for (k, ds) in deps.iter() {
    let _ = k;
    *indeg.get_mut(k).unwrap() += ds.len();
  }

  // queue nodes with indegree 0
  let mut q = VecDeque::new();
  for (k, n) in indeg.iter() {
    if *n == 0 {
      q.push_back(k.clone());
    }
  }

  let mut ordered = Vec::new();

  while let Some(n) = q.pop_front() {
    ordered.push(n.clone());

    // reduce indegree of nodes that depend on n
    for (k, ds) in deps.iter() {
      if ds.contains(&n) {
        let e = indeg.get_mut(k).unwrap();
        *e -= 1;
        if *e == 0 {
          q.push_back(k.clone());
        }
      }
    }
  }

  // cycle fallback: append remaining in lexical order
  if ordered.len() != assigns.len() {
    for k in assigns.keys() {
      if !ordered.contains(k) {
        ordered.push(k.clone());
      }
    }
  }

  ordered
    .into_iter()
    .map(|k| (k.clone(), assigns.get(&k).cloned().unwrap_or_default()))
    .collect()
}

fn extract_deps(v: &str, shell: Shell) -> Vec<String> {
  match shell {
    Shell::Pwsh => {
      // $env:FOO or ${env:FOO}
      let re = Regex::new(r"\$env:([A-Za-z_][A-Za-z0-9_]*)|\$\{env:([A-Za-z_][A-Za-z0-9_]*)\}").unwrap();
      re.captures_iter(v)
        .filter_map(|c| c.get(1).or_else(|| c.get(2)).map(|m| m.as_str().to_string()))
        .collect()
    }
    _ => {
      // $FOO or ${FOO}
      let re = Regex::new(r"\$([A-Za-z_][A-Za-z0-9_]*)|\$\{([A-Za-z_][A-Za-z0-9_]*)\}").unwrap();
      re.captures_iter(v)
        .filter_map(|c| c.get(1).or_else(|| c.get(2)).map(|m| m.as_str().to_string()))
        .collect()
    }
  }
}

// ---------------- quoting helpers ----------------

fn sh_escape_double(s: &str) -> String {
  s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn sh_quote_single(s: &str) -> String {
  // 'foo'"'"'bar' trick
  if !s.contains('\'') {
    return format!("'{s}'");
  }
  let mut out = String::from("'");
  for ch in s.chars() {
    if ch == '\'' {
      out.push_str("'\"'\"'");
    } else {
      out.push(ch);
    }
  }
  out.push('\'');
  out
}
