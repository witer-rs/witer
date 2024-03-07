use ezwin::prelude::*;

fn main() {
  let settings = WindowSettings::default()
    .with_close_on_x(false)
    .with_flow(Flow::Wait)
    .with_size((1280, 720))
    .with_title("Example");

  let window = Window::new(settings).unwrap();

  for message in &window {
    if let Message::Window(WindowMessage::Key {
      key: Key::Escape, ..
    }) = message
    {
      window.close();
    }
  }
}
