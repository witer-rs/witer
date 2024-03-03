use std::sync::Arc;

use super::{window_message::Message, Window};

#[allow(unused)]
pub trait WindowProcedure {
  fn procedure(&mut self, window: &Arc<Window>, message: Message);
}
