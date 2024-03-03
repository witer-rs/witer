#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use std::{
  sync::Arc,
  time::{Duration, Instant},
};

use ezwin::{prelude::*, window::settings::Size};
use foxy_time::{Time, TimeSettings};
use wgpu::PresentMode;

fn main() -> WindowResult<()> {
  let window = Window::new::<App>(
    WindowSettings::default()
      .with_flow(Flow::Poll)
      .with_title("Easy Window")
      .with_size((800, 600)),
  )?;

  Window::run(&window);

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
  fn resize(&mut self, new_size: Size) {
    if new_size.width > 0 && new_size.height > 0 {
      self.size = new_size;
      self.config.width = new_size.width as u32;
      self.config.height = new_size.height as u32;
      self.surface.configure(&self.device, &self.config);
    }
  }

  fn draw(&mut self, window: &Arc<Window>) {
    self.time.update();
    while self.time.should_do_tick_unchecked() {
      self.time.tick();
    }

    let now = Instant::now();
    let elapsed = now.duration_since(self.last_time);
    if elapsed >= Duration::from_secs_f64(0.20) {
      let title = format!(" | FPS: {:.1}", 1.0 / self.time.average_delta_secs());
      window.set_subtitle(title);
      self.last_time = now;
    }

    let output = match self.surface.get_current_texture() {
      Ok(output) => output,
      Err(wgpu::SurfaceError::Lost) => {
        self.resize(window.inner_size());
        return;
      }
      Err(error) => {
        eprintln!("{error}");
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

impl WindowProcedure for App {
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

  fn procedure(&mut self, window: &Arc<Window>, message: Message) {
    if let Message::Window(WindowMessage::Resized(..)) = message {
      self.resize(window.inner_size());
    }

    if matches!(
      message,
      Message::Window(WindowMessage::Key { .. } | WindowMessage::MouseButton { .. })
    ) {
      println!("{message:?}");
    }

    match message {
      Message::Window(WindowMessage::Draw) => self.draw(window),
      _ => window.request_redraw(),
    }
  }
}
