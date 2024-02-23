use crate::debug::error::WindowError;

pub mod error;

pub type WindowResult<T> = Result<T, WindowError>;
