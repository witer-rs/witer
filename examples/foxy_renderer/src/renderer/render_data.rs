use std::fmt::Debug;

#[derive(Default)]
pub struct RenderData {}

impl Debug for RenderData {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    writeln!(f, "RenderData {{ .. }}")
  }
}
