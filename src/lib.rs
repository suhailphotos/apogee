pub mod apps;
pub mod cloud;
pub mod config;
pub mod context;
pub mod deps;
pub mod emit;
pub mod hooks;
pub mod resolve;
pub mod runtime;
pub mod templates;
pub mod init;

// Convenience re-exports
pub use deps::{
    module_key, normalize_require_key, normalize_requires_list, requires_satisfied,
    topo_sort_group, DepNode,
};

pub use cloud::{detect_cloud_modules, emit_cloud_seq, emit_cloud_with_active, DetectedCloud};

pub use apps::{detect_app_modules, emit_apps, emit_apps_seq, emit_apps_with_active, DetectedApp};

pub use templates::emit_templates_with_active;

pub use config::{Config, Platform, Shell};
pub use context::ContextEnv;
pub use emit::Emitter;
pub use runtime::RuntimeEnv;

pub mod global;
pub use global::emit_global;

pub use hooks::emit_hooks;
