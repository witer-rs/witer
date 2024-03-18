/*
  This example showcases the minimal amount to code required to open a window.
*/

use witer::prelude::*;

fn main() {
  for _ in &Window::builder().build().unwrap() {}
}
