use std::sync::Arc;

use super::{message::Message, Window};

#[allow(unused)]
/// Implement this for your app to allow the app to open a Win32 window and
/// react to it.
pub trait WindowProcedure<T> {
  /// Use to initialize app state and recieve extra data from
  /// [`WindowSettings::new()`]
  fn on_create(window: &Arc<Window>, additional_data: T) -> Option<Self>
  where
    Self: Sized;

  /// Use to react to messages and manipulate app state
  fn on_message(&mut self, window: &Arc<Window>, message: Message) {}
}
