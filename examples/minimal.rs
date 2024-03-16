/*
  This example showcases the minimal amount to code required to open a window.
*/

use witer::prelude::*;

fn main() {
  let s = WindowSettings::default()
    .with_title("Minimal")
    .with_outer_size(LogicalSize::new(800.0, 600.0));
  for _ in &Window::new(s).unwrap() {}
}
