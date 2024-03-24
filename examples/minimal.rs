/*
  This example showcases the minimal amount to code required to open a window.
*/

use std::{
  sync::{Arc, Barrier},
  thread::JoinHandle,
};

use crossbeam::channel::Receiver;
use winit::{
  event::{Event, WindowEvent},
  event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
  platform::windows::EventLoopBuilderExtWindows,
  window::WindowBuilder,
};

struct Window {
  sync_barrier: Arc<Barrier>,
  event_reciever: Receiver<Event<()>>,
  handle: Option<JoinHandle<()>>,
  raw: Arc<winit::window::Window>,
}

impl Drop for Window {
  fn drop(&mut self) {
    self.sync_barrier.wait();
    self.handle.take().unwrap().join().unwrap();
  }
}

impl Window {
  pub fn new() -> Self {
    let sync_barrier = Arc::new(Barrier::new(2));
    let (window_sender, window_reciever) = crossbeam::channel::bounded(0);
    let (event_sender, event_reciever) = crossbeam::channel::bounded(0);

    let sync_barrer_clone = sync_barrier.clone();
    let handle = Some(
      std::thread::Builder::new()
        .name("window".to_owned())
        .spawn(move || {
          let event_loop = EventLoopBuilder::new()
            .with_any_thread(true)
            .build()
            .unwrap();
          let window = Arc::new(WindowBuilder::new().build(&event_loop).unwrap());
          event_loop.set_control_flow(ControlFlow::Poll);
          window_sender.send(window.clone()).unwrap();
          event_loop
            .run(move |event, elwt| {
              if let Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
              } = event
              {
                elwt.exit();
              }
              event_sender.send(event).unwrap();
            })
            .unwrap();
          sync_barrer_clone.wait();
        })
        .unwrap(),
    );

    let raw = window_reciever.recv().unwrap();

    Self {
      sync_barrier,
      event_reciever,
      handle,
      raw,
    }
  }
}

impl Iterator for Window {
  type Item = Event<()>;

  fn next(&mut self) -> Option<Self::Item> {
    self.event_reciever.recv().ok()
  }
}

fn main() {
  // for _ in &witer::Window::builder().build().unwrap() {}
  let window = Window::new();

  for event in window {
    println!("{}: {event:?}", "a window...");
    if let Event::LoopExiting = event {
      break;
    }
  }
}
