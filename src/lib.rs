pub mod cloud;
pub mod config;
pub mod context;
pub mod emit;
pub mod resolve;
pub mod runtime;
pub mod apps;
pub mod deps;
pub mod templates;

// Convenience re-exports (optional, but nice)
pub use deps::{
    module_key,
    normalize_require_key,
    normalize_requires_list,
    requires_satisfied,
    topo_sort_group,
    DepNode,
};

pub use cloud::{
    detect_cloud_modules,
    emit_cloud_seq,
    emit_cloud_with_active,
    DetectedCloud,
};

pub use apps::{
    detect_app_modules,
    emit_apps,
    emit_apps_seq,
    emit_apps_with_active,
    DetectedApp,
};

pub use templates::emit_templates_with_active;

pub use config::{Config, Platform, Shell};
pub use context::ContextEnv;
pub use emit::Emitter;
pub use runtime::RuntimeEnv;
