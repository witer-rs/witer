pub use std::sync::Arc;

pub use crate::{
  debug::WindowResult,
  window::{
    self,
    callback::WindowProcedure,
    input::{
      key::Key,
      mouse::Mouse,
      state::{ButtonState, KeyState},
      Input,
    },
    message::{Message, WindowMessage},
    settings::{Flow, Size, Visibility, WindowSettings},
    Window,
  },
};
