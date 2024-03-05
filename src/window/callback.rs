use std::{marker::PhantomData, sync::Arc};

use super::{message::Message, Window};

/// Configures the window callback to be used, along with extra data to be
/// passed to [`WindowCallback::on_create`].
pub struct CallbackSettings<P: WindowCallback<T>, T> {
  pub additional_data: T,
  _p: PhantomData<P>,
}

impl<P: WindowCallback<T>, T> CallbackSettings<P, T> {
  pub fn new(additional_data: T) -> Self {
    Self {
      _p: PhantomData,
      additional_data,
    }
  }
}

#[allow(unused)]
/// Implement this for your app to allow the app to open a Win32 window and
/// react to it.
pub trait WindowCallback<T> {
  /// Use to initialize app state and recieve extra data from
  /// [`CallbackSettings::new()`]
  fn on_create(window: &Arc<Window>, additional_data: T) -> Option<Self>
  where
    Self: Sized;

  /// Use to react to messages and manipulate app state
  fn on_message(&mut self, window: &Arc<Window>, message: Message) {}
}
