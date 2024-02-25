use strum::Display;

// KEEP THESE SMALL since you need to clone them for each iteration
#[derive(Debug, Display, Clone, Copy)]
pub enum Stage {
  Looping,
  Exiting,
  ExitLoop,
}
