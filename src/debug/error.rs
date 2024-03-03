use std::io;

use thiserror::Error;

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
    $crate::core::WindowError::Error("window error".to_string())
  };
  ($($arg:tt)*) => {{
    $crate::core::WindowError::Error(format!($($arg)*))
  }}
}
