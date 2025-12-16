pub mod config;
pub mod context;
pub mod resolve;
pub mod runtime;
pub mod cloud;
pub mod emit;

// Convenience re-exports (optional, but nice)
pub use config::{Config, Platform, Shell};
pub use context::ContextEnv;
pub use runtime::RuntimeEnv;
pub use cloud::{detect_cloud_modules, emit_cloud, DetectedCloud};
