use std::{
  mem::size_of,
  sync::{Arc, Condvar, Mutex},
};

use crossbeam::channel::{Receiver, Sender};
use tracing::error;
use windows::Win32::{
  Foundation::*,
  Graphics::Gdi::{
    self,
    GetMonitorInfoW,
    InvalidateRgn,
    MonitorFromWindow,
    RedrawWindow,
    MONITORINFO,
  },
  UI::{
    Shell::{DefSubclassProc, SetWindowSubclass},
    WindowsAndMessaging::{
      self,
      DestroyWindow,
      GetWindowLongPtrW,
      PostQuitMessage,
      SetWindowLongW,
      SetWindowPos,
      SetWindowTextW,
      ShowWindow,
      CREATESTRUCTW,
    },
  },
};

#[allow(unused)]
use super::message::{Message, WindowMessage};
use super::{
  command::Command,
  settings::WindowSettings,
  state::{Fullscreen, Visibility},
  Window,
};
use crate::{
  get_window_ex_style,
  get_window_style,
  handle::Handle,
  prelude::Input,
  window::{stage::Stage, state::InternalState},
};

pub struct CreateInfo {
  pub settings: WindowSettings,
  pub window: Option<(Window, Handle<InternalState>)>,
  pub sync: SyncData,
  pub command_sender: Sender<Command>,
  pub command_receiver: Receiver<Command>,
  pub message_sender: Sender<Message>,
  pub message_receiver: Receiver<Message>,
}

#[derive(Clone)]
pub struct SyncData {
  pub new_message: Arc<(Mutex<bool>, Condvar)>,
  pub next_frame: Arc<(Mutex<bool>, Condvar)>,
}

impl SyncData {
  pub fn signal_new_message(&self) {
    let (lock, cvar) = self.new_message.as_ref();
    let mut new = lock.lock().unwrap();
    *new = true;
    cvar.notify_one();
  }

  pub fn wait_on_frame(&self, interrupt: impl Fn() -> bool) {
    let (lock, cvar) = self.next_frame.as_ref();
    let mut next = cvar
      .wait_while(lock.lock().unwrap(), |next| !*next && !interrupt())
      .unwrap();
    *next = false;
  }
}

pub struct SubclassWindowData {
  pub state: Handle<InternalState>,
  pub sync: SyncData,
  pub command_receiver: Receiver<Command>,
  pub message_sender: Sender<Message>,
}

pub extern "system" fn wnd_proc(
  hwnd: HWND,
  msg: u32,
  w_param: WPARAM,
  l_param: LPARAM,
) -> LRESULT {
  // Might be used in the future
  let user_data_ptr =
    unsafe { GetWindowLongPtrW(hwnd, WindowsAndMessaging::GWLP_USERDATA) };

  match (user_data_ptr, msg) {
    (0, WindowsAndMessaging::WM_CREATE) => on_create(hwnd, msg, w_param, l_param),
    _ => unsafe { WindowsAndMessaging::DefWindowProcW(hwnd, msg, w_param, l_param) },
  }
}

fn on_create(hwnd: HWND, msg: u32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
  let create_struct = unsafe { (l_param.0 as *mut CREATESTRUCTW).as_mut().unwrap() };
  let create_info = unsafe {
    (create_struct.lpCreateParams as *mut CreateInfo)
      .as_mut()
      .unwrap()
  };

  // create state
  let input = Input::new();
  let state = Handle::new(InternalState {
    thread: None,
    title: Default::default(),
    subtitle: Default::default(),
    theme: Default::default(),
    visibility: Default::default(),
    fullscreen: Default::default(),
    windowed_position: Default::default(),
    windowed_size: Default::default(),
    cursor_mode: Default::default(),
    flow: Default::default(),
    close_on_x: Default::default(),
    stage: Stage::Looping,
    input,
    requested_redraw: false,
  });

  if let Err(e) =
    create_info
      .message_sender
      .try_send(Message::Window(WindowMessage::Created {
        hwnd,
        hinstance: create_struct.hInstance,
      }))
  {
    error!("{e}");
    return LRESULT(-1);
  }

  create_info.sync.signal_new_message();

  let window = Window {
    hinstance: create_struct.hInstance,
    hwnd,
    state: state.clone(),
    sync: create_info.sync.clone(),
    command_sender: create_info.command_sender.clone(),
    message_receiver: create_info.message_receiver.clone(),
  };

  let subclass_data = SubclassWindowData {
    state: state.clone(),
    sync: create_info.sync.clone(),
    command_receiver: create_info.command_receiver.clone(),
    message_sender: create_info.message_sender.clone(),
  };

  create_info.window = Some((window, state));

  // create subclass ptr
  let window_data_ptr = Box::into_raw(Box::new(subclass_data));

  // attach subclass ptr
  let result = unsafe {
    SetWindowSubclass(
      hwnd,
      Some(subclass_proc),
      Window::WINDOW_SUBCLASS_ID,
      window_data_ptr as usize,
    )
  }
  .as_bool();
  debug_assert!(result);

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
  let data: &mut SubclassWindowData = unsafe { std::mem::transmute(dw_ref_data) };

  on_message(hwnd, msg, w_param, l_param, data)
}

fn on_message(
  hwnd: HWND,
  msg: u32,
  w_param: WPARAM,
  l_param: LPARAM,
  data: &SubclassWindowData,
) -> LRESULT {
  let message = Message::new(hwnd, msg, w_param, l_param);
  // handle from wndproc
  let result = match msg {
    WindowsAndMessaging::WM_SIZING
    | WindowsAndMessaging::WM_MOVING
    | WindowsAndMessaging::WM_MOVE => {
      // ignore certain messages
      return unsafe { DefSubclassProc(hwnd, msg, w_param, l_param) };
    }
    WindowsAndMessaging::WM_DESTROY => {
      unsafe { PostQuitMessage(0) };
      return unsafe { DefSubclassProc(hwnd, msg, w_param, l_param) };
    }
    WindowsAndMessaging::WM_CLOSE => LRESULT(0),
    _ => unsafe { DefSubclassProc(hwnd, msg, w_param, l_param) },
  };

  // handle command requests
  if process_commands(hwnd, data) {
    // process commands returns true to interrupt
    return result;
  }

  // Wait for previous message to be handled
  if !data.message_sender.is_empty() {
    data
      .sync
      .wait_on_frame(|| data.state.get().stage == Stage::ExitLoop);
  }

  // pass message to main thread
  if let Some(message) = message {
    update_state(data, &message);
    data.message_sender.try_send(message).unwrap();
    // data.sync.next_message.lock().unwrap().replace(message);
    data.sync.signal_new_message();
    data
      .sync
      .wait_on_frame(|| data.state.get().stage == Stage::ExitLoop);
  }

  result
}

fn update_state(data: &SubclassWindowData, message: &Message) {
  if let Message::Window(window_message) = &message {
    match window_message {
      &WindowMessage::Resized(size) => {
        let is_windowed = data.state.get().fullscreen.is_none();
        if is_windowed {
          data.state.get_mut().windowed_size = size;
        }
      }
      &WindowMessage::Moved(position) => {
        let is_windowed = data.state.get().fullscreen.is_none();
        if is_windowed {
          data.state.get_mut().windowed_position = position;
        }
      }
      &WindowMessage::Key { key, state, .. } => {
        data.state.get_mut().input.update_key_state(key, state);
        data.state.get_mut().input.update_modifiers_state();
      }
      &WindowMessage::MouseButton { button, state, .. } => data
        .state
        .get_mut()
        .input
        .update_mouse_button_state(button, state),
      WindowMessage::Paint => {
        data.state.get_mut().requested_redraw = false;
      }
      _ => (),
    }
  }
}

fn process_commands(hwnd: HWND, data: &SubclassWindowData) -> bool {
  while let Ok(command) = data.command_receiver.try_recv() {
    match command {
      Command::Destroy => {
        unsafe { DestroyWindow(hwnd) }.unwrap();
        return true; // hwnd will be invalid
      }
      Command::Redraw => unsafe {
        RedrawWindow(hwnd, None, None, Gdi::RDW_INTERNALPAINT);
      },
      Command::SetVisibility(visibility) => unsafe {
        ShowWindow(hwnd, match visibility {
          Visibility::Hidden => WindowsAndMessaging::SW_HIDE,
          Visibility::Shown => WindowsAndMessaging::SW_SHOW,
        });
      },
      Command::SetWindowText(text) => unsafe {
        SetWindowTextW(hwnd, &text).unwrap();
      },
      Command::SetSize(_size) => todo!(),
      Command::SetPosition(_position) => todo!(),
      Command::SetFullscreen(fullscreen) => {
        // update style
        let visible = data.state.get().visibility;
        unsafe {
          SetWindowLongW(
            hwnd,
            WindowsAndMessaging::GWL_STYLE,
            get_window_style(fullscreen, visible).0 as i32,
          )
        };
        unsafe {
          SetWindowLongW(
            hwnd,
            WindowsAndMessaging::GWL_EXSTYLE,
            get_window_ex_style(fullscreen, visible).0 as i32,
          )
        };
        // update size
        match fullscreen {
          Some(Fullscreen::Borderless) => {
            let monitor =
              unsafe { MonitorFromWindow(hwnd, Gdi::MONITOR_DEFAULTTONEAREST) };
            let mut info = MONITORINFO {
              cbSize: size_of::<MONITORINFO>() as u32,
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
                  WindowsAndMessaging::SWP_NOZORDER,
                )
                .expect("Failed to set window to fullscreen");
              }
              unsafe { InvalidateRgn(hwnd, None, false) };
            }
          }
          None => {
            let size = data.state.get().windowed_size;
            let position = data.state.get().windowed_position;
            unsafe {
              SetWindowPos(
                hwnd,
                None,
                position.x,
                position.y,
                size.width,
                size.height,
                WindowsAndMessaging::SWP_NOZORDER,
              )
              .expect("Failed to set window to windowed");
            };
            unsafe { InvalidateRgn(hwnd, None, false) };
          }
        }
      }
    }
  }

  false
}
