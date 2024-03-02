#[cfg(all(feature = "rwh_06", not(feature = "rwh_05")))]
use rwh_06::{RawDisplayHandle, RawWindowHandle};
use windows::core::HSTRING;

use crate::{
  prelude::Input,
  window::settings::{ColorMode, Flow, Visibility},
};

use super::{stage::Stage, window_message::Message};

#[derive(Debug)]
pub struct InternalState {
  #[cfg(all(feature = "rwh_06", not(feature = "rwh_05")))]
  pub raw_window_handle: RawWindowHandle,
  #[cfg(all(feature = "rwh_06", not(feature = "rwh_05")))]
  pub raw_display_handle: RawDisplayHandle,
  // pub window_mode: WindowMode,
  pub title: HSTRING,
  pub subtitle: HSTRING,
  pub color_mode: ColorMode,
  pub visibility: Visibility,
  pub flow: Flow,
  pub close_on_x: bool,
  pub stage: Stage,
  // pub receiver: Receiver<Message>,
  // pub window_thread: Option<JoinHandle<WindowResult<()>>>,
  pub input: Input,
  pub message: Option<Message>,
}
