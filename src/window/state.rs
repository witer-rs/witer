#[cfg(all(feature = "rwh_06", not(feature = "rwh_05")))]
use rwh_06::{RawDisplayHandle, RawWindowHandle};
use windows::core::HSTRING;

use crate::{
  prelude::Input,
  window::{
    settings::{ColorMode, Flow, Visibility},
    stage::Stage,
  },
};

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
  pub current_stage: Stage,
  pub close_on_x: bool,
  pub is_sizing_or_moving: bool,
  pub is_closing: bool,
  // pub receiver: Receiver<Message>,
  // pub window_thread: Option<JoinHandle<WindowResult<()>>>,
  pub input: Input,
}
