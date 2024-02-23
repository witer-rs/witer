use ezwin::prelude::*;

fn main() {
  let window = Window::new(
    WindowSettings::default()
      .with_flow(Flow::Poll)
      .with_title("Easy Window")
      .with_size((800, 450)),
  )
  .unwrap();

  for message in window {
    if !matches!(
      message,
      Message::None
        | Message::Other { .. }
        | Message::Mouse(MouseMessage::Cursor { .. })
    ) {
      println!("{message:?}");
    }
  }
}
