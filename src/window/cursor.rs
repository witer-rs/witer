use cursor_icon::CursorIcon;

use crate::{CursorMode, PhysicalPosition, Visibility};

#[derive(Debug, Clone)]
pub struct Cursor {
  pub mode: CursorMode,
  pub visibility: Visibility,
  pub inside_window: bool,
  pub last_position: PhysicalPosition,
  pub selected_icon: CursorIcon,
}
