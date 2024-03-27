use super::{
  data::{CursorMode, Flow, Fullscreen, LogicalSize, Position, Size, Theme, Visibility},
  Window,
};
use crate::error::WindowError;

/// Optional onfiguration for the window to be built.
#[derive(Debug, Clone)]
pub struct WindowSettings {
  pub flow: Flow,
  pub theme: Theme,
  pub visibility: Visibility,
  pub decorations: Visibility,
  pub resizeable: bool,
  pub fullscreen: Option<Fullscreen>,
  pub cursor_mode: CursorMode,
  pub close_on_x: bool,
}

impl Default for WindowSettings {
  fn default() -> Self {
    let flow = Flow::default();
    let theme = Theme::default();
    let fullscreen = None;
    let cursor_mode = CursorMode::default();
    let visibility = Visibility::default();
    let decorations = Visibility::default();
    let resizeable = true;
    let close_on_x = true;

    Self {
      flow,
      theme,
      visibility,
      decorations,
      close_on_x,
      fullscreen,
      resizeable,
      cursor_mode,
    }
  }
}

impl WindowSettings {
  pub fn with_flow(mut self, flow: Flow) -> Self {
    self.flow = flow;
    self
  }

  pub fn with_theme(mut self, theme: Theme) -> Self {
    self.theme = theme;
    self
  }

  pub fn with_visibility(mut self, visibility: Visibility) -> Self {
    self.visibility = visibility;
    self
  }

  pub fn with_decorations(mut self, visibility: Visibility) -> Self {
    self.decorations = visibility;
    self
  }

  pub fn with_fullscreen(mut self, fullscreen: Option<Fullscreen>) -> Self {
    self.fullscreen = fullscreen;

    self
  }

  pub fn with_cursor_mode(mut self, cursor_mode: CursorMode) -> Self {
    self.cursor_mode = cursor_mode;
    self
  }

  pub fn with_close_on_x(mut self, close_on_x: bool) -> Self {
    self.close_on_x = close_on_x;
    self
  }

  pub fn with_resizeable(mut self, resizeable: bool) -> Self {
    self.resizeable = resizeable;
    self
  }
}

pub struct WindowBuilder {
  title: String,
  size: Size,
  position: Option<Position>,
  settings: WindowSettings,
}

impl Default for WindowBuilder {
  fn default() -> Self {
    Self {
      title: "Window".into(),
      size: LogicalSize::new(800.0, 500.0).into(),
      position: None,
      settings: WindowSettings::default(),
    }
  }
}

impl From<WindowSettings> for WindowBuilder {
  fn from(settings: WindowSettings) -> Self {
    Self {
      settings,
      ..Default::default()
    }
  }
}

impl WindowBuilder {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn with_title(mut self, title: impl Into<String>) -> Self {
    self.title = title.into();
    self
  }

  /// Relative to the whole window frame, not just the client area
  pub fn with_size(mut self, size: impl Into<Size>) -> Self {
    self.size = size.into();
    self
  }

  pub fn with_position(mut self, position: impl Into<Option<Position>>) -> Self {
    self.position = position.into();
    self
  }

  pub fn with_flow(mut self, flow: Flow) -> Self {
    self.settings = self.settings.with_flow(flow);
    self
  }

  pub fn with_theme(mut self, theme: Theme) -> Self {
    self.settings = self.settings.with_theme(theme);
    self
  }

  pub fn with_visibility(mut self, visibility: Visibility) -> Self {
    self.settings = self.settings.with_visibility(visibility);
    self
  }

  pub fn with_decorations(mut self, visibility: Visibility) -> Self {
    self.settings = self.settings.with_decorations(visibility);
    self
  }

  pub fn with_fullscreen(mut self, fullscreen: Option<Fullscreen>) -> Self {
    self.settings = self.settings.with_fullscreen(fullscreen);

    self
  }

  pub fn with_cursor_mode(mut self, cursor_mode: CursorMode) -> Self {
    self.settings = self.settings.with_cursor_mode(cursor_mode);
    self
  }

  pub fn with_close_on_x(mut self, close_on_x: bool) -> Self {
    self.settings = self.settings.with_close_on_x(close_on_x);
    self
  }

  pub fn with_resizeable(mut self, resizeable: bool) -> Self {
    self.settings = self.settings.with_resizeable(resizeable);
    self
  }

  pub fn build(self) -> Result<Window, WindowError> {
    Window::new(self.title, self.size, self.position, self.settings)
  }
}
