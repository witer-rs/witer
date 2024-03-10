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
