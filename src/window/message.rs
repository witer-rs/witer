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
    Input::KeyboardAndMouse::{MapVirtualKeyW, MAPVK_VSC_TO_VK_EX, VIRTUAL_KEY},
    WindowsAndMessaging::{self, GetClientRect},
  },
};

use super::{
  command::Command,
  input::{mouse::MouseButton, state::RawKeyState},
  state::{PhysicalPosition, PhysicalSize},
};
use crate::{
  utilities::{hi_word, is_flag_set, lo_byte, lo_word, signed_hi_word, signed_lo_word},
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
  /// Message sent when a text character is typed containing that character.
  Char(char),
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
  /// Sent when the message pump is about to do GetMessageW.
  GetMessage,
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
