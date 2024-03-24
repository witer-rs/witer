mod common;

/*
  This example showcases the minimal amount to code required to open a window.
*/

fn main() {
  common::init_log(env!("CARGO_CRATE_NAME"));
  for m in &witer::Window::builder().build().unwrap() {
    tracing::trace!("{m:?}");
  }
}
