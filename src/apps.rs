use anyhow::{Context as _, Result};
use regex::Regex;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use crate::{
    config::{AppModule, Config, EmitBlock, Platform, PlatformAnyOf, Shell, VersionDetect, VersionDetectSpec},
    context::ContextEnv,
    emit::Emitter,
    resolve::{DetectVars, Resolver},
    runtime::RuntimeEnv,
};

#[derive(Debug, Clone)]
pub struct DetectedApp {
    pub name: String,
    pub detect: DetectVars, // detect.* tokens (ex: detect.path, detect.command, detect.version)
    pub module: AppModule,
}

pub fn detect_app_modules(ctx: &ContextEnv, rt: &RuntimeEnv, cfg: &Config) -> Result<Vec<DetectedApp>> {
    if !cfg.modules.enable_apps || !cfg.modules.apps.enabled {
        return Ok(vec![]);
    }

    let mut out = Vec::new();
    for (name, m) in cfg.modules.apps.items.iter() {
        if !m.enabled {
            continue;
        }
        if !module_supports_platform(m, ctx.platform) {
            continue;
        }

        if let Some(det) = detect_one_app(ctx, rt, name, m)? {
            out.push(det);
        }
    }

    Ok(out)
}

fn module_supports_platform(m: &AppModule, p: Platform) -> bool {
    m.platforms.is_empty() || m.platforms.contains(&p)
}

fn detect_one_app(ctx: &ContextEnv, rt: &RuntimeEnv, name: &str, m: &AppModule) -> Result<Option<DetectedApp>> {
    // We'll progressively fill detect vars, and consider the module "active"
    // as soon as any detection method matches.
    let mut detect = DetectVars::new();

    // 1) env detection (first present wins)
    if let Some((k, val)) = first_present_env(&rt.vars, &m.detect.env.any_of) {
        detect.insert("env".to_string(), k);
        detect.insert("path".to_string(), val); // convention: env value often *is* a path
        attach_version_if_any(ctx, rt, m.detect.version.as_ref(), &mut detect)?;
        return Ok(Some(DetectedApp {
            name: name.to_string(),
            detect,
            module: m.clone(),
        }));
    }

    // 2) command detection (first present wins)
    for raw in m.detect.commands.any_of.iter() {
        let r = Resolver::new(ctx, &rt.vars);
        let cmd = r
            .resolve(raw)
            .with_context(|| format!("apps.{name}: failed to resolve detect command: {raw}"))?;

        if let Some(found) = resolve_command(ctx.platform, &rt.vars, &cmd) {
            let dir = found.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();

            detect.insert("command".to_string(), cmd.clone());
            detect.insert("command_path".to_string(), found.to_string_lossy().to_string());
            detect.insert("command_dir".to_string(), dir);

            attach_version_if_any(ctx, rt, m.detect.version.as_ref(), &mut detect)?;
            return Ok(Some(DetectedApp {
                name: name.to_string(),
                detect,
                module: m.clone(),
            }));
        }
    }

    // 3) file detection (platform any_of + optional globs; first match wins)
    for raw in platform_any_of(&m.detect.files, ctx.platform).iter() {
        let r = Resolver::new(ctx, &rt.vars);
        let resolved = r
            .resolve(raw)
            .with_context(|| format!("apps.{name}: failed to resolve detect file pattern: {raw}"))?;

        if let Some(found) = first_path_match(&resolved)? {
            detect.insert("file".to_string(), found);
            attach_version_if_any(ctx, rt, m.detect.version.as_ref(), &mut detect)?;
            return Ok(Some(DetectedApp {
                name: name.to_string(),
                detect,
                module: m.clone(),
            }));
        }
    }

    // 4) path detection (platform any_of + optional globs; first match wins)
    for raw in platform_any_of(&m.detect.paths, ctx.platform).iter() {
        let r = Resolver::new(ctx, &rt.vars);
        let resolved = r
            .resolve(raw)
            .with_context(|| format!("apps.{name}: failed to resolve detect path pattern: {raw}"))?;

        if let Some(found) = first_path_match(&resolved)? {
            detect.insert("path".to_string(), found);
            attach_version_if_any(ctx, rt, m.detect.version.as_ref(), &mut detect)?;
            return Ok(Some(DetectedApp {
                name: name.to_string(),
                detect,
                module: m.clone(),
            }));
        }
    }

    Ok(None)
}

fn attach_version_if_any(
    ctx: &ContextEnv,
    rt: &RuntimeEnv,
    spec: Option<&VersionDetectSpec>,
    detect: &mut DetectVars,
) -> Result<()> {
    let Some(spec) = spec else { return Ok(()); };

    let Some(vd) = spec.for_platform(ctx.platform) else {
        return Ok(());
    };

    let v = detect_version(ctx, rt, detect, vd)?;
    if let Some(v) = v {
        detect.insert("version".to_string(), v);
    }

    Ok(())
}

fn detect_version(ctx: &ContextEnv, rt: &RuntimeEnv, detect: &DetectVars, vd: &VersionDetect) -> Result<Option<String>> {
    match vd {
        VersionDetect::Command { command, args, regex, capture } => {
            let r = Resolver::new(ctx, &rt.vars).with_detect(detect);

            let cmd = if let Some(p) = detect.get("command_path") {
                p.clone()
            } else {
                r.resolve(command)?
            };

            let mut resolved_args = Vec::with_capacity(args.len());
            for a in args {
                resolved_args.push(r.resolve(a)
                    .with_context(|| format!("failed to resolve version arg: {a}"))?);
            }

            let out = match Command::new(&cmd).args(&resolved_args).output() {
                Ok(o) => o,
                Err(_) => return Ok(None), // can't run command => no version
            };
            if !out.status.success() {
                return Ok(None);
            }

            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            let text = if !stdout.is_empty() { stdout } else { stderr };

            if text.is_empty() {
                return Ok(None);
            }

            if let Some(re_s) = regex.as_ref() {
                let re = Regex::new(re_s)
                    .with_context(|| format!("invalid version regex: {re_s}"))?;
                let caps = re.captures(&text);
                let Some(caps) = caps else { return Ok(None); };

                let m = caps
                    .name(capture)
                    .or_else(|| caps.get(1))
                    .map(|m| m.as_str().to_string());

                Ok(m)
            } else {
                Ok(text.lines().next().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()))
            }
        }

        VersionDetect::PathRegex { regex, capture } => {
            let target = detect
                .get("path")
                .or_else(|| detect.get("file"))
                .or_else(|| detect.get("command"));

            let Some(target) = target else { return Ok(None); };

            let re = Regex::new(regex)
                .with_context(|| format!("invalid path regex: {regex}"))?;

            let caps = re.captures(target);
            let Some(caps) = caps else { return Ok(None); };

            let m = caps
                .name(capture)
                .or_else(|| caps.get(1))
                .map(|m| m.as_str().to_string());

            Ok(m)
        }
    }
}

// --------------------- EMIT (apps only) ---------------------

pub fn emit_apps(ctx: &ContextEnv, rt: &RuntimeEnv, cfg: &Config, shell: Shell) -> Result<String> {
    if !cfg.modules.enable_apps || !cfg.modules.apps.enabled {
        return Ok(String::new());
    }

    let em = Emitter::new(shell);

    let mut out = String::new();
    em.header(&mut out, "apogee (apps)");

    // IMPORTANT: sequential pass with a mutable env map.
    // This ensures PATH updates from earlier modules affect later detect.commands.
    let mut work = rt.clone();

    // Deterministic order: priority, then name.
    let mut items: Vec<(&String, &AppModule)> = cfg.modules.apps.items.iter().collect();
    items.sort_by(|(an, am), (bn, bm)| am.priority.cmp(&bm.priority).then_with(|| an.cmp(bn)));

    let mut emitted_any = false;

    for (name, m) in items {
        if !m.enabled {
            continue;
        }
        if !module_supports_platform(m, ctx.platform) {
            continue;
        }

        // Detect using the *current* work.vars (which may already include earlier PATH/env).
        if let Some(det) = detect_one_app(ctx, &work, name, m)? {
            emitted_any = true;
            em.comment(&mut out, &format!("--- app: {} ---", det.name));
            emit_app_module_into(&em, &mut out, ctx, &work, &det.detect, &det.module.emit)?;

            // Mutate work.vars to reflect what we just emitted (env + PATH)
            // so the next module can detect against it.
            apply_emit_effects_to_runtime(ctx, &mut work, &det.detect, &det.module.emit)?;
            em.blank(&mut out);
        }
    }

    if !emitted_any {
        // keep output clean; if nothing active, return empty (no header)
        return Ok(String::new());
    }

    Ok(out)
}

fn emit_app_module_into(
    em: &Emitter,
    out: &mut String,
    ctx: &ContextEnv,
    rt: &RuntimeEnv,
    detect: &DetectVars,
    emit: &EmitBlock,
) -> Result<()> {
    let r = Resolver::new(ctx, &rt.vars).with_detect(detect);

    // Combine env + env_derived into one assignment map (tokens resolved, $VARS preserved)
    let mut assigns: BTreeMap<String, String> = BTreeMap::new();
    for (k, v) in emit.env.iter() {
        assigns.insert(k.clone(), r.resolve(v)?);
    }
    for (k, v) in emit.env_derived.iter() {
        assigns.insert(k.clone(), r.resolve(v)?);
    }

    // Emit env exports in dependency order (based on $VAR refs)
    for (k, v) in order_env_assignments(&assigns) {
        em.set_env(out, &k, &v);
    }

    // PATH mods (emit earlier so functions/init see tools on PATH)
    if !emit.paths.prepend_if_exists.is_empty() || !emit.paths.append_if_exists.is_empty() {
        em.blank(out);
        for p in emit.paths.prepend_if_exists.iter() {
            let s = r.resolve(p)?;
            em.path_prepend_if_exists(out, &s);
        }
        for p in emit.paths.append_if_exists.iter() {
            let s = r.resolve(p)?;
            em.path_append_if_exists(out, &s);
        }
    }


    // Functions (source external scripts)
    if !emit.functions.files.is_empty() {
        em.blank(out);

        let mut seen: BTreeSet<String> = BTreeSet::new();

        for raw in emit.functions.files.iter() {
            let p = r.resolve(raw)?;
            if seen.insert(p.clone()) {
                em.source_if_exists(out, &p);
            }
        }
    }

    // Source vendor scripts (completions, keybindings, etc.)
    if !emit.source.files.is_empty() {
        em.blank(out);

        let mut seen: BTreeSet<String> = BTreeSet::new();
        for raw in emit.source.files.iter() {
            let p = r.resolve(raw)?;
            if seen.insert(p.clone()) {
                em.source_if_exists(out, &p);
            }
        }
    }

    // Aliases
    if !emit.aliases.is_empty() {
        em.blank(out);
        for (name, raw) in emit.aliases.iter() {
            let val = r.resolve(raw)?;
            em.alias(out, name, &val);
        }
    }

    // Init commands (evaluate tool-provided shell code, e.g. starship/zoxide)
    if !emit.init.is_empty() {
        em.blank(out);

        for init in emit.init.iter() {
            let cmd = r.resolve(&init.command)?;
            let mut args = Vec::with_capacity(init.args.len());
            for a in init.args.iter() {
                args.push(r.resolve(a)?);
            }

            em.init_eval_if_exists(out, &cmd, &args, init.pwsh_out_string);
        }
    }
    Ok(())
}

fn apply_emit_effects_to_runtime(
    ctx: &ContextEnv,
    rt: &mut RuntimeEnv,
    detect: &DetectVars,
    emit: &EmitBlock,
) -> Result<()> {
    // -------- 1) ENV: resolve using a snapshot, then apply into rt.vars --------
    let snap1 = rt.vars.clone();
    let r1 = Resolver::new(ctx, &snap1).with_detect(detect);

    let mut assigns: BTreeMap<String, String> = BTreeMap::new();
    for (k, v) in emit.env.iter() {
        assigns.insert(k.clone(), r1.resolve(v)?);
    }
    for (k, v) in emit.env_derived.iter() {
        assigns.insert(k.clone(), r1.resolve(v)?);
    }

    for (k, v) in order_env_assignments(&assigns) {
        rt.vars.insert(k, v);
    }

    // -------- 2) PATH: resolve using a new snapshot (now includes env above) ---
    let snap2 = rt.vars.clone();
    let r2 = Resolver::new(ctx, &snap2).with_detect(detect);

    let sep = if matches!(ctx.platform, Platform::Windows) { ';' } else { ':' };
    let path_key_primary = if matches!(ctx.platform, Platform::Windows) { "Path" } else { "PATH" };

    let mut path_val = snap2
        .get(path_key_primary)
        .cloned()
        .or_else(|| snap2.get("PATH").cloned())
        .or_else(|| snap2.get("Path").cloned())
        .unwrap_or_default();

    let mut parts: Vec<String> = path_val
        .split(sep)
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();

    let mut have: BTreeSet<String> = parts.iter().cloned().collect();

    // prepend
    for raw in emit.paths.prepend_if_exists.iter() {
        let d = r2.resolve(raw)?;
        if d.is_empty() || !Path::new(&d).is_dir() {
            continue;
        }
        if have.insert(d.clone()) {
            parts.insert(0, d);
        }
    }

    // append
    for raw in emit.paths.append_if_exists.iter() {
        let d = r2.resolve(raw)?;
        if d.is_empty() || !Path::new(&d).is_dir() {
            continue;
        }
        if have.insert(d.clone()) {
            parts.push(d);
        }
    }

    path_val = parts.join(&sep.to_string());

    // keep both keys in sync (your existing behavior)
    rt.vars.insert("PATH".to_string(), path_val.clone());
    rt.vars.insert("Path".to_string(), path_val);

    Ok(())
}


// --------------------- helpers ---------------------

fn platform_any_of(block: &PlatformAnyOf, p: Platform) -> &Vec<String> {
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
    if !pattern.contains('*') && !pattern.contains('?') {
        return Ok(Path::new(pattern).exists().then(|| pattern.to_string()));
    }

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

fn resolve_command(platform: Platform, vars: &BTreeMap<String, String>, cmd: &str) -> Option<PathBuf> {
    // If it contains a path separator, treat as an explicit path.
    if cmd.contains('/') || cmd.contains('\\') {
        let p = PathBuf::from(cmd);
        return p.is_file().then_some(p);
    }

    // 1) Try PATH first (if any)
    if let Some(p) = resolve_on_path(platform, vars, cmd) {
        return Some(p);
    }

    // 2) Fallback: scan standard locations
    for dir in fallback_command_dirs(platform, vars) {
        if let Some(p) = resolve_in_dir(platform, vars, &dir, cmd) {
            return Some(p);
        }
    }

    None
}

fn resolve_on_path(platform: Platform, vars: &BTreeMap<String, String>, cmd: &str) -> Option<PathBuf> {
    let path_key = if matches!(platform, Platform::Windows) { "Path" } else { "PATH" };
    let path_val = vars.get(path_key)
        .or_else(|| vars.get("PATH"))
        .or_else(|| vars.get("Path"))
        .map(|s| s.as_str())
        .unwrap_or("");

    if path_val.trim().is_empty() {
        return None;
    }

    let sep = if matches!(platform, Platform::Windows) { ';' } else { ':' };
    for dir in path_val.split(sep).map(|s| s.trim()).filter(|s| !s.is_empty()) {
        let base = PathBuf::from(dir);
        if let Some(p) = resolve_in_dir(platform, vars, &base, cmd) {
            return Some(p);
        }
    }

    None
}

fn resolve_in_dir(platform: Platform, vars: &BTreeMap<String, String>, dir: &Path, cmd: &str) -> Option<PathBuf> {
    if !dir.is_dir() {
        return None;
    }

    if matches!(platform, Platform::Windows) {
        let exts = pathext_list(vars);

        // If user already provided an extension, try it as-is first.
        if cmd.contains('.') {
            let p = dir.join(cmd);
            return p.is_file().then_some(p);
        }

        for ext in exts {
            let p = dir.join(format!("{cmd}{ext}"));
            if p.is_file() {
                return Some(p);
            }
        }
        None
    } else {
        let p = dir.join(cmd);
        p.is_file().then_some(p)
    }
}

fn fallback_command_dirs(platform: Platform, vars: &BTreeMap<String, String>) -> Vec<PathBuf> {
    fn push(out: &mut Vec<PathBuf>, p: &str) {
        if !p.is_empty() {
            out.push(PathBuf::from(p));
        }
    }

    fn push_home(out: &mut Vec<PathBuf>, home: &str, suffix: &str) {
        if !home.is_empty() {
            out.push(PathBuf::from(home).join(suffix));
        }
    }

    let mut out: Vec<PathBuf> = Vec::new();

    let home = vars
        .get("HOME")
        .cloned()
        .or_else(|| vars.get("USERPROFILE").cloned())
        .unwrap_or_else(String::new);

    match platform {
        Platform::Mac => {
            push(&mut out, "/opt/homebrew/bin");
            push(&mut out, "/usr/local/bin");
            push(&mut out, "/usr/bin");
            push(&mut out, "/bin");
            push(&mut out, "/usr/sbin");
            push(&mut out, "/sbin");

            push_home(&mut out, &home, ".local/bin");
            push_home(&mut out, &home, ".cargo/bin");
        }
        Platform::Linux | Platform::Other => {
            push(&mut out, "/usr/local/sbin");
            push(&mut out, "/usr/local/bin");
            push(&mut out, "/usr/sbin");
            push(&mut out, "/usr/bin");
            push(&mut out, "/sbin");
            push(&mut out, "/bin");

            push_home(&mut out, &home, ".local/bin");
            push_home(&mut out, &home, ".cargo/bin");
        }
        Platform::Wsl => {
            push(&mut out, "/usr/local/sbin");
            push(&mut out, "/usr/local/bin");
            push(&mut out, "/usr/sbin");
            push(&mut out, "/usr/bin");
            push(&mut out, "/sbin");
            push(&mut out, "/bin");

            push_home(&mut out, &home, ".local/bin");
            push_home(&mut out, &home, ".cargo/bin");

            if let Some(user) = vars.get("USERNAME").or_else(|| vars.get("USER")).cloned() {
                if !user.trim().is_empty() {
                    out.push(PathBuf::from(format!("/mnt/c/Users/{user}/.cargo/bin")));
                    out.push(PathBuf::from(format!("/mnt/c/Users/{user}/scoop/shims")));
                }
            }
        }
        Platform::Windows => {
            push(&mut out, r"C:\Windows\System32");
            push(&mut out, r"C:\Windows");

            if !home.is_empty() {
                out.push(PathBuf::from(&home).join(".cargo").join("bin"));
                out.push(PathBuf::from(&home).join("scoop").join("shims"));
                out.push(
                    PathBuf::from(&home)
                        .join("AppData")
                        .join("Local")
                        .join("Microsoft")
                        .join("WindowsApps"),
                );
            }

            if let Some(pf) = vars.get("ProgramFiles").cloned() {
                if !pf.trim().is_empty() {
                    out.push(PathBuf::from(pf).join("Git").join("cmd"));
                }
            }
        }
    }

    // De-dupe while preserving order
    let mut seen = BTreeSet::new();
    out.into_iter()
        .filter(|p| seen.insert(p.to_string_lossy().to_string()))
        .collect()
}

fn pathext_list(vars: &BTreeMap<String, String>) -> Vec<String> {
    // Prefer PATHEXT, else common defaults.
    let raw = vars
        .get("PATHEXT")
        .map(|s| s.as_str())
        .unwrap_or(".COM;.EXE;.BAT;.CMD");

    let mut out = Vec::new();
    for part in raw.split(';') {
        let p = part.trim();
        if p.is_empty() {
            continue;
        }
        let mut p = p.to_string();
        if !p.starts_with('.') {
            p.insert(0, '.');
        }
        out.push(p.to_ascii_lowercase());
    }

    if out.is_empty() {
        out = vec![".com", ".exe", ".bat", ".cmd"].into_iter().map(|s| s.to_string()).collect();
    }

    out
}

// ---------------- ordering (same idea as cloud) ----------------

fn order_env_assignments(assigns: &BTreeMap<String, String>) -> Vec<(String, String)> {
    let keys: BTreeSet<String> = assigns.keys().cloned().collect();

    let mut deps: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut indeg: BTreeMap<String, usize> = BTreeMap::new();

    for (k, v) in assigns.iter() {
        let ds = extract_deps_posix(v)
            .into_iter()
            .filter(|d| keys.contains(d))
            .collect::<BTreeSet<_>>();

        deps.insert(k.clone(), ds);
        indeg.insert(k.clone(), 0);
    }

    for (k, ds) in deps.iter() {
        *indeg.get_mut(k).unwrap() = ds.len();
    }

    let mut q = VecDeque::new();
    for (k, n) in indeg.iter() {
        if *n == 0 {
            q.push_back(k.clone());
        }
    }

    let mut ordered_keys = Vec::new();

    while let Some(n) = q.pop_front() {
        ordered_keys.push(n.clone());

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

    if ordered_keys.len() != assigns.len() {
        for k in assigns.keys() {
            if !ordered_keys.contains(k) {
                ordered_keys.push(k.clone());
            }
        }
    }

    ordered_keys
        .into_iter()
        .map(|k| (k.clone(), assigns.get(&k).cloned().unwrap_or_default()))
        .collect()
}

fn extract_deps_posix(v: &str) -> Vec<String> {
    let re = Regex::new(r"\$([A-Za-z_][A-Za-z0-9_]*)|\$\{([A-Za-z_][A-Za-z0-9_]*)\}").unwrap();
    re.captures_iter(v)
        .filter_map(|c| c.get(1).or_else(|| c.get(2)).map(|m| m.as_str().to_string()))
        .collect()
}
