use std::sync::Arc;

use windows::Win32::{
  Foundation::*,
  UI::{
    Shell::DefSubclassProc,
    WindowsAndMessaging::{self, PostQuitMessage},
  },
};

#[allow(unused)]
use super::message::{Message, WindowMessage};
use super::{Window, WindowProcedure};

pub struct SubclassWindowData {
  pub window: Arc<Window>,
  pub wndproc: Box<dyn WindowProcedure>,
}

pub extern "system" fn wnd_proc(
  hwnd: HWND,
  msg: u32,
  w_param: WPARAM,
  l_param: LPARAM,
) -> LRESULT {
  unsafe { WindowsAndMessaging::DefWindowProcW(hwnd, msg, w_param, l_param) }
}

pub extern "system" fn subclass_proc(
  hwnd: HWND,
  msg: u32,
  w_param: WPARAM,
  l_param: LPARAM,
  _u_id_subclass: usize,
  dw_ref_data: usize,
) -> LRESULT {
  let SubclassWindowData { window, wndproc }: &mut SubclassWindowData =
    unsafe { std::mem::transmute(dw_ref_data) };

  let message = Message::new(hwnd, msg, w_param, l_param);
  if message != Message::Ignored {
    wndproc.on_message(window, handle_message(window, message));
  }

  match msg {
    WindowsAndMessaging::WM_CLOSE => LRESULT(0),
    WindowsAndMessaging::WM_DESTROY => LRESULT(0),
    _ => unsafe { DefSubclassProc(hwnd, msg, w_param, l_param) },
  }
}

fn handle_message(window: &Arc<Window>, message: Message) -> Message {
  if let Message::Window(window_message) = &message {
    match window_message {
      WindowMessage::CloseRequested => {
        if window.state.get().close_on_x {
          window.close();
        }
      }
      WindowMessage::Closed => {
        unsafe { PostQuitMessage(0) };
      }
      &WindowMessage::Key { key, state, .. } => {
        window.state.get_mut().input.update_key_state(key, state);
        window.state.get_mut().input.update_modifiers_state();
      }
      &WindowMessage::MouseButton { button, state, .. } => window
        .state
        .get_mut()
        .input
        .update_mouse_state(button, state),
      _ => (),
    }
  }

  message
}
