use std::sync::{Arc, Condvar, Mutex};

use crossbeam::queue::{ArrayQueue, SegQueue};
use windows::Win32::{
  Foundation::*,
  Graphics::Gdi::{self, RedrawWindow},
  UI::{
    Shell::DefSubclassProc,
    WindowsAndMessaging::{
      self,
      DestroyWindow,
      PostQuitMessage,
      SetWindowTextW,
      ShowWindow,
    },
  },
};

#[allow(unused)]
use super::message::{Message, WindowMessage};
use super::{command::Command, settings::Visibility};

pub struct SubclassWindowData {
  pub command_queue: Arc<SegQueue<Command>>,
  pub new_message: Arc<(Mutex<bool>, Condvar)>,
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
    command_queue,
    new_message,
    next_frame,
    next_message,
    is_closing,
  }: &mut SubclassWindowData = unsafe { std::mem::transmute(dw_ref_data) };

  match msg {
    // ignored
    WindowsAndMessaging::WM_SIZING
    | WindowsAndMessaging::WM_MOVING
    | WindowsAndMessaging::WM_MOVE => {
      return unsafe { DefSubclassProc(hwnd, msg, w_param, l_param) }
    }
    _ => (),
  }

  // if let Ok(thread_message) = ThreadMessage::try_from(msg) {
  //   match thread_message {
  //     ThreadMessage::CloseConfirmed => unsafe { DestroyWindow(hwnd) }.unwrap(),
  //     ThreadMessage::ShowWindow => unsafe {
  //       ShowWindow(hwnd, match w_param.0 {
  //         0 => WindowsAndMessaging::SW_HIDE,
  //         _ => WindowsAndMessaging::SW_SHOW,
  //       });
  //     },
  //     ThreadMessage::RequestRedraw => unsafe {
  //       RedrawWindow(hwnd, None, None, Gdi::RDW_INTERNALPAINT);
  //     },
  //     _ => (),
  //   }
  // } else {
  //   let message = Message::new(hwnd, msg, w_param, l_param);
  //   next_message.lock().unwrap().replace(message);
  // }

  let result = match msg {
    WindowsAndMessaging::WM_CLOSE => LRESULT(0),
    WindowsAndMessaging::WM_DESTROY => {
      *is_closing = true;
      unsafe { PostQuitMessage(0) };
      LRESULT(0)
    }
    _ => unsafe { DefSubclassProc(hwnd, msg, w_param, l_param) },
  };

  if msg == WindowsAndMessaging::WM_APP {
    while let Some(command) = command_queue.pop() {
      match command {
        Command::Close => {
          unsafe { DestroyWindow(hwnd) }.unwrap();
          break;
        }
        Command::SetVisibility(visibility) => unsafe {
          ShowWindow(hwnd, match visibility {
            Visibility::Hidden => WindowsAndMessaging::SW_HIDE,
            Visibility::Shown => WindowsAndMessaging::SW_SHOW,
          });
        },
        Command::Redraw => unsafe {
          RedrawWindow(hwnd, None, None, Gdi::RDW_INTERNALPAINT);
        },
        Command::SetWindowText(text) => unsafe {
          SetWindowTextW(hwnd, &text).unwrap();
        },
      }
    }
  } else {
    let message = Message::new(hwnd, msg, w_param, l_param);
    let is_none = matches!(*next_message.lock().unwrap(), Message::None);

    if is_none {
      next_message.lock().unwrap().replace(message);
    } else {
      let (lock, cvar) = next_frame.as_ref();
      let mut next = cvar
        .wait_while(lock.lock().unwrap(), |next| !*next && !*is_closing)
        .unwrap();
      *next = false;
      next_message.lock().unwrap().replace(message);
    }
    // if let Message::Window(..) = message {
    // }
  }

  signal_new_message(new_message);

  let (lock, cvar) = next_frame.as_ref();
  let mut next = cvar
    .wait_while(lock.lock().unwrap(), |next| !*next && !*is_closing)
    .unwrap();
  *next = false;

  result
}

fn signal_new_message(new_message: &mut Arc<(Mutex<bool>, Condvar)>) {
  let (lock, cvar) = new_message.as_ref();
  {
    let mut new = lock.lock().unwrap();
    *new = true;
  }
  cvar.notify_one();
}
