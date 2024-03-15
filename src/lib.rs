// #![feature(c_str_literals)]
#![cfg(target_os = "windows")]
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
