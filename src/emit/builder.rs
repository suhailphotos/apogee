#[derive(Debug, Default)]
pub struct ScriptBuilder {
  buf: String,
}

impl ScriptBuilder {
  pub fn new() -> Self {
    Self { buf: String::new() }
  }

  pub fn push_line(&mut self, line: &str) {
    self.buf.push_str(line);
    self.buf.push('\n');
  }

  pub fn push_fmt(&mut self, s: &str) {
    self.buf.push_str(s);
  }

  pub fn finish(self) -> String {
    self.buf
  }
}
