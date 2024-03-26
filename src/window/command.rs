use windows::{
  core::HSTRING,
  Win32::{
    Foundation::{HWND, LPARAM, WPARAM},
    UI::WindowsAndMessaging::{self, PostMessageW, SendMessageW},
  },
};

use super::state::{CursorMode, Fullscreen, Position, Size, Visibility};

#[repr(u32)]
#[derive(Debug, Clone, PartialEq)]
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

impl Command {
  pub const MESSAGE_ID: u32 = WindowsAndMessaging::WM_USER + 69;

  pub fn post(self, hwnd: HWND) {
    let command = Box::leak(Box::new(self));
    let addr = command as *mut Command as usize;
    unsafe {
      if let Err(e) = PostMessageW(hwnd, Self::MESSAGE_ID, WPARAM(addr), LPARAM(0)) {
        tracing::error!("{e}");
      }
    }
  }

  pub(crate) fn send(self, hwnd: HWND) {
    let command = Box::leak(Box::new(self));
    let addr = command as *mut Command as usize;
    unsafe {
      SendMessageW(hwnd, Self::MESSAGE_ID, WPARAM(addr), LPARAM(0));
    }
  }
}
