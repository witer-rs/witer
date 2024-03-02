use std::sync::Arc;

use super::{window_message::Message, Window};

#[allow(unused)]
pub trait Callback {
  fn callback(&mut self, window: &Arc<Window>, message: Message);
}

pub struct WindowCallback;

impl WindowCallback {
  pub fn new() -> Self {
    Self
  }
}

impl Default for WindowCallback {
  fn default() -> Self {
    Self::new()
  }
}

impl Callback for WindowCallback {
  fn callback(&mut self, window: &Arc<Window>, message: Message) {
    println!("Callback: {message:?}");
    window.state.get_mut().message = Some(message);
  }
}
