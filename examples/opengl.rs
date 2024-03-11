use std::num::NonZeroU32;

// OPENGL SUPPORT IS WIP
use glium::{
  glutin::{
    config::ConfigTemplateBuilder,
    context::{ContextApi, ContextAttributesBuilder, NotCurrentGlContext},
    display::{GetGlDisplay, GlDisplay},
    surface::{SurfaceAttributesBuilder, WindowSurface},
  },
  Display,
  Surface,
};
use rwh_05::HasRawWindowHandle;
use witer::{opengl::DisplayBuilder, prelude::*};

fn main() {
  let settings = WindowSettings::default()
    .with_flow(Flow::Wait)
    .with_size((1280, 720))
    .with_title("Simple Example");

  let template = ConfigTemplateBuilder::new()
    .prefer_hardware_accelerated(Some(true))
    .with_alpha_size(8)
    .with_transparency(true);
  let display_builder = DisplayBuilder::new(settings);
  let (window, gl_config) = display_builder
    .build(template, |mut configs| configs.next().unwrap())
    .unwrap();

  let raw_window_handle = window.raw_window_handle();
  let context_attributes = ContextAttributesBuilder::new().build(Some(raw_window_handle));
  let fallback_context_attributes = ContextAttributesBuilder::new()
    .with_context_api(ContextApi::Gles(None))
    .build(Some(raw_window_handle));

  let not_current_gl_context = unsafe {
    gl_config
      .display()
      .create_context(&gl_config, &context_attributes)
      .unwrap_or_else(|_| {
        gl_config
          .display()
          .create_context(&gl_config, &fallback_context_attributes)
          .expect("failed to create context")
      })
  };

  let (width, height): (u32, u32) = window.inner_size().into();
  let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
    raw_window_handle,
    NonZeroU32::new(width).unwrap(),
    NonZeroU32::new(height).unwrap(),
  );
  // Now we can create our surface, use it to make our context current and finally
  // create our display
  let surface = unsafe {
    gl_config
      .display()
      .create_window_surface(&gl_config, &attrs)
      .unwrap()
  };
  let current_context = not_current_gl_context.make_current(&surface).unwrap();
  let display = Display::from_context_surface(
    glium::glutin::context::PossiblyCurrentContext::Wgl(current_context),
    glium::glutin::surface::Surface::Wgl(surface),
  )
  .unwrap();

  let mut target = display.draw();
  target.clear_color(0.0, 0.0, 1.0, 1.0);
  target.finish().unwrap();

  for message in &window {
    if let Message::Window(WindowMessage::Key {
      key: Key::Escape, ..
    }) = message
    {
      window.close();
    }
  }
}
