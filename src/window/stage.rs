use strum::Display;

// KEEP THESE SMALL since you need to clone them for each iteration
#[derive(Display, Clone, Copy)]
pub enum Stage {
  Looping,
  Exiting,
  ExitLoop,
}
