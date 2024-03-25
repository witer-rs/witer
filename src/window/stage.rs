#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
  Setup,
  Looping,
  Closing,
  ExitLoop,
}
