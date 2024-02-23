use strum::EnumIter;

#[derive(EnumIter, Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u16)]
pub enum MouseCode {
  Unknown = 0,
  Left = 1,
  Right = 2,
  Middle = 3,
  Back = 4,
  Forward = 5,
}
