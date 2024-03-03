use crossbeam::channel::Sender;
use windows::Win32::{
  Foundation::*,
  UI::{Shell::DefSubclassProc, WindowsAndMessaging, WindowsAndMessaging::DestroyWindow},
};

#[allow(unused)]
use super::window_message::{Message, WindowMessage};
use crate::prelude::Window;

pub struct SubclassWindowData {
  pub sender: Sender<Message>,
}

pub extern "system" fn wnd_proc(
  h_wnd: HWND,
  message: u32,
  w_param: WPARAM,
  l_param: LPARAM,
) -> LRESULT {
  match message {
    WindowsAndMessaging::WM_CLOSE => LRESULT(0),
    _ => unsafe { WindowsAndMessaging::DefWindowProcW(h_wnd, message, w_param, l_param) },
  }
}

pub extern "system" fn subclass_proc(
  h_wnd: HWND,
  message: u32,
  w_param: WPARAM,
  l_param: LPARAM,
  _u_id_subclass: usize,
  dw_ref_data: usize,
) -> LRESULT {
  let data: &SubclassWindowData = unsafe { std::mem::transmute(dw_ref_data) };

  let win_message = Message::new(h_wnd, message, w_param, l_param);
  if !matches!(
    message,
    WindowsAndMessaging::WM_SIZING
      | WindowsAndMessaging::WM_MOVING
      | WindowsAndMessaging::WM_MOVE
      | WindowsAndMessaging::WM_SETTEXT
  ) {
    let _ = data.sender.try_send(win_message);
  }

  match message {
    Window::MSG_MAIN_CLOSE_REQ => {
      unsafe { DestroyWindow(h_wnd) }.expect("failed to destroy window");
      LRESULT(0)
    }
    WindowsAndMessaging::WM_CLOSE => LRESULT(0),
    WindowsAndMessaging::WM_DESTROY => {
      unsafe {
        WindowsAndMessaging::PostQuitMessage(0);
      }
      LRESULT(0)
    }
    _ => unsafe { DefSubclassProc(h_wnd, message, w_param, l_param) },
  }
}
