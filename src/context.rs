use crate::config::{Platform, Shell};

#[derive(Debug, Clone)]
pub struct Context {
  pub platform: Platform,
  pub shell: Shell,
  pub hostname: Option<String>,
}

impl Context {
  pub fn hostname_str(&self) -> Option<&str> {
    self.hostname.as_deref()
  }
}
