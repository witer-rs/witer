use std::thread::JoinHandle;

use super::stage::Stage;
use crate::{debug::WindowResult, window::Input};

#[derive(Debug, Clone, Copy)]
pub struct StyleInfo {
  pub visibility: Visibility,
  pub decorations: Visibility,
  pub fullscreen: Option<Fullscreen>,
  pub resizeable: bool,
}

pub struct InternalState {
  pub thread: Option<JoinHandle<WindowResult<()>>>,
  pub title: String,
  pub subtitle: String,
  pub theme: Theme,
  pub style: StyleInfo,
  pub windowed_position: Position,
  pub windowed_size: Size,
  pub cursor_mode: CursorMode,
  pub cursor_visibility: Visibility,
  pub scale_factor: f64,
  pub flow: Flow,
  pub close_on_x: bool,
  pub stage: Stage,
  pub input: Input,
  pub requested_redraw: bool,
}

impl InternalState {
  pub fn is_closing(&self) -> bool {
    matches!(self.stage, Stage::Closing | Stage::ExitLoop)
  }
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
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

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Size {
  pub width: i32,
  pub height: i32,
}

impl Size {
  pub fn is_positive(&self) -> bool {
    self.width > 0 && self.height > 0
  }

  pub fn is_negative(&self) -> bool {
    self.width < 0 && self.height < 0
  }

  pub fn is_zero(&self) -> bool {
    self.width == 0 && self.height == 0
  }
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Fullscreen {
  // Exclusive, // todo
  Borderless,
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum CursorMode {
  #[default]
  Normal,
  Confined,
}

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Flow {
  #[default]
  Wait,
  Poll,
}

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Visibility {
  #[default]
  Shown,
  Hidden,
}

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Theme {
  #[default]
  Auto,
  Dark,
  Light,
}
