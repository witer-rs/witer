use std::{
  ops::{Div, Mul},
  thread::JoinHandle,
};

use windows::Win32::{
  Foundation::{HWND, RECT},
  UI::WindowsAndMessaging::GetWindowRect,
};

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
  // pub position: Position,
  // pub size: Size,
  pub last_windowed_position: Position,
  pub last_windowed_size: Size,
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

  pub(crate) fn update_last_windowed_pos_size(&mut self, hwnd: HWND) {
    let mut window_rect = RECT::default();
    let _ = unsafe { GetWindowRect(hwnd, &mut window_rect) };
    let size = PhysicalSize {
      width: (window_rect.right - window_rect.left) as u32,
      height: (window_rect.bottom - window_rect.top) as u32,
    };
    self.last_windowed_size = size.into();
    let position = PhysicalPosition {
      x: window_rect.left,
      y: window_rect.top,
    };
    self.last_windowed_position = position.into();
  }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Position {
  Logical(LogicalPosition),
  Physical(PhysicalPosition),
}

impl Position {
  pub fn new(position: impl Into<Self>) -> Self {
    position.into()
  }

  pub fn as_logical(&self, scale_factor: f64) -> LogicalPosition {
    match *self {
      Position::Logical(position) => position,
      Position::Physical(position) => position.as_logical(scale_factor),
    }
  }

  pub fn as_physical(&self, scale_factor: f64) -> PhysicalPosition {
    match *self {
      Position::Logical(position) => position.as_physical(scale_factor),
      Position::Physical(position) => position,
    }
  }
}

impl From<LogicalPosition> for Position {
  fn from(val: LogicalPosition) -> Self {
    Self::Logical(val)
  }
}

impl From<(f64, f64)> for Position {
  fn from(val: (f64, f64)) -> Self {
    Self::Logical(val.into())
  }
}

impl From<[f64; 2]> for Position {
  fn from(val: [f64; 2]) -> Self {
    Self::Logical(val.into())
  }
}

impl From<PhysicalPosition> for Position {
  fn from(val: PhysicalPosition) -> Self {
    Self::Physical(val)
  }
}

impl From<(i32, i32)> for Position {
  fn from(val: (i32, i32)) -> Self {
    Self::Physical(val.into())
  }
}

impl From<[i32; 2]> for Position {
  fn from(val: [i32; 2]) -> Self {
    Self::Physical(val.into())
  }
}

#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct LogicalPosition {
  pub x: f64,
  pub y: f64,
}

impl LogicalPosition {
  pub fn new(x: f64, y: f64) -> Self {
    Self { x, y }
  }

  pub fn as_physical(&self, scale_factor: f64) -> PhysicalPosition {
    PhysicalPosition::new(self.x.round() as i32, self.y.round() as i32) * scale_factor
  }

  pub fn is_positive(&self) -> bool {
    self.x > 0.0 && self.y > 0.0
  }

  pub fn is_negative(&self) -> bool {
    self.x < 0.0 && self.y < 0.0
  }

  pub fn is_zero(&self) -> bool {
    self.x == 0.0 && self.y == 0.0
  }
}

impl Div<f64> for LogicalPosition {
  type Output = Self;

  fn div(self, rhs: f64) -> Self::Output {
    Self {
      y: (self.y / rhs).round(),
      x: (self.x / rhs).round(),
    }
  }
}

impl Mul<f64> for LogicalPosition {
  type Output = Self;

  fn mul(self, rhs: f64) -> Self::Output {
    Self {
      y: (self.y * rhs).round(),
      x: (self.x * rhs).round(),
    }
  }
}

impl From<LogicalPosition> for (f64, f64) {
  fn from(val: LogicalPosition) -> Self {
    (val.x, val.y)
  }
}

impl From<LogicalPosition> for [f64; 2] {
  fn from(val: LogicalPosition) -> Self {
    [val.x, val.y]
  }
}

impl From<(f64, f64)> for LogicalPosition {
  fn from(value: (f64, f64)) -> Self {
    Self {
      x: value.0,
      y: value.1,
    }
  }
}

impl From<[f64; 2]> for LogicalPosition {
  fn from(value: [f64; 2]) -> Self {
    Self {
      x: value[0],
      y: value[1],
    }
  }
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct PhysicalPosition {
  pub x: i32,
  pub y: i32,
}

impl PhysicalPosition {
  pub fn new(x: i32, y: i32) -> Self {
    Self { x, y }
  }

  pub fn as_logical(&self, scale_factor: f64) -> LogicalPosition {
    LogicalPosition::new(self.x as f64, self.y as f64) / scale_factor
  }

  pub fn is_positive(&self) -> bool {
    self.x > 0 && self.y > 0
  }

  pub fn is_negative(&self) -> bool {
    self.x < 0 && self.y < 0
  }

  pub fn is_zero(&self) -> bool {
    self.x == 0 && self.y == 0
  }
}

impl Div<f64> for PhysicalPosition {
  type Output = Self;

  fn div(self, rhs: f64) -> Self::Output {
    Self {
      y: (self.y as f64 / rhs).round() as i32,
      x: (self.x as f64 / rhs).round() as i32,
    }
  }
}

impl Mul<f64> for PhysicalPosition {
  type Output = Self;

  fn mul(self, rhs: f64) -> Self::Output {
    Self {
      y: (self.y as f64 * rhs).trunc() as i32,
      x: (self.x as f64 * rhs).trunc() as i32,
    }
  }
}

impl From<PhysicalPosition> for (u32, u32) {
  fn from(val: PhysicalPosition) -> Self {
    (val.x as u32, val.y as u32)
  }
}

impl From<PhysicalPosition> for (i32, i32) {
  fn from(val: PhysicalPosition) -> Self {
    (val.x, val.y)
  }
}

impl From<PhysicalPosition> for [u32; 2] {
  fn from(val: PhysicalPosition) -> Self {
    [val.x as u32, val.y as u32]
  }
}

impl From<PhysicalPosition> for [i32; 2] {
  fn from(val: PhysicalPosition) -> Self {
    [val.x, val.y]
  }
}

impl From<(i32, i32)> for PhysicalPosition {
  fn from(value: (i32, i32)) -> Self {
    Self {
      x: value.0,
      y: value.1,
    }
  }
}

impl From<[i32; 2]> for PhysicalPosition {
  fn from(value: [i32; 2]) -> Self {
    Self {
      x: value[0],
      y: value[1],
    }
  }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Size {
  Logical(LogicalSize),
  Physical(PhysicalSize),
}

impl Size {
  pub fn new(size: impl Into<Self>) -> Self {
    size.into()
  }

  pub fn as_logical(&self, scale_factor: f64) -> LogicalSize {
    match *self {
      Size::Logical(size) => size,
      Size::Physical(size) => size.as_logical(scale_factor),
    }
  }

  pub fn as_physical(&self, scale_factor: f64) -> PhysicalSize {
    match *self {
      Size::Logical(size) => size.as_physical(scale_factor),
      Size::Physical(size) => size,
    }
  }
}

impl From<LogicalSize> for Size {
  fn from(val: LogicalSize) -> Self {
    Self::Logical(val)
  }
}

impl From<PhysicalSize> for Size {
  fn from(val: PhysicalSize) -> Self {
    Self::Physical(val)
  }
}

#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct LogicalSize {
  pub width: f64,
  pub height: f64,
}

impl LogicalSize {
  pub fn new(width: f64, height: f64) -> Self {
    Self { width, height }
  }

  pub fn as_physical(&self, scale_factor: f64) -> PhysicalSize {
    PhysicalSize::new(self.width.round() as u32, self.height.round() as u32)
      * scale_factor
  }

  pub fn is_any_positive(&self) -> bool {
    self.width > 0.0 || self.height > 0.0
  }

  pub fn is_all_positive(&self) -> bool {
    self.width > 0.0 && self.height > 0.0
  }

  pub fn is_any_negative(&self) -> bool {
    self.width < 0.0 || self.height < 0.0
  }

  pub fn is_all_negative(&self) -> bool {
    self.width < 0.0 && self.height < 0.0
  }

  pub fn is_any_zero(&self) -> bool {
    self.width == 0.0 || self.height == 0.0
  }

  pub fn is_all_zero(&self) -> bool {
    self.width == 0.0 && self.height == 0.0
  }
}

impl Div<f64> for LogicalSize {
  type Output = Self;

  fn div(self, rhs: f64) -> Self::Output {
    Self {
      height: (self.height / rhs).round(),
      width: (self.width / rhs).round(),
    }
  }
}

impl Mul<f64> for LogicalSize {
  type Output = Self;

  fn mul(self, rhs: f64) -> Self::Output {
    Self {
      height: (self.height * rhs).round(),
      width: (self.width * rhs).round(),
    }
  }
}

impl From<LogicalSize> for (f64, f64) {
  fn from(val: LogicalSize) -> Self {
    (val.width, val.height)
  }
}

impl From<LogicalSize> for [f64; 2] {
  fn from(val: LogicalSize) -> Self {
    [val.width, val.height]
  }
}

impl From<(f64, f64)> for LogicalSize {
  fn from(value: (f64, f64)) -> Self {
    Self {
      width: value.0,
      height: value.1,
    }
  }
}

impl From<[f64; 2]> for LogicalSize {
  fn from(value: [f64; 2]) -> Self {
    Self {
      width: value[0],
      height: value[1],
    }
  }
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct PhysicalSize {
  pub width: u32,
  pub height: u32,
}

impl PhysicalSize {
  pub fn new(width: u32, height: u32) -> Self {
    Self { width, height }
  }

  pub fn as_logical(&self, scale_factor: f64) -> LogicalSize {
    LogicalSize::new(self.width as f64, self.height as f64) / scale_factor
  }

  pub fn is_any_zero(&self) -> bool {
    self.width == 0 || self.height == 0
  }

  pub fn is_all_zero(&self) -> bool {
    self.width == 0 && self.height == 0
  }
}

impl Div<f64> for PhysicalSize {
  type Output = Self;

  fn div(self, rhs: f64) -> Self::Output {
    Self {
      height: (self.height as f64 / rhs).round() as u32,
      width: (self.width as f64 / rhs).round() as u32,
    }
  }
}

impl Mul<f64> for PhysicalSize {
  type Output = Self;

  fn mul(self, rhs: f64) -> Self::Output {
    Self {
      height: (self.height as f64 * rhs).trunc() as u32,
      width: (self.width as f64 * rhs).trunc() as u32,
    }
  }
}

impl From<PhysicalSize> for (u32, u32) {
  fn from(val: PhysicalSize) -> Self {
    (val.width, val.height)
  }
}

impl From<PhysicalSize> for (i32, i32) {
  fn from(val: PhysicalSize) -> Self {
    (val.width as i32, val.height as i32)
  }
}

impl From<PhysicalSize> for [u32; 2] {
  fn from(val: PhysicalSize) -> Self {
    [val.width, val.height]
  }
}

impl From<PhysicalSize> for [i32; 2] {
  fn from(val: PhysicalSize) -> Self {
    [val.width as i32, val.height as i32]
  }
}

impl From<(u32, u32)> for PhysicalSize {
  fn from(value: (u32, u32)) -> Self {
    Self {
      width: value.0,
      height: value.1,
    }
  }
}

impl From<[u32; 2]> for PhysicalSize {
  fn from(value: [u32; 2]) -> Self {
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
