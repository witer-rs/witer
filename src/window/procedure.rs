use std::{
  mem::size_of,
  sync::{Arc, Condvar, Mutex},
};

use crossbeam::{
  channel::{Receiver, Sender},
  queue::SegQueue,
};
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
    HiDpi::{
      SetProcessDpiAwarenessContext,
      DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE,
      DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    },
    Shell::{DefSubclassProc, SetWindowSubclass},
    WindowsAndMessaging::{
      self,
      DefWindowProcW,
      DestroyWindow,
      GetClientRect,
      GetWindowLongPtrW,
      PostMessageW,
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
use super::message::Message;
use super::{
  command::Command,
  settings::WindowSettings,
  state::{CursorMode, Fullscreen, StyleInfo, Visibility},
  Window,
};
use crate::{
  handle::Handle,
  prelude::Input,
  utilities::{
    dpi_to_scale_factor,
    get_window_ex_style,
    get_window_style,
    hwnd_dpi,
    set_cursor_clip,
    set_cursor_visibility,
  },
  window::{
    stage::Stage,
    state::{InternalState, PhysicalPosition},
  },
};

pub struct CreateInfo {
  pub settings: WindowSettings,
  pub window: Option<(Window, Handle<InternalState>)>,
  pub sync: SyncData,
  pub command_queue: Arc<SegQueue<Command>>,
  pub message_sender: Sender<Message>,
  pub message_receiver: Receiver<Message>,
  pub style: StyleInfo,
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

  pub fn signal_next_frame(&self) {
    let (lock, cvar) = self.next_frame.as_ref();
    let mut next = lock.lock().unwrap();
    *next = true;
    cvar.notify_one();
  }
}

pub struct SubclassWindowData {
  pub state: Handle<InternalState>,
  pub sync: SyncData,
  pub command_queue: Arc<SegQueue<Command>>,
  pub message_sender: Sender<Message>,
  pub processing_command: bool, // required to prevent re-entrant commands
}

////////////////////////
/// WINDOW PROCEDURE ///
////////////////////////

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
    (0, WindowsAndMessaging::WM_NCCREATE) => on_nccreate(hwnd, msg, w_param, l_param),
    (0, WindowsAndMessaging::WM_CREATE) => on_create(hwnd, msg, w_param, l_param),
    _ => unsafe { DefWindowProcW(hwnd, msg, w_param, l_param) },
  }
}

fn on_nccreate(hwnd: HWND, msg: u32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
  if unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) }
    .is_err()
  {
    unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE) }
      .unwrap();
  }

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
  let size = create_info.settings.size;
  let position = create_info.settings.position.unwrap_or(
    PhysicalPosition::new((
      WindowsAndMessaging::CW_USEDEFAULT,
      WindowsAndMessaging::CW_USEDEFAULT,
    ))
    .into(),
  );

  // create state
  let input = Input::new();
  let state = Handle::new(InternalState {
    thread: None,
    title: create_info.settings.title.clone(),
    subtitle: Default::default(),
    theme: Default::default(),
    style: create_info.style,
    scale_factor,
    position,
    size,
    last_windowed_position: position,
    last_windowed_size: size,
    cursor_mode: create_info.settings.cursor_mode,
    cursor_visibility: Visibility::Shown,
    flow: create_info.settings.flow,
    close_on_x: create_info.settings.close_on_x,
    stage: Stage::Looping,
    input,
    requested_redraw: false,
  });

  if let Err(_e) = create_info.message_sender.try_send(Message::Created {
    hwnd,
    hinstance: create_struct.hInstance,
  }) {
    tracing::error!("{_e}");
    return LRESULT(-1);
  }

  create_info.sync.signal_new_message();

  let window = Window {
    hinstance: create_struct.hInstance,
    hwnd,
    state: state.clone(),
    sync: create_info.sync.clone(),
    command_queue: create_info.command_queue.clone(),
    message_receiver: create_info.message_receiver.clone(),
  };

  create_info.window = Some((window, state.clone()));

  let subclass_data = SubclassWindowData {
    state: state.clone(),
    sync: create_info.sync.clone(),
    command_queue: create_info.command_queue.clone(),
    message_sender: create_info.message_sender.clone(),
    processing_command: false,
  };

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

  unsafe { DefWindowProcW(hwnd, msg, w_param, l_param) }
}

//////////////////////////
/// SUBCLASS PROCEDURE ///
//////////////////////////

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
  data: &mut SubclassWindowData,
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
    update_state(hwnd, data, &message);
    data.message_sender.try_send(message).unwrap();
    // data.sync.next_message.lock().unwrap().replace(message);
    data.sync.signal_new_message();
    data
      .sync
      .wait_on_frame(|| data.state.get().stage == Stage::ExitLoop);
  }

  result
}

fn update_state(hwnd: HWND, data: &SubclassWindowData, message: &Message) {
  match message {
    &Message::Focus(is_focused) => {
      let cursor_visibility = data.state.get().cursor_visibility;
      let cursor_mode = data.state.get().cursor_mode;
      if is_focused {
        data.command_queue.push(Command::SetCursorMode(cursor_mode));
        data
          .command_queue
          .push(Command::SetCursorVisibility(cursor_visibility));
        unsafe { PostMessageW(hwnd, WindowsAndMessaging::WM_APP, WPARAM(0), LPARAM(0)) }
          .unwrap();
      }
    }
    &Message::Resized(size) => {
      let is_windowed = data.state.get().style.fullscreen.is_none();
      data.state.get_mut().size = size;
      if is_windowed {
        data.state.get_mut().last_windowed_size = size;
      }
    }
    &Message::Moved(position) => {
      let is_windowed = data.state.get().style.fullscreen.is_none();
      data.state.get_mut().position = position;
      if is_windowed {
        data.state.get_mut().last_windowed_position = position;
      }
    }
    &Message::Key { key, state, .. } => {
      data.state.get_mut().input.update_key_state(key, state);
      data.state.get_mut().input.update_modifiers_state();
    }
    &Message::MouseButton { button, state, .. } => data
      .state
      .get_mut()
      .input
      .update_mouse_button_state(button, state),
    Message::Paint => {
      data.state.get_mut().requested_redraw = false;
    }
    _ => (),
  }
}

fn process_commands(hwnd: HWND, data: &mut SubclassWindowData) -> bool {
  if data.processing_command {
    return false;
  }

  while let Some(command) = data.command_queue.pop() {
    data.processing_command = true;

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
      Command::SetDecorations(decorations) => {
        let style = data.state.get().style;
        match decorations {
          Visibility::Shown => {
            unsafe {
              SetWindowLongW(
                hwnd,
                WindowsAndMessaging::GWL_STYLE,
                get_window_style(&style).0 as i32,
              )
            };
            unsafe {
              SetWindowLongW(
                hwnd,
                WindowsAndMessaging::GWL_EXSTYLE,
                get_window_ex_style(&style).0 as i32,
              )
            };
          }
          Visibility::Hidden => {
            unsafe {
              SetWindowLongW(
                hwnd,
                WindowsAndMessaging::GWL_STYLE,
                get_window_style(&style).0 as i32,
              )
            };
            unsafe {
              SetWindowLongW(
                hwnd,
                WindowsAndMessaging::GWL_EXSTYLE,
                get_window_ex_style(&style).0 as i32,
              )
            };
          }
        }
      }
      Command::SetWindowText(text) => unsafe {
        SetWindowTextW(hwnd, &text).unwrap();
      },
      Command::SetSize(size) => {
        let physical_size = size.as_physical(data.state.get().scale_factor);
        unsafe {
          SetWindowPos(
            hwnd,
            None,
            0,
            0,
            physical_size.width as i32,
            physical_size.height as i32,
            WindowsAndMessaging::SWP_ASYNCWINDOWPOS
              | WindowsAndMessaging::SWP_NOZORDER
              | WindowsAndMessaging::SWP_NOMOVE
              | WindowsAndMessaging::SWP_NOREPOSITION
              | WindowsAndMessaging::SWP_NOACTIVATE,
          )
          .expect("Failed to set window size");
        }
        unsafe { InvalidateRgn(hwnd, None, false) };
      }
      Command::SetPosition(position) => {
        let physical_position = position.as_physical(data.state.get().scale_factor);
        unsafe {
          SetWindowPos(
            hwnd,
            None,
            physical_position.x,
            physical_position.y,
            0,
            0,
            WindowsAndMessaging::SWP_ASYNCWINDOWPOS
              | WindowsAndMessaging::SWP_NOZORDER
              | WindowsAndMessaging::SWP_NOSIZE
              | WindowsAndMessaging::SWP_NOREPOSITION
              | WindowsAndMessaging::SWP_NOACTIVATE,
          )
          .expect("Failed to set window position");
        }
        unsafe { InvalidateRgn(hwnd, None, false) };
      }
      Command::SetFullscreen(fullscreen) => {
        // update style
        let style = data.state.get().style;
        unsafe {
          SetWindowLongW(
            hwnd,
            WindowsAndMessaging::GWL_STYLE,
            get_window_style(&style).0 as i32,
          )
        };
        unsafe {
          SetWindowLongW(
            hwnd,
            WindowsAndMessaging::GWL_EXSTYLE,
            get_window_ex_style(&style).0 as i32,
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
            let scale_factor = data.state.get().scale_factor;
            let size = data
              .state
              .get()
              .last_windowed_size
              .as_physical(scale_factor);
            let position = data
              .state
              .get()
              .last_windowed_position
              .as_physical(scale_factor);
            unsafe {
              SetWindowPos(
                hwnd,
                None,
                position.x,
                position.y,
                size.width as i32,
                size.height as i32,
                WindowsAndMessaging::SWP_NOZORDER,
              )
              .expect("Failed to set window to windowed");
            };
            unsafe { InvalidateRgn(hwnd, None, false) };
          }
        }
      }
      Command::SetCursorMode(mode) => {
        match mode {
          CursorMode::Normal => {
            set_cursor_clip(None);
          }
          CursorMode::Confined => {
            let mut client_rect = RECT::default();
            unsafe { GetClientRect(hwnd, &mut client_rect) }.unwrap();

            set_cursor_clip(Some(&client_rect));
          }
        };
      }
      Command::SetCursorVisibility(visibility) => match visibility {
        Visibility::Shown => {
          set_cursor_visibility(Visibility::Shown);
        }
        Visibility::Hidden => {
          set_cursor_visibility(Visibility::Hidden);
        }
      },
    }

    data.processing_command = false;
  }

  false
}
