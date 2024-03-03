use std::sync::Arc;

use super::{message::Message, Window};

#[allow(unused)]
pub trait WindowProcedure {
  fn new(window: &Arc<Window>) -> Self
  where
    Self: Sized;
  fn procedure(&mut self, window: &Arc<Window>, message: Message);
}
