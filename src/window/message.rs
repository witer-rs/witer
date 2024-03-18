use windows::Win32::{
  Foundation::{HINSTANCE, HWND, LPARAM, RECT, WPARAM},
  System::SystemServices::{
    MK_LBUTTON,
    MK_MBUTTON,
    MK_RBUTTON,
    MK_XBUTTON1,
    MK_XBUTTON2,
    MODIFIERKEYS_FLAGS,
  },
  UI::{
    Controls,
    Input::{
      self,
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
    WindowsAndMessaging::{self, GetClientRect, SetWindowPos, WINDOWPOS},
  },
};

use super::{
  input::{
    mouse::{mouse_button_states, MouseButton},
    state::RawKeyState,
  },
  state::{InternalState, PhysicalPosition, PhysicalSize},
};
use crate::{
  handle::Handle,
  utilities::{
    dpi_to_scale_factor,
    hi_word,
    lo_byte,
    lo_word,
    read_raw_input,
    signed_hi_word,
    signed_lo_word,
  },
  window::input::{
    key::Key,
    state::{ButtonState, KeyState},
  },
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
  Created { hwnd: HWND, hinstance: HINSTANCE },
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
  /// Sent when the message pump is polled, but there are no messages.
  Empty,
  /// Sent when the message pump is about to do GetMessageW.
  Wait,
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
  pub fn collect(
    hwnd: HWND,
    message: u32,
    w_param: WPARAM,
    l_param: LPARAM,
    state: &Handle<InternalState>,
  ) -> Option<Vec<Self>> {
    let mut out = Vec::with_capacity(0);
    out.reserve_exact(1);
    match message {
      WindowsAndMessaging::WM_CLOSE => out.push(Message::CloseRequested),
      WindowsAndMessaging::WM_PAINT => out.push(Message::Paint),
      WindowsAndMessaging::WM_SIZE => {
        let width = lo_word(l_param.0 as u32) as u32;
        let height = hi_word(l_param.0 as u32) as u32;

        out.push(Message::Resized(PhysicalSize::new(width, height)))
      }
      WindowsAndMessaging::WM_MOVE => {
        let x = lo_word(l_param.0 as u32) as i32;
        let y = hi_word(l_param.0 as u32) as i32;

        out.push(Message::Moved(PhysicalPosition::new(x, y)))
      }
      WindowsAndMessaging::WM_WINDOWPOSCHANGED => {
        let window_pos = unsafe { &*(l_param.0 as *const WINDOWPOS) };
        // if (window_pos.flags & WindowsAndMessaging::SWP_NOMOVE) !=
        // WindowsAndMessaging::SWP_NOMOVE {
        //   out.push(Message::Moved(PhysicalPosition::new((x, y))))
        // }

        out.push(Message::BoundsChanged {
          outer_position: PhysicalPosition::new(window_pos.x, window_pos.y),
          outer_size: PhysicalSize::new(window_pos.cx as u32, window_pos.cy as u32),
        })
      }
      WindowsAndMessaging::WM_SETFOCUS => out.push(Message::Focus(Focus::Gained)),
      WindowsAndMessaging::WM_KILLFOCUS => out.push(Message::Focus(Focus::Lost)),
      WindowsAndMessaging::WM_COMMAND => out.push(Message::Command),
      WindowsAndMessaging::WM_SYSCOMMAND => out.push(Message::SystemCommand),
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
        out.push(Message::ScaleFactorChanged(dpi_to_scale_factor(dpi)))
      }
      WindowsAndMessaging::WM_INPUT => {
        let data = read_raw_input(HRAWINPUT(l_param.0))?;

        match RID_DEVICE_INFO_TYPE(data.header.dwType) {
          Input::RIM_TYPEMOUSE => {
            let mouse_data = unsafe { data.data.mouse };
            let button_flags = unsafe { mouse_data.Anonymous.Anonymous.usButtonFlags };

            if mouse_data.usFlags == Input::MOUSE_MOVE_RELATIVE {
              let x = mouse_data.lLastX as f32;
              let y = mouse_data.lLastY as f32;

              if x != 0.0 || y != 0.0 {
                out.push(Message::RawInput(RawInputMessage::MouseMove {
                  delta_x: x,
                  delta_y: y,
                }));
              }
            }

            for (id, state) in mouse_button_states(button_flags).iter().enumerate() {
              if let Some(state) = *state {
                let button = MouseButton::from_state(id);
                out
                  .push(Message::RawInput(RawInputMessage::MouseButton { button, state }))
              }
            }
          }
          Input::RIM_TYPEKEYBOARD => {
            let keyboard_data = unsafe { data.data.keyboard };

            let key = Key::from_raw(keyboard_data)?;

            let pressed = matches!(
              keyboard_data.Message,
              WindowsAndMessaging::WM_KEYDOWN | WindowsAndMessaging::WM_SYSKEYDOWN
            );
            let released = matches!(
              keyboard_data.Message,
              WindowsAndMessaging::WM_KEYUP | WindowsAndMessaging::WM_SYSKEYUP
            );

            if let Some(state) = RawKeyState::from_bools(pressed, released) {
              out.push(Message::RawInput(RawInputMessage::Keyboard { key, state }))
            }
          }
          _ => return None,
        }
      }
      WindowsAndMessaging::WM_KEYDOWN
      | WindowsAndMessaging::WM_SYSKEYDOWN
      | WindowsAndMessaging::WM_KEYUP
      | WindowsAndMessaging::WM_SYSKEYUP => {
        let (changed, shift, ctrl, alt, win) =
          state.write_lock().input.update_modifiers_state();
        if changed {
          out.push(Message::ModifiersChanged {
            shift,
            ctrl,
            alt,
            win,
          });
        }
        out.push(Self::new_keyboard_message(l_param))
      }
      WindowsAndMessaging::WM_MOUSEMOVE => {
        let x = signed_lo_word(l_param.0 as i32) as i32;
        let y = signed_hi_word(l_param.0 as i32) as i32;
        let position = PhysicalPosition::new(x, y);

        let kind =
          get_cursor_move_kind(hwnd, state.read_lock().cursor.inside_window, x, y);

        let send_message = {
          match kind {
            CursorMoveKind::Entered => {
              state.write_lock().cursor.inside_window = true;

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
              state.write_lock().cursor.inside_window = false;

              true
            }
            CursorMoveKind::Inside => state.read_lock().cursor.last_position != position,
          }
        };

        if send_message {
          out.push(Message::CursorMove { position, kind });
          state.write_lock().cursor.last_position = position;
        }
      }
      Controls::WM_MOUSELEAVE => {
        state.write_lock().cursor.inside_window = false;
        out.push(Message::CursorMove {
          position: state.read_lock().cursor.last_position,
          kind: CursorMoveKind::Left,
        });
      }
      WindowsAndMessaging::WM_MOUSEWHEEL => {
        let delta = signed_hi_word(w_param.0 as i32) as f32
          / WindowsAndMessaging::WHEEL_DELTA as f32;
        out.push(Message::MouseWheel {
          delta_x: 0.0,
          delta_y: delta,
        });
      }
      WindowsAndMessaging::WM_MOUSEHWHEEL => {
        let delta = signed_hi_word(w_param.0 as i32) as f32
          / WindowsAndMessaging::WHEEL_DELTA as f32;
        out.push(Message::MouseWheel {
          delta_x: delta,
          delta_y: 0.0,
        });
      }
      msg
        if (WindowsAndMessaging::WM_MOUSEFIRST..=WindowsAndMessaging::WM_MOUSELAST)
          .contains(&msg) =>
      {
        // mouse move / wheels will match earlier
        out.push(Self::new_mouse_button_message(message, w_param, l_param));
      }
      _ => return None,
    }

    Some(out)
  }

  fn new_keyboard_message(l_param: LPARAM) -> Message {
    let flags = hi_word(unsafe { std::mem::transmute::<i32, u32>(l_param.0 as i32) });

    let is_extended_key = (flags & WindowsAndMessaging::KF_EXTENDED as u16)
      == WindowsAndMessaging::KF_EXTENDED as u16;

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

      let was_key_down = (flags & WindowsAndMessaging::KF_REPEAT as u16)
        == WindowsAndMessaging::KF_REPEAT as u16;
      let is_key_up =
        (flags & WindowsAndMessaging::KF_UP as u16) == WindowsAndMessaging::KF_UP as u16;

      if is_key_up {
        KeyState::Released
      } else if was_key_down {
        KeyState::Held(repeat_count)
      } else {
        KeyState::Pressed
      }
    };

    Message::Key {
      key: key_code,
      state,
      scan_code,
      is_extended_key,
    }
  }

  fn new_mouse_button_message(message: u32, w_param: WPARAM, l_param: LPARAM) -> Message {
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

  /// Returns `true` if the message is `Message::Loop(LoopMessage::Empty)`
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

fn get_cursor_move_kind(
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
