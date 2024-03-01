#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use std::time::{Duration, Instant};

use ezwin::prelude::*;
use foxy_time::{EngineTime, TimeSettings};

fn main() -> WindowResult<()> {
  let window = Window::new(
    WindowSettings::default()
      .with_flow(Flow::Poll)
      .with_title("Easy Window")
      .with_size((800, 600)),
  )?;

  println!("{:?} | {:?}", window.size(), window.inner_size());

  // Loop

  let mut last_time = Instant::now();
  let mut engine_time = TimeSettings::default().build();

  for msg in &window {
    if let Message::None = msg {
      window.redraw();
    }

    if let Message::Window(WindowMessage::Draw) = msg {
      if window.key(Key::Escape).is_pressed() {
        window.close();
      }
    }

    if let Message::Window(WindowMessage::Resized { .. }) = msg {
      println!("{:?} | {:?}", window.size(), window.inner_size());
    }

    let time = engine_time.time();
    on_frame_infallible(
      &mut engine_time,
      || {},
      || {
        let now = Instant::now();
        let elapsed = now.duration_since(last_time);
        if elapsed >= Duration::from_secs_f64(0.20) {
          window.set_subtitle(format!(" | FPS: {:.1?}", 1.0 / time.average_delta_secs()));
          last_time = now;
        }
      },
    );
  }

  Ok(())
}

// fn on_frame<T, E>(time: &mut EngineTime, mut on_tick: impl FnMut() ->
// Result<T, E>, mut on_update: impl FnMut() -> Result<T, E>) -> Result<T, E> {
//   time.update();
//   while time.should_do_tick_unchecked() {
//     time.tick();
//     on_tick()?;
//   }
//   on_update()
// }

fn on_frame_infallible(
  time: &mut EngineTime,
  mut on_tick: impl FnMut(),
  mut on_update: impl FnMut(),
) {
  time.update();
  while time.should_do_tick_unchecked() {
    time.tick();
    on_tick();
  }
  on_update();
}
