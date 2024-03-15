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

  pub(crate) fn from_flag(
    flags: u16,
    down_flag: u16,
    up_flag: u16,
    was_down_flag: u16,
    repeat_count: u16,
  ) -> Option<Self> {
    if is_flag_set(flags as u32, was_down_flag as u32) {
      Some(KeyState::Held(repeat_count))
    } else if is_flag_set(flags as u32, down_flag as u32) {
      Some(KeyState::Pressed)
    } else if is_flag_set(flags as u32, up_flag as u32) {
      Some(KeyState::Released)
    } else {
      None
    }
  }
}
