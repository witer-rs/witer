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
    Input::{
      self,
      KeyboardAndMouse::{MapVirtualKeyW, MAPVK_VSC_TO_VK_EX, VIRTUAL_KEY},
      HRAWINPUT,
      RID_DEVICE_INFO_TYPE,
    },
    WindowsAndMessaging::{self, SetWindowPos, WINDOWPOS},
  },
};

use super::{
  input::{
    mouse::{mouse_button_states, Mouse},
    state::RawKeyState,
  },
  state::{PhysicalPosition, PhysicalSize, Position, Size},
};
use crate::{
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

#[derive(Debug, PartialEq, Clone)]
pub enum Message {
  Loop(LoopMessage),
  RawInput(RawInputMessage),
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
    position: PhysicalPosition,
    is_double_click: bool,
  },
  Cursor(Position),
  Scroll {
    delta_x: f32,
    delta_y: f32,
  },
  Resized(Size),
  Moved(Position),
  Command,
  SystemCommand,
  Focus(bool),
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
  Keyboard { key: Key, state: RawKeyState },
  MouseButton { button: Mouse, state: ButtonState },
  MouseMotion { delta_x: f32, delta_y: f32 },
}

impl Message {
  pub fn collect(
    hwnd: HWND,
    message: u32,
    w_param: WPARAM,
    l_param: LPARAM,
  ) -> Option<Vec<Self>> {
    let mut out = Vec::with_capacity(0);
    out.reserve_exact(1);
    match message {
      WindowsAndMessaging::WM_CLOSE => out.push(Message::CloseRequested),
      WindowsAndMessaging::WM_PAINT => out.push(Message::Paint),
      WindowsAndMessaging::WM_SIZE => {
        let width = lo_word(l_param.0 as u32) as i32;
        let height = hi_word(l_param.0 as u32) as i32;

        out
          .push(Message::Resized(PhysicalSize::new((width as u32, height as u32)).into()))
      }
      WindowsAndMessaging::WM_WINDOWPOSCHANGED => {
        let window_pos = unsafe { &*(l_param.0 as *const WINDOWPOS) };

        out.push(Message::Moved(
          PhysicalPosition::new((window_pos.x, window_pos.y)).into(),
        ))
      }
      WindowsAndMessaging::WM_SETFOCUS => out.push(Message::Focus(true)),
      WindowsAndMessaging::WM_KILLFOCUS => out.push(Message::Focus(false)),
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
                out.push(Message::RawInput(RawInputMessage::MouseMotion {
                  delta_x: x,
                  delta_y: y,
                }));
              }
            }

            for (id, state) in mouse_button_states(button_flags).iter().enumerate() {
              if let Some(state) = *state {
                let button = Mouse::from_state(id);
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
      | WindowsAndMessaging::WM_SYSKEYUP => out.push(Self::new_keyboard_message(l_param)),
      WindowsAndMessaging::WM_MOUSEMOVE => {
        let x = signed_lo_word(l_param.0 as i32) as i32;
        let y = signed_hi_word(l_param.0 as i32) as i32;

        out.push(Message::Cursor(PhysicalPosition::new((x, y)).into()));
      }
      WindowsAndMessaging::WM_MOUSEWHEEL => {
        let delta = signed_hi_word(w_param.0 as i32) as f32
          / WindowsAndMessaging::WHEEL_DELTA as f32;
        out.push(Message::Scroll {
          delta_x: 0.0,
          delta_y: delta,
        });
      }
      WindowsAndMessaging::WM_MOUSEHWHEEL => {
        let delta = signed_hi_word(w_param.0 as i32) as f32
          / WindowsAndMessaging::WHEEL_DELTA as f32;
        out.push(Message::Scroll {
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

    let position = PhysicalPosition::new((x as i32, y as i32));

    Message::MouseButton {
      button: mouse_code,
      state,
      position,
      is_double_click,
    }
  }

  pub fn is_key(&self, key: Key, state: KeyState) -> bool {
    matches!(self, Message::Key { key: k, state: s, .. } if *k == key && *s == state)
  }

  pub fn is_mouse_button(&self, button: Mouse, state: ButtonState) -> bool {
    matches!(self, Message::MouseButton { button: b, state: s, .. } if *b == button && *s == state)
  }

  pub fn is_empty(&self) -> bool {
    matches!(self, Message::Loop(LoopMessage::Empty))
  }
}
