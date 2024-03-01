use std::sync::Arc;

use tracing::{error, warn};
use vulkano::instance::{
  debug::{
    DebugUtilsMessageSeverity,
    DebugUtilsMessageType,
    DebugUtilsMessenger,
    DebugUtilsMessengerCallback,
    DebugUtilsMessengerCreateInfo,
  },
  Instance,
};

use super::instance::FoxyInstance;
use crate::error::RendererError;

pub struct Debug {
  _debug: Option<DebugUtilsMessenger>,
}

impl Debug {
  pub fn new(instance: Arc<Instance>) -> Result<Arc<Self>, RendererError> {
    if FoxyInstance::ENABLE_VALIDATION_LAYERS {
      let debug = DebugUtilsMessenger::new(instance, DebugUtilsMessengerCreateInfo {
        message_severity: DebugUtilsMessageSeverity::ERROR | DebugUtilsMessageSeverity::WARNING,
        message_type: DebugUtilsMessageType::VALIDATION | DebugUtilsMessageType::PERFORMANCE,
        ..DebugUtilsMessengerCreateInfo::user_callback(unsafe {
          DebugUtilsMessengerCallback::new(|sev, ty, data| {
            let ty = if ty.intersects(DebugUtilsMessageType::GENERAL) {
              "General"
            } else if ty.intersects(DebugUtilsMessageType::VALIDATION) {
              "Validation"
            } else {
              "Performance"
            };

            let msg = format!("Vulkan {ty}: {:?}", data.message);

            match sev {
              DebugUtilsMessageSeverity::ERROR => error!(msg),
              DebugUtilsMessageSeverity::WARNING => warn!(msg),
              _ => (),
            }
          })
        })
      })?;
      Ok(Arc::new(Self { _debug: Some(debug) }))
    } else {
      Ok(Arc::new(Self { _debug: None }))
    }
  }
}
