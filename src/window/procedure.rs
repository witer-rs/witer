use crossbeam::channel::{Receiver, Sender};
use windows::Win32::{
  Foundation::*,
  Graphics::Gdi::{self, RedrawWindow},
  UI::{
    Shell::DefSubclassProc,
    WindowsAndMessaging::{self, DestroyWindow, PostQuitMessage, ShowWindow},
  },
};

#[allow(unused)]
use super::message::{Message, WindowMessage};
use super::sync::ThreadMessage;
use crate::window::sync::Response;

pub struct SubclassWindowData {
  pub message_sender: Sender<Message>,
  pub response_receiver: Receiver<Response>,
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
  let SubclassWindowData {
    message_sender,
    response_receiver,
  }: &mut SubclassWindowData = unsafe { std::mem::transmute(dw_ref_data) };

  if let Ok(thread_message) = ThreadMessage::try_from(msg) {
    match thread_message {
      ThreadMessage::CloseConfirmed => unsafe { DestroyWindow(hwnd) }.unwrap(),
      ThreadMessage::ShowWindow => unsafe {
        ShowWindow(hwnd, match w_param.0 {
          0 => WindowsAndMessaging::SW_HIDE,
          _ => WindowsAndMessaging::SW_SHOW,
        });
      },
      ThreadMessage::RequestRedraw => unsafe {
        RedrawWindow(hwnd, None, None, Gdi::RDW_INTERNALPAINT);
      },
      _ => (),
    }
  } else {
    let message = Message::new(hwnd, msg, w_param, l_param);
    message_sender.try_send(message).unwrap();
    response_receiver.recv().unwrap();
  }

  match msg {
    WindowsAndMessaging::WM_CLOSE => LRESULT(0),
    WindowsAndMessaging::WM_DESTROY => {
      unsafe { PostQuitMessage(0) };
      LRESULT(0)
    }
    _ => unsafe { DefSubclassProc(hwnd, msg, w_param, l_param) },
  }
}
