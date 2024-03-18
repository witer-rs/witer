/*!
  `witer` (window iterator, "wit-er") is an iterator-based Win32 window library built in Rust.

  # Example

  ```
  use witer::prelude::*;
  /*
    Build
  */
  let window = Window::new(
    "My App", // Title
    LogicalSize::new(800.0, 600.0), // Size
    None, // Optional position (None lets Windows decide)
    WindowSettings::default() // Extra settings
  ).unwrap();

  /*
    Run
  */
  for message in &window {
    if let Message::Key { .. } = message {
      println!("{message:?}");
    }
  }
  # Ok::<(), witer::error::WindowError>(())
  ```
*/

#![cfg(any(target_os = "windows", doc))]
#![deny(unsafe_op_in_unsafe_fn)]

#[cfg(all(feature = "rwh_05", not(feature = "rwh_06")))]
pub use rwh_05 as raw_window_handle;
#[cfg(all(feature = "rwh_06", not(feature = "rwh_05")))]
pub use rwh_06 as raw_window_handle;

pub mod compat;
pub mod error;
mod handle;
pub mod prelude;
pub mod utilities;
pub mod window;

#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadMe;

#[cfg(feature = "egui")]
pub use egui;
