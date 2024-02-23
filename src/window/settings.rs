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
pub enum ColorMode {
  #[default]
  Dark,
  Light,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Size {
  pub width: i32,
  pub height: i32,
}

impl Default for Size {
  fn default() -> Self {
    Self {
      width: 800,
      height: 600,
    }
  }
}

impl From<(i32, i32)> for Size {
  fn from(value: (i32, i32)) -> Self {
    Self {
      width: value.0,
      height: value.1,
    }
  }
}

#[derive(Clone)]
pub struct WindowSettings {
  pub title: String,
  pub size: Size,
  pub flow: Flow,
  pub color_mode: ColorMode,
  pub visibility: Visibility,
  pub close_on_x: bool,
}

impl Default for WindowSettings {
  fn default() -> Self {
    let title: String = "Window".into();
    let size = Size::default();
    let flow = Flow::default();
    let color_mode = ColorMode::default();
    let visibility = Visibility::default();
    let close_on_x = true;

    Self {
      title,
      size,
      flow,
      color_mode,
      visibility,
      close_on_x,
    }
  }
}

impl WindowSettings {
  pub fn with_title(mut self, title: &'static str) -> Self {
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

  pub fn with_color_mode(mut self, color_mode: ColorMode) -> Self {
    self.color_mode = color_mode;
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
