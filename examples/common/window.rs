use witer::PhysicalSize;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WindowUniform {
  pub resolution: [f32; 2],
}

impl WindowUniform {
  pub fn new() -> Self {
    Self {
      resolution: [0.0; 2],
    }
  }

  pub fn update(&mut self, size: PhysicalSize) {
    self.resolution = [size.width as f32, size.height as f32];
  }
}
