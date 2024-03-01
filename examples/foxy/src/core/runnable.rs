use ezwin::prelude::Message;

use super::{builder::FoxySettings, framework::Framework, state::Foxy, FoxyResult};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Flow {
  Exit,
  Continue,
}

#[allow(unused)]
pub trait Runnable {
  fn new(foxy: &mut Foxy) -> Self;

  fn start(&mut self, foxy: &mut Foxy) {}

  fn fixed_update(&mut self, foxy: &mut Foxy, message: &Message) {}

  fn update(&mut self, foxy: &mut Foxy, message: &Message) {}

  fn late_update(&mut self, foxy: &mut Foxy, message: &Message) {}

  fn egui(&mut self, foxy: &Foxy, egui: &egui::Context) {}

  fn stop(&mut self, foxy: &mut Foxy) -> Flow {
    Flow::Exit
  }

  fn delete(mut self)
  where
    Self: Sized,
  {
  }

  fn settings() -> FoxySettings {
    FoxySettings::default()
  }

  /// ## You don't want to override this method. It's implemented as a simple wrapper around the Framework::run() method.
  fn run() -> FoxyResult<()>
  where
    Self: Sized,
  {
    Framework::new::<Self>(Self::settings())?.run()
  }
}
