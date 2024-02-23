use std::io;

use crossbeam::channel::{RecvError, SendError, TryRecvError};
use thiserror::Error;

use crate::prelude::{MainMessage, Message};

#[derive(Error, Debug)]
pub enum WindowError {
  #[error("{0}")]
  Error(String),
  #[error("{0}")]
  IOError(#[from] io::Error),
  #[error("{0}")]
  Win32Error(#[from] windows::core::Error),
  #[error("{0}")]
  RecvError(#[from] RecvError),
  #[error("{0}")]
  TryRecvError(#[from] TryRecvError),
  #[error("{0}")]
  WindowSendError(#[from] SendError<Message>),
  #[error("{0}")]
  MainSendError(#[from] SendError<MainMessage>),
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
