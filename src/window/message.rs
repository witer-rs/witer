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
    WindowsAndMessaging::{self, SetWindowPos, WINDOWPOS},
  },
};

use super::{
  input::mouse::Mouse,
  state::{Position, Size},
};
use crate::{
  utilities::{
    dpi_to_scale_factor,
    hi_word,
    lo_byte,
    lo_word,
    signed_hi_word,
    signed_lo_word,
  },
  window::input::{
    key::Key,
    state::{ButtonState, KeyState},
  },
};

#[derive(Debug, Default, PartialEq, Clone)]
pub enum Message {
  #[default]
  None,
  Window(WindowMessage),
  Wait,
  ExitLoop,
}

#[derive(Debug, PartialEq, Clone)]
pub enum WindowMessage {
  Created {
    hwnd: HWND,
    hinstance: HINSTANCE,
  },
  CloseRequested,
  Paint,
  Key {
    key: Key,
    state: KeyState,
    scan_code: u16,
    is_extended_key: bool,
  },
  MouseButton {
    button: Mouse,
    state: ButtonState,
    x: i16,
    y: i16,
    is_double_click: bool,
  },
  Cursor(Position),
  Scroll {
    x: f32,
    y: f32,
  },
  Resized(Size),
  Moved(Position),
  Command,
  SystemCommand,
  Focus(bool),
  ScaleFactorChanged(f64),
}

impl Message {
  pub fn take(&mut self) -> Message {
    std::mem::take(self)
  }

  pub fn replace(&mut self, message: Message) -> Message {
    std::mem::replace(self, message)
  }

  pub fn new(hwnd: HWND, message: u32, w_param: WPARAM, l_param: LPARAM) -> Option<Self> {
    Some(match message {
      WindowsAndMessaging::WM_CLOSE => Message::Window(WindowMessage::CloseRequested),
      WindowsAndMessaging::WM_PAINT => Message::Window(WindowMessage::Paint),
      WindowsAndMessaging::WM_SIZE => {
        let width = lo_word(l_param.0 as u32) as i32;
        let height = hi_word(l_param.0 as u32) as i32;

        Message::Window(WindowMessage::Resized(Size { width, height }))
      }
      WindowsAndMessaging::WM_WINDOWPOSCHANGED => {
        let window_pos = unsafe { &*(l_param.0 as *const WINDOWPOS) };

        Message::Window(WindowMessage::Moved(Position {
          x: window_pos.x,
          y: window_pos.y,
        }))
      }
      WindowsAndMessaging::WM_SETFOCUS => Message::Window(WindowMessage::Focus(true)),
      WindowsAndMessaging::WM_KILLFOCUS => Message::Window(WindowMessage::Focus(false)),
      WindowsAndMessaging::WM_COMMAND => Message::Window(WindowMessage::Command),
      WindowsAndMessaging::WM_SYSCOMMAND => Message::Window(WindowMessage::SystemCommand),
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
        Message::Window(WindowMessage::ScaleFactorChanged(dpi_to_scale_factor(dpi)))
      }
      WindowsAndMessaging::WM_KEYDOWN
      | WindowsAndMessaging::WM_SYSKEYDOWN
      | WindowsAndMessaging::WM_KEYUP
      | WindowsAndMessaging::WM_SYSKEYUP => Self::new_keyboard_message(l_param),
      WindowsAndMessaging::WM_MOUSEMOVE => {
        let position = Position {
          x: signed_lo_word(l_param.0 as i32) as i32,
          y: signed_hi_word(l_param.0 as i32) as i32,
        };
        Message::Window(WindowMessage::Cursor(position))
      }
      WindowsAndMessaging::WM_MOUSEWHEEL => {
        let delta = signed_hi_word(w_param.0 as i32) as f32
          / WindowsAndMessaging::WHEEL_DELTA as f32;
        Message::Window(WindowMessage::Scroll { x: 0.0, y: delta })
      }
      WindowsAndMessaging::WM_MOUSEHWHEEL => {
        let delta = signed_hi_word(w_param.0 as i32) as f32
          / WindowsAndMessaging::WHEEL_DELTA as f32;
        Message::Window(WindowMessage::Scroll { x: delta, y: 0.0 })
      }
      msg
        if (WindowsAndMessaging::WM_MOUSEFIRST..=WindowsAndMessaging::WM_MOUSELAST)
          .contains(&msg) =>
      {
        // mouse move / wheels will match earlier
        Self::new_mouse_button_message(message, w_param, l_param)
      }
      _ => return None,
    })
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

    Message::Window(WindowMessage::Key {
      key: key_code,
      state,
      scan_code,
      is_extended_key,
    })
  }

  fn new_mouse_button_message(message: u32, w_param: WPARAM, l_param: LPARAM) -> Message {
    let flags = w_param.0 as u32;

    let mouse_code: Mouse = {
      match message {
        WindowsAndMessaging::WM_LBUTTONDBLCLK
        | WindowsAndMessaging::WM_LBUTTONDOWN
        | WindowsAndMessaging::WM_LBUTTONUP => Mouse::Left,
        WindowsAndMessaging::WM_MBUTTONDBLCLK
        | WindowsAndMessaging::WM_MBUTTONDOWN
        | WindowsAndMessaging::WM_MBUTTONUP => Mouse::Middle,
        WindowsAndMessaging::WM_RBUTTONDBLCLK
        | WindowsAndMessaging::WM_RBUTTONDOWN
        | WindowsAndMessaging::WM_RBUTTONUP => Mouse::Right,
        WindowsAndMessaging::WM_XBUTTONDBLCLK
        | WindowsAndMessaging::WM_XBUTTONDOWN
        | WindowsAndMessaging::WM_XBUTTONUP => {
          let hi_flags = hi_word(flags);
          if (hi_flags & WindowsAndMessaging::XBUTTON1) == WindowsAndMessaging::XBUTTON1 {
            Mouse::Back
          } else {
            Mouse::Forward
          }
        }
        _ => Mouse::Unknown,
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

    Message::Window(WindowMessage::MouseButton {
      button: mouse_code,
      state,
      x,
      y,
      is_double_click,
    })
  }

  pub fn is_key(&self, key: Key, state: KeyState) -> bool {
    matches!(self, Message::Window(WindowMessage::Key { key: k, state: s, .. }) if *k == key && *s == state)
  }

  pub fn is_mouse_button(&self, button: Mouse, state: ButtonState) -> bool {
    matches!(self, Message::Window(WindowMessage::MouseButton { button: b, state: s, .. }) if *b == button && *s == state)
  }

  pub fn is_some(&self) -> bool {
    !matches!(self, Message::None)
  }

  pub fn is_none(&self) -> bool {
    matches!(self, Message::None)
  }
}
