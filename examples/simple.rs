use winit::{
  dpi::LogicalSize,
  event::{Event, KeyEvent, WindowEvent},
  keyboard::{Key, KeyCode, PhysicalKey},
};
use witer::prelude::*;

/*
  This example showcases how to open a simple window that
  closes when Escape is pressed. It also showcases how to set
  the window to be a specific inner size.
*/

fn main() {
  let window = Window::builder()
    .with_title("Press Esc to close!")
    .with_visible(false)
    .build()
    .unwrap();

  window.set_inner_size(LogicalSize::new(1280.0, 720.0));
  window.set_visible(true);

  for message in &window {
    if let Event::WindowEvent {
      event:
        WindowEvent::KeyboardInput {
          event:
            KeyEvent {
              physical_key: PhysicalKey::Code(KeyCode::Escape),
              ..
            },
          ..
        },
      ..
    } = message
    {
      window.close();
    }
    // if let Message::Key {
    //   key: Key::Escape, ..
    // } = message
    // {
    //   window.close();
    // }
  }
}
