use crate::debug::error::WindowError;

pub mod error;
pub mod validation;

pub type WindowResult<T> = Result<T, WindowError>;
