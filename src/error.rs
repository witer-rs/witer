use std::io;

use thiserror::Error;

pub type WindowResult<T> = Result<T, WindowError>;

#[derive(Error, Debug)]
pub enum WindowError {
  #[error("{0}")]
  Error(String),
  #[error("{0}")]
  IOError(#[from] io::Error),
  #[error("{0}")]
  Win32Error(#[from] windows::core::Error),
}

#[macro_export]
macro_rules! window_error {
  () => {
    $crate::debug::error::WindowError::Error("window error".to_string())
  };
  ($($arg:tt)*) => {{
    $crate::debug::error::WindowError::Error(format!($($arg)*))
  }}
}
