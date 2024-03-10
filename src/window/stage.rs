#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
  Looping,
  Closing,
  ExitLoop,
}
