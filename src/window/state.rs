use std::thread::JoinHandle;

use windows::core::HSTRING;

use super::stage::Stage;
use crate::{
  debug::WindowResult,
  window::{
    settings::{ColorMode, Flow, Visibility},
    Input,
  },
};

pub struct InternalState {
  pub thread: Option<JoinHandle<WindowResult<()>>>,
  pub title: HSTRING,
  pub subtitle: HSTRING,
  pub color_mode: ColorMode,
  pub visibility: Visibility,
  pub flow: Flow,
  pub close_on_x: bool,
  pub stage: Stage,
  pub input: Input,
  pub requested_redraw: bool,
}
