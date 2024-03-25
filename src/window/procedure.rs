use std::{
  mem::size_of,
  sync::{Arc, Condvar, Mutex},
};

// use crossbeam::channel::{Receiver, Sender};
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
    self,
    Controls,
    HiDpi::{
      SetProcessDpiAwarenessContext,
      DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE,
      DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    },
    Input::{
      KeyboardAndMouse::{self, TrackMouseEvent, TRACKMOUSEEVENT},
      HRAWINPUT,
      RID_DEVICE_INFO_TYPE,
    },
    Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
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
      WINDOWPOS,
    },
  },
};
#[allow(unused)]
use super::message::Message;
use super::{
  command::Command,
  input::mouse::mouse_button_states,
  message::{get_cursor_move_kind, CursorMoveKind, Focus},
  settings::WindowSettings,
  state::{CursorMode, Fullscreen, Position, Size, StyleInfo, Visibility},
  Window,
};
use crate::{
  handle::Handle,
  prelude::Input,
  utilities::{
    dpi_to_scale_factor,
    get_window_ex_style,
    get_window_style,
    hi_word,
    hwnd_dpi,
    lo_word,
    read_raw_input,
    register_all_mice_and_keyboards_for_raw_input,
    set_cursor_clip,
    set_cursor_visibility,
    signed_hi_word,
    signed_lo_word,
  },
  window::{
    stage::Stage,
    state::{CursorInfo, InternalState, PhysicalPosition},
  },
  Key,
  MouseButton,
  PhysicalSize,
  RawInputMessage,
  RawKeyState,
};

pub struct CreateInfo {
  pub title: String,
  pub size: Size,
  pub position: Option<Position>,
  pub settings: WindowSettings,
  pub class_atom: u16,
  pub window: Option<(Window, Handle<InternalState>)>,
  pub sync: SyncData,
  pub style: StyleInfo,
}

#[derive(Clone)]
pub struct SyncData {
  pub message: Arc<Mutex<Option<Message>>>,
  pub new_message: Arc<(Mutex<bool>, Condvar)>,
  pub next_frame: Arc<(Mutex<bool>, Condvar)>,
}

impl SyncData {
  pub fn send_to_main(&self, message: Message, state: &Handle<InternalState>) {
    let should_wait = self.message.lock().unwrap().is_some();
    if should_wait {
      self.wait_on_frame(|| {
        matches!(state.read_lock().stage, Stage::Setup | Stage::ExitLoop)
      });
    }

    self.message.lock().unwrap().replace(message);
    self.signal_new_message();

    self.wait_on_frame(|| {
      matches!(state.read_lock().stage, Stage::Setup | Stage::ExitLoop)
    });
  }

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
  let state = Handle::new(InternalState {
    thread: None,
    title: create_info.title.clone(),
    subtitle: Default::default(),
    theme: Default::default(),
    style: create_info.style,
    scale_factor,
    last_windowed_position: position,
    last_windowed_size: size,
    cursor: CursorInfo {
      mode: create_info.settings.cursor_mode,
      visibility: Visibility::Shown,
      inside_window: false,
      last_position: PhysicalPosition::default(),
    },
    flow: create_info.settings.flow,
    close_on_x: create_info.settings.close_on_x,
    stage: Stage::Setup,
    input,
    requested_redraw: false,
  });

  create_info.sync.send_to_main(
    Message::Created {
      hwnd,
      hinstance: create_struct.hInstance,
    },
    &state,
  );

  let window = Window {
    hinstance: create_struct.hInstance,
    hwnd,
    class_atom: create_info.class_atom,
    state: state.clone(),
    sync: create_info.sync.clone(),
  };

  create_info.window = Some((window, state.clone()));

  let subclass_data = SubclassWindowData {
    state: state.clone(),
    sync: create_info.sync.clone(),
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
  let mut messages = Vec::with_capacity(0);
  messages.reserve_exact(1);

  // handle from wndproc
  let result = {
    match msg {
      Command::MESSAGE_ID => {
        let command = unsafe { Box::from_raw(w_param.0 as *mut Command) };
        tracing::debug!("{command:?}");
        match *command {
          Command::Destroy => {
            unsafe { DestroyWindow(hwnd) }.unwrap();
            return LRESULT(0); // hwnd will be invalid
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
            let style = data.state.read_lock().style;
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
            unsafe {
              SetWindowPos(
                hwnd,
                None,
                0,
                0,
                0,
                0,
                WindowsAndMessaging::SWP_NOZORDER
                  | WindowsAndMessaging::SWP_NOMOVE
                  | WindowsAndMessaging::SWP_NOSIZE
                  | WindowsAndMessaging::SWP_NOACTIVATE
                  | WindowsAndMessaging::SWP_FRAMECHANGED,
              )
              .expect("Failed to set window size");
            }
          }
          Command::SetWindowText(text) => unsafe {
            SetWindowTextW(hwnd, &text).unwrap();
          },
          Command::SetSize(size) => {
            let physical_size = size.as_physical(data.state.read_lock().scale_factor);
            unsafe {
              SetWindowPos(
                hwnd,
                None,
                0,
                0,
                physical_size.width as i32,
                physical_size.height as i32,
                WindowsAndMessaging::SWP_NOZORDER
                  | WindowsAndMessaging::SWP_NOMOVE
                  | WindowsAndMessaging::SWP_NOREPOSITION
                  | WindowsAndMessaging::SWP_NOACTIVATE,
              )
              .expect("Failed to set window size");
            }
            unsafe { InvalidateRgn(hwnd, None, false) };
          }
          Command::SetPosition(position) => {
            let physical_position =
              position.as_physical(data.state.read_lock().scale_factor);
            unsafe {
              SetWindowPos(
                hwnd,
                None,
                physical_position.x,
                physical_position.y,
                0,
                0,
                WindowsAndMessaging::SWP_NOZORDER
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
            let style = data.state.read_lock().style;
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
                      WindowsAndMessaging::SWP_ASYNCWINDOWPOS
                        | WindowsAndMessaging::SWP_NOZORDER
                        | WindowsAndMessaging::SWP_FRAMECHANGED,
                    )
                    .expect("Failed to set window to fullscreen");
                  }
                  unsafe { InvalidateRgn(hwnd, None, false) };
                }
              }
              None => {
                let scale_factor = data.state.read_lock().scale_factor;
                let size = data
                  .state
                  .read_lock()
                  .last_windowed_size
                  .as_physical(scale_factor);
                let position = data
                  .state
                  .read_lock()
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
                    WindowsAndMessaging::SWP_ASYNCWINDOWPOS
                      | WindowsAndMessaging::SWP_NOZORDER
                      | WindowsAndMessaging::SWP_FRAMECHANGED,
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

        LRESULT(0)
      }
      WindowsAndMessaging::WM_SIZING | WindowsAndMessaging::WM_MOVING => {
        // ignore certain messages
        return unsafe { DefSubclassProc(hwnd, msg, w_param, l_param) };
      }
      WindowsAndMessaging::WM_DESTROY => {
        unsafe { PostQuitMessage(0) };
        unsafe {
          RemoveWindowSubclass(hwnd, Some(subclass_proc), Window::WINDOW_SUBCLASS_ID)
        };
        unsafe { drop(Box::from_raw(data)) };
        return unsafe { DefSubclassProc(hwnd, msg, w_param, l_param) };
      }
      WindowsAndMessaging::WM_CLOSE => {
        messages.push(Message::CloseRequested);
        LRESULT(0)
      }
      WindowsAndMessaging::WM_PAINT => {
        messages.push(Message::Paint);
        unsafe { DefSubclassProc(hwnd, msg, w_param, l_param) }
      }
      WindowsAndMessaging::WM_SIZE => {
        let width = lo_word(l_param.0 as u32) as u32;
        let height = hi_word(l_param.0 as u32) as u32;

        messages.push(Message::Resized(PhysicalSize::new(width, height)));
        unsafe { DefSubclassProc(hwnd, msg, w_param, l_param) }
      }
      WindowsAndMessaging::WM_MOVE => {
        let x = lo_word(l_param.0 as u32) as i32;
        let y = hi_word(l_param.0 as u32) as i32;

        messages.push(Message::Moved(PhysicalPosition::new(x, y)));
        unsafe { DefSubclassProc(hwnd, msg, w_param, l_param) }
      }
      WindowsAndMessaging::WM_WINDOWPOSCHANGED => {
        let window_pos = unsafe { &*(l_param.0 as *const WINDOWPOS) };
        // if (window_pos.flags & WindowsAndMessaging::SWP_NOMOVE) !=
        // WindowsAndMessaging::SWP_NOMOVE {
        //   out.push(Message::Moved(PhysicalPosition::new((x, y))))
        // }

        messages.push(Message::BoundsChanged {
          outer_position: PhysicalPosition::new(window_pos.x, window_pos.y),
          outer_size: PhysicalSize::new(window_pos.cx as u32, window_pos.cy as u32),
        });
        unsafe { DefSubclassProc(hwnd, msg, w_param, l_param) }
      }
      WindowsAndMessaging::WM_SETFOCUS => {
        messages.push(Message::Focus(Focus::Gained));
        unsafe { DefSubclassProc(hwnd, msg, w_param, l_param) }
      }
      WindowsAndMessaging::WM_KILLFOCUS => {
        messages.push(Message::Focus(Focus::Lost));
        unsafe { DefSubclassProc(hwnd, msg, w_param, l_param) }
      }
      WindowsAndMessaging::WM_COMMAND => {
        messages.push(Message::Command);
        unsafe { DefSubclassProc(hwnd, msg, w_param, l_param) }
      }
      WindowsAndMessaging::WM_SYSCOMMAND => {
        messages.push(Message::SystemCommand);
        unsafe { DefSubclassProc(hwnd, msg, w_param, l_param) }
      }
      WindowsAndMessaging::WM_DPICHANGED => {
        let dpi = lo_word(w_param.0 as u32) as u32;
        let suggested_rect = unsafe { *(l_param.0 as *const RECT) };
        unsafe {
          SetWindowPos(
            hwnd,
            None,
            suggested_rect.left,
            suggested_rect.top,
            suggested_rect.right - suggested_rect.left,
            suggested_rect.bottom - suggested_rect.top,
            WindowsAndMessaging::SWP_NOZORDER | WindowsAndMessaging::SWP_NOACTIVATE,
          )
        }
        .unwrap();
        messages.push(Message::ScaleFactorChanged(dpi_to_scale_factor(dpi)));
        unsafe { DefSubclassProc(hwnd, msg, w_param, l_param) }
      }
      WindowsAndMessaging::WM_INPUT => {
        let Some(data) = read_raw_input(HRAWINPUT(l_param.0)) else {
          return unsafe { DefSubclassProc(hwnd, msg, w_param, l_param) };
        };

        match RID_DEVICE_INFO_TYPE(data.header.dwType) {
          UI::Input::RIM_TYPEMOUSE => {
            let mouse_data = unsafe { data.data.mouse };
            let button_flags = unsafe { mouse_data.Anonymous.Anonymous.usButtonFlags };

            if mouse_data.usFlags == UI::Input::MOUSE_MOVE_RELATIVE {
              let x = mouse_data.lLastX as f32;
              let y = mouse_data.lLastY as f32;

              if x != 0.0 || y != 0.0 {
                messages.push(Message::RawInput(RawInputMessage::MouseMove {
                  delta_x: x,
                  delta_y: y,
                }));
              }
            }

            for (id, state) in mouse_button_states(button_flags).iter().enumerate() {
              if let Some(state) = *state {
                let button = MouseButton::from_state(id);
                messages
                  .push(Message::RawInput(RawInputMessage::MouseButton { button, state }))
              }
            }
          }
          UI::Input::RIM_TYPEKEYBOARD => {
            let keyboard_data = unsafe { data.data.keyboard };

            let Some(key) = Key::from_raw(keyboard_data) else {
              return unsafe { DefSubclassProc(hwnd, msg, w_param, l_param) };
            };

            let pressed = matches!(
              keyboard_data.Message,
              WindowsAndMessaging::WM_KEYDOWN | WindowsAndMessaging::WM_SYSKEYDOWN
            );
            let released = matches!(
              keyboard_data.Message,
              WindowsAndMessaging::WM_KEYUP | WindowsAndMessaging::WM_SYSKEYUP
            );

            if let Some(state) = RawKeyState::from_bools(pressed, released) {
              messages.push(Message::RawInput(RawInputMessage::Keyboard { key, state }));
            }
          }
          _ => (),
        };
        unsafe { DefSubclassProc(hwnd, msg, w_param, l_param) }
      }
      WindowsAndMessaging::WM_CHAR => {
        let c = char::from_u32(w_param.0 as u32).unwrap_or_default();
        messages.push(Message::Char(c));
        unsafe { DefSubclassProc(hwnd, msg, w_param, l_param) }
      }
      WindowsAndMessaging::WM_KEYDOWN
      | WindowsAndMessaging::WM_SYSKEYDOWN
      | WindowsAndMessaging::WM_KEYUP
      | WindowsAndMessaging::WM_SYSKEYUP => {
        let (changed, shift, ctrl, alt, win) =
          data.state.write_lock().input.update_modifiers_state();
        if changed {
          messages.push(Message::ModifiersChanged {
            shift,
            ctrl,
            alt,
            win,
          });
        }
        messages.push(Message::new_keyboard_message(l_param));
        unsafe { DefSubclassProc(hwnd, msg, w_param, l_param) }
      }
      WindowsAndMessaging::WM_MOUSEMOVE => {
        let x = signed_lo_word(l_param.0 as i32) as i32;
        let y = signed_hi_word(l_param.0 as i32) as i32;
        let position = PhysicalPosition::new(x, y);

        let kind =
          get_cursor_move_kind(hwnd, data.state.read_lock().cursor.inside_window, x, y);

        let send_message = {
          match kind {
            CursorMoveKind::Entered => {
              data.state.write_lock().cursor.inside_window = true;

              unsafe {
                TrackMouseEvent(&mut TRACKMOUSEEVENT {
                  cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                  dwFlags: KeyboardAndMouse::TME_LEAVE,
                  hwndTrack: hwnd,
                  dwHoverTime: Controls::HOVER_DEFAULT,
                })
              }
              .unwrap();

              true
            }
            CursorMoveKind::Left => {
              data.state.write_lock().cursor.inside_window = false;

              true
            }
            CursorMoveKind::Inside => {
              data.state.read_lock().cursor.last_position != position
            }
          }
        };

        if send_message {
          messages.push(Message::CursorMove { position, kind });
          data.state.write_lock().cursor.last_position = position;
        }
        unsafe { DefSubclassProc(hwnd, msg, w_param, l_param) }
      }
      Controls::WM_MOUSELEAVE => {
        data.state.write_lock().cursor.inside_window = false;
        messages.push(Message::CursorMove {
          position: data.state.read_lock().cursor.last_position,
          kind: CursorMoveKind::Left,
        });
        unsafe { DefSubclassProc(hwnd, msg, w_param, l_param) }
      }
      WindowsAndMessaging::WM_MOUSEWHEEL => {
        let delta = signed_hi_word(w_param.0 as i32) as f32
          / WindowsAndMessaging::WHEEL_DELTA as f32;
        messages.push(Message::MouseWheel {
          delta_x: 0.0,
          delta_y: delta,
        });
        unsafe { DefSubclassProc(hwnd, msg, w_param, l_param) }
      }
      WindowsAndMessaging::WM_MOUSEHWHEEL => {
        let delta = signed_hi_word(w_param.0 as i32) as f32
          / WindowsAndMessaging::WHEEL_DELTA as f32;
        messages.push(Message::MouseWheel {
          delta_x: delta,
          delta_y: 0.0,
        });
        unsafe { DefSubclassProc(hwnd, msg, w_param, l_param) }
      }
      msg
        if (WindowsAndMessaging::WM_MOUSEFIRST..=WindowsAndMessaging::WM_MOUSELAST)
          .contains(&msg) =>
      {
        // mouse move / wheels will match earlier
        messages.push(Message::new_mouse_button_message(msg, w_param, l_param));
        unsafe { DefSubclassProc(hwnd, msg, w_param, l_param) }
      }
      _ => unsafe { DefSubclassProc(hwnd, msg, w_param, l_param) },
    }
  };

  // pass message to main thread
  if !messages.is_empty() {
    for message in messages {
      update_state(hwnd, &data.state, &message);
      data.sync.send_to_main(message, &data.state);
    }
  }

  result
}

fn update_state(hwnd: HWND, state: &Handle<InternalState>, message: &Message) {
  match message {
    &Message::Focus(focus) => {
      let cursor_visibility = state.read_lock().cursor.visibility;
      let cursor_mode = state.read_lock().cursor.mode;
      if focus == Focus::Gained {
        Command::SetCursorMode(cursor_mode).post(hwnd);
        Command::SetCursorVisibility(cursor_visibility).post(hwnd);
        unsafe { PostMessageW(hwnd, WindowsAndMessaging::WM_APP, WPARAM(0), LPARAM(0)) }
          .unwrap();
      }
    }
    &Message::Resized(_size) => {
      // info!("RESIZED: {_size:?}");
      let is_windowed = state.read_lock().style.fullscreen.is_none();
      // // data.state.write_lock().size = size;
      if is_windowed {
        state.write_lock().update_last_windowed_pos_size(hwnd);
      }
    }
    &Message::BoundsChanged {
      outer_position: _,
      outer_size: _,
    } => {
      // info!("BOUNDSCHANGED: {outer_position:?}, {outer_size:?}");
      let is_windowed = state.read_lock().style.fullscreen.is_none();
      // // data.state.write_lock().position = position;
      if is_windowed {
        state.write_lock().update_last_windowed_pos_size(hwnd);
      }
    }
    &Message::Key {
      key,
      state: key_state,
      ..
    } => {
      state.write_lock().input.update_key_state(key, key_state);
    }
    &Message::MouseButton {
      button,
      state: button_state,
      ..
    } => state
      .write_lock()
      .input
      .update_mouse_button_state(button, button_state),
    Message::Paint => {
      state.write_lock().requested_redraw = false;
    }
    _ => (),
  }
}
