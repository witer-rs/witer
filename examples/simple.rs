use witer::prelude::*;

mod common;

/*
  This example showcases how to open a simple window that
  closes when Escape is pressed. It also showcases how to set
  the window to be a specific inner size.
*/

fn main() {
  common::init_log(env!("CARGO_CRATE_NAME"));

  let window = Window::builder()
    .with_title("Press Esc to close!")
    // .with_size(size) <-- this would set the outer size, not the inner size
    .with_visibility(Visibility::Hidden)
    .build()
    .unwrap();

  window.set_inner_size(LogicalSize::new(1280.0, 720.0));
  window.set_visibility(Visibility::Shown);

  for message in &window {
    if let Message::Key {
      key: Key::Escape, ..
    } = message
    {
      window.close();
    }

    if window.has_focus() {
      tracing::debug!("{message:?}")
    }
  }
}
