use enumflags2::bitflags;
use strum::EnumIter;

#[bitflags]
#[repr(u8)]
#[derive(EnumIter, Debug, Copy, Clone, Eq, PartialEq)]
pub enum Modifiers {
  Shift = 0b00000001,
  Ctrl = 0b00000010,
  Alt = 0b00000100,
  Windows = 0b00001000,
}

// impl Into<ModifiersState> for Modifiers {
//   fn into(self) -> ModifiersState {
//     match self {
//       Modifiers::Shift   => ModifiersState::SHIFT,
//       Modifiers::Ctrl    => ModifiersState::CTRL,
//       Modifiers::Alt     => ModifiersState::ALT,
//       Modifiers::Windows => ModifiersState::LOGO,
//     }
//   }
// }
