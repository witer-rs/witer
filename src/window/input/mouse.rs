use windows::Win32::UI::WindowsAndMessaging;

use super::state::ButtonState;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u16)]
pub enum Mouse {
  Unknown = 0,
  Left = 1,
  Right = 2,
  Middle = 3,
  Back = 4,
  Forward = 5,
}

impl Mouse {
  pub(crate) fn from_state(id: usize) -> Mouse {
    match id {
      0 => Self::Left,
      1 => Self::Right,
      2 => Self::Middle,
      3 => Self::Back,
      4 => Self::Forward,
      _ => Self::Unknown,
    }
  }
}

pub(crate) fn mouse_button_states(flags: u16) -> [Option<ButtonState>; 5] {
  [
    ButtonState::from_flag(
      flags,
      WindowsAndMessaging::RI_MOUSE_BUTTON_1_DOWN,
      WindowsAndMessaging::RI_MOUSE_BUTTON_1_UP,
    ),
    ButtonState::from_flag(
      flags,
      WindowsAndMessaging::RI_MOUSE_BUTTON_2_DOWN,
      WindowsAndMessaging::RI_MOUSE_BUTTON_2_UP,
    ),
    ButtonState::from_flag(
      flags,
      WindowsAndMessaging::RI_MOUSE_BUTTON_3_DOWN,
      WindowsAndMessaging::RI_MOUSE_BUTTON_3_UP,
    ),
    ButtonState::from_flag(
      flags,
      WindowsAndMessaging::RI_MOUSE_BUTTON_4_DOWN,
      WindowsAndMessaging::RI_MOUSE_BUTTON_4_UP,
    ),
    ButtonState::from_flag(
      flags,
      WindowsAndMessaging::RI_MOUSE_BUTTON_5_DOWN,
      WindowsAndMessaging::RI_MOUSE_BUTTON_5_UP,
    ),
  ]
}
