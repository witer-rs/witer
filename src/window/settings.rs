use super::state::Size;

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Flow {
  #[default]
  Wait,
  Poll,
}

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Visibility {
  #[default]
  Shown,
  Hidden,
}

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Theme {
  #[default]
  Auto,
  Dark,
  Light,
}

impl Default for Size {
  fn default() -> Self {
    Self {
      width: 800,
      height: 600,
    }
  }
}

/// Configures the window to be built.
#[derive(Clone)]
pub struct WindowSettings {
  pub title: String,
  pub size: Size,
  pub flow: Flow,
  pub theme: Theme,
  pub visibility: Visibility,
  pub close_on_x: bool,
}

impl Default for WindowSettings {
  fn default() -> Self {
    let title: String = "Window".into();
    let size = Size::default();
    let flow = Flow::default();
    let color_mode = Theme::default();
    let visibility = Visibility::default();
    let close_on_x = true;

    Self {
      title,
      size,
      flow,
      theme: color_mode,
      visibility,
      close_on_x,
    }
  }
}

impl WindowSettings {
  pub fn with_title(mut self, title: impl Into<String>) -> Self {
    self.title = title.into();
    self
  }

  pub fn with_size(mut self, size: impl Into<Size>) -> Self {
    self.size = size.into();
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

  pub fn with_close_on_x(mut self, close_on_x: bool) -> Self {
    self.close_on_x = close_on_x;
    self
  }
}
