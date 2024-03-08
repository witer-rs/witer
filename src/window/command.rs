use windows::core::HSTRING;

use super::state::{Fullscreen, Position, Size, Visibility};

#[repr(u32)]
#[derive(Debug)]
pub enum Command {
  Destroy,
  Redraw,
  SetVisibility(Visibility),
  SetWindowText(HSTRING),
  SetSize(Size),
  SetPosition(Position),
  SetFullscreen(Option<Fullscreen>),
}
