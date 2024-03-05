// use ezwin::prelude::*;

// #[allow(unused)]
// struct App {
//   z: i32,
// }

// // Implement
// impl WindowCallback for App {
//   fn on_message(&mut self, window: &Arc<Window>, message: Message) {
//     if let Message::Window(WindowMessage::Key {
//       key: Key::Escape, ..
//     }) = message
//     {
//       window.close();
//     }
//   }
// }

// fn main() {
//   let x = 69;
//   let y = 34;

//   // Configure
//   let settings = WindowSettings::default()
//     .with_flow(Flow::Wait)
//     .with_size((1280, 720))
//     .with_title("Example");

//   // Build
//   let window = Window::new(settings).unwrap();

//   // Run
//   window.run(App { z: x + y });
// }

fn main() {}
