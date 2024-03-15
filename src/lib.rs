//! ```
//! use witer::prelude::*;
//!
//! // Configure
//! let settings = WindowSettings::default();
//!
//! // Build
//! let window = Window::new(settings).unwrap();
//!
//! // Run
//! for message in &window {
//!   if let Message::Window(..) = message {
//!     println!("{message:?}");
//!   }
//! }
//! ```

#![cfg(target_os = "windows")]
#![deny(unsafe_op_in_unsafe_fn)]
#![cfg_attr(clippy, deny(warnings))]
#![allow(clippy::missing_safety_doc)]

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
