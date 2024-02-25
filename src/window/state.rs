use super::window_message::WindowMode;
use crate::window::settings::{ColorMode, Flow, Visibility};

#[derive(Debug)]
pub struct WindowState {
  pub window_mode: WindowMode,
  pub title: String,
  pub subtitle: String,
  pub color_mode: ColorMode,
  pub visibility: Visibility,
  pub flow: Flow,
  pub close_on_x: bool,
  pub sizing_or_moving: bool,
}
