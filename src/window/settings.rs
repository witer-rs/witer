use super::state::{
  CursorMode,
  Flow,
  Fullscreen,
  LogicalSize,
  Position,
  Size,
  Theme,
  Visibility,
};

/// Configures the window to be built.
#[derive(Debug, Clone)]
pub struct WindowSettings {
  pub title: String,
  pub size: Size,
  pub position: Option<Position>,
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
    let title: String = "Window".into();
    let size = LogicalSize::new((800.0, 600.0)).into();
    let position = None;
    let flow = Flow::default();
    let theme = Theme::default();
    let fullscreen = None;
    let cursor_mode = CursorMode::default();
    let visibility = Visibility::default();
    let decorations = Visibility::default();
    let resizeable = true;
    let close_on_x = true;

    Self {
      title,
      size,
      position,
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
  pub fn with_title(mut self, title: impl Into<String>) -> Self {
    self.title = title.into();
    self
  }

  pub fn with_outer_size(mut self, size: impl Into<Size>) -> Self {
    self.size = size.into();
    self
  }

  pub fn with_position(mut self, position: Option<impl Into<Position>>) -> Self {
    self.position = position.map(|p| p.into());
    self
  }

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
