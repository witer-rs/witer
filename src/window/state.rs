use std::thread::JoinHandle;

use super::stage::Stage;
use crate::{
  debug::WindowResult,
  window::{
    settings::{Flow, Theme, Visibility},
    Input,
  },
};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Position {
  pub x: i32,
  pub y: i32,
}

impl From<Position> for (i32, i32) {
  fn from(val: Position) -> Self {
    (val.x, val.y)
  }
}

impl From<Position> for [i32; 2] {
  fn from(val: Position) -> Self {
    [val.x, val.y]
  }
}

impl From<(i32, i32)> for Position {
  fn from(value: (i32, i32)) -> Self {
    Self {
      x: value.0,
      y: value.1,
    }
  }
}

impl From<[i32; 2]> for Position {
  fn from(value: [i32; 2]) -> Self {
    Self {
      x: value[0],
      y: value[1],
    }
  }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Size {
  pub width: i32,
  pub height: i32,
}

impl From<Size> for (u32, u32) {
  fn from(val: Size) -> Self {
    (val.width as u32, val.height as u32)
  }
}

impl From<Size> for (i32, i32) {
  fn from(val: Size) -> Self {
    (val.width, val.height)
  }
}

impl From<Size> for [u32; 2] {
  fn from(val: Size) -> Self {
    [val.width as u32, val.height as u32]
  }
}

impl From<Size> for [i32; 2] {
  fn from(val: Size) -> Self {
    [val.width, val.height]
  }
}

impl From<(i32, i32)> for Size {
  fn from(value: (i32, i32)) -> Self {
    Self {
      width: value.0,
      height: value.1,
    }
  }
}

impl From<[i32; 2]> for Size {
  fn from(value: [i32; 2]) -> Self {
    Self {
      width: value[0],
      height: value[1],
    }
  }
}

pub struct InternalState {
  pub thread: Option<JoinHandle<WindowResult<()>>>,
  pub title: String,
  pub subtitle: String,
  pub theme: Theme,
  pub visibility: Visibility,
  pub flow: Flow,
  pub close_on_x: bool,
  pub stage: Stage,
  pub input: Input,
  pub requested_redraw: bool,
}

impl InternalState {
  pub fn is_closing(&self) -> bool {
    matches!(self.stage, Stage::Closing | Stage::Destroyed | Stage::ExitLoop)
  }

  pub fn is_destroyed(&self) -> bool {
    matches!(self.stage, Stage::Destroyed | Stage::ExitLoop)
  }
}
