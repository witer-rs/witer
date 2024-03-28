use std::sync::{Arc, Mutex};

use cursor_icon::CursorIcon;
// use crossbeam::channel::{Receiver, Sender};
use windows::Win32::{
  Foundation::*,
  UI::{
    HiDpi::{
      SetProcessDpiAwarenessContext,
      DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE,
      DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    },
    WindowsAndMessaging::{
      self,
      DefWindowProcW,
      DestroyWindow,
      GetWindowLongPtrW,
      PostQuitMessage,
      SetWindowLongPtrW,
      CREATESTRUCTW,
    },
  },
};

#[allow(unused)]
use super::message::Message;
use super::{
  command::Command,
  data::{Data, Position, Size, SyncData, Visibility},
  frame::Style,
  settings::WindowSettings,
  Window,
};
use crate::{
  prelude::Input,
  utilities::{
    dpi_to_scale_factor,
    hwnd_dpi,
    register_all_mice_and_keyboards_for_raw_input,
  },
  window::{
    cursor::Cursor,
    data::{Internal, PhysicalPosition},
    stage::Stage,
  },
};

pub struct CreateInfo {
  pub title: String,
  pub size: Size,
  pub position: Option<Position>,
  pub settings: WindowSettings,
  pub class_atom: u16,
  pub window: Option<Window>,
  pub sync: SyncData,
  pub style: Style,
}

pub struct UserData {
  state: Arc<Internal>,
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
  let user_data_ptr =
    unsafe { GetWindowLongPtrW(hwnd, WindowsAndMessaging::GWLP_USERDATA) };

  match (user_data_ptr, msg) {
    (0, WindowsAndMessaging::WM_NCCREATE) => on_nccreate(hwnd, msg, wparam, lparam),
    (0, WindowsAndMessaging::WM_CREATE) => on_create(hwnd, msg, wparam, lparam),
    (0, _) => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    (state_ptr, message) => {
      let result = match message {
        Command::MESSAGE_ID => {
          let command = unsafe { (wparam.0 as *mut Command).as_mut() }.unwrap();
          match command {
            Command::Exit => {
              let user_data = unsafe { Box::from_raw(state_ptr as *mut UserData) };
              drop(user_data);
              LRESULT(0)
            }
            Command::Destroy => {
              unsafe { DestroyWindow(hwnd) }.unwrap();
              LRESULT(0)
            }
            _ => {
              if let Some(user_data) = unsafe { (state_ptr as *mut UserData).as_mut() } {
                user_data.state.on_message(hwnd, msg, wparam, lparam)
              } else {
                unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
              }
            }
          }
        }
        WindowsAndMessaging::WM_DESTROY => {
          unsafe { PostQuitMessage(0) };
          unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
        _ => {
          if let Some(user_data) = unsafe { (state_ptr as *mut UserData).as_mut() } {
            user_data.state.on_message(hwnd, msg, wparam, lparam)
          } else {
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
          }
        }
      };
      result
    }
  }
}

fn on_nccreate(hwnd: HWND, msg: u32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
  if unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) }
    .is_err()
  {
    unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE) }
      .unwrap();
  }

  register_all_mice_and_keyboards_for_raw_input(hwnd);

  unsafe { DefWindowProcW(hwnd, msg, w_param, l_param) }
}

fn on_create(hwnd: HWND, msg: u32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
  let create_struct = unsafe { (l_param.0 as *mut CREATESTRUCTW).as_mut().unwrap() };
  let create_info = unsafe {
    (create_struct.lpCreateParams as *mut CreateInfo)
      .as_mut()
      .unwrap()
  };

  let scale_factor = dpi_to_scale_factor(hwnd_dpi(hwnd));
  let size = create_info.size;
  let position = create_info.position.unwrap_or(
    PhysicalPosition::new(
      WindowsAndMessaging::CW_USEDEFAULT,
      WindowsAndMessaging::CW_USEDEFAULT,
    )
    .into(),
  );

  // create state
  let input = Input::new();
  let state = Arc::new(Internal {
    hinstance: create_struct.hInstance,
    hwnd,
    class_atom: create_info.class_atom,
    sync: create_info.sync.clone(),
    thread: Mutex::new(None),
    data: Mutex::new(Data {
      title: create_info.title.clone(),
      subtitle: Default::default(),
      theme: Default::default(),
      style: create_info.style.clone(),
      scale_factor,
      last_windowed_position: position,
      last_windowed_size: size,
      cursor: Cursor {
        mode: create_info.settings.cursor_mode,
        visibility: Visibility::Shown,
        inside_window: false,
        last_position: PhysicalPosition::default(),
        selected_icon: CursorIcon::Default,
      },
      flow: create_info.settings.flow,
      close_on_x: create_info.settings.close_on_x,
      stage: Stage::Setup,
      input,
      requested_redraw: false,
    }),
  });

  // create data ptr
  let user_data = UserData {
    state: state.clone(),
  };
  let user_data_ptr = Box::into_raw(Box::new(user_data));
  unsafe {
    SetWindowLongPtrW(hwnd, WindowsAndMessaging::GWLP_USERDATA, user_data_ptr as isize)
  };

  tracing::trace!("[`{}`]: finalizing window settings", create_info.title);

  let window = Window(state.clone());
  window.force_set_theme(create_info.settings.theme);

  if let Some(position) = create_info.position {
    Command::SetPosition(position).send(hwnd);
  }
  Command::SetSize(size).send(hwnd);
  Command::SetDecorations(create_info.settings.decorations).send(hwnd);
  Command::SetVisibility(create_info.settings.visibility).send(hwnd);
  Command::SetFullscreen(create_info.settings.fullscreen).send(hwnd);

  window.0.data.lock().unwrap().stage = Stage::Ready;

  tracing::trace!("[`{}`]: window is ready", create_info.title);

  create_info.window = Some(window);

  create_info
    .sync
    .message
    .lock()
    .unwrap()
    .replace(Message::Created {
      hwnd,
      hinstance: create_struct.hInstance,
    });
  create_info.sync.signal_new_message();

  unsafe { DefWindowProcW(hwnd, msg, w_param, l_param) }
}
