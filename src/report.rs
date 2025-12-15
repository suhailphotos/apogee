use crate::{cli, config};

pub fn build_report(cfg: &config::Config, mode: cli::ReportMode) -> String {
    match mode {
        cli::ReportMode::Off => String::new(),
        cli::ReportMode::Summary => summary(cfg),
        cli::ReportMode::Full => format!("{cfg:#?}"),
    }
}

fn summary(cfg: &config::Config) -> String {
    let mut out = String::new();

    out.push_str("apogee report (summary)\n");
    out.push_str("======================\n");
    out.push_str(&format!("schema_version: {}\n", cfg.apogee.schema_version));
    out.push_str(&format!("default_shell: {:?}\n", cfg.apogee.default_shell));

    out.push_str("\nmodules\n");
    out.push_str(&format!(
        "  knobs: cloud={} apps={} hooks={}\n",
        cfg.modules.enable_cloud, cfg.modules.enable_apps, cfg.modules.enable_hooks
    ));

    out.push_str(&format!(
        "  cloud: enabled={} items={}\n",
        cfg.modules.cloud.enabled,
        cfg.modules.cloud.items.len()
    ));
    for (name, m) in &cfg.modules.cloud.items {
        out.push_str(&format!(
            "    - {} (enabled={}, kind={:?})\n",
            name, m.enabled, m.kind
        ));
    }

    out.push_str(&format!(
        "  apps: enabled={} items={}\n",
        cfg.modules.apps.enabled,
        cfg.modules.apps.items.len()
    ));
    for (name, m) in &cfg.modules.apps.items {
        out.push_str(&format!(
            "    - {} (enabled={}, kind={:?})\n",
            name, m.enabled, m.kind
        ));
    }

    out.push_str(&format!(
        "  hooks: enabled={} items={}\n",
        cfg.modules.hooks.enabled,
        cfg.modules.hooks.items.len()
    ));

    out
}
