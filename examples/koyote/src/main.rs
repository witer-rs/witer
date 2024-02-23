use ezwin::prelude::*;

fn main() -> WindowResult<()> {
  let window = Window::new(
    WindowSettings::default()
      .with_flow(Flow::Wait)
      .with_title("Easy Window")
      .with_size((800, 600)),
  )?;

  for msg in &window {
    println!("{msg:?}")
  }

  Ok(())
}
