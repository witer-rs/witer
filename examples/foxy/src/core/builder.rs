use ezwin::prelude::WindowSettings;
use foxy_time::TimeSettings;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[allow(unused)]
pub enum Polling {
  Poll,
  #[default]
  Wait,
}

#[derive(Debug, Default)]
pub enum DebugInfo {
  Shown,
  #[default]
  Hidden,
}

#[derive(Default)]
pub struct FoxySettings {
  pub time: TimeSettings,
  pub window: WindowSettings,
  pub debug_info: DebugInfo,
}

impl FoxySettings {
  pub fn with_window(mut self, window: WindowSettings) -> Self {
    self.window = window;
    self
  }

  pub fn with_time(mut self, time: TimeSettings) -> Self {
    self.time = time;
    self
  }

  pub fn with_debug_info(mut self, debug_info: DebugInfo) -> Self {
    self.debug_info = debug_info;
    self
  }
}
