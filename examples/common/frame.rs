use witer::PhysicalSize;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct FrameUniform {
  pub frame_index: u32,
}

impl FrameUniform {
  pub fn new() -> Self {
    Self { frame_index: 1 }
  }

  pub fn update(&mut self) {
    self.frame_index = self.frame_index.wrapping_add(1);
  }
}
