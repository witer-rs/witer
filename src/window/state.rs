use super::{input::Input, window_message::SizeState};
use crate::window::settings::{ColorMode, Flow, Visibility};

#[derive(Debug)]
pub struct WindowState {
  pub h_wnd: isize,
  pub hinstance: isize,
  pub size_state: SizeState,
  pub title: String,
  pub subtitle: String,
  pub color_mode: ColorMode,
  pub visibility: Visibility,
  pub flow: Flow,
  pub input: Input,
  pub close_on_x: bool,
  pub sizing_or_moving: bool,
}
