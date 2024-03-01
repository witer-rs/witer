use foxy_renderer::error::RendererError;
use foxy_utils::thread::error::ThreadError;
use thiserror::Error;

pub mod builder;
pub mod framework;
pub mod message;
pub mod runnable;
pub mod state;

pub type FoxyResult<T> = Result<T, FoxyError>;

#[derive(Debug, Error)]
pub enum FoxyError {
  #[error("{0}")]
  Error(String),
  #[error("{0}")]
  RendererError(#[from] RendererError),
  #[error("{0}")]
  ThreadError(#[from] ThreadError),
  #[error("{0}")]
  IOError(#[from] std::io::Error),
  #[error("{0}")]
  EzwinError(#[from] ezwin::debug::error::WindowError),
}

#[macro_export]
macro_rules! foxy_error {
  () => {
    $crate::core::FoxyError::Error("foxy error".to_string())
  };
  ($($arg:tt)*) => {{
    $crate::core::FoxyError::Error(format!($($arg)*))
  }}
}

// #[macro_export]
// macro_rules! foxy_err {
//   () => {
//     Err($crate::core::FoxyError::Error("foxy error".to_string()))
//   };
//   ($($arg:tt)*) => {{
//     Err($crate::core::FoxyError::Error(format!($($arg)*)))
//   }}
// }
