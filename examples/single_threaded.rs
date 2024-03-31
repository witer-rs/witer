#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use std::time::{Duration, Instant};

use egui_wgpu::ScreenDescriptor;
use foxy_time::{Time, TimeSettings};
use wgpu::util::DeviceExt;
use witer::{compat::egui::EventResponse, error::*, prelude::*};

use self::common::{
  camera::{Camera, CameraController, CameraUniform},
  egui::EguiRenderer,
  frame::FrameUniform,
  model::ModelUniform,
  window::WindowUniform,
};

mod common;

/*
  This example showcases a simple app rendering a scene using WGPU.
*/

fn main() -> Result<(), WindowError> {
  common::init_log(env!("CARGO_CRATE_NAME"));

  // start hidden to prevent first frame white flash
  let window = Window::builder()
    .with_title("wgpu Example")
    .with_flow(Flow::Poll)
    .with_visibility(Visibility::Hidden)
    .build()?;

  let mut app = App::new(window.clone());

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

    // if let Message::Resized(new_size) = &message {
    //   app.resize(*new_size);
    // }

    app.update(&message, &response);
    match app.draw(&response) {
      Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
        app.resize(window.inner_size());
      }
      Err(error) => {
        tracing::error!("{error}");
      }
      _ => (),
    };
  }

  Ok(())
}

struct App {
  last_time: Instant,
  time: Time,

  window: Window,
  surface: wgpu::Surface<'static>,
  device: wgpu::Device,
  queue: wgpu::Queue,
  config: wgpu::SurfaceConfiguration,
  size: PhysicalSize,
  render_pipeline: wgpu::RenderPipeline,

  frame_count: u32,
  fps: f32,

  egui_renderer: EguiRenderer,

  window_uniform: WindowUniform,
  window_buffer: wgpu::Buffer,
  window_bind_group: wgpu::BindGroup,

  frame_uniform: FrameUniform,
  frame_buffer: wgpu::Buffer,
  frame_bind_group: wgpu::BindGroup,

  camera: Camera,
  camera_controller: CameraController,
  camera_uniform: CameraUniform,
  camera_buffer: wgpu::Buffer,
  camera_bind_group: wgpu::BindGroup,

  model_uniform: ModelUniform,
  model_buffer: wgpu::Buffer,
  model_bind_group: wgpu::BindGroup,
}

impl App {
  fn new(window: Window) -> Self {
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
        EguiRenderer::new(&device, wgpu::TextureFormat::Bgra8UnormSrgb, None, 1, &window);

      let frame_uniform = FrameUniform::new();

      let frame_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Frame Buffer"),
        contents: bytemuck::cast_slice(&[frame_uniform]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
      });

      let frame_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
          entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
              ty: wgpu::BufferBindingType::Uniform,
              has_dynamic_offset: false,
              min_binding_size: None,
            },
            count: None,
          }],
          label: Some("frame_bind_group_layout"),
        });

      let frame_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &frame_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
          binding: 0,
          resource: frame_buffer.as_entire_binding(),
        }],
        label: Some("frame_bind_group"),
      });

      let mut window_uniform = WindowUniform::new();
      window_uniform.update(window.inner_size());

      let window_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Window Buffer"),
        contents: bytemuck::cast_slice(&[window_uniform]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
      });

      let window_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
          entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
              ty: wgpu::BufferBindingType::Uniform,
              has_dynamic_offset: false,
              min_binding_size: None,
            },
            count: None,
          }],
          label: Some("window_bind_group_layout"),
        });

      let window_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &window_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
          binding: 0,
          resource: window_buffer.as_entire_binding(),
        }],
        label: Some("window_bind_group"),
      });

      let camera = Camera {
        // position the camera 2 units back
        // +z is out of the screen
        eye: (0.0, 0.0, 2.0).into(),
        // have it look at the origin
        target: (0.0, 0.0, 0.0).into(),
        // which way is "up"
        up: cgmath::Vector3::unit_y(),
        aspect: config.width as f32 / config.height as f32,
        fovy: 45.0,
        znear: 0.1,
        zfar: 100.0,
      };

      let camera_controller = CameraController::new(1.0);
      let mut camera_uniform = CameraUniform::new();
      camera_uniform.update_view_proj(&camera);

      let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Camera Buffer"),
        contents: bytemuck::cast_slice(&[camera_uniform]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
      });

      let camera_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
          entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
              ty: wgpu::BufferBindingType::Uniform,
              has_dynamic_offset: false,
              min_binding_size: None,
            },
            count: None,
          }],
          label: Some("camera_bind_group_layout"),
        });

      let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &camera_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
          binding: 0,
          resource: camera_buffer.as_entire_binding(),
        }],
        label: Some("camera_bind_group"),
      });

      let model_uniform = ModelUniform::new();

      let model_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Model Buffer"),
        contents: bytemuck::cast_slice(&[model_uniform]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
      });

      let model_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
          entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
              ty: wgpu::BufferBindingType::Uniform,
              has_dynamic_offset: false,
              min_binding_size: None,
            },
            count: None,
          }],
          label: Some("model_bind_group_layout"),
        });

      let model_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &model_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
          binding: 0,
          resource: model_buffer.as_entire_binding(),
        }],
        label: Some("model_bind_group"),
      });

      let render_pipeline_layout =
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
          label: Some("Render Pipeline Layout"),
          bind_group_layouts: &[
            &window_bind_group_layout,
            &frame_bind_group_layout,
            &camera_bind_group_layout,
            &model_bind_group_layout,
          ],
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
        window,
        surface,
        device,
        queue,
        config,
        size,
        render_pipeline,
        frame_count: 0,
        fps: 0.0,
        egui_renderer,
        window_uniform,
        window_buffer,
        window_bind_group,
        frame_uniform,
        frame_buffer,
        frame_bind_group,
        camera,
        camera_controller,
        camera_uniform,
        camera_buffer,
        camera_bind_group,
        model_uniform,
        model_buffer,
        model_bind_group,
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

  fn update(&mut self, message: &Message, response: &EventResponse) {
    self.time.update();
    while self.time.should_do_tick_unchecked() {
      self.time.tick();
    }

    if !response.consumed {
      self.camera_controller.process_events(message);
    }

    self
      .camera_controller
      .update_camera(&mut self.camera, self.time.delta_secs() as f32);
    self.camera_uniform.update_view_proj(&self.camera);

    // if !matches!(
    //   message,
    //   Message::Paint
    //     | Message::Loop(..)
    //     | Message::RawInput(..)
    //     | Message::CursorMove { .. }
    // ) {
    //   tracing::info!("{message:?}");
    // }
  }

  fn update_buffers(&mut self) {
    self.window_uniform.update(self.window.inner_size());
    self.queue.write_buffer(
      &self.window_buffer,
      0,
      bytemuck::cast_slice(&[self.window_uniform]),
    );

    self.frame_uniform.update();
    self.queue.write_buffer(
      &self.frame_buffer,
      0,
      bytemuck::cast_slice(&[self.frame_uniform]),
    );

    self.queue.write_buffer(
      &self.camera_buffer,
      0,
      bytemuck::cast_slice(&[self.camera_uniform]),
    );

    self.queue.write_buffer(
      &self.model_buffer,
      0,
      bytemuck::cast_slice(&[self.model_uniform]),
    );
  }

  fn draw(&mut self, _response: &EventResponse) -> Result<(), wgpu::SurfaceError> {
    if self.window.inner_size().is_any_zero() {
      return Ok(()); // this early out prevents issues with serious lag after minimization
    }

    let output = self.surface.get_current_texture()?;

    let now = Instant::now();
    let elapsed = now.duration_since(self.last_time);
    if elapsed >= Duration::from_secs_f64(0.20) {
      self.fps = (1.0 / self.time.average_delta_secs()) as f32;
      self.last_time = now;
    }

    self.update_buffers();

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
              r: 0.0,
              g: 1.0,
              b: 0.0,
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
      render_pass.set_bind_group(0, &self.window_bind_group, &[]);
      render_pass.set_bind_group(1, &self.frame_bind_group, &[]);
      render_pass.set_bind_group(2, &self.camera_bind_group, &[]);
      render_pass.set_bind_group(3, &self.model_bind_group, &[]);
      render_pass.draw(0..3, 0..1); // 3.
    }

    let screen_descriptor = ScreenDescriptor {
      size_in_pixels: [self.config.width, self.config.height],
      pixels_per_point: self.window.scale_factor() as f32,
    };

    self.egui_renderer.draw(
      &self.device,
      &self.queue,
      &mut encoder,
      &self.window,
      &view,
      screen_descriptor,
      |ctx| {
        egui::Window::new("Debug")
          .default_open(true)
          .resizable(false)
          .anchor(egui::Align2::LEFT_BOTTOM, (5.0, -5.0))
          .show(ctx, |ctx| {
            ctx.label(format!("fps: {:.1}", self.fps));
            ctx.label(format!("resolution: {:.1?}", self.window_uniform.resolution));
            ctx.label(format!("frame_index: {:.1}", self.frame_uniform.frame_index));

            ctx.label("Model Position:");
            ctx
              .add(egui::Slider::new(&mut self.model_uniform.model_pos[0], -5.0..=5.0))
              .labelled_by("X: ".into());
            ctx
              .add(egui::Slider::new(&mut self.model_uniform.model_pos[1], -5.0..=5.0))
              .labelled_by("Y: ".into());
            ctx
              .add(egui::Slider::new(&mut self.model_uniform.model_pos[2], -5.0..=0.0))
              .labelled_by("Z: ".into());
          });
      },
    );

    self.queue.submit(std::iter::once(encoder.finish()));
    output.present();

    Ok(())
  }
}

// TODO: see comment in SyncData
