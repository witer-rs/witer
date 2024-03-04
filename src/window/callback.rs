use std::sync::Arc;

use super::{message::Message, Window};

#[allow(unused)]
pub trait WindowProcedure {
  fn on_message(&mut self, window: &Arc<Window>, message: Message);
}
