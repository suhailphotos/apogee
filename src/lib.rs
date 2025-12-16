pub mod config;
pub mod context;
pub mod resolve;
pub mod runtime;

// Convenience re-exports (optional, but nice)
pub use config::{Config, Platform, Shell};
pub use context::ContextEnv;

pub use runtime::RuntimeEnv;
