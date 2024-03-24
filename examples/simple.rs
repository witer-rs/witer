use witer::prelude::*;

use self::util::init_log;

mod util;

/*
  This example showcases how to open a simple window that
  closes when Escape is pressed. It also showcases how to set
  the window to be a specific inner size.
*/

fn main() {
  init_log(env!("CARGO_CRATE_NAME"));

  let window = Window::builder()
    .with_title("Press Esc to close!")
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
  }
}
