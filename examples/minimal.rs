use ezwin::prelude::*;

struct App;

impl WindowCallback for App {}

fn main() {
  Window::new(Default::default()).unwrap().run(App);
}
