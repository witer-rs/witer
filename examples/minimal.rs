use ezwin::prelude::*;

struct App;

impl WindowCallback<()> for App {
  fn on_create(_: &Arc<Window>, _: ()) -> Option<Self> {
    Some(Self)
  }
}

fn main() {
  let settings = WindowSettings::default();
  let callback = CallbackSettings::<App, _>::new(());
  let window = Window::new(settings, callback).unwrap();
  while window.pump() {}
}
