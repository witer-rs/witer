use crate::utilities::is_flag_set;

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ButtonState {
  #[default]
  Released,
  Pressed,
}

impl ButtonState {
  pub fn is_pressed(self) -> bool {
    self == ButtonState::Pressed
  }

  pub(crate) fn from_flag(flags: u16, down_flag: u32, up_flag: u32) -> Option<Self> {
    if is_flag_set(flags as u32, down_flag) {
      Some(ButtonState::Pressed)
    } else if is_flag_set(flags as u32, up_flag) {
      Some(ButtonState::Released)
    } else {
      None
    }
  }
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub enum KeyState {
  #[default]
  Released,
  Pressed,
  Held(u16),
}

impl KeyState {
  pub fn is_pressed(self) -> bool {
    // this covers the first moments of the keypress as well
    self != KeyState::Released
  }
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub enum RawKeyState {
  #[default]
  Released,
  Pressed,
}

impl RawKeyState {
  pub fn is_pressed(self) -> bool {
    // this covers the first moments of the keypress as well
    self != RawKeyState::Released
  }

  pub(crate) fn from_bools(down_flag: bool, up_flag: bool) -> Option<Self> {
    if down_flag {
      Some(RawKeyState::Pressed)
    } else if up_flag {
      Some(RawKeyState::Released)
    } else {
      None
    }
  }
}
