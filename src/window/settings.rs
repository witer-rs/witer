use super::state::{CursorMode, Flow, Fullscreen, Position, Size, Theme, Visibility};

#[derive(Debug, Clone)]
pub enum SizeType {
  /// Size of the entire window including borders.
  Outer(Size),
  /// Size of the client area of the window.
  Inner(Size),
}

impl SizeType {
  pub fn size(&self) -> Size {
    match *self {
      Self::Outer(s) => s,
      Self::Inner(s) => s,
    }
  }

  pub fn outer(size: impl Into<Size>) -> Self {
    Self::Outer(size.into())
  }

  pub fn inner(size: impl Into<Size>) -> Self {
    Self::Inner(size.into())
  }
}

#[derive(Debug, Clone)]
pub struct NoTitle;
#[derive(Debug, Clone)]
pub struct HasTitle(pub String);

#[derive(Debug, Clone)]
pub struct NoSize;
#[derive(Debug, Clone)]
pub struct HasSize(pub SizeType);

/// Configures the window to be built.
#[derive(Debug, Clone)]
pub struct WindowSettings<T, S> {
  pub title: T,
  pub size: S,
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

impl Default for WindowSettings<NoTitle, NoSize> {
  fn default() -> Self {
    let title = NoTitle;
    let size = NoSize;
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

impl<T, S> WindowSettings<T, S> {
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

impl<S> WindowSettings<NoTitle, S> {
  /// You must declare a title
  pub fn with_title(self, title: impl Into<String>) -> WindowSettings<HasTitle, S> {
    WindowSettings {
      size: self.size,
      title: HasTitle(title.into()),
      position: self.position,
      flow: self.flow,
      theme: self.theme,
      visibility: self.visibility,
      decorations: self.decorations,
      resizeable: self.resizeable,
      fullscreen: self.fullscreen,
      cursor_mode: self.cursor_mode,
      close_on_x: self.close_on_x,
    }
  }
}

impl<T> WindowSettings<T, NoSize> {
  /// You can only pick either `with_inner_size` or `with_outer_size`
  pub fn with_inner_size(self, size: impl Into<Size>) -> WindowSettings<T, HasSize> {
    WindowSettings {
      size: HasSize(SizeType::Inner(size.into())),
      title: self.title,
      position: self.position,
      flow: self.flow,
      theme: self.theme,
      visibility: self.visibility,
      decorations: self.decorations,
      resizeable: self.resizeable,
      fullscreen: self.fullscreen,
      cursor_mode: self.cursor_mode,
      close_on_x: self.close_on_x,
    }
  }

  /// You can only pick either `with_inner_size` or `with_outer_size`
  pub fn with_outer_size(self, size: impl Into<Size>) -> WindowSettings<T, HasSize> {
    WindowSettings {
      size: HasSize(SizeType::Outer(size.into())),
      title: self.title,
      position: self.position,
      flow: self.flow,
      theme: self.theme,
      visibility: self.visibility,
      decorations: self.decorations,
      resizeable: self.resizeable,
      fullscreen: self.fullscreen,
      cursor_mode: self.cursor_mode,
      close_on_x: self.close_on_x,
    }
  }
}
