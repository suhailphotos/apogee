use anyhow::{bail, Result};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone)]
pub struct DepNode {
    pub key: String,           // e.g. "apps.uv"
    pub name: String,          // e.g. "uv" (module name within group)
    pub priority: i32,         // for tie-breaking
    pub requires: Vec<String>, // normalized keys
}

pub fn module_key(group: &str, name: &str) -> String {
    format!("{}.{}", group, name)
}

/// Accepts:
/// - "apps.uv"
/// - "cloud.dropbox"
/// - "modules.apps.uv"
/// - "modules.cloud.dropbox"
pub fn normalize_require_key(raw: &str) -> Result<String> {
    let s = raw.trim();
    if s.is_empty() {
        bail!("requires entry cannot be empty");
    }

    let s = s.strip_prefix("modules.").unwrap_or(s);
    let mut parts = s.split('.').collect::<Vec<_>>();

    if parts.len() != 2 {
        bail!(
            "requires must be like 'apps.uv' or 'cloud.dropbox' (optionally prefixed with 'modules.'): got '{raw}'"
        );
    }

    let group = parts.remove(0).trim().to_ascii_lowercase();
    let name = parts.remove(0).trim().to_string();

    if group.is_empty() || name.is_empty() {
        bail!("invalid requires key '{raw}'");
    }

    Ok(format!("{group}.{name}"))
}

pub fn normalize_requires_list(raw: &[String]) -> Result<Vec<String>> {
    let mut out = Vec::with_capacity(raw.len());
    for r in raw {
        out.push(normalize_require_key(r)?);
    }
    Ok(out)
}

pub fn requires_satisfied(active: &BTreeSet<String>, requires: &[String]) -> bool {
    requires.iter().all(|k| active.contains(k))
}

/// Topo-sort nodes by SAME-GROUP dependencies only.
/// - If a node requires "apps.xyz" and xyz is a node in this list, it becomes an edge.
/// - Cross-group requires (e.g. "cloud.dropbox") are ignored for ordering here.
/// - Tie-break: priority, then name, then key.
/// - Cycles => error.
pub fn topo_sort_group(nodes: Vec<DepNode>, group: &str) -> Result<Vec<DepNode>> {
    let group_prefix = format!("{}.", group);

    let mut map: BTreeMap<String, DepNode> = BTreeMap::new();
    for n in nodes {
        map.insert(n.key.clone(), n);
    }

    // Build indegree and outgoing edges
    let mut indeg: BTreeMap<String, usize> = BTreeMap::new();
    let mut outgoing: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for k in map.keys() {
        indeg.insert(k.clone(), 0);
        outgoing.insert(k.clone(), Vec::new());
    }

    // Validate + wire edges
    for (k, node) in map.iter() {
        for dep in node.requires.iter() {
            // Only enforce ordering for same-group deps
            if !dep.starts_with(&group_prefix) {
                continue;
            }

            if !map.contains_key(dep) {
                bail!("{}: requires unknown {} module '{}'", k, group, dep);
            }

            // edge dep -> k
            outgoing.get_mut(dep).unwrap().push(k.clone());
            *indeg.get_mut(k).unwrap() += 1;
        }
    }

    // Ready set (sorted by priority/name/key)
    let mut ready: BTreeSet<(i32, String, String)> = BTreeSet::new();
    for (k, d) in indeg.iter() {
        if *d == 0 {
            let n = map.get(k).unwrap();
            ready.insert((n.priority, n.name.clone(), n.key.clone()));
        }
    }

    let mut ordered_keys: Vec<String> = Vec::with_capacity(map.len());

    while let Some((_, _, key)) = ready.pop_first() {
        ordered_keys.push(key.clone());

        for child in outgoing.get(&key).unwrap().iter() {
            let e = indeg.get_mut(child).unwrap();
            *e -= 1;
            if *e == 0 {
                let n = map.get(child).unwrap();
                ready.insert((n.priority, n.name.clone(), n.key.clone()));
            }
        }
    }

    if ordered_keys.len() != map.len() {
        // Find nodes still in the cycle
        let mut stuck: Vec<String> = indeg
            .iter()
            .filter(|(_, v)| **v > 0)
            .map(|(k, _)| k.clone())
            .collect();
        stuck.sort();
        bail!("cycle detected in {} requires graph: {:?}", group, stuck);
    }

    Ok(ordered_keys
        .into_iter()
        .map(|k| map.remove(&k).unwrap())
        .collect())
}
