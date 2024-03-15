/*
  This contrived example showcases the minimal amount to code required to open a window.
*/

fn main() {
  for _ in &witer::window::Window::new(Default::default()).unwrap() {}
}
