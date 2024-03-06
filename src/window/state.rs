use windows::core::HSTRING;

use super::stage::Stage;
use crate::{
  prelude::Input,
  window::settings::{ColorMode, Flow, Visibility},
};

pub struct InternalState {
  pub subclass: Option<usize>,
  pub title: HSTRING,
  pub subtitle: HSTRING,
  pub color_mode: ColorMode,
  pub visibility: Visibility,
  pub flow: Flow,
  pub close_on_x: bool,
  pub stage: Stage,
  pub input: Input,
}
