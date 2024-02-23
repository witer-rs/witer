use std::sync::{Arc, Barrier};

use crossbeam::channel::Sender;
use windows::Win32::{
  Foundation::*,
  UI::{Shell::DefSubclassProc, WindowsAndMessaging::*},
};

#[allow(unused)]
use super::window_message::{Message, StateMessage};

pub struct SubclassWindowData {
  pub sender: Sender<Message>,
  pub barrier: Arc<Barrier>,
}

pub extern "system" fn wnd_proc(
  h_wnd: HWND,
  message: u32,
  w_param: WPARAM,
  l_param: LPARAM,
) -> LRESULT {
  match message {
    WM_CLOSE => LRESULT(0),
    _ => unsafe { DefWindowProcW(h_wnd, message, w_param, l_param) },
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
  match win_message {
    Message::State(
      StateMessage::Resizing { .. } | StateMessage::Moving { .. },
    ) => {
      // let string = format!("PROC: {win_message:?}");
      let _ = data.sender.send(win_message);
      data.barrier.wait();
      // println!("PROC: {string}");
    }
    _ => {
      let _ = data.sender.send(win_message);
    }
  };

  match message {
    WM_CLOSE => LRESULT(0),
    WM_DESTROY => {
      unsafe {
        PostQuitMessage(0);
      }
      LRESULT(0)
    }
    _ => unsafe { DefSubclassProc(h_wnd, message, w_param, l_param) },
  }
}
