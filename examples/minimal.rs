/*
  This example showcases the minimal amount to code required to open a window.
*/

fn main() {
  for _ in &witer::Window::builder().build().unwrap() {}
}
