use std::sync::{Arc, RwLock};

use crossbeam::channel::Sender;
use windows::{
  core::{HSTRING, PCWSTR},
  Win32::{
    Foundation::{HINSTANCE, HWND},
    System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::{
      CreateWindowExW,
      DestroyWindow,
      DispatchMessageW,
      GetMessageW,
      LoadCursorW,
      RegisterClassExW,
      TranslateMessage,
      CS_DBLCLKS,
      CS_HREDRAW,
      CS_VREDRAW,
      CW_USEDEFAULT,
      IDC_ARROW,
      MSG,
      WINDOW_EX_STYLE,
      WNDCLASSEXW,
      WS_OVERLAPPEDWINDOW,
    },
  },
};

use super::{
  builder::{HasSize, HasTitle, WindowCreateInfo},
  window_message::Message,
};
use crate::window::{procs::SubclassWindowData, window_message::StateMessage, Window};

pub struct WindowThreadCreateInfo {
  create_info: WindowCreateInfo<HasTitle, HasSize>,
  proc_sender: Sender<Message>,
}

impl WindowThreadCreateInfo {
  pub fn new(create_info: WindowCreateInfo<HasTitle, HasSize>, proc_sender: Sender<Message>) -> Self {
    Self {
      create_info,
      proc_sender,
    }
  }
}

pub struct WindowLoop;

impl ThreadLoop for WindowLoop {
  type Params = WindowThreadCreateInfo;

  
}

impl WindowLoop {
  pub fn new() -> Self {
    Self
  }

  
}

impl Default for WindowLoop {
  fn default() -> Self {
    Self::new()
  }
}
