use witer::prelude::*;

/*
  This example showcases how to open a simple window that
  closes when Escape is pressed.
*/

fn main() {
  let window = Window::new(
    "Press Esc to close!",
    LogicalSize::new(800.0, 450.0),
    None,
    WindowSettings::default(),
  )
  .unwrap();

  for message in &window {
    if let Message::Key {
      key: Key::Escape, ..
    } = message
    {
      window.close();
    }
  }
}
