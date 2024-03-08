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
