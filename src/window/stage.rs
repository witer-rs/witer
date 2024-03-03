use strum::Display;

// KEEP THESE SMALL since you need to clone them for each iteration
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
  Ready,
  Looping,
  Closing,
  Destroyed,
}
