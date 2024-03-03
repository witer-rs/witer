use ezwin::prelude::*;

fn main() {
  WindowSettings::default().build::<App>().unwrap().run();
}

struct App;

impl WindowProcedure for App {
  fn new(_window: &Arc<Window>) -> Self {
    Self
  }

  fn procedure(&mut self, _window: &Arc<Window>, _message: Message) {}
}
