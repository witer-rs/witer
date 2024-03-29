/*!
  `witer` (window iterator, "wit-er") is an iterator-based Win32 window library built in Rust.

  # Example

  ```
  use witer::prelude::*;

  // Build
  let window = Window::builder()
    .with_title("My App")
    .with_size(LogicalSize::new(800.0, 500.0))
    .build()?;

  // Run
  for message in &window {
    if let Message::Key { .. } = message {
      println!("{message:?}");
    }
  }
  # Ok::<(), witer::error::WindowError>(())
  ```

  Please note that the window will wait to process new messages until the end of each cycle of the loop, despite
  being on a separate thread. This keeps the window in sync with the main thread to prevent things such as input
  lag.
*/

#![cfg(any(target_os = "windows", doc))]
#![deny(unsafe_op_in_unsafe_fn)]

#[cfg(all(feature = "rwh_05", not(feature = "rwh_06")))]
pub use rwh_05 as raw_window_handle;
#[cfg(all(feature = "rwh_06", not(feature = "rwh_05")))]
pub use rwh_06 as raw_window_handle;

pub mod compat;
pub mod error;
pub mod prelude;
pub mod utilities;
pub mod window;

// re-exports
pub use window::{
  data::{
    CursorMode,
    Flow,
    Fullscreen,
    LogicalPosition,
    LogicalSize,
    PhysicalPosition,
    PhysicalSize,
    Position,
    Size,
    Theme,
    Visibility,
  },
  input::{
    key::Key,
    mouse::MouseButton,
    state::{ButtonState, KeyState, RawKeyState},
    Input,
  },
  message::{LoopMessage, Message, RawInputMessage},
  settings::{WindowBuilder, WindowSettings},
  Window,
};

#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadMe;
