use anyhow::{Context as _, Result};
use minijinja::Environment;
use serde_json::json;
use std::collections::BTreeSet;
use std::fs;

use crate::{
    config::{Config, Platform, Shell, TemplateModule},
    context::ContextEnv,
    deps::{module_key, normalize_requires_list, requires_satisfied, topo_sort_group, DepNode},
    emit::Emitter,
    resolve::Resolver,
    runtime::RuntimeEnv,
};

#[derive(Debug, Clone)]
pub struct RenderedTemplate {
    pub name: String,
    pub text: String,
}

/// Emit templates after other groups have had a chance to mutate the runtime.
/// Uses deps gating via `requires` and marks `templates.<name>` active when emitted.
pub fn emit_templates_with_active(
    ctx: &ContextEnv,
    rt: &RuntimeEnv,
    cfg: &Config,
    shell: Shell,
    active: &mut BTreeSet<String>,
) -> Result<String> {
    if !cfg.modules.enable_templates || !cfg.modules.templates.enabled {
        return Ok(String::new());
    }

    // Build DepNodes for eligible modules (enabled + platform)
    let mut nodes: Vec<DepNode> = Vec::new();
    for (name, m) in cfg.modules.templates.items.iter() {
        if !m.enabled {
            continue;
        }
        if !module_supports_platform(m, ctx.platform) {
            continue;
        }

        let key = module_key("templates", name);
        let requires = normalize_requires_list(&m.requires)?;

        nodes.push(DepNode {
            key,
            name: name.clone(),
            priority: m.priority,
            requires,
        });
    }

    let ordered = topo_sort_group(nodes, "templates")?;

    let em = Emitter::new(shell);
    let mut out = String::new();
    em.header(&mut out, "apogee (templates)");

    let mut emitted_any = false;

    for node in ordered {
        if !requires_satisfied(active, &node.requires) {
            continue;
        }

        let m = cfg
            .modules
            .templates
            .items
            .get(&node.name)
            .expect("template node exists");

        let rendered = render_one_template(ctx, rt, shell, &node.name, m)?;
        let Some(rendered) = rendered else { continue; };

        emitted_any = true;

        em.comment(&mut out, &format!("--- template: {} ---", rendered.name));
        out.push_str(&rendered.text);
        if !rendered.text.ends_with('\n') {
            out.push('\n');
        }
        em.blank(&mut out);

        active.insert(module_key("templates", &node.name));
    }

    if !emitted_any {
        return Ok(String::new());
    }

    Ok(out)
}

fn module_supports_platform(m: &TemplateModule, p: Platform) -> bool {
    m.platforms.is_empty() || m.platforms.contains(&p)
}

fn render_one_template(
    ctx: &ContextEnv,
    rt: &RuntimeEnv,
    shell: Shell,
    name: &str,
    m: &TemplateModule,
) -> Result<Option<RenderedTemplate>> {
    let Some(tpl_raw) = m.templates.for_shell(shell) else {
        // No template provided for this shell => skip
        return Ok(None);
    };

    // Resolve the template path (supports {vars} via Resolver)
    let r = Resolver::new(ctx, &rt.vars);
    let tpl_path = r
        .resolve(tpl_raw)
        .with_context(|| format!("templates.{name}: failed to resolve template path: {tpl_raw}"))?;

    let source = fs::read_to_string(&tpl_path)
        .with_context(|| format!("templates.{name}: failed to read template file: {tpl_path}"))?;

    // Context passed to MiniJinja:
    // - shell/platform (small but useful)
    // - vars (current runtime env map)
    // - data (module-specific arbitrary user data)
    let ctx_json = json!({
        "apogee": {
            "shell": shell.to_string(),
            "platform": ctx.platform.to_string(),
        },
        "vars": rt.vars,
        "data": m.data,
    });

    let rendered = render_minijinja(&source, &ctx_json)
        .with_context(|| format!("templates.{name}: render failed ({tpl_path})"))?;

    Ok(Some(RenderedTemplate {
        name: name.to_string(),
        text: rendered,
    }))
}

fn render_minijinja(source: &str, ctx_json: &serde_json::Value) -> Result<String> {
    let mut env = Environment::new();

    // Jinja-style `tojson` filter (string-only for now).
    // Produces a JSON string literal like "my_project", with proper escaping.
    env.add_filter("tojson", |s: String| -> Result<String, minijinja::Error> {
        serde_json::to_string(&s).map_err(|e| {
            minijinja::Error::new(minijinja::ErrorKind::InvalidOperation, e.to_string())
        })
    });

    env.add_template("tpl", source)?;
    let tpl = env.get_template("tpl")?;
    let v = minijinja::value::Value::from_serialize(ctx_json);
    Ok(tpl.render(v)?)
}
