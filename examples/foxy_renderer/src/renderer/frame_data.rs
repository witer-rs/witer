use std::sync::Arc;

use vulkano::{
  command_buffer::{
    allocator::StandardCommandBufferAllocator,
    AutoCommandBufferBuilder,
    CommandBufferUsage,
    PrimaryAutoCommandBuffer,
  },
  device::Queue,
};

use super::device::FoxyDevice;
use crate::error::RendererError;

pub type PrimaryCommandBufferBuilder = AutoCommandBufferBuilder<
  PrimaryAutoCommandBuffer<Arc<StandardCommandBufferAllocator>>,
  Arc<StandardCommandBufferAllocator>,
>;

pub struct FrameData {
  pub cmd_buffer_allocator: Arc<StandardCommandBufferAllocator>,
  pub imm_cmd_buffer_allocator: Arc<StandardCommandBufferAllocator>,
}

impl FrameData {
  pub fn new(device: &FoxyDevice) -> Result<FrameData, RendererError> {
    let cmd_buffer_allocator = Arc::new(StandardCommandBufferAllocator::new(device.vk().clone(), Default::default()));
    let imm_cmd_buffer_allocator =
      Arc::new(StandardCommandBufferAllocator::new(device.vk().clone(), Default::default()));

    Ok(FrameData {
      cmd_buffer_allocator,
      imm_cmd_buffer_allocator,
    })
  }

  pub fn primary_command(&self, queue: &Arc<Queue>) -> Result<PrimaryCommandBufferBuilder, RendererError> {
    let builder = AutoCommandBufferBuilder::primary(
      &self.cmd_buffer_allocator,
      queue.queue_family_index(),
      CommandBufferUsage::OneTimeSubmit,
    )?;

    Ok(builder)
  }
}
