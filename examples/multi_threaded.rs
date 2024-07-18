#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use std::{
  sync::{mpsc::Receiver, Arc, Barrier},
  thread::JoinHandle,
  time::{Duration, Instant},
};

use egui_wgpu::ScreenDescriptor;
use foxy_time::{Time, TimeSettings};
use witer::{error::*, prelude::*};

use self::common::egui::EguiRenderer;

mod common;

/*
  This example showcases how to render a triangle using WGPU on a separate thread while
  staying in lockstep with the window in key scenarios. This is done to prevent desync
  issues such as input lag or swapchain losses.
*/

fn main() -> Result<(), WindowError> {
  common::init_log(env!("CARGO_CRATE_NAME"));

  let window = Window::builder()
    .with_title("Threaded Example")
    .with_flow(Flow::Poll)
    .with_visibility(Visibility::Hidden)
    .build()?;

  let (message_sender, message_receiver) = std::sync::mpsc::channel();
  let sync_barrier = Arc::new(Barrier::new(2));
  let handle = app_loop(window.clone(), message_receiver, sync_barrier.clone());

  for message in &window {
    if message.is_key(Key::F11, KeyState::Pressed) {
      let fullscreen = window.fullscreen();
      match fullscreen {
        Some(Fullscreen::Borderless) => {
          window.set_fullscreen(None);
          window.set_cursor_mode(CursorMode::Normal);
          window.set_cursor_visibility(Visibility::Shown);
        }
        None => {
          window.set_fullscreen(Some(Fullscreen::Borderless));
          window.set_cursor_mode(CursorMode::Confined);
          window.set_cursor_visibility(Visibility::Hidden);
        }
      }
    }

    if !message.is_empty() {
      message_sender.send(message).unwrap();
    }

    sync_barrier.wait();
  }

  handle.join().unwrap();

  Ok(())
}

fn app_loop(
  window: Window,
  message_receiver: Receiver<Message>,
  sync_barrier: Arc<Barrier>,
) -> JoinHandle<()> {
  std::thread::Builder::new()
    .name("app".to_owned())
    .spawn(move || {
      let mut app = App::new(&window);

      loop {
        let mut message = message_receiver.try_recv().ok();

        let consumed = if let Some(message) = &message {
          app.egui_renderer.handle_input(&window, message).consumed
        } else {
          false
        };

        if consumed {
          message = None;
        }

        match &message {
          Some(Message::Resized(new_size)) => {
            app.resize(*new_size);
          }
          Some(Message::Loop(LoopMessage::Exit)) => break,
          _ => (),
        }

        app.update(&window);
        app.draw(&window);

        sync_barrier.wait();
      }
    })
    .unwrap()
}

struct App {
  last_render_time: Instant,
  time: Time,

  frame_count: u32,
  is_revealed: bool,

  surface: wgpu::Surface<'static>,
  device: wgpu::Device,
  queue: wgpu::Queue,
  config: wgpu::SurfaceConfiguration,
  size: PhysicalSize,
  render_pipeline: wgpu::RenderPipeline,

  egui_renderer: EguiRenderer,
  text: String,
}

impl App {
  fn new(window: &Window) -> Self {
    pollster::block_on(async {
      let last_render_time = Instant::now();
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

      let shader =
        device.create_shader_module(wgpu::include_wgsl!("common/simple_shader.wgsl"));

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
            compilation_options: wgpu::PipelineCompilationOptions::default(),
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
            compilation_options: wgpu::PipelineCompilationOptions::default(),
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
        last_render_time,
        time,
        frame_count: 0,
        is_revealed: false,
        surface,
        device,
        queue,
        config,
        size,
        egui_renderer,
        render_pipeline,
        text: String::new(),
      }
    })
  }

  fn resize(&mut self, new_size: PhysicalSize) {
    if new_size.is_any_zero() {
      return;
    }

    self.size = new_size;
    self.config.width = new_size.width;
    self.config.height = new_size.height;
    self.surface.configure(&self.device, &self.config);
  }

  fn update(&mut self, _window: &Window) {
    // info!("update");
    self.time.update();
    while self.time.should_do_tick_unchecked() {
      self.time.tick();
    }

    // if !matches!(
    //   message,
    //   Some(
    //     Message::Paint
    //       | Message::Loop(..)
    //       | Message::RawInput(..)
    //       | Message::CursorMove { .. }
    //   ) | None
    // ) {
    //   tracing::info!("{message:?}");
    // }
  }

  fn draw(&mut self, window: &Window) {
    if window.inner_size().is_any_zero() {
      return;
    }

    let now = Instant::now();
    let elapsed = now.duration_since(self.last_render_time);
    if elapsed >= Duration::from_secs_f64(0.20) {
      let fps = format!(" | Avg FPS: {:.0}", 1.0 / self.time.average_delta_secs());
      window.set_subtitle(fps);
      self.last_render_time = now;
    }

    match (self.is_revealed, self.frame_count) {
      (false, 1) => {
        Self::center_window(window);
        window.set_visibility(Visibility::Shown);
        self.is_revealed = true;
      }
      (false, _) => self.frame_count = self.frame_count.wrapping_add(1),
      _ => (),
    };

    let output = match self.surface.get_current_texture() {
      Ok(output) => output,
      Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
        // info!("wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated");
        self.resize(window.inner_size());
        return;
      }
      Err(error) => {
        tracing::error!("{error}");
        return;
      }
    };

    // info!("draw");

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
          // .default_open(false)
          // .default_size((50.0, 50.0))
          // .resizable(false)
          // .anchor(egui::Align2::LEFT_BOTTOM, (5.0, -5.0))
          .show(ctx, |ctx| {
            if ctx.button("Test").clicked() {
              tracing::debug!("PRESSED");
            }
            ctx.text_edit_multiline(&mut self.text);
          });
      },
    );

    self.queue.submit(std::iter::once(encoder.finish()));
    output.present();
  }

  fn center_window(window: &Window) {
    let window_size = window.outer_size();
    let monitor_pos = window.current_monitor().position();
    let monitor_size = window.current_monitor().size();
    let monitor_center = PhysicalPosition {
      x: monitor_pos.x + (monitor_size.width as f32 * 0.5) as i32,
      y: monitor_pos.y + (monitor_size.height as f32 * 0.5) as i32,
    };
    let adjusted_position = PhysicalPosition {
      x: monitor_center.x - (window_size.width as f32 * 0.5) as i32,
      y: monitor_center.y - (window_size.height as f32 * 0.5) as i32,
    };
    window.set_outer_position(adjusted_position.into());
  }
}
