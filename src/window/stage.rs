use strum::Display;

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
  Looping,
  Closing,
  Destroyed,
}
