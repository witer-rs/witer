use windows::core::HSTRING;

use super::state::{CursorMode, Fullscreen, Position, Size, Visibility};

#[repr(u32)]
#[derive(Debug)]
pub enum Command {
  Destroy,
  Redraw,
  SetVisibility(Visibility),
  SetDecorations(Visibility),
  SetWindowText(HSTRING),
  SetSize(Size),
  SetPosition(Position),
  SetFullscreen(Option<Fullscreen>),
  SetCursorMode(CursorMode),
  SetCursorVisibility(Visibility),
}
