use std::sync::{Mutex, MutexGuard};

static VALIDATION_LAYER_INSTANCE: Mutex<ValidationLayer> =
  Mutex::new(ValidationLayer {});

pub struct ValidationLayer {}

impl ValidationLayer {
  pub fn instance() -> MutexGuard<'static, Self> {
    VALIDATION_LAYER_INSTANCE.lock().unwrap()
  }

  pub fn init(&mut self) -> bool {
    if cfg!(debug_assertions) {
      return true;
    }

    false
  }

  pub fn shutdown(&mut self) {
    if cfg!(debug_assertions) {}
  }
}
