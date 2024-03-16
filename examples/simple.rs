use witer::prelude::*;

/*
  This example showcases how to open a simple window that
  only closes when Escape is pressed.
*/

fn main() {
  let settings = WindowSettings::default()
    .with_title("Press Esc to close!")
    .with_outer_size(LogicalSize::new(800.0, 450.0))
    .with_close_on_x(false)
    .with_flow(Flow::Wait);

  let window = Window::new(settings).unwrap();

  for message in &window {
    if let Message::Key {
      key: Key::Escape, ..
    } = message
    {
      window.close();
    }
  }
}
