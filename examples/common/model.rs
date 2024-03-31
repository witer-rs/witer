#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelUniform {
  pub model_pos: [f32; 3],
  pub padding: u32,
}

impl ModelUniform {
  pub fn new() -> Self {
    Self {
      model_pos: [0.0, 0.0, 0.0],
      padding: 0,
    }
  }
}
