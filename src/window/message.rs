use std::sync::{Arc, Mutex};

use cursor_icon::CursorIcon;
use windows::Win32::{
  Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM},
  System::SystemServices::{
    MK_LBUTTON,
    MK_MBUTTON,
    MK_RBUTTON,
    MK_XBUTTON1,
    MK_XBUTTON2,
    MODIFIERKEYS_FLAGS,
  },
  UI::{
    self,
    Controls,
    HiDpi::EnableNonClientDpiScaling,
    Input::{
      KeyboardAndMouse::{
        self,
        MapVirtualKeyW,
        TrackMouseEvent,
        MAPVK_VSC_TO_VK_EX,
        TRACKMOUSEEVENT,
        VIRTUAL_KEY,
      },
      HRAWINPUT,
      RID_DEVICE_INFO_TYPE,
    },
    WindowsAndMessaging::{
      self,
      DefWindowProcW,
      GetClientRect,
      GetWindowLongPtrW,
      LoadCursorW,
      PostQuitMessage,
      SetCursor,
      SetWindowLongPtrW,
      SetWindowPos,
      CREATESTRUCTW,
      WINDOWPOS,
    },
  },
};

use super::{
  command::Command,
  data::{Internal, PhysicalPosition, PhysicalSize},
  input::{
    mouse::{mouse_button_states, MouseButton},
    state::RawKeyState,
  },
  procedure::UserData,
};
use crate::{
  utilities::{
    dpi_to_scale_factor,
    hi_word,
    hwnd_dpi,
    is_flag_set,
    lo_byte,
    lo_word,
    read_raw_input,
    register_all_mice_and_keyboards_for_raw_input,
    signed_hi_word,
    signed_lo_word,
    to_windows_cursor,
  },
  window::{
    cursor::Cursor,
    data::Data,
    input::{
      key::Key,
      state::{ButtonState, KeyState},
    },
    procedure::CreateInfo,
    stage::Stage,
  },
  Input,
  Visibility,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Focus {
  Gained,
  Lost,
}

/// Messages sent by the window, message loop, or attached devices.
#[derive(Debug, PartialEq, Clone)]
pub enum Message {
  /// Artificial window messages sent by the window loop.
  Loop(LoopMessage),
  /// Messages sent by devices registered for raw input.
  RawInput(RawInputMessage),
  /// Message sent when window is created.
  Created { hwnd: usize, hinstance: usize },
  /// Message sent when window X button is pressed.
  CloseRequested,
  /// Message sent when Windows requests the window be repainted.
  Paint,
  /// Message sent when a key is pressed, held, or released.
  Key {
    key: Key,
    state: KeyState,
    scan_code: u16,
    is_extended_key: bool,
  },
  /// Message sent when a text character is typed containing that character.
  Text(String),
  ModifiersChanged {
    shift: ButtonState,
    ctrl: ButtonState,
    alt: ButtonState,
    win: ButtonState,
  },
  /// Message sent when a mouse button is pressed or released.
  MouseButton {
    button: MouseButton,
    state: ButtonState,
    position: PhysicalPosition,
    is_double_click: bool,
  },
  /// Message sent when the scroll wheel is actuated.
  MouseWheel { delta_x: f32, delta_y: f32 },
  /// Message sent when the cursor is moved within the window bounds. Don't
  /// use this for mouse input in cases such as first-person cameras as it is
  /// locked to the bounds of the window.
  CursorMove {
    position: PhysicalPosition,
    kind: CursorMoveKind,
  },
  /// Message sent when the window is resized. Sent after [`BoundsChanged`]
  Resized(PhysicalSize),
  /// Message sent when the window is moved. Sent after [`BoundsChanged`]
  Moved(PhysicalPosition),
  /// Message sent first when the window is moved or resized.
  BoundsChanged {
    outer_position: PhysicalPosition,
    outer_size: PhysicalSize,
  },
  /// Message sent by Windows when certain actions are taken. WIP
  Command,
  /// Message sent by Windows when certain actions are taken. WIP
  SystemCommand,
  /// Message sent when the window gains or loses focus.
  Focus(Focus),
  /// Message sent when the scale factor of the window has changed.
  ScaleFactorChanged(f64),
}

/// Artificial window messages sent by the window loop.
#[derive(Debug, PartialEq, Clone)]
pub enum LoopMessage {
  /// Sent when the window receives a command request.
  Command(Command),
  /// Sent when the message pump is polled, but there are no messages.
  Empty,
  /// Sent when the message pump is exiting.
  Exit,
}

#[derive(Debug, PartialEq, Clone)]
pub enum RawInputMessage {
  /// Raw keyboard input
  Keyboard { key: Key, state: RawKeyState },
  /// Raw mouse button input
  MouseButton {
    button: MouseButton,
    state: ButtonState,
  },
  /// Raw mouse motion. Use this for mouse input in cases such as first-person
  /// cameras.
  MouseMove { delta_x: f32, delta_y: f32 },
}

impl Message {
  pub(crate) fn new_keyboard_message(l_param: LPARAM) -> Message {
    let flags = hi_word(unsafe { std::mem::transmute::<i32, u32>(l_param.0 as i32) });

    let is_extended_key = is_flag_set(flags, WindowsAndMessaging::KF_EXTENDED as u16);

    let mut scan_code = lo_byte(flags) as u16;

    let key_code: Key = {
      let extended_scan_code = u16::from_le_bytes([scan_code as u8, 0xE0]);
      let extended_virtual_keycode = VIRTUAL_KEY(lo_word(unsafe {
        MapVirtualKeyW(extended_scan_code as u32, MAPVK_VSC_TO_VK_EX)
      }));

      let virtual_keycode =
        if extended_virtual_keycode != VIRTUAL_KEY(0) && is_extended_key {
          scan_code = extended_scan_code;
          extended_virtual_keycode
        } else {
          VIRTUAL_KEY(lo_word(unsafe {
            MapVirtualKeyW(scan_code as u32, MAPVK_VSC_TO_VK_EX)
          }))
        };

      virtual_keycode.into()
    };

    let state = {
      let repeat_count = lo_word(l_param.0 as u32);
      let was_key_down = is_flag_set(flags, WindowsAndMessaging::KF_REPEAT as u16);
      let is_key_up = is_flag_set(flags, WindowsAndMessaging::KF_UP as u16);

      match (is_key_up, was_key_down) {
        (true, _) => KeyState::Released,
        (false, true) => KeyState::Held(repeat_count),
        (..) => KeyState::Pressed,
      }
    };

    Message::Key {
      key: key_code,
      state,
      scan_code,
      is_extended_key,
    }
  }

  pub(crate) fn new_mouse_button_message(
    message: u32,
    w_param: WPARAM,
    l_param: LPARAM,
  ) -> Message {
    let flags = w_param.0 as u32;

    let mouse_code: MouseButton = {
      match message {
        WindowsAndMessaging::WM_LBUTTONDBLCLK
        | WindowsAndMessaging::WM_LBUTTONDOWN
        | WindowsAndMessaging::WM_LBUTTONUP => MouseButton::Left,
        WindowsAndMessaging::WM_MBUTTONDBLCLK
        | WindowsAndMessaging::WM_MBUTTONDOWN
        | WindowsAndMessaging::WM_MBUTTONUP => MouseButton::Middle,
        WindowsAndMessaging::WM_RBUTTONDBLCLK
        | WindowsAndMessaging::WM_RBUTTONDOWN
        | WindowsAndMessaging::WM_RBUTTONUP => MouseButton::Right,
        WindowsAndMessaging::WM_XBUTTONDBLCLK
        | WindowsAndMessaging::WM_XBUTTONDOWN
        | WindowsAndMessaging::WM_XBUTTONUP => {
          let hi_flags = hi_word(flags);
          if (hi_flags & WindowsAndMessaging::XBUTTON1) == WindowsAndMessaging::XBUTTON1 {
            MouseButton::Back
          } else {
            MouseButton::Forward
          }
        }
        _ => MouseButton::Unknown,
      }
    };

    let is_double_click = matches!(
      message,
      WindowsAndMessaging::WM_LBUTTONDBLCLK
        | WindowsAndMessaging::WM_MBUTTONDBLCLK
        | WindowsAndMessaging::WM_RBUTTONDBLCLK
        | WindowsAndMessaging::WM_XBUTTONDBLCLK
    );

    let state = {
      let mod_flags = MODIFIERKEYS_FLAGS(flags);
      let is_l_down = (mod_flags & MK_LBUTTON) == MK_LBUTTON;
      let is_m_down = (mod_flags & MK_MBUTTON) == MK_MBUTTON;
      let is_r_down = (mod_flags & MK_RBUTTON) == MK_RBUTTON;
      let is_x1_down = (mod_flags & MK_XBUTTON1) == MK_XBUTTON1;
      let is_x2_down = (mod_flags & MK_XBUTTON2) == MK_XBUTTON2;

      let is_down = match message {
        WindowsAndMessaging::WM_LBUTTONDBLCLK | WindowsAndMessaging::WM_LBUTTONDOWN
          if is_l_down =>
        {
          true
        }
        WindowsAndMessaging::WM_MBUTTONDBLCLK | WindowsAndMessaging::WM_MBUTTONDOWN
          if is_m_down =>
        {
          true
        }
        WindowsAndMessaging::WM_RBUTTONDBLCLK | WindowsAndMessaging::WM_RBUTTONDOWN
          if is_r_down =>
        {
          true
        }
        WindowsAndMessaging::WM_XBUTTONDBLCLK | WindowsAndMessaging::WM_XBUTTONDOWN
          if is_x1_down || is_x2_down =>
        {
          true
        }
        _ => false,
      };

      if is_down {
        ButtonState::Pressed
      } else {
        ButtonState::Released
      }
    };

    let (x, y) = (signed_lo_word(l_param.0 as i32), signed_hi_word(l_param.0 as i32));

    let position = PhysicalPosition::new(x as i32, y as i32);

    Message::MouseButton {
      button: mouse_code,
      state,
      position,
      is_double_click,
    }
  }

  /// Returns `true` if the message matches the supplied key and key state
  pub fn is_key(&self, key: Key, state: KeyState) -> bool {
    matches!(self, Message::Key { key: k, state: s, .. } if *k == key && *s == state)
  }

  /// Returns `true` if the message matches the supplied mouse button and mouse
  /// button state
  pub fn is_mouse_button(&self, button: MouseButton, state: ButtonState) -> bool {
    matches!(self, Message::MouseButton { button: b, state: s, .. } if *b == button && *s == state)
  }

  /// Returns `true` if the message is [`LoopMessage::Empty`]
  pub fn is_empty(&self) -> bool {
    matches!(self, Message::Loop(LoopMessage::Empty))
  }
}

/*
  Adapted from `winit` according to Apache-2.0 license. (https://github.com/rust-windowing/winit/blob/master/src/platform_impl/windows/event_loop.rs#L2568)
  Adapted for windows crate.
*/
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum CursorMoveKind {
  /// Cursor entered to the window.
  Entered,
  /// Cursor left the window client area.
  Left,
  /// Cursor is inside the window or `GetClientRect` failed.
  Inside,
}

pub(crate) fn get_cursor_move_kind(
  hwnd: HWND,
  mouse_was_inside_window: bool,
  x: i32,
  y: i32,
) -> CursorMoveKind {
  let rect: RECT = {
    let mut rect = RECT::default();
    if unsafe { GetClientRect(hwnd, &mut rect) }.is_err() {
      return CursorMoveKind::Inside; // exit early if GetClientRect failed
    }
    rect
  };

  let x = (rect.left..rect.right).contains(&x);
  let y = (rect.top..rect.bottom).contains(&y);

  if !mouse_was_inside_window && x && y {
    CursorMoveKind::Entered
  } else if mouse_was_inside_window && !(x && y) {
    CursorMoveKind::Left
  } else {
    CursorMoveKind::Inside
  }
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct RawMessage {
  pub id: u32,
  pub w: WPARAM,
  pub l: LPARAM,
}

impl From<WindowsAndMessaging::MSG> for RawMessage {
  fn from(value: WindowsAndMessaging::MSG) -> Self {
    Self {
      id: value.message,
      w: value.wParam,
      l: value.lParam,
    }
  }
}

impl RawMessage {
  pub(crate) fn process(&self, hwnd: HWND, internal: Option<Arc<Internal>>) -> LRESULT {
    match (internal, self.id) {
      (internal, Command::MESSAGE_ID) => {
        Command::from_wparam(self.w).process(hwnd, internal)
      }
      (None, WindowsAndMessaging::WM_NCCREATE) => self.on_nccreate(hwnd),
      (None, WindowsAndMessaging::WM_CREATE) => self.on_create(hwnd),
      (None, WindowsAndMessaging::WM_DESTROY) => {
        let user_data_ptr =
          unsafe { GetWindowLongPtrW(hwnd, WindowsAndMessaging::GWLP_USERDATA) };
        drop(unsafe { Box::from_raw(user_data_ptr as *mut UserData) });
        unsafe { SetWindowLongPtrW(hwnd, WindowsAndMessaging::GWLP_USERDATA, 0) };
        unsafe { PostQuitMessage(0) };

        unsafe { DefWindowProcW(hwnd, self.id, self.w, self.l) }
      }
      (Some(internal), WindowsAndMessaging::WM_SETCURSOR) => {
        let in_client_area =
          lo_word(self.l.0 as u32) as u32 == WindowsAndMessaging::HTCLIENT;

        if in_client_area {
          let icon = internal.data.lock().unwrap().cursor.selected_icon;
          let cursor_icon = to_windows_cursor(icon);
          let hcursor =
            unsafe { LoadCursorW(HINSTANCE::default(), cursor_icon) }.unwrap();
          unsafe { SetCursor(hcursor) };
        }

        unsafe { DefWindowProcW(hwnd, self.id, self.w, self.l) }
      }
      // WindowsAndMessaging::WM_SIZING | WindowsAndMessaging::WM_MOVING => {
      //   // ignore certain messages
      //   return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
      // }
      (Some(internal), WindowsAndMessaging::WM_CLOSE) => {
        internal.send_message_to_main(Message::CloseRequested);
        LRESULT(0)
      }
      (Some(internal), WindowsAndMessaging::WM_PAINT) => {
        internal.data.lock().unwrap().requested_redraw = false;
        internal.send_message_to_main(Message::Paint);

        unsafe { DefWindowProcW(hwnd, self.id, self.w, self.l) }
      }
      (Some(internal), WindowsAndMessaging::WM_SIZE) => {
        internal.data.lock().unwrap().style.maximized =
          is_flag_set(self.w.0 as u32, WindowsAndMessaging::SIZE_MAXIMIZED);

        // info!("RESIZED: {_size:?}");
        let is_windowed = internal.data.lock().unwrap().style.fullscreen.is_none();
        // // data.state.write_lock().size = size;
        if is_windowed {
          internal.update_last_windowed_pos_size(hwnd);
        }

        let width = lo_word(self.l.0 as u32) as u32;
        let height = hi_word(self.l.0 as u32) as u32;

        internal.send_message_to_main(Message::Resized(PhysicalSize::new(width, height)));

        unsafe { DefWindowProcW(hwnd, self.id, self.w, self.l) }
      }
      (Some(internal), WindowsAndMessaging::WM_MOVE) => {
        let x = lo_word(self.l.0 as u32) as i32;
        let y = hi_word(self.l.0 as u32) as i32;

        internal.send_message_to_main(Message::Moved(PhysicalPosition::new(x, y)));

        unsafe { DefWindowProcW(hwnd, self.id, self.w, self.l) }
      }
      (Some(internal), WindowsAndMessaging::WM_WINDOWPOSCHANGED) => {
        let window_pos = unsafe { &*(self.l.0 as *const WINDOWPOS) };

        // if (window_pos.flags & WindowsAndMessaging::SWP_NOMOVE) !=
        // WindowsAndMessaging::SWP_NOMOVE {
        //   out.push(Message::Moved(PhysicalPosition::new((x, y))))
        // }
        // info!("BOUNDSCHANGED: {outer_position:?}, {outer_size:?}");
        let is_windowed = internal.data.lock().unwrap().style.fullscreen.is_none();
        // // data.state.write_lock().position = position;
        if is_windowed {
          internal.update_last_windowed_pos_size(hwnd);
        }

        internal.send_message_to_main(Message::BoundsChanged {
          outer_position: PhysicalPosition::new(window_pos.x, window_pos.y),
          outer_size: PhysicalSize::new(window_pos.cx as u32, window_pos.cy as u32),
        });

        unsafe { DefWindowProcW(hwnd, self.id, self.w, self.l) }
      }
      (Some(internal), WindowsAndMessaging::WM_NCACTIVATE) => {
        let is_active = self.w.0 == true.into();
        internal.data.lock().unwrap().style.active = is_active;

        unsafe { DefWindowProcW(hwnd, self.id, self.w, self.l) }
      }
      (Some(internal), WindowsAndMessaging::WM_SETFOCUS) => {
        internal.data.lock().unwrap().style.focused = true;
        if let Err(e) = internal.refresh_os_cursor() {
          tracing::error!("{e}");
        };
        internal.send_message_to_main(Message::Focus(Focus::Gained));

        unsafe { DefWindowProcW(hwnd, self.id, self.w, self.l) }
      }
      (Some(internal), WindowsAndMessaging::WM_KILLFOCUS) => {
        internal.data.lock().unwrap().style.focused = false;
        if let Err(e) = internal.refresh_os_cursor() {
          tracing::error!("{e}");
        };
        internal.send_message_to_main(Message::Focus(Focus::Lost));

        unsafe { DefWindowProcW(hwnd, self.id, self.w, self.l) }
      }
      (Some(internal), WindowsAndMessaging::WM_COMMAND) => {
        internal.send_message_to_main(Message::Command);

        unsafe { DefWindowProcW(hwnd, self.id, self.w, self.l) }
      }
      (Some(internal), WindowsAndMessaging::WM_SYSCOMMAND) => {
        match self.w.0 as u32 {
          WindowsAndMessaging::SC_MINIMIZE => {
            internal.data.lock().unwrap().style.minimized = true;
          }
          WindowsAndMessaging::SC_RESTORE => {
            internal.data.lock().unwrap().style.minimized = false;
          }
          _ => {}
        }

        internal.send_message_to_main(Message::SystemCommand);

        unsafe { DefWindowProcW(hwnd, self.id, self.w, self.l) }
      }
      (Some(internal), WindowsAndMessaging::WM_DPICHANGED) => {
        let dpi = lo_word(self.w.0 as u32) as u32;
        let suggested_rect = unsafe { *(self.l.0 as *const RECT) };
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
        internal.data.lock().unwrap().scale_factor = scale_factor;
        internal.send_message_to_main(Message::ScaleFactorChanged(scale_factor));

        unsafe { DefWindowProcW(hwnd, self.id, self.w, self.l) }
      }
      (Some(internal), WindowsAndMessaging::WM_INPUT) => {
        let Some(data) = read_raw_input(HRAWINPUT(self.l.0 as _)) else {
          return unsafe { DefWindowProcW(hwnd, self.id, self.w, self.l) };
        };

        if self.w.0 as u32 == WindowsAndMessaging::RIM_INPUT {
          unsafe { DefWindowProcW(hwnd, self.id, self.w, self.l) };
        }

        match RID_DEVICE_INFO_TYPE(data.header.dwType) {
          UI::Input::RIM_TYPEMOUSE => {
            let mouse_data = unsafe { data.data.mouse };
            let button_flags = unsafe { mouse_data.Anonymous.Anonymous.usButtonFlags };

            if is_flag_set(mouse_data.usFlags.0, UI::Input::MOUSE_MOVE_RELATIVE.0) {
              let x = mouse_data.lLastX as f32;
              let y = mouse_data.lLastY as f32;

              if mouse_data.lLastX != 0 || mouse_data.lLastY != 0 {
                internal.send_message_to_main(Message::RawInput(
                  RawInputMessage::MouseMove {
                    delta_x: x,
                    delta_y: y,
                  },
                ));
              }
            }

            for (id, button_state) in mouse_button_states(button_flags).iter().enumerate()
            {
              if let Some(button_state) = *button_state {
                let button = MouseButton::from_state(id);
                internal.send_message_to_main(Message::RawInput(
                  RawInputMessage::MouseButton {
                    button,
                    state: button_state,
                  },
                ))
              }
            }
          }
          UI::Input::RIM_TYPEKEYBOARD => {
            let keyboard_data = unsafe { data.data.keyboard };

            let Some(key) = Key::from_raw(keyboard_data) else {
              return unsafe { DefWindowProcW(hwnd, self.id, self.w, self.l) };
            };

            let pressed = matches!(
              keyboard_data.Message,
              WindowsAndMessaging::WM_KEYDOWN | WindowsAndMessaging::WM_SYSKEYDOWN
            );
            let released = matches!(
              keyboard_data.Message,
              WindowsAndMessaging::WM_KEYUP | WindowsAndMessaging::WM_SYSKEYUP
            );

            if let Some(key_state) = RawKeyState::from_bools(pressed, released) {
              internal.send_message_to_main(Message::RawInput(
                RawInputMessage::Keyboard {
                  key,
                  state: key_state,
                },
              ));
            }
          }
          _ => (),
        };

        LRESULT(0)
      }
      (Some(internal), WindowsAndMessaging::WM_CHAR) => {
        let text = char::from_u32(self.w.0 as u32)
          .unwrap_or_default()
          .to_string();
        internal.send_message_to_main(Message::Text(text));

        unsafe { DefWindowProcW(hwnd, self.id, self.w, self.l) }
      }
      (
        Some(internal),
        WindowsAndMessaging::WM_KEYDOWN
        | WindowsAndMessaging::WM_SYSKEYDOWN
        | WindowsAndMessaging::WM_KEYUP
        | WindowsAndMessaging::WM_SYSKEYUP,
      ) => {
        let (changed, shift, ctrl, alt, win) =
          internal.data.lock().unwrap().input.update_modifiers_state();
        if changed {
          internal.send_message_to_main(Message::ModifiersChanged {
            shift,
            ctrl,
            alt,
            win,
          });
        }
        let message = Message::new_keyboard_message(self.l);
        if let Message::Key { key, state, .. } = &message {
          internal
            .data
            .lock()
            .unwrap()
            .input
            .update_key_state(*key, *state);
        }
        internal.send_message_to_main(message);

        // messages.push();
        unsafe { DefWindowProcW(hwnd, self.id, self.w, self.l) }
      }
      (Some(internal), WindowsAndMessaging::WM_MOUSEMOVE) => {
        let x = signed_lo_word(self.l.0 as i32) as i32;
        let y = signed_hi_word(self.l.0 as i32) as i32;
        let position = PhysicalPosition::new(x, y);

        let kind = get_cursor_move_kind(
          hwnd,
          internal.data.lock().unwrap().cursor.inside_window,
          x,
          y,
        );

        let send_message = {
          match kind {
            CursorMoveKind::Entered => {
              internal.data.lock().unwrap().cursor.inside_window = true;
              if let Err(e) = internal.refresh_os_cursor() {
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
              internal.data.lock().unwrap().cursor.inside_window = false;
              if let Err(e) = internal.refresh_os_cursor() {
                tracing::error!("{e}");
              };

              true
            }
            CursorMoveKind::Inside => {
              internal.data.lock().unwrap().cursor.last_position != position
            }
          }
        };

        if send_message {
          internal.send_message_to_main(Message::CursorMove { position, kind });
          internal.data.lock().unwrap().cursor.last_position = position;
          if let Err(e) = internal.refresh_os_cursor() {
            tracing::error!("{e}");
          };
        }

        unsafe { DefWindowProcW(hwnd, self.id, self.w, self.l) }
      }
      (Some(internal), Controls::WM_MOUSELEAVE) => {
        internal.data.lock().unwrap().cursor.inside_window = false;
        if let Err(e) = internal.refresh_os_cursor() {
          tracing::error!("{e}");
        };
        let position = internal.data.lock().unwrap().cursor.last_position;
        internal.send_message_to_main(Message::CursorMove {
          position,
          kind: CursorMoveKind::Left,
        });

        unsafe { DefWindowProcW(hwnd, self.id, self.w, self.l) }
      }
      (Some(internal), WindowsAndMessaging::WM_MOUSEWHEEL) => {
        let delta = signed_hi_word(self.w.0 as i32) as f32
          / WindowsAndMessaging::WHEEL_DELTA as f32;
        internal.send_message_to_main(Message::MouseWheel {
          delta_x: 0.0,
          delta_y: delta,
        });

        unsafe { DefWindowProcW(hwnd, self.id, self.w, self.l) }
      }
      (Some(internal), WindowsAndMessaging::WM_MOUSEHWHEEL) => {
        let delta = signed_hi_word(self.w.0 as i32) as f32
          / WindowsAndMessaging::WHEEL_DELTA as f32;
        internal.send_message_to_main(Message::MouseWheel {
          delta_x: delta,
          delta_y: 0.0,
        });

        unsafe { DefWindowProcW(hwnd, self.id, self.w, self.l) }
      }
      (Some(internal), ..)
        if (WindowsAndMessaging::WM_MOUSEFIRST..=WindowsAndMessaging::WM_MOUSELAST)
          .contains(&self.id) =>
      {
        // mouse move / wheels will match earlier
        let message = Message::new_mouse_button_message(self.id, self.w, self.l);
        if let Message::MouseButton { button, state, .. } = &message {
          internal
            .data
            .lock()
            .unwrap()
            .input
            .update_mouse_button_state(*button, *state);
        }
        internal.send_message_to_main(message);

        unsafe { DefWindowProcW(hwnd, self.id, self.w, self.l) }
      }
      (..) => unsafe { DefWindowProcW(hwnd, self.id, self.w, self.l) },
    }
  }

  fn on_nccreate(&self, hwnd: HWND) -> LRESULT {
    if let Err(e) = unsafe { EnableNonClientDpiScaling(hwnd) } {
      tracing::error!("{e}");
    }

    register_all_mice_and_keyboards_for_raw_input(hwnd);

    unsafe { DefWindowProcW(hwnd, self.id, self.w, self.l) }
  }

  fn on_create(&self, hwnd: HWND) -> LRESULT {
    let create_struct = unsafe { (self.l.0 as *mut CREATESTRUCTW).as_mut().unwrap() };
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
      hinstance: create_struct.hInstance.0 as _,
      hwnd: hwnd.0 as _,
      class_name: create_info.class_name.clone().into(),
      message: create_info.message.clone(),
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
      internal: Arc::downgrade(&state),
    };
    let user_data_ptr = Box::into_raw(Box::new(user_data));
    unsafe {
      SetWindowLongPtrW(hwnd, WindowsAndMessaging::GWLP_USERDATA, user_data_ptr as isize)
    };

    tracing::trace!("[`{}`]: finalizing window settings", create_info.title);

    let window = crate::window::Window(state.clone());
    window.force_set_theme(create_info.settings.theme);

    if let Some(position) = create_info.position {
      Command::SetPosition(position).send(hwnd);
    }
    Command::SetSize(size).send(hwnd);
    Command::SetDecorations(create_info.settings.decorations).send(hwnd);
    Command::SetVisibility(create_info.settings.visibility).send(hwnd);
    Command::SetFullscreen(create_info.settings.fullscreen).send(hwnd);

    tracing::trace!("[`{}`]: window is ready", create_info.title);
    window.0.data.lock().unwrap().stage = Stage::Ready;
    *window.0.sync.skip_wait.lock().unwrap() = false;

    create_info.window = Some(window);

    create_info
      .message
      .lock()
      .unwrap()
      .replace(Message::Created {
        hwnd: hwnd.0 as _,
        hinstance: create_struct.hInstance.0 as _,
      });
    create_info.sync.signal_new_message();

    unsafe { DefWindowProcW(hwnd, self.id, self.w, self.l) }
  }
}
