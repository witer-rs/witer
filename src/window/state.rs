use std::thread::JoinHandle;

use crossbeam::channel::{Receiver, Sender};
use windows::core::HSTRING;

use super::{message::Message, stage::Stage};
use crate::{
  debug::WindowResult,
  window::{
    settings::{ColorMode, Flow, Visibility},
    sync::Response,
    Input,
  },
};

pub struct InternalState {
  pub subclass: Option<usize>,
  pub title: HSTRING,
  pub subtitle: HSTRING,
  pub color_mode: ColorMode,
  pub visibility: Visibility,
  pub flow: Flow,
  pub close_on_x: bool,
  pub stage: Stage,
  pub input: Input,
  pub message: Option<Message>,
  pub thread: Option<JoinHandle<WindowResult<()>>>,
  pub message_receiver: Receiver<Message>,
  pub response_sender: Sender<Response>,
  pub requested_redraw: bool,
}
