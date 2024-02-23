#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum KeyState {
  Pressed,
  Held { repeat_count: u16 },
  Released,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ButtonState {
  Pressed,
  Released,
}

// #[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
// pub enum ButtonStateRepeatable {
//   Pressed,
//   Held { repeat_count: u32 },
//   Released,
// }
