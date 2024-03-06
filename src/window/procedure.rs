use std::sync::{Arc, Barrier, Condvar, Mutex};

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

pub struct SubclassWindowData {
  pub next_frame: Arc<(Mutex<bool>, Condvar)>,
  pub next_message: Arc<Mutex<Message>>,
  pub is_closing: bool,
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
    next_frame,
    next_message,
    is_closing,
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
    if let Message::Window(..) = message {
      println!("PROC: {message:?}");
    }
    next_message.lock().unwrap().replace(message);
  }

  let result = match msg {
    WindowsAndMessaging::WM_CLOSE => LRESULT(0),
    WindowsAndMessaging::WM_DESTROY => {
      *is_closing = true;
      unsafe { PostQuitMessage(0) };
      LRESULT(0)
    }
    _ => unsafe { DefSubclassProc(hwnd, msg, w_param, l_param) },
  };

  // println!("approaching next_frame gate");

  let (lock, cvar) = next_frame.as_ref();
  let mut next = cvar
    .wait_while(lock.lock().unwrap(), |next| !*next && !*is_closing)
    .unwrap();
  *next = false;

  // println!("passed next_frame gate");

  // if msg != WindowsAndMessaging::WM_QUIT {
  //   println!("approaching next_frame gate");

  //   let (lock, cvar) = next_frame.as_ref();
  //   let mut next = cvar
  //     .wait_while(lock.lock().unwrap(), |next| !*next)
  //     .unwrap();
  //   *next = false;

  //   println!("passed next_frame gate");
  // }

  result
}
