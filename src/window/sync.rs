use windows::{core::HSTRING, Win32::UI::WindowsAndMessaging};

use super::settings::Visibility;
#[repr(u32)]

pub enum ThreadCommand {
  CloseConfirmed,
  ShowWindow(Visibility),
  RequestRedraw,
  SetWindowText(HSTRING),
}

#[derive(Clone)]
pub enum Response {
  NextFrame,
}

#[repr(u32)]
pub enum ThreadMessage {
  Empty = WindowsAndMessaging::WM_APP,
  CloseConfirmed,
  ShowWindow,
  SetWindowText,
  RequestRedraw,
}

impl TryFrom<u32> for ThreadMessage {
  type Error = ();

  fn try_from(value: u32) -> Result<Self, Self::Error> {
    match value {
      x if x == Self::Empty as u32 => Ok(Self::Empty),
      x if x == Self::CloseConfirmed as u32 => Ok(Self::CloseConfirmed),
      x if x == Self::ShowWindow as u32 => Ok(Self::ShowWindow),
      x if x == Self::SetWindowText as u32 => Ok(Self::SetWindowText),
      x if x == Self::RequestRedraw as u32 => Ok(Self::RequestRedraw),
      _ => Err(()),
    }
  }
}
