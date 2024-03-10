use std::num::NonZeroU32;

use glutin::{
  context::PossiblyCurrentContext,
  surface::{
    ResizeableSurface,
    Surface,
    SurfaceAttributes,
    SurfaceAttributesBuilder,
    SurfaceTypeTrait,
    WindowSurface,
  },
};
use raw_window_handle::{HasRawWindowHandle, RawDisplayHandle};
use rwh_05 as raw_window_handle;

use crate::prelude::{Size, Window, WindowSettings};
// use winit::window::Window;

/// [`Window`] extensions for working with [`glutin`] surfaces.
pub trait GlWindow {
  fn build_surface_attributes(
    &self,
    builder: SurfaceAttributesBuilder<WindowSurface>,
  ) -> SurfaceAttributes<WindowSurface>;

  fn resize_surface(
    &self,
    surface: &Surface<impl SurfaceTypeTrait + ResizeableSurface>,
    context: &PossiblyCurrentContext,
  );
}

impl GlWindow for Window {
  fn build_surface_attributes(
    &self,
    builder: SurfaceAttributesBuilder<WindowSurface>,
  ) -> SurfaceAttributes<WindowSurface> {
    let (w, h) = self
      .inner_size()
      .non_zero()
      .expect("invalid zero inner size");
    builder.build(self.raw_window_handle(), w, h)
  }

  fn resize_surface(
    &self,
    surface: &Surface<impl SurfaceTypeTrait + ResizeableSurface>,
    context: &PossiblyCurrentContext,
  ) {
    if let Some((w, h)) = self.inner_size().non_zero() {
      surface.resize(context, w, h)
    }
  }
}

trait NonZeroU32PhysicalSize {
  fn non_zero(self) -> Option<(NonZeroU32, NonZeroU32)>;
}

impl NonZeroU32PhysicalSize for Size {
  fn non_zero(self) -> Option<(NonZeroU32, NonZeroU32)> {
    let w = NonZeroU32::new(self.width as u32)?;
    let h = NonZeroU32::new(self.height as u32)?;
    Some((w, h))
  }
}

use std::error::Error;

#[cfg(x11_platform)]
use glutin::platform::x11::X11GlConfigExt;
use glutin::{
  config::{Config, ConfigTemplateBuilder},
  display::{Display, DisplayApiPreference},
  prelude::*,
};
#[cfg(wgl_backend)]
use raw_window_handle::HasRawWindowHandle;
use raw_window_handle::{HasRawDisplayHandle, RawWindowHandle};

/// The helper to perform [`Display`] creation and OpenGL platform
/// bootstrapping with the help of [`winit`] with little to no platform specific
/// code.
///
/// This is only required for the initial setup. If you want to create
/// additional windows just use the [`finalize_window`] function and the
/// configuration you've used either for the original window or picked with the
/// existing [`Display`].
///
/// [`winit`]: winit
/// [`Display`]: glutin::display::Display
#[derive(Debug, Clone)]
pub struct DisplayBuilder {
  window_settings: WindowSettings,
}

impl DisplayBuilder {
  /// Create new display builder.
  pub fn new(window_settings: WindowSettings) -> Self {
    Self { window_settings }
  }

  /// The window builder to use when building a window.
  ///
  /// By default no window is created.
  pub fn with_window_settings(mut self, window_settings: WindowSettings) -> Self {
    self.window_settings = window_settings;
    self
  }

  pub fn build<Picker>(
    self,
    template_builder: ConfigTemplateBuilder,
    config_picker: Picker,
  ) -> Result<(Window, Config), Box<dyn Error>>
  where
    Picker: FnOnce(Box<dyn Iterator<Item = Config> + '_>) -> Config,
  {
    let window = Window::new(self.window_settings)?;

    let raw_window_handle = window.raw_window_handle();

    let gl_display =
      create_display(window.raw_display_handle(), Some(raw_window_handle))?;

    let template_builder =
      template_builder.compatible_with_native_window(raw_window_handle);
    let template = template_builder.build();

    let gl_config = unsafe {
      let configs = gl_display.find_configs(template)?;
      config_picker(configs)
    };

    Ok((window, gl_config))
  }
}

fn create_display(
  _raw_display_handle: RawDisplayHandle,
  _raw_window_handle: Option<RawWindowHandle>,
) -> Result<Display, Box<dyn Error>> {
  let _preference = DisplayApiPreference::Wgl(_raw_window_handle);

  unsafe { Ok(Display::new(_raw_display_handle, _preference)?) }
}
