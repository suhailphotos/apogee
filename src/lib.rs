pub mod cloud;
pub mod config;
pub mod context;
pub mod emit;
pub mod resolve;
pub mod runtime;
pub mod apps;

// Convenience re-exports (optional, but nice)
pub use apps::{detect_app_modules, emit_apps, DetectedApp};
pub use cloud::{detect_cloud_modules, emit_cloud, DetectedCloud};
pub use config::{Config, Platform, Shell};
pub use context::ContextEnv;
pub use emit::Emitter;
pub use runtime::RuntimeEnv;
