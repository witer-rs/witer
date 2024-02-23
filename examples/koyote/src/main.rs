#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use std::time::{Duration, Instant};

use ezwin::prelude::*;

fn main() -> WindowResult<()> {
  let window = Window::new(
    WindowSettings::default()
      .with_flow(Flow::Poll)
      .with_title("Easy Window")
      .with_size((800, 600)),
  )?;

  // Loop

  let mut frame_count = 0;
  let mut last_time = Instant::now();

  for msg in &window {
    if let Message::None = msg {
      let now = Instant::now();
      let elapsed = now.duration_since(last_time);
      if elapsed >= Duration::from_secs(1) {
        println!("FPS: {}", frame_count);
        frame_count = 0;
        last_time = now;
      } else {
        frame_count += 1;
      }
    }
  }

  Ok(())
}
