pub trait MessageReciever {
  fn dispatch(&self) {}
}

fn main() {}

struct App {}

impl MessageReciever for App {}
