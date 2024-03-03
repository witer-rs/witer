#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use std::{
  sync::Arc,
  time::{Duration, Instant},
};

use ezwin::{
  prelude::*,
  window::{callback::WindowProcedure, run},
};
use foxy_time::{Time, TimeSettings};

fn main() -> WindowResult<()> {
  let window = Window::new(
    App::new(),
    WindowSettings::default()
      .with_flow(Flow::Poll)
      .with_title("Easy Window")
      .with_size((800, 600)),
  )?;

  run(&window);

  Ok(())
}

struct App {
  last_time: Instant,
  time: Time,
}

impl App {
  fn new() -> Self {
    let last_time = Instant::now();
    let time = TimeSettings::default().build();

    Self { last_time, time }
  }

  fn draw(&mut self, window: &Arc<Window>) {
    self.time.update();
    while self.time.should_do_tick_unchecked() {
      self.time.tick();
    }

    if window.key(Key::Escape).is_pressed() {
      window.close();
    }

    let now = Instant::now();
    let elapsed = now.duration_since(self.last_time);
    if elapsed >= Duration::from_secs_f64(0.20) {
      let title = format!(" | FPS: {:.1}", 1.0 / self.time.average_delta_secs());
      window.set_subtitle(title);
      self.last_time = now;
    }
  }
}

impl WindowProcedure for App {
  fn procedure(&mut self, window: &Arc<Window>, message: Message) {
    if !matches!(
      message,
      Message::Unidentified { .. } | Message::None | Message::Window(WindowMessage::Draw)
    ) {
      println!("{message:?}");
    }

    match message {
      Message::Window(WindowMessage::Draw) => self.draw(window),
      _ => window.request_redraw(),
    }
  }
}
