use crate::{Fullscreen, Visibility};

#[derive(Debug, Clone)]
pub struct Style {
  pub visibility: Visibility,
  pub decorations: Visibility,
  pub fullscreen: Option<Fullscreen>,
  pub resizeable: bool,
  pub minimized: bool,
  pub maximized: bool,
  pub focused: bool,
  pub active: bool,
}
