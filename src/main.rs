use ezwin::prelude::*;

fn main() -> WindowResult<()> {
  let window = Window::new(
    WindowSettings::default()
      .with_close_on_x(false)
      .with_flow(Flow::Wait)
      .with_title("Easy Window")
      .with_size((800, 600)),
  )?;

  for msg in &window {
    if let Message::CloseRequested = msg {
      window.close();
    }

    println!("{msg:?}")
  }

  Ok(())
}
