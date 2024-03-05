use ezwin::prelude::*;

#[allow(unused)]
struct App(i32);

// Implement
impl WindowProcedure<i32> for App {
  fn on_create(_: &Arc<Window>, x: i32) -> Option<Self> {
    Some(Self(x))
  }

  fn on_message(&mut self, _: &Arc<Window>, _: Message) {}
}

fn main() {
  let x = 69;

  // Configure
  let settings = WindowSettings::<App, _>::new(x)
    .with_flow(Flow::Wait)
    .with_size((1280, 720))
    .with_title("Example");

  // Build
  let window = Window::new(settings).unwrap();

  // Run
  while window.pump() {}
}
