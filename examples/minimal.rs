use ezwin::prelude::*;

fn main() {
  let window = Window::new(WindowSettings::default()).unwrap();

  for message in window.as_ref() {
    if let Message::Window(..) = message {
      println!("{message:?}");
    }
  }
}
