#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ImageFormat {
  pub present_mode: PresentMode,
  pub color_space: ColorSpace,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ColorSpace {
  Unorm,
  #[default]
  Srgb,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum PresentMode {
  AutoImmediate,
  #[default]
  AutoVsync,
}

impl PresentMode {
  pub fn select_from(&self, modes: Vec<vulkano::swapchain::PresentMode>) -> vulkano::swapchain::PresentMode {
    *match self {
      PresentMode::AutoImmediate => modes
        .iter()
        .find(|&mode| *mode == vulkano::swapchain::PresentMode::Immediate)
        .unwrap_or_else(|| {
          modes
            .iter()
            .find(|&mode| *mode == vulkano::swapchain::PresentMode::Mailbox)
            .unwrap_or(&vulkano::swapchain::PresentMode::Fifo)
        }),
      PresentMode::AutoVsync => modes
        .iter()
        .find(|&mode| *mode == vulkano::swapchain::PresentMode::FifoRelaxed)
        .unwrap_or(&vulkano::swapchain::PresentMode::Fifo),
    }
  }
}
