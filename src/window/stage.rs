#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
  Setup,
  Ready,
  Looping,
  Closing,
  ExitLoop,
  Destroyed,
}
