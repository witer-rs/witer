use std::thread::JoinHandle;

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
  pub title: String,
  pub subtitle: String,
  pub color_mode: ColorMode,
  pub visibility: Visibility,
  pub flow: Flow,
  pub close_on_x: bool,
  pub stage: Stage,
  pub input: Input,
  pub requested_redraw: bool,
}

impl InternalState {
  pub fn is_closing(&self) -> bool {
    matches!(self.stage, Stage::Closing | Stage::Destroyed)
  }

  pub fn is_destroyed(&self) -> bool {
    self.stage == Stage::Destroyed
  }
}
