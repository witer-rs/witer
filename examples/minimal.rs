/*
  This example showcases the minimal amount to code required to open a window.
*/

use witer::prelude::*;

fn main() {
  for _ in &Window::new(
    "Minimal",
    LogicalSize::new(640.0, 480.0),
    None,
    WindowSettings::default(),
  )
  .unwrap()
  {}
}
