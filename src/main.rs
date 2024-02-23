use ezwin::prelude::*;

fn main() {
  let window = Window::new(
    WindowSettings::default()
      .with_flow(Flow::Wait)
      .with_title("Easy Window")
      .with_size((800, 600)),
  )
  .unwrap();

  for message in window {
    println!("{message:?}");
  }
}
