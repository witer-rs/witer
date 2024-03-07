use std::{
  sync::{Arc, Condvar, Mutex},
  thread::JoinHandle,
};

use windows::core::HSTRING;

use super::{message::Message, stage::Stage};
use crate::{
  debug::WindowResult,
  window::{
    settings::{ColorMode, Flow, Visibility},
    Input,
  },
};

pub struct InternalState {
  pub thread: Option<JoinHandle<WindowResult<()>>>,
  pub subclass: Option<usize>,
  pub title: HSTRING,
  pub subtitle: HSTRING,
  pub color_mode: ColorMode,
  pub visibility: Visibility,
  pub flow: Flow,
  pub close_on_x: bool,
  pub stage: Stage,
  pub input: Input,
  pub requested_redraw: bool,
  pub new_message: Arc<(Mutex<bool>, Condvar)>,
  pub next_frame: Arc<(Mutex<bool>, Condvar)>,
  pub next_message: Arc<Mutex<Message>>,
}
