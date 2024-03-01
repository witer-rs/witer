use thiserror::Error;
use vulkano::{command_buffer::CommandBufferExecError, Validated};

#[derive(Error, Debug)]
pub enum RendererError {
  #[error("{0}")]
  Error(String),
  #[error("{0}")]
  VulkanoError(#[from] vulkano::VulkanError),
  #[error("{0}")]
  ValidatedVulkanoError(#[from] Validated<vulkano::VulkanError>),
  #[error("{0}")]
  AllocateImageError(#[from] Validated<vulkano::image::AllocateImageError>),
  #[error("{0}")]
  LoadingError(#[from] vulkano::LoadingError),
  #[error("{0}")]
  ValidationError(#[from] Box<vulkano::ValidationError>),
  #[error("{0}")]
  CommandBufferExecError(#[from] CommandBufferExecError),
  #[error("{0}")]
  IO(#[from] std::io::Error),
}

#[macro_export]
macro_rules! renderer_error {
  () => {
    $crate::error::RendererError::Error("renderer error".to_string())
  };
  ($($arg:tt)*) => {{
    $crate::error::RendererError::Error(format!($($arg)*))
  }}
}
