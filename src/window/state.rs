use windows::core::HSTRING;

use super::{stage::Stage, window_message::Message};
use crate::{
  prelude::Input,
  window::settings::{ColorMode, Flow, Visibility},
};

#[derive(Debug)]
pub struct InternalState {
  pub title: HSTRING,
  pub subtitle: HSTRING,
  pub color_mode: ColorMode,
  pub visibility: Visibility,
  pub flow: Flow,
  pub close_on_x: bool,
  pub stage: Stage,
  pub input: Input,
  pub message: Option<Message>,
}
