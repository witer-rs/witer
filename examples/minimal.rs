use ezwin::prelude::*;

struct App;

impl WindowCallback for App {}

fn main() {
  let window = Window::new(WindowSettings::default().with_flow(Flow::Poll)).unwrap();

  for message in window.as_ref() {
    println!("{message:?}");
  }
}

// TODO: Try the iterator API, but look into the proper way to sleep the thread
// before each message to keep them synced
