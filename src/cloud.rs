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
    deps::{module_key, normalize_requires_list, requires_satisfied, topo_sort_group, DepNode},
    emit::Emitter,
    resolve::{DetectVars, Resolver},
    runtime::RuntimeEnv,
};

#[derive(Debug, Clone)]
pub struct DetectedCloud {
    pub name: String,
    pub detect: DetectVars, // e.g. { "path": "/Users/..../Dropbox" }
    pub module: CloudModule,
}

pub fn detect_cloud_modules(
    ctx: &ContextEnv,
    rt: &RuntimeEnv,
    cfg: &Config,
) -> Result<Vec<DetectedCloud>> {
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
    m.platforms.is_empty() || m.platforms.contains(&p)
}

fn detect_one_cloud(
    ctx: &ContextEnv,
    rt: &RuntimeEnv,
    name: &str,
    m: &CloudModule,
) -> Result<Option<DetectedCloud>> {
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
        let resolved = r.resolve(raw).with_context(|| {
            format!("cloud.{name}: failed to resolve detect path pattern: {raw}")
        })?;

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
    let dir = pb
        .parent()
        .map(|x| x.to_string_lossy().to_string())
        .unwrap_or_else(|| ".".to_string());
    let glob = pb
        .file_name()
        .map(|x| x.to_string_lossy().to_string())
        .unwrap_or_else(|| p.to_string());
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

// --------------------- EMIT (cloud, deps-gated + seq runtime) ---------------------

pub fn emit_cloud_with_active(
    ctx: &ContextEnv,
    rt: &RuntimeEnv,
    cfg: &Config,
    shell: Shell,
    active: &mut BTreeSet<String>,
) -> Result<String> {
    let mut work = rt.clone();
    emit_cloud_seq(ctx, &mut work, cfg, shell, active)
}

pub fn emit_cloud_seq(
    ctx: &ContextEnv,
    rt: &mut RuntimeEnv,
    cfg: &Config,
    shell: Shell,
    active: &mut BTreeSet<String>,
) -> Result<String> {
    if !cfg.modules.enable_cloud || !cfg.modules.cloud.enabled {
        return Ok(String::new());
    }

    let em = Emitter::new(shell);
    let mut out = String::new();
    em.header(&mut out, "apogee (cloud)");

    // Build DepNodes for eligible modules (enabled + platform)
    let mut nodes: Vec<DepNode> = Vec::new();
    for (name, m) in cfg.modules.cloud.items.iter() {
        if !m.enabled {
            continue;
        }
        if !module_supports_platform(m, ctx.platform) {
            continue;
        }

        let key = module_key("cloud", name);
        let requires = normalize_requires_list(&m.requires)?;

        nodes.push(DepNode {
            key,
            name: name.clone(),
            priority: m.priority,
            requires,
        });
    }

    let ordered = topo_sort_group(nodes, "cloud")?;

    let mut emitted_any = false;

    for node in ordered {
        if !requires_satisfied(active, &node.requires) {
            continue;
        }

        let m = cfg
            .modules
            .cloud
            .items
            .get(&node.name)
            .expect("node name exists");

        if let Some(det) = detect_one_cloud(ctx, rt, &node.name, m)? {
            emitted_any = true;

            em.comment(&mut out, &format!("--- cloud: {} ---", det.name));
            emit_cloud_module_into(&em, &mut out, ctx, rt, &det.detect, &det.module.emit)?;

            active.insert(module_key("cloud", &node.name));

            apply_emit_effects_to_runtime(ctx, rt, &det.detect, &det.module.emit)?;

            em.blank(&mut out);
        }
    }

    if !emitted_any {
        return Ok(String::new());
    }

    Ok(out)
}

fn emit_cloud_module_into(
    em: &Emitter,
    out: &mut String,
    ctx: &ContextEnv,
    rt: &RuntimeEnv,
    detect: &DetectVars,
    emit: &EmitBlock,
) -> Result<()> {
    let r = Resolver::new(ctx, &rt.vars).with_detect(detect);

    let mut assigns: BTreeMap<String, String> = BTreeMap::new();

    for (k, v) in emit.env.iter() {
        assigns.insert(k.clone(), r.resolve(v)?);
    }
    for (k, v) in emit.env_derived.iter() {
        assigns.insert(k.clone(), r.resolve(v)?);
    }

    for (k, v) in order_env_assignments(&assigns) {
        em.set_env(out, &k, &v);
    }

    if !emit.aliases.is_empty() {
        em.blank(out);
        for (name, raw) in emit.aliases.iter() {
            let val = r.resolve(raw)?;
            em.alias(out, name, &val);
        }
    }

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

    Ok(())
}

fn apply_emit_effects_to_runtime(
    ctx: &ContextEnv,
    rt: &mut RuntimeEnv,
    detect: &DetectVars,
    emit: &EmitBlock,
) -> Result<()> {
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

    // NOTE: cloud may also have PATH effects; keep this consistent with apps behavior
    let snap2 = rt.vars.clone();
    let r2 = Resolver::new(ctx, &snap2).with_detect(detect);

    let sep = if matches!(ctx.platform, Platform::Windows) {
        ';'
    } else {
        ':'
    };
    let path_key_primary = if matches!(ctx.platform, Platform::Windows) {
        "Path"
    } else {
        "PATH"
    };

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

    for raw in emit.paths.prepend_if_exists.iter() {
        let d = r2.resolve(raw)?;
        if d.is_empty() || !Path::new(&d).is_dir() {
            continue;
        }
        if have.insert(d.clone()) {
            parts.insert(0, d);
        }
    }

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

    rt.vars.insert("PATH".to_string(), path_val.clone());
    rt.vars.insert("Path".to_string(), path_val);

    Ok(())
}

// ---------------- ordering ----------------

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
        .filter_map(|c| {
            c.get(1)
                .or_else(|| c.get(2))
                .map(|m| m.as_str().to_string())
        })
        .collect()
}
