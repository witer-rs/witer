#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use std::{
  sync::Barrier,
  thread::JoinHandle,
  time::{Duration, Instant},
};

use crossbeam::channel::Receiver;
use ezwin::prelude::*;
use foxy_time::{Time, TimeSettings};
use tracing::{error, info, Level};
use wgpu::PresentMode;

fn main() -> WindowResult<()> {
  tracing_subscriber::fmt()
    .with_max_level(Level::INFO)
    .with_thread_names(true)
    .init();

  let settings = WindowSettings::default()
    .with_flow(Flow::Poll)
    .with_title("Threaded Example")
    .with_size((800, 600));

  let window = Arc::new(Window::new(settings)?);

  let (message_sender, message_receiver) = crossbeam::channel::unbounded();
  let sync_barrier = Arc::new(Barrier::new(2));
  let handle = app_loop(window.clone(), message_receiver, sync_barrier.clone());

  for message in window.as_ref() {
    if message.is_key(Key::F11, KeyState::Pressed) {
      let fullscreen = window.fullscreen();
      match fullscreen {
        Some(Fullscreen::Borderless) => window.set_fullscreen(None),
        None => window.set_fullscreen(Some(Fullscreen::Borderless)),
      }
    }

    let should_sync = matches!(message, Message::Window(WindowMessage::Resized(..)));

    if message.is_some() {
      message_sender.try_send(message).unwrap();
    }

    if should_sync {
      sync_barrier.wait();
    }
  }

  handle.join().unwrap();

  Ok(())
}

struct App {
  last_time: Instant,
  time: Time,

  surface: wgpu::Surface<'static>,
  device: wgpu::Device,
  queue: wgpu::Queue,
  config: wgpu::SurfaceConfiguration,
  size: Size,
}

impl App {
  fn new(window: &Arc<Window>) -> Self {
    pollster::block_on(async {
      let last_time = Instant::now();
      let time = TimeSettings::default().build();
      let size = window.inner_size();

      let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
      });

      let surface = instance.create_surface(window.clone()).unwrap();

      let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
          power_preference: wgpu::PowerPreference::HighPerformance,
          compatible_surface: Some(&surface),
          force_fallback_adapter: false,
        })
        .await
        .unwrap();

      let (device, queue) = adapter
        .request_device(
          &wgpu::DeviceDescriptor {
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            label: None,
          },
          None,
        )
        .await
        .unwrap();

      let surface_caps = surface.get_capabilities(&adapter);
      let surface_format = surface_caps
        .formats
        .iter()
        .copied()
        .find(|f| f.is_srgb())
        .unwrap_or(surface_caps.formats[0]);
      let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: size.width as u32,
        height: size.height as u32,
        present_mode: PresentMode::AutoNoVsync,
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
      };
      surface.configure(&device, &config);

      Self {
        last_time,
        time,
        surface,
        device,
        queue,
        config,
        size,
      }
    })
  }

  fn resize(&mut self, new_size: Size) {
    if new_size.width > 0 && new_size.height > 0 {
      self.size = new_size;
      self.config.width = new_size.width as u32;
      self.config.height = new_size.height as u32;
      self.surface.configure(&self.device, &self.config);
    }
  }

  fn update(&mut self, _window: &Window) {
    self.time.update();
    while self.time.should_do_tick_unchecked() {
      self.time.tick();
    }

    let now = Instant::now();
    let elapsed = now.duration_since(self.last_time);
    if elapsed >= Duration::from_secs_f64(0.20) {
      let fps = format!("Update FPS: {:.1}", 1.0 / self.time.average_delta_secs(),);
      info!("{fps}");
      self.last_time = now;
    }
  }

  fn draw(&mut self, window: &Window) {
    let size = window.inner_size();
    if size.width <= 1 || size.height <= 1 {
      return;
    }

    let output = match self.surface.get_current_texture() {
      Ok(output) => output,
      Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
        self.resize(window.inner_size());
        return;
      }
      Err(error) => {
        error!("{error}");
        return;
      }
    };

    let view = output
      .texture
      .create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder =
      self
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
          label: Some("Render Encoder"),
        });
    {
      let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Render Pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
          view: &view,
          resolve_target: None,
          ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color {
              r: 0.1,
              g: 0.2,
              b: 0.3,
              a: 1.0,
            }),
            store: wgpu::StoreOp::Store,
          },
        })],
        depth_stencil_attachment: None,
        occlusion_query_set: None,
        timestamp_writes: None,
      });
    }

    self.queue.submit(std::iter::once(encoder.finish()));
    output.present();
  }
}

fn app_loop(
  window: Arc<Window>,
  message_receiver: Receiver<Message>,
  sync_barrier: Arc<Barrier>,
) -> JoinHandle<()> {
  std::thread::Builder::new()
    .name("app".to_owned())
    .spawn(move || {
      let mut app = App::new(&window);

      loop {
        let message = message_receiver.try_recv().ok();

        if let Some(Message::Window(
          WindowMessage::MouseButton { .. } | WindowMessage::Key { .. },
        )) = message
        {
          info!("{message:?}");
        }

        app.update(&window);

        match &message {
          Some(Message::Window(WindowMessage::Resized(..))) => {
            app.resize(window.inner_size());
            sync_barrier.wait();
          }
          Some(Message::Window(WindowMessage::Paint)) => {
            app.draw(&window);
          }
          Some(Message::Wait) => window.request_redraw(),
          _ => (),
        }

        if let Some(Message::ExitLoop) = message {
          break;
        }
      }
    })
    .unwrap()
}
