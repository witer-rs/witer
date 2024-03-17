use witer::prelude::*;

/*
  This example showcases how to open a simple window that
  closes when Escape is pressed. It also showcases how to set
  the window to be a specific inner size.
*/

fn main() {
  let window = Window::new(
    "Press Esc to close!",
    LogicalSize::new(0.0, 0.0), // starting outer size
    None,
    WindowSettings::default().with_visibility(Visibility::Hidden),
  )
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
