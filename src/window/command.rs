use winit::{
  dpi::{Position, Size},
  window::{CursorGrabMode, Fullscreen},
};

#[repr(u32)]
#[derive(Debug, Clone)]
pub enum Command {
  Close,
  Redraw,
  SetVisibility(bool),
  SetDecorations(bool),
  SetWindowText(String),
  SetSize(Size),
  SetPosition(Position),
  SetFullscreen(Option<Fullscreen>),
  SetCursorMode(CursorGrabMode),
  SetCursorVisibility(bool),
}
