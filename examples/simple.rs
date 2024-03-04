use ezwin::prelude::*;

fn main() {
  let settings = WindowSettings::default();

  let window = Window::new(settings).unwrap();

  window.run(App);
}

struct App;

impl WindowProcedure for App {
  fn on_message(&mut self, _window: &Arc<Window>, _message: Message) {}
}
