use ezwin::prelude::*;

struct App;

impl WindowProcedure<()> for App {
  fn on_create(_: &Arc<Window>, _: ()) -> Option<Self> {
    Some(Self)
  }
}

fn main() {
  let settings = WindowSettings::<App, _>::new(());
  let window = Window::new(settings).unwrap();
  while window.pump() {}
}
