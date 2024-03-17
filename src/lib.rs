/*!
  `witer` (window iterator, "wit-er") is an iterator-based Win32 window library built in Rust.

  # Example

  ```
  use witer::prelude::*;
  /// Configure
  let settings = WindowSettings::default()
    .with_title("My App")
    .with_size(LogicalSize::new(800.0, 600.0), SizeFlag::Outer);
  /// Build
  let window = Window::new(settings)?;
  /// Run
  for message in &window {
    if let Message::Key { .. } = message {
      println!("{message:?}");
    }
  }
  # Ok::<(), witer::debug::error::WindowError>(())
  ```
*/

#![cfg(any(target_os = "windows", doc))]
#![deny(unsafe_op_in_unsafe_fn)]

#[cfg(all(feature = "rwh_05", not(feature = "rwh_06")))]
pub use rwh_05 as raw_window_handle;
#[cfg(all(feature = "rwh_06", not(feature = "rwh_05")))]
pub use rwh_06 as raw_window_handle;

pub mod debug;
#[cfg(feature = "egui")]
pub mod egui;
mod handle;
#[cfg(feature = "opengl")]
pub mod opengl;
pub mod prelude;
pub mod utilities;
pub mod window;

#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadMe;
