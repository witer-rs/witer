use std::sync::Arc;

use cursor_icon::CursorIcon;
use windows::{
  core::HSTRING,
  Win32::{
    Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
    Graphics::Gdi::{
      self,
      GetMonitorInfoW,
      InvalidateRgn,
      MonitorFromWindow,
      RedrawWindow,
      MONITORINFO,
    },
    UI::WindowsAndMessaging::{
      self,
      DestroyWindow,
      LoadCursorW,
      PostMessageW,
      SendMessageW,
      SetCursor,
      SetWindowLongW,
      SetWindowPos,
      SetWindowTextW,
      ShowWindow,
    },
  },
};

use super::data::{CursorMode, Fullscreen, Internal, Position, Size, Visibility};
use crate::{
  utilities::{get_window_ex_style, get_window_style, to_windows_cursor},
  LoopMessage,
  Message,
};

#[repr(u32)]
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
  Exit,
  Destroy,
  Redraw,
  SetVisibility(Visibility),
  SetDecorations(Visibility),
  SetWindowText(HSTRING),
  SetSize(Size),
  SetPosition(Position),
  SetFullscreen(Option<Fullscreen>),
  SetCursorIcon(CursorIcon),
  SetCursorMode(CursorMode),
  SetCursorVisibility(Visibility),
}

impl Command {
  pub const MESSAGE_ID: u32 = WindowsAndMessaging::WM_USER + 69;

  pub(crate) fn from_wparam(wparam: WPARAM) -> Self {
    *unsafe { Box::from_raw(wparam.0 as *mut Command) }
  }

  pub fn post(self, hwnd: usize) {
    let command = Box::leak(Box::new(self));
    let addr = command as *mut Command as usize;
    unsafe {
      if let Err(e) =
        PostMessageW(HWND(hwnd as _), Self::MESSAGE_ID, WPARAM(addr), LPARAM(0))
      {
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

  pub(crate) fn process(self, hwnd: HWND, internal: Option<Arc<Internal>>) -> LRESULT {
    match (internal, self) {
      (Some(internal), Command::Exit) => {
        internal.send_message_to_main(Message::Loop(LoopMessage::Exit));
      }
      (None, Command::Destroy) => {
        unsafe { DestroyWindow(hwnd) }.unwrap();
      }
      (Some(_), Command::Redraw) => unsafe {
        let _ = RedrawWindow(hwnd, None, None, Gdi::RDW_INTERNALPAINT);
      },
      (Some(_), Command::SetVisibility(visibility)) => unsafe {
        let _ = ShowWindow(hwnd, match visibility {
          Visibility::Hidden => WindowsAndMessaging::SW_HIDE,
          Visibility::Shown => WindowsAndMessaging::SW_SHOW,
        });
      },
      (Some(internal), Command::SetDecorations(decorations)) => {
        let style = internal.data.lock().unwrap().style.clone();
        match decorations {
          Visibility::Shown => {
            unsafe {
              SetWindowLongW(
                hwnd,
                WindowsAndMessaging::GWL_STYLE,
                get_window_style(&style).0 as i32,
              )
            };
            unsafe {
              SetWindowLongW(
                hwnd,
                WindowsAndMessaging::GWL_EXSTYLE,
                get_window_ex_style(&style).0 as i32,
              )
            };
          }
          Visibility::Hidden => {
            unsafe {
              SetWindowLongW(
                hwnd,
                WindowsAndMessaging::GWL_STYLE,
                get_window_style(&style).0 as i32,
              )
            };
            unsafe {
              SetWindowLongW(
                hwnd,
                WindowsAndMessaging::GWL_EXSTYLE,
                get_window_ex_style(&style).0 as i32,
              )
            };
          }
        }
        unsafe {
          SetWindowPos(
            hwnd,
            None,
            0,
            0,
            0,
            0,
            WindowsAndMessaging::SWP_NOZORDER
              | WindowsAndMessaging::SWP_NOMOVE
              | WindowsAndMessaging::SWP_NOSIZE
              | WindowsAndMessaging::SWP_NOACTIVATE
              | WindowsAndMessaging::SWP_FRAMECHANGED,
          )
          .expect("Failed to set window size");
        }
      }
      (Some(_), Command::SetWindowText(text)) => unsafe {
        SetWindowTextW(hwnd, &text).unwrap();
      },
      (Some(internal), Command::SetSize(size)) => {
        let physical_size = size.as_physical(internal.data.lock().unwrap().scale_factor);
        unsafe {
          SetWindowPos(
            hwnd,
            None,
            0,
            0,
            physical_size.width as i32,
            physical_size.height as i32,
            WindowsAndMessaging::SWP_NOZORDER
              | WindowsAndMessaging::SWP_NOMOVE
              | WindowsAndMessaging::SWP_NOREPOSITION
              | WindowsAndMessaging::SWP_NOACTIVATE,
          )
          .expect("Failed to set window size");
        }
        let _ = unsafe { InvalidateRgn(hwnd, None, false) };
      }
      (Some(internal), Command::SetPosition(position)) => {
        let physical_position =
          position.as_physical(internal.data.lock().unwrap().scale_factor);
        unsafe {
          SetWindowPos(
            hwnd,
            None,
            physical_position.x,
            physical_position.y,
            0,
            0,
            WindowsAndMessaging::SWP_NOZORDER
              | WindowsAndMessaging::SWP_NOSIZE
              | WindowsAndMessaging::SWP_NOREPOSITION
              | WindowsAndMessaging::SWP_NOACTIVATE,
          )
          .expect("Failed to set window position");
        }
        let _ = unsafe { InvalidateRgn(hwnd, None, false) };
      }
      (Some(internal), Command::SetFullscreen(fullscreen)) => {
        // update style
        let style = internal.data.lock().unwrap().style.clone();
        unsafe {
          SetWindowLongW(
            hwnd,
            WindowsAndMessaging::GWL_STYLE,
            get_window_style(&style).0 as i32,
          )
        };
        unsafe {
          SetWindowLongW(
            hwnd,
            WindowsAndMessaging::GWL_EXSTYLE,
            get_window_ex_style(&style).0 as i32,
          )
        };
        // update size
        match fullscreen {
          Some(Fullscreen::Borderless) => {
            let monitor =
              unsafe { MonitorFromWindow(hwnd, Gdi::MONITOR_DEFAULTTONEAREST) };
            let mut info = MONITORINFO {
              cbSize: std::mem::size_of::<MONITORINFO>() as u32,
              ..Default::default()
            };
            if unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
              unsafe {
                SetWindowPos(
                  hwnd,
                  None,
                  info.rcMonitor.left,
                  info.rcMonitor.top,
                  info.rcMonitor.right - info.rcMonitor.left,
                  info.rcMonitor.bottom - info.rcMonitor.top,
                  WindowsAndMessaging::SWP_ASYNCWINDOWPOS
                    | WindowsAndMessaging::SWP_NOZORDER
                    | WindowsAndMessaging::SWP_FRAMECHANGED,
                )
                .expect("Failed to set window to fullscreen");
              }
              let _ = unsafe { InvalidateRgn(hwnd, None, false) };
            }
          }
          None => {
            let scale_factor = internal.data.lock().unwrap().scale_factor;
            let size = internal
              .data
              .lock()
              .unwrap()
              .last_windowed_size
              .as_physical(scale_factor);
            let position = internal
              .data
              .lock()
              .unwrap()
              .last_windowed_position
              .as_physical(scale_factor);
            unsafe {
              SetWindowPos(
                hwnd,
                None,
                position.x,
                position.y,
                size.width as i32,
                size.height as i32,
                WindowsAndMessaging::SWP_ASYNCWINDOWPOS
                  | WindowsAndMessaging::SWP_NOZORDER
                  | WindowsAndMessaging::SWP_FRAMECHANGED,
              )
              .expect("Failed to set window to windowed");
            };
            let _ = unsafe { InvalidateRgn(hwnd, None, false) };
          }
        }
      }
      (Some(internal), Command::SetCursorIcon(icon)) => {
        internal.data.lock().unwrap().cursor.selected_icon = icon;
        let cursor_icon = to_windows_cursor(icon);
        let hcursor = unsafe { LoadCursorW(HINSTANCE::default(), cursor_icon) }.unwrap();
        unsafe { SetCursor(hcursor) };
      }
      (Some(internal), Command::SetCursorMode(mode)) => {
        internal.data.lock().unwrap().cursor.mode = mode;
        if let Err(e) = internal.refresh_os_cursor() {
          tracing::error!("{e}");
        };

        // match mode {
        //   CursorMode::Normal => {
        //     set_cursor_clip(None);
        //   }
        //   CursorMode::Confined => {
        //     let mut client_rect = RECT::default();
        //     unsafe { GetClientRect(hwnd, &mut client_rect) }.unwrap();
        //     tracing::debug!("{client_rect:?}");
        //     set_cursor_clip(Some(&client_rect));
        //   }
        // };
      }
      (Some(internal), Command::SetCursorVisibility(visibility)) => {
        internal.data.lock().unwrap().cursor.visibility = visibility;
        if let Err(e) = internal.refresh_os_cursor() {
          tracing::error!("{e}");
        };
      }
      (..) => (),
    }

    LRESULT(0)
  }
}
