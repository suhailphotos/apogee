// src/main.rs
use anyhow::Result;
use std::{collections::BTreeSet, env};
use apogee::init;


fn print_version() {
    println!("apogee {}", env!("CARGO_PKG_VERSION"));
}

fn print_help() {
    println!(
        r#"apogee {}

USAGE:
  apogee                Emit shell config (default)
  apogee init           Install a starter config + shell hook
  apogee --version|-V   Print version
  apogee --help|-h      Show help
"#,
        env!("CARGO_PKG_VERSION")
    );
}

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("--version") | Some("-V") => {
            print_version();
            Ok(())
        }
        Some("--help") | Some("-h") | Some("help") => {
            print_help();
            Ok(())
        }
        Some("init") => init::run(),
        _ => {
            // your existing emit path...
            let mut ctx = apogee::ContextEnv::new()?;
            let cfg = ctx.load_config()?;

            let shell = ctx
                .vars
                .get("APOGEE_SHELL")
                .and_then(|s| apogee::Shell::parse(s))
                .or(ctx.shell_type)
                .unwrap_or(cfg.apogee.default_shell);

            ctx.shell_type = Some(shell);
            ctx.vars
                .insert("APOGEE_SHELL".to_string(), shell.to_string());

            let baseline = ctx.vars.clone();

            let rt0 = apogee::RuntimeEnv::build(&ctx, &cfg)?;
            let dotenv_script = apogee::runtime::emit_env_delta(shell, &baseline, &rt0.vars);

            let mut work = rt0.clone();
            let mut active: BTreeSet<String> = BTreeSet::new();

            let global_script = apogee::emit_global(&ctx, &work, &cfg, shell)?;

            // 1) CLOUD first
            let cloud_script = apogee::emit_cloud_seq(&ctx, &mut work, &cfg, shell, &mut active)?;

            // 2) APPS second
            let apps_script = apogee::emit_apps_seq(&ctx, &mut work, &cfg, shell, &mut active)?;

            // 3) HOOKS after apps
            let hooks_script = apogee::emit_hooks(&ctx, &work, &cfg, shell)?;

            // 4) TEMPLATES last
            let templates_script =
                apogee::emit_templates_with_active(&ctx, &work, &cfg, shell, &mut active)?;

            // Stitch output with clean spacing
            let mut out = String::new();

            if !dotenv_script.trim().is_empty() {
                out.push_str(&dotenv_script);
            }

            if !global_script.trim().is_empty() {
                out.push_str(&global_script);
            }

            if !cloud_script.trim().is_empty() {
                if !out.is_empty() && !out.ends_with('\n') {
                    out.push('\n');
                }
                if !out.is_empty() {
                    out.push('\n');
                }
                out.push_str(&cloud_script);
            }

            if !apps_script.trim().is_empty() {
                if !out.is_empty() && !out.ends_with('\n') {
                    out.push('\n');
                }
                if !out.is_empty() {
                    out.push('\n');
                }
                out.push_str(&apps_script);
            }

            if !hooks_script.trim().is_empty() {
                if !out.is_empty() && !out.ends_with('\n') {
                    out.push('\n');
                }
                if !out.is_empty() {
                    out.push('\n');
                }
                out.push_str(&hooks_script);
            }

            if !templates_script.trim().is_empty() {
                if !out.is_empty() && !out.ends_with('\n') {
                    out.push('\n');
                }
                if !out.is_empty() {
                    out.push('\n');
                }
                out.push_str(&templates_script);
            }

            print!("{out}");
            Ok(())
        }
    }
}
