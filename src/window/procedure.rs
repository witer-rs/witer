use std::sync::{Arc, Mutex, Weak};

use windows::Win32::{
  Foundation::{HWND, LPARAM, LRESULT, WPARAM},
  UI::WindowsAndMessaging::{self, GetWindowLongPtrW},
};

#[allow(unused)]
use super::message::Message;
use super::{
  data::{Position, Size, SyncData},
  frame::Style,
  message::RawMessage,
  settings::WindowSettings,
  Window,
};
use crate::window::data::Internal;

pub struct CreateInfo {
  pub class_name: String,
  pub title: String,
  pub size: Size,
  pub position: Option<Position>,
  pub settings: WindowSettings,
  pub window: Option<Window>,
  pub message: Arc<Mutex<Option<Message>>>,
  pub sync: SyncData,
  pub style: Style,
}

pub struct UserData {
  pub internal: Weak<Internal>,
  // if this is a strong reference, internal is never dropped and will cause Drop not to kick into action the rest.
  // maybe convert this to use the dropping of the internal to signal the quitting of the window.
  // this would mean you don't need to pre-handle dropping the internal userdata, and the two matches
  // below can be combined.
}

////////////////////////
/// WINDOW PROCEDURE ///
////////////////////////

pub extern "system" fn wnd_proc(
  hwnd: HWND,
  msg: u32,
  wparam: WPARAM,
  lparam: LPARAM,
) -> LRESULT {
  let message = RawMessage {
    id: msg,
    w: wparam,
    l: lparam,
  };

  let user_data_ptr =
    unsafe { GetWindowLongPtrW(hwnd, WindowsAndMessaging::GWLP_USERDATA) };
  let internal: Option<Arc<Internal>> =
    unsafe { (user_data_ptr as *mut UserData).as_mut() }
      .and_then(|ptr| ptr.internal.upgrade());

  message.process(hwnd, internal)
}
