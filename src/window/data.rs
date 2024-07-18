use std::{
  ops::{Div, Mul},
  sync::{Arc, Condvar, Mutex, MutexGuard},
  thread::JoinHandle,
};

use windows::{
  core::PCWSTR,
  Win32::{
    Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
    Graphics::Gdi::{
      self,
      ClientToScreen,
      GetMonitorInfoW,
      InvalidateRgn,
      MonitorFromWindow,
      RedrawWindow,
      MONITORINFO,
    },
    UI::{
      self,
      Controls,
      Input::{
        KeyboardAndMouse::{self, TrackMouseEvent, TRACKMOUSEEVENT},
        HRAWINPUT,
        RID_DEVICE_INFO_TYPE,
      },
      WindowsAndMessaging::{
        self,
        DefWindowProcW,
        GetClientRect,
        GetWindowLongPtrW,
        GetWindowRect,
        LoadCursorW,
        SetCursor,
        SetWindowLongW,
        SetWindowPos,
        SetWindowTextW,
        ShowWindow,
        UnregisterClassW,
        GWLP_HINSTANCE,
        WINDOWPOS,
      },
    },
  },
};

use super::{
  command::Command,
  cursor::Cursor,
  frame::Style,
  input::mouse::mouse_button_states,
  message::{get_cursor_move_kind, CursorMoveKind, Focus},
  stage::Stage,
};
use crate::{
  error::WindowError,
  utilities::{
    self,
    dpi_to_scale_factor,
    get_window_ex_style,
    get_window_style,
    hi_word,
    is_flag_set,
    lo_word,
    read_raw_input,
    signed_hi_word,
    signed_lo_word,
    to_windows_cursor,
  },
  window::Input,
  Key,
  Message,
  MouseButton,
  RawInputMessage,
  RawKeyState,
};

#[derive(Clone)]
pub struct SyncData {
  pub new_message: Arc<(Mutex<bool>, Condvar)>,
  pub next_frame: Arc<(Mutex<bool>, Condvar)>,
  pub skip_wait: Arc<Mutex<bool>>,
}

impl SyncData {
  pub fn signal_new_message(&self) {
    let (lock, cvar) = self.new_message.as_ref();
    let mut new = lock.lock().unwrap();
    if !*new {
      *new = true;
      cvar.notify_all();
    }
  }

  pub fn wait_on_frame(&self) {
    let (lock, cvar) = self.next_frame.as_ref();
    let mut next = cvar
      .wait_while(lock.lock().unwrap(), |next| !*next)
      .unwrap();
    *next = *self.skip_wait.lock().unwrap();
  }

  pub fn signal_next_frame(&self) {
    let (lock, cvar) = self.next_frame.as_ref();
    let mut next = lock.lock().unwrap();
    if !*next {
      *next = true;
      cvar.notify_all();
    }
  }
}

pub struct Internal {
  pub hinstance: usize,
  pub hwnd: usize,
  pub class_atom: u16,
  pub message: Arc<Mutex<Option<Message>>>,
  pub sync: SyncData,
  pub thread: Mutex<Option<JoinHandle<Result<(), WindowError>>>>,
  pub data: Mutex<Data>,
}

/// Window is destroyed on drop.
impl Drop for Internal {
  fn drop(&mut self) {
    let title = self.data_lock().title.clone();

    if self.data_lock().stage == Stage::Destroyed {
      return;
    } else {
      self.data_lock().stage = Stage::Destroyed;
    }

    tracing::trace!("[`{}`]: destroying window", title);

    Command::Destroy.post(self.hwnd);
    self.join_thread();

    tracing::trace!("[`{}`]: unregistering window class", title);
    let hinstance =
      HINSTANCE(unsafe { GetWindowLongPtrW(HWND(self.hwnd as _), GWLP_HINSTANCE) } as _);
    unsafe { UnregisterClassW(PCWSTR(self.class_atom as *const u16), hinstance) }
      .unwrap();

    tracing::trace!("[`{}`]: destroyed window", title);
  }
}

pub struct Data {
  pub title: String,
  pub subtitle: String,
  pub theme: Theme,
  pub flow: Flow,
  pub close_on_x: bool,

  pub stage: Stage,
  pub style: Style,
  pub input: Input,
  pub cursor: Cursor,

  pub last_windowed_position: Position,
  pub last_windowed_size: Size,
  pub scale_factor: f64,

  pub requested_redraw: bool,
}

impl Internal {
  pub(crate) fn data_lock(&self) -> MutexGuard<Data> {
    self.data.lock().unwrap()
  }

  pub(crate) fn set_thread(&self, handle: Option<JoinHandle<Result<(), WindowError>>>) {
    *self.thread.lock().unwrap() = handle;
  }

  pub fn send_message_to_main(&self, message: Message) {
    let should_wait = self.message.lock().unwrap().is_some();
    if should_wait {
      self.sync.wait_on_frame();
    }

    self.message.lock().unwrap().replace(message);
    self.sync.signal_new_message();

    // TODO: try inverting these locks so that they don't lock unless the main thread tells them to lock.

    self.sync.wait_on_frame();
  }

  pub(crate) fn join_thread(&self) {
    let thread = self.thread.lock().unwrap().take();
    if let Some(thread) = thread {
      tracing::trace!("[`{}`]: joining window thread", self.data.lock().unwrap().title);
      let _ = thread.join();
      tracing::trace!("[`{}`]: joined window thread", self.data.lock().unwrap().title);
    }
  }

  pub(crate) fn is_closing(&self) -> bool {
    matches!(
      self.data.lock().unwrap().stage,
      Stage::Closing | Stage::ExitLoop | Stage::Destroyed
    )
  }

  // pub(crate) fn exit_loop(&self) {
  // }
  pub fn refresh_os_cursor(&self) -> Result<(), WindowError> {
    let mut client_rect = RECT::default();
    unsafe { GetClientRect(HWND(self.hwnd as _), &mut client_rect) }.unwrap();
    let mut top_left = POINT::default();
    unsafe { ClientToScreen(HWND(self.hwnd as _), &mut top_left) }.unwrap();
    client_rect.left += top_left.x;
    client_rect.top += top_left.y;
    client_rect.right += top_left.x;
    client_rect.bottom += top_left.y;

    let is_focused = {
      let style = &self.data_lock().style;
      style.focused && style.active
    };
    if is_focused {
      let is_confined = matches!(self.data_lock().cursor.mode, CursorMode::Confined);
      let is_hidden = matches!(self.data_lock().cursor.visibility, Visibility::Hidden);
      let cursor_clip = match is_confined {
        true => {
          if is_hidden {
            // Confine the cursor to the center of the window if the cursor is hidden. This avoids
            // problems with the cursor activating the taskbar if the window borders or overlaps that.
            let cx = (client_rect.left + client_rect.right) / 2;
            let cy = (client_rect.top + client_rect.bottom) / 2;
            Some(RECT {
              left: cx,
              right: cx + 1,
              top: cy,
              bottom: cy + 1,
            })
          } else {
            Some(client_rect)
          }
        }
        false => None,
      };

      let rect_to_tuple = |rect: RECT| (rect.left, rect.top, rect.right, rect.bottom);
      let active_cursor_clip = rect_to_tuple(utilities::get_cursor_clip()?);
      let desktop_rect = rect_to_tuple(utilities::get_desktop_rect());

      let active_cursor_clip = match desktop_rect == active_cursor_clip {
        true => None,
        false => Some(active_cursor_clip),
      };

      // We do this check because calling `set_cursor_clip` incessantly will flood the event
      // loop with `WM_MOUSEMOVE` events, and `refresh_os_cursor` is called by `set_cursor_flags`
      // which at times gets called once every iteration of the eventloop.
      if active_cursor_clip != cursor_clip.map(rect_to_tuple) {
        utilities::set_cursor_clip(cursor_clip.as_ref());
      }
    }

    let cursor_visibility = self.data_lock().cursor.visibility;
    let cursor_in_client = self.data_lock().cursor.inside_window;
    if cursor_in_client {
      utilities::set_cursor_visibility(cursor_visibility);
    } else {
      utilities::set_cursor_visibility(Visibility::Shown);
    }

    Ok(())
  }

  pub(crate) fn update_last_windowed_pos_size(&self, hwnd: HWND) {
    let mut window_rect = RECT::default();
    let _ = unsafe { GetWindowRect(hwnd, &mut window_rect) };
    let size = PhysicalSize {
      width: (window_rect.right - window_rect.left) as u32,
      height: (window_rect.bottom - window_rect.top) as u32,
    };
    self.data.lock().unwrap().last_windowed_size = size.into();
    let position = PhysicalPosition {
      x: window_rect.left,
      y: window_rect.top,
    };
    self.data.lock().unwrap().last_windowed_position = position.into();
  }

  pub(crate) fn on_message(
    &self,
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
  ) -> LRESULT {
    match msg {
      Command::MESSAGE_ID => {
        let command = unsafe { Box::from_raw(wparam.0 as *mut Command) };
        // tracing::debug!("{command:?}");
        match *command {
          Command::Exit => (),
          Command::Destroy => (),
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
            let style = self.data.lock().unwrap().style.clone();
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
            let physical_size = size.as_physical(self.data.lock().unwrap().scale_factor);
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
              position.as_physical(self.data.lock().unwrap().scale_factor);
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
            let style = self.data.lock().unwrap().style.clone();
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
                  cbSize: std::mem::size_of::<MONITORINFO>() as u32,
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
                let scale_factor = self.data.lock().unwrap().scale_factor;
                let size = self
                  .data
                  .lock()
                  .unwrap()
                  .last_windowed_size
                  .as_physical(scale_factor);
                let position = self
                  .data
                  .lock()
                  .unwrap()
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
          Command::SetCursorIcon(icon) => {
            self.data.lock().unwrap().cursor.selected_icon = icon;
            let cursor_icon = to_windows_cursor(icon);
            let hcursor =
              unsafe { LoadCursorW(HINSTANCE::default(), cursor_icon) }.unwrap();
            unsafe { SetCursor(hcursor) };
          }
          Command::SetCursorMode(mode) => {
            // match mode {
            //   CursorMode::Normal => {
            //     set_cursor_clip(None);
            //   }
            //   CursorMode::Confined => {
            //     let mut client_rect = RECT::default();
            //     unsafe { GetClientRect(hwnd, &mut client_rect) }.unwrap();
            //     tracing::debug!("{client_rect:?}");
            //     set_cursor_clip(Some(&client_rect));
            //   }
            // };

            self.data.lock().unwrap().cursor.mode = mode;
            if let Err(e) = self.refresh_os_cursor() {
              tracing::error!("{e}");
            };
          }
          Command::SetCursorVisibility(visibility) => {
            self.data.lock().unwrap().cursor.visibility = visibility;
            if let Err(e) = self.refresh_os_cursor() {
              tracing::error!("{e}");
            };
          }
        }

        LRESULT(0)
      }
      WindowsAndMessaging::WM_SETCURSOR => {
        let in_client_area =
          lo_word(lparam.0 as u32) as u32 == WindowsAndMessaging::HTCLIENT;

        if in_client_area {
          let icon = self.data.lock().unwrap().cursor.selected_icon;
          let cursor_icon = to_windows_cursor(icon);
          let hcursor =
            unsafe { LoadCursorW(HINSTANCE::default(), cursor_icon) }.unwrap();
          unsafe { SetCursor(hcursor) };
        }

        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
      }
      // WindowsAndMessaging::WM_SIZING | WindowsAndMessaging::WM_MOVING => {
      //   // ignore certain messages
      //   return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
      // }
      WindowsAndMessaging::WM_CLOSE => {
        self.send_message_to_main(Message::CloseRequested);
        LRESULT(0)
      }
      WindowsAndMessaging::WM_PAINT => {
        self.data.lock().unwrap().requested_redraw = false;
        self.send_message_to_main(Message::Paint);
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
      }
      WindowsAndMessaging::WM_SIZE => {
        self.data.lock().unwrap().style.maximized =
          is_flag_set(wparam.0 as u32, WindowsAndMessaging::SIZE_MAXIMIZED);

        // info!("RESIZED: {_size:?}");
        let is_windowed = self.data.lock().unwrap().style.fullscreen.is_none();
        // // data.state.write_lock().size = size;
        if is_windowed {
          self.update_last_windowed_pos_size(hwnd);
        }

        let width = lo_word(lparam.0 as u32) as u32;
        let height = hi_word(lparam.0 as u32) as u32;

        self.send_message_to_main(Message::Resized(PhysicalSize::new(width, height)));
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
      }
      WindowsAndMessaging::WM_MOVE => {
        let x = lo_word(lparam.0 as u32) as i32;
        let y = hi_word(lparam.0 as u32) as i32;

        self.send_message_to_main(Message::Moved(PhysicalPosition::new(x, y)));
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
      }
      WindowsAndMessaging::WM_WINDOWPOSCHANGED => {
        let window_pos = unsafe { &*(lparam.0 as *const WINDOWPOS) };

        // if (window_pos.flags & WindowsAndMessaging::SWP_NOMOVE) !=
        // WindowsAndMessaging::SWP_NOMOVE {
        //   out.push(Message::Moved(PhysicalPosition::new((x, y))))
        // }
        // info!("BOUNDSCHANGED: {outer_position:?}, {outer_size:?}");
        let is_windowed = self.data.lock().unwrap().style.fullscreen.is_none();
        // // data.state.write_lock().position = position;
        if is_windowed {
          self.update_last_windowed_pos_size(hwnd);
        }

        self.send_message_to_main(Message::BoundsChanged {
          outer_position: PhysicalPosition::new(window_pos.x, window_pos.y),
          outer_size: PhysicalSize::new(window_pos.cx as u32, window_pos.cy as u32),
        });

        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
      }
      WindowsAndMessaging::WM_NCACTIVATE => {
        let is_active = wparam.0 == true.into();
        self.data.lock().unwrap().style.active = is_active;

        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
      }
      WindowsAndMessaging::WM_SETFOCUS => {
        self.data.lock().unwrap().style.focused = true;
        if let Err(e) = self.refresh_os_cursor() {
          tracing::error!("{e}");
        };
        self.send_message_to_main(Message::Focus(Focus::Gained));

        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
      }
      WindowsAndMessaging::WM_KILLFOCUS => {
        self.data.lock().unwrap().style.focused = false;
        if let Err(e) = self.refresh_os_cursor() {
          tracing::error!("{e}");
        };
        self.send_message_to_main(Message::Focus(Focus::Lost));
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
      }
      WindowsAndMessaging::WM_COMMAND => {
        self.send_message_to_main(Message::Command);
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
      }
      WindowsAndMessaging::WM_SYSCOMMAND => {
        match wparam.0 as u32 {
          WindowsAndMessaging::SC_MINIMIZE => {
            self.data.lock().unwrap().style.minimized = true;
          }
          WindowsAndMessaging::SC_RESTORE => {
            self.data.lock().unwrap().style.minimized = false;
          }
          _ => {}
        }

        self.send_message_to_main(Message::SystemCommand);
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
      }
      WindowsAndMessaging::WM_DPICHANGED => {
        let dpi = lo_word(wparam.0 as u32) as u32;
        let suggested_rect = unsafe { *(lparam.0 as *const RECT) };
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
        let scale_factor = dpi_to_scale_factor(dpi);
        self.data.lock().unwrap().scale_factor = scale_factor;
        self.send_message_to_main(Message::ScaleFactorChanged(scale_factor));
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
      }
      WindowsAndMessaging::WM_INPUT => {
        let Some(data) = read_raw_input(HRAWINPUT(lparam.0 as _)) else {
          return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
        };

        if wparam.0 as u32 == WindowsAndMessaging::RIM_INPUT {
          unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
        }

        match RID_DEVICE_INFO_TYPE(data.header.dwType) {
          UI::Input::RIM_TYPEMOUSE => {
            let mouse_data = unsafe { data.data.mouse };
            let button_flags = unsafe { mouse_data.Anonymous.Anonymous.usButtonFlags };

            if is_flag_set(mouse_data.usFlags.0, UI::Input::MOUSE_MOVE_RELATIVE.0) {
              let x = mouse_data.lLastX as f32;
              let y = mouse_data.lLastY as f32;

              if mouse_data.lLastX != 0 || mouse_data.lLastY != 0 {
                self.send_message_to_main(Message::RawInput(
                  RawInputMessage::MouseMove {
                    delta_x: x,
                    delta_y: y,
                  },
                ));
              }
            }

            for (id, state) in mouse_button_states(button_flags).iter().enumerate() {
              if let Some(state) = *state {
                let button = MouseButton::from_state(id);
                self.send_message_to_main(Message::RawInput(
                  RawInputMessage::MouseButton { button, state },
                ))
              }
            }
          }
          UI::Input::RIM_TYPEKEYBOARD => {
            let keyboard_data = unsafe { data.data.keyboard };

            let Some(key) = Key::from_raw(keyboard_data) else {
              return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
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
              self.send_message_to_main(Message::RawInput(RawInputMessage::Keyboard {
                key,
                state,
              }));
            }
          }
          _ => (),
        };
        LRESULT(0)
      }
      WindowsAndMessaging::WM_CHAR => {
        let text = char::from_u32(wparam.0 as u32)
          .unwrap_or_default()
          .to_string();
        self.send_message_to_main(Message::Text(text));
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
      }
      WindowsAndMessaging::WM_KEYDOWN
      | WindowsAndMessaging::WM_SYSKEYDOWN
      | WindowsAndMessaging::WM_KEYUP
      | WindowsAndMessaging::WM_SYSKEYUP => {
        let (changed, shift, ctrl, alt, win) =
          self.data.lock().unwrap().input.update_modifiers_state();
        if changed {
          self.send_message_to_main(Message::ModifiersChanged {
            shift,
            ctrl,
            alt,
            win,
          });
        }
        let message = Message::new_keyboard_message(lparam);
        if let Message::Key { key, state, .. } = &message {
          self
            .data
            .lock()
            .unwrap()
            .input
            .update_key_state(*key, *state);
        }
        self.send_message_to_main(message);
        // messages.push();
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
      }
      WindowsAndMessaging::WM_MOUSEMOVE => {
        let x = signed_lo_word(lparam.0 as i32) as i32;
        let y = signed_hi_word(lparam.0 as i32) as i32;
        let position = PhysicalPosition::new(x, y);

        let kind = get_cursor_move_kind(
          hwnd,
          self.data.lock().unwrap().cursor.inside_window,
          x,
          y,
        );

        let send_message = {
          match kind {
            CursorMoveKind::Entered => {
              self.data.lock().unwrap().cursor.inside_window = true;
              if let Err(e) = self.refresh_os_cursor() {
                tracing::error!("{e}");
              };

              unsafe {
                TrackMouseEvent(&mut TRACKMOUSEEVENT {
                  cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                  dwFlags: KeyboardAndMouse::TME_LEAVE,
                  hwndTrack: hwnd,
                  dwHoverTime: Controls::HOVER_DEFAULT,
                })
              }
              .unwrap();

              true
            }
            CursorMoveKind::Left => {
              self.data.lock().unwrap().cursor.inside_window = false;
              if let Err(e) = self.refresh_os_cursor() {
                tracing::error!("{e}");
              };

              true
            }
            CursorMoveKind::Inside => {
              self.data.lock().unwrap().cursor.last_position != position
            }
          }
        };

        if send_message {
          self.send_message_to_main(Message::CursorMove { position, kind });
          self.data.lock().unwrap().cursor.last_position = position;
          if let Err(e) = self.refresh_os_cursor() {
            tracing::error!("{e}");
          };
        }
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
      }
      Controls::WM_MOUSELEAVE => {
        self.data.lock().unwrap().cursor.inside_window = false;
        if let Err(e) = self.refresh_os_cursor() {
          tracing::error!("{e}");
        };
        let position = self.data.lock().unwrap().cursor.last_position;
        self.send_message_to_main(Message::CursorMove {
          position,
          kind: CursorMoveKind::Left,
        });
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
      }
      WindowsAndMessaging::WM_MOUSEWHEEL => {
        let delta = signed_hi_word(wparam.0 as i32) as f32
          / WindowsAndMessaging::WHEEL_DELTA as f32;
        self.send_message_to_main(Message::MouseWheel {
          delta_x: 0.0,
          delta_y: delta,
        });
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
      }
      WindowsAndMessaging::WM_MOUSEHWHEEL => {
        let delta = signed_hi_word(wparam.0 as i32) as f32
          / WindowsAndMessaging::WHEEL_DELTA as f32;
        self.send_message_to_main(Message::MouseWheel {
          delta_x: delta,
          delta_y: 0.0,
        });
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
      }
      msg
        if (WindowsAndMessaging::WM_MOUSEFIRST..=WindowsAndMessaging::WM_MOUSELAST)
          .contains(&msg) =>
      {
        // mouse move / wheels will match earlier
        let message = Message::new_mouse_button_message(msg, wparam, lparam);
        if let Message::MouseButton { button, state, .. } = &message {
          self
            .data
            .lock()
            .unwrap()
            .input
            .update_mouse_button_state(*button, *state);
        }
        self.send_message_to_main(message);
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
      }
      _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
  }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Position {
  Logical(LogicalPosition),
  Physical(PhysicalPosition),
}

impl Position {
  pub fn new(position: impl Into<Self>) -> Self {
    position.into()
  }

  pub fn as_logical(&self, scale_factor: f64) -> LogicalPosition {
    match *self {
      Position::Logical(position) => position,
      Position::Physical(position) => position.as_logical(scale_factor),
    }
  }

  pub fn as_physical(&self, scale_factor: f64) -> PhysicalPosition {
    match *self {
      Position::Logical(position) => position.as_physical(scale_factor),
      Position::Physical(position) => position,
    }
  }
}

impl From<LogicalPosition> for Position {
  fn from(val: LogicalPosition) -> Self {
    Self::Logical(val)
  }
}

impl From<(f64, f64)> for Position {
  fn from(val: (f64, f64)) -> Self {
    Self::Logical(val.into())
  }
}

impl From<[f64; 2]> for Position {
  fn from(val: [f64; 2]) -> Self {
    Self::Logical(val.into())
  }
}

impl From<PhysicalPosition> for Position {
  fn from(val: PhysicalPosition) -> Self {
    Self::Physical(val)
  }
}

impl From<(i32, i32)> for Position {
  fn from(val: (i32, i32)) -> Self {
    Self::Physical(val.into())
  }
}

impl From<[i32; 2]> for Position {
  fn from(val: [i32; 2]) -> Self {
    Self::Physical(val.into())
  }
}

#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct LogicalPosition {
  pub x: f64,
  pub y: f64,
}

impl LogicalPosition {
  pub fn new(x: f64, y: f64) -> Self {
    Self { x, y }
  }

  pub fn as_physical(&self, scale_factor: f64) -> PhysicalPosition {
    PhysicalPosition::new(self.x.round() as i32, self.y.round() as i32) * scale_factor
  }

  pub fn is_positive(&self) -> bool {
    self.x > 0.0 && self.y > 0.0
  }

  pub fn is_negative(&self) -> bool {
    self.x < 0.0 && self.y < 0.0
  }

  pub fn is_zero(&self) -> bool {
    self.x == 0.0 && self.y == 0.0
  }
}

impl Div<f64> for LogicalPosition {
  type Output = Self;

  fn div(self, rhs: f64) -> Self::Output {
    Self {
      y: (self.y / rhs).round(),
      x: (self.x / rhs).round(),
    }
  }
}

impl Mul<f64> for LogicalPosition {
  type Output = Self;

  fn mul(self, rhs: f64) -> Self::Output {
    Self {
      y: (self.y * rhs).round(),
      x: (self.x * rhs).round(),
    }
  }
}

impl From<LogicalPosition> for (f64, f64) {
  fn from(val: LogicalPosition) -> Self {
    (val.x, val.y)
  }
}

impl From<LogicalPosition> for [f64; 2] {
  fn from(val: LogicalPosition) -> Self {
    [val.x, val.y]
  }
}

impl From<(f64, f64)> for LogicalPosition {
  fn from(value: (f64, f64)) -> Self {
    Self {
      x: value.0,
      y: value.1,
    }
  }
}

impl From<[f64; 2]> for LogicalPosition {
  fn from(value: [f64; 2]) -> Self {
    Self {
      x: value[0],
      y: value[1],
    }
  }
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct PhysicalPosition {
  pub x: i32,
  pub y: i32,
}

impl PhysicalPosition {
  pub fn new(x: i32, y: i32) -> Self {
    Self { x, y }
  }

  pub fn as_logical(&self, scale_factor: f64) -> LogicalPosition {
    LogicalPosition::new(self.x as f64, self.y as f64) / scale_factor
  }

  pub fn is_positive(&self) -> bool {
    self.x > 0 && self.y > 0
  }

  pub fn is_negative(&self) -> bool {
    self.x < 0 && self.y < 0
  }

  pub fn is_zero(&self) -> bool {
    self.x == 0 && self.y == 0
  }
}

impl Div<f64> for PhysicalPosition {
  type Output = Self;

  fn div(self, rhs: f64) -> Self::Output {
    Self {
      y: (self.y as f64 / rhs).round() as i32,
      x: (self.x as f64 / rhs).round() as i32,
    }
  }
}

impl Mul<f64> for PhysicalPosition {
  type Output = Self;

  fn mul(self, rhs: f64) -> Self::Output {
    Self {
      y: (self.y as f64 * rhs).trunc() as i32,
      x: (self.x as f64 * rhs).trunc() as i32,
    }
  }
}

impl From<PhysicalPosition> for (u32, u32) {
  fn from(val: PhysicalPosition) -> Self {
    (val.x as u32, val.y as u32)
  }
}

impl From<PhysicalPosition> for (i32, i32) {
  fn from(val: PhysicalPosition) -> Self {
    (val.x, val.y)
  }
}

impl From<PhysicalPosition> for [u32; 2] {
  fn from(val: PhysicalPosition) -> Self {
    [val.x as u32, val.y as u32]
  }
}

impl From<PhysicalPosition> for [i32; 2] {
  fn from(val: PhysicalPosition) -> Self {
    [val.x, val.y]
  }
}

impl From<(i32, i32)> for PhysicalPosition {
  fn from(value: (i32, i32)) -> Self {
    Self {
      x: value.0,
      y: value.1,
    }
  }
}

impl From<[i32; 2]> for PhysicalPosition {
  fn from(value: [i32; 2]) -> Self {
    Self {
      x: value[0],
      y: value[1],
    }
  }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Size {
  Logical(LogicalSize),
  Physical(PhysicalSize),
}

impl Size {
  pub fn new(size: impl Into<Self>) -> Self {
    size.into()
  }

  pub fn as_logical(&self, scale_factor: f64) -> LogicalSize {
    match *self {
      Size::Logical(size) => size,
      Size::Physical(size) => size.as_logical(scale_factor),
    }
  }

  pub fn as_physical(&self, scale_factor: f64) -> PhysicalSize {
    match *self {
      Size::Logical(size) => size.as_physical(scale_factor),
      Size::Physical(size) => size,
    }
  }
}

impl From<LogicalSize> for Size {
  fn from(val: LogicalSize) -> Self {
    Self::Logical(val)
  }
}

impl From<PhysicalSize> for Size {
  fn from(val: PhysicalSize) -> Self {
    Self::Physical(val)
  }
}

#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct LogicalSize {
  pub width: f64,
  pub height: f64,
}

impl LogicalSize {
  pub fn new(width: f64, height: f64) -> Self {
    Self { width, height }
  }

  pub fn as_physical(&self, scale_factor: f64) -> PhysicalSize {
    PhysicalSize::new(self.width.round() as u32, self.height.round() as u32)
      * scale_factor
  }

  pub fn is_any_positive(&self) -> bool {
    self.width > 0.0 || self.height > 0.0
  }

  pub fn is_all_positive(&self) -> bool {
    self.width > 0.0 && self.height > 0.0
  }

  pub fn is_any_negative(&self) -> bool {
    self.width < 0.0 || self.height < 0.0
  }

  pub fn is_all_negative(&self) -> bool {
    self.width < 0.0 && self.height < 0.0
  }

  pub fn is_any_zero(&self) -> bool {
    self.width == 0.0 || self.height == 0.0
  }

  pub fn is_all_zero(&self) -> bool {
    self.width == 0.0 && self.height == 0.0
  }
}

impl Div<f64> for LogicalSize {
  type Output = Self;

  fn div(self, rhs: f64) -> Self::Output {
    Self {
      height: (self.height / rhs).round(),
      width: (self.width / rhs).round(),
    }
  }
}

impl Mul<f64> for LogicalSize {
  type Output = Self;

  fn mul(self, rhs: f64) -> Self::Output {
    Self {
      height: (self.height * rhs).round(),
      width: (self.width * rhs).round(),
    }
  }
}

impl From<LogicalSize> for (f64, f64) {
  fn from(val: LogicalSize) -> Self {
    (val.width, val.height)
  }
}

impl From<LogicalSize> for [f64; 2] {
  fn from(val: LogicalSize) -> Self {
    [val.width, val.height]
  }
}

impl From<(f64, f64)> for LogicalSize {
  fn from(value: (f64, f64)) -> Self {
    Self {
      width: value.0,
      height: value.1,
    }
  }
}

impl From<[f64; 2]> for LogicalSize {
  fn from(value: [f64; 2]) -> Self {
    Self {
      width: value[0],
      height: value[1],
    }
  }
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct PhysicalSize {
  pub width: u32,
  pub height: u32,
}

impl PhysicalSize {
  pub fn new(width: u32, height: u32) -> Self {
    Self { width, height }
  }

  pub fn as_logical(&self, scale_factor: f64) -> LogicalSize {
    LogicalSize::new(self.width as f64, self.height as f64) / scale_factor
  }

  pub fn is_any_zero(&self) -> bool {
    self.width == 0 || self.height == 0
  }

  pub fn is_all_zero(&self) -> bool {
    self.width == 0 && self.height == 0
  }
}

impl Div<f64> for PhysicalSize {
  type Output = Self;

  fn div(self, rhs: f64) -> Self::Output {
    Self {
      height: (self.height as f64 / rhs).round() as u32,
      width: (self.width as f64 / rhs).round() as u32,
    }
  }
}

impl Mul<f64> for PhysicalSize {
  type Output = Self;

  fn mul(self, rhs: f64) -> Self::Output {
    Self {
      height: (self.height as f64 * rhs).trunc() as u32,
      width: (self.width as f64 * rhs).trunc() as u32,
    }
  }
}

impl From<PhysicalSize> for (u32, u32) {
  fn from(val: PhysicalSize) -> Self {
    (val.width, val.height)
  }
}

impl From<PhysicalSize> for (i32, i32) {
  fn from(val: PhysicalSize) -> Self {
    (val.width as i32, val.height as i32)
  }
}

impl From<PhysicalSize> for [u32; 2] {
  fn from(val: PhysicalSize) -> Self {
    [val.width, val.height]
  }
}

impl From<PhysicalSize> for [i32; 2] {
  fn from(val: PhysicalSize) -> Self {
    [val.width as i32, val.height as i32]
  }
}

impl From<(u32, u32)> for PhysicalSize {
  fn from(value: (u32, u32)) -> Self {
    Self {
      width: value.0,
      height: value.1,
    }
  }
}

impl From<[u32; 2]> for PhysicalSize {
  fn from(value: [u32; 2]) -> Self {
    Self {
      width: value[0],
      height: value[1],
    }
  }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Fullscreen {
  // Exclusive, // todo
  Borderless,
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum CursorMode {
  #[default]
  Normal,
  Confined,
}

/// The wait behaviour of the window.
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Flow {
  /// Window will block if there are no new messages.
  #[default]
  Wait,
  /// Window will send an artificial
  /// [`LoopMessage::Empty`](`crate::LoopMessage::Empty`) when there are no
  /// new messages and will not block.
  Poll,
}

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Visibility {
  #[default]
  Shown,
  Hidden,
}

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Theme {
  #[default]
  Auto,
  Dark,
  Light,
}
