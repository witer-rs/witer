#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use std::time::{Duration, Instant};

use egui_wgpu::ScreenDescriptor;
use foxy_time::{Time, TimeSettings};
use witer::{compat::egui::EventResponse, error::*, prelude::*};

use self::common::egui::EguiRenderer;

mod common;

/*
  This example showcases a simple app rendering a blank screen using WGPU.
*/

fn main() -> Result<(), WindowError> {
  common::init_log(env!("CARGO_CRATE_NAME"));

  // start hidden to prevent first frame white flash
  let window = Window::builder()
    .with_title("wgpu Example")
    .with_flow(Flow::Poll)
    .with_visibility(Visibility::Hidden)
    .build()?;

  let mut app = App::new(&window);

  for message in &window {
    if message.is_key(Key::F11, KeyState::Pressed) {
      let fullscreen = window.fullscreen();
      match fullscreen {
        Some(Fullscreen::Borderless) => window.set_fullscreen(None),
        None => window.set_fullscreen(Some(Fullscreen::Borderless)),
      }
    }

    match app.frame_count {
      0..=2 => app.frame_count = app.frame_count.saturating_add(1),
      3 => {
        window.set_visibility(Visibility::Shown);
        app.frame_count = app.frame_count.saturating_add(1);
      }
      _ => (),
    }

    let response = app.egui_renderer.handle_input(&window, &message);
    let message = if response.consumed {
      Message::Loop(LoopMessage::Empty)
    } else {
      message
    };

    match &message {
      Message::Resized(..) => app.resize(window.inner_size()),
      Message::Paint => (),
      _ => (),
    }

    app.update(&window, &message, &response);
    app.draw(&window, &response);
  }

  Ok(())
}

struct App {
  last_time: Instant,
  time: Time,

  surface: wgpu::Surface<'static>,
  device: wgpu::Device,
  queue: wgpu::Queue,
  config: wgpu::SurfaceConfiguration,
  size: PhysicalSize,
  render_pipeline: wgpu::RenderPipeline,

  frame_count: u32,

  egui_renderer: EguiRenderer,
}

impl App {
  fn new(window: &Window) -> Self {
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
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::AutoNoVsync,
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
      };
      surface.configure(&device, &config);

      let shader = device.create_shader_module(wgpu::include_wgsl!("common/shader.wgsl"));

      let egui_renderer =
        EguiRenderer::new(&device, wgpu::TextureFormat::Bgra8UnormSrgb, None, 1, window);

      let render_pipeline_layout =
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
          label: Some("Render Pipeline Layout"),
          bind_group_layouts: &[],
          push_constant_ranges: &[],
        });

      let render_pipeline =
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
          label: Some("Render Pipeline"),
          layout: Some(&render_pipeline_layout),
          vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main", // 1.
            buffers: &[],           // 2.
          },
          fragment: Some(wgpu::FragmentState {
            // 3.
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
              // 4.
              format: config.format,
              blend: Some(wgpu::BlendState::REPLACE),
              write_mask: wgpu::ColorWrites::ALL,
            })],
          }),
          primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList, // 1.
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw, // 2.
            cull_mode: Some(wgpu::Face::Back),
            // Setting this to anything other than Fill requires
            // Features::NON_FILL_POLYGON_MODE
            polygon_mode: wgpu::PolygonMode::Fill,
            // Requires Features::DEPTH_CLIP_CONTROL
            unclipped_depth: false,
            // Requires Features::CONSERVATIVE_RASTERIZATION
            conservative: false,
          },
          depth_stencil: None, // 1.
          multisample: wgpu::MultisampleState {
            count: 1,                         // 2.
            mask: !0,                         // 3.
            alpha_to_coverage_enabled: false, // 4.
          },
          multiview: None, // 5.
        });

      Self {
        last_time,
        time,
        surface,
        device,
        queue,
        config,
        size,
        render_pipeline,
        frame_count: 0,
        egui_renderer,
      }
    })
  }

  fn resize(&mut self, new_size: PhysicalSize) {
    if !new_size.is_any_zero() {
      self.size = new_size;
      self.config.width = new_size.width;
      self.config.height = new_size.height;
      self.surface.configure(&self.device, &self.config);
    }
  }

  fn update(&mut self, _window: &Window, message: &Message, _response: &EventResponse) {
    self.time.update();
    while self.time.should_do_tick_unchecked() {
      self.time.tick();
    }

    if !matches!(
      message,
      Message::Paint
        | Message::Loop(..)
        | Message::RawInput(..)
        | Message::CursorMove { .. }
    ) {
      tracing::info!("{message:?}");
    }
  }

  fn draw(&mut self, window: &Window, _response: &EventResponse) {
    let size = window.inner_size();
    if size.width <= 1 || size.height <= 1 {
      return;
    }

    let now = Instant::now();
    let elapsed = now.duration_since(self.last_time);
    if elapsed >= Duration::from_secs_f64(0.20) {
      let title = format!(" | U: {:.1}", 1.0 / self.time.average_delta_secs(),);
      window.set_subtitle(title);
      self.last_time = now;
    }

    let output = match self.surface.get_current_texture() {
      Ok(output) => output,
      Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
        self.resize(window.inner_size());
        return;
      }
      Err(error) => {
        tracing::error!("{error}");
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
      let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Render Pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
          view: &view,
          resolve_target: None,
          ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color {
              r: 0.1,
              g: 0.3,
              b: 0.7,
              a: 1.0,
            }),
            store: wgpu::StoreOp::Store,
          },
        })],
        depth_stencil_attachment: None,
        occlusion_query_set: None,
        timestamp_writes: None,
      });
      render_pass.set_pipeline(&self.render_pipeline); // 2.
      render_pass.draw(0..3, 0..1); // 3.
    }

    let screen_descriptor = ScreenDescriptor {
      size_in_pixels: [self.config.width, self.config.height],
      pixels_per_point: window.scale_factor() as f32,
    };

    self.egui_renderer.draw(
      &self.device,
      &self.queue,
      &mut encoder,
      window,
      &view,
      screen_descriptor,
      |ctx| {
        egui::Window::new("Settings")
          .default_open(false)
          .default_size((50.0, 50.0))
          .resizable(false)
          .anchor(egui::Align2::LEFT_BOTTOM, (5.0, -5.0))
          .show(ctx, |ctx| {
            if ctx.button("Test").clicked() {
              tracing::debug!("PRESSED");
            }
          });
      },
    );

    self.queue.submit(std::iter::once(encoder.finish()));
    output.present();
  }
}
