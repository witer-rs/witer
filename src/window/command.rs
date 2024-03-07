use windows::core::HSTRING;

use super::settings::Visibility;

#[repr(u32)]
#[derive(Debug)]
pub enum Command {
  Close,
  SetVisibility(Visibility),
  Redraw,
  SetWindowText(HSTRING),
}
