use std::sync::Arc;

use tracing::*;
use vulkano::{
  device::{
    physical::{PhysicalDevice, PhysicalDeviceType},
    Device,
    DeviceCreateInfo,
    DeviceExtensions,
    Features,
    Queue,
    QueueCreateInfo,
    QueueFlags,
  },
  instance::Instance,
  swapchain::Surface,
};

use crate::{error::RendererError, renderer_error};

pub struct FoxyDevice {
  device: Arc<Device>,
  graphics_queue: Arc<Queue>,
}

impl FoxyDevice {
  pub fn new(instance: Arc<Instance>, surface: Arc<Surface>) -> Result<Self, RendererError> {
    let device_extensions = DeviceExtensions {
      khr_swapchain: true,
      ..Default::default()
    };

    let device_features = Features {
      dynamic_rendering: true,
      synchronization2: true,
      buffer_device_address: true,
      descriptor_indexing: true,
      ..Default::default()
    };

    let (physical_device, queue_family_index) =
      Self::pick_physical_device(&surface, &instance, &device_extensions, &device_features)?;

    let (device, graphics_queue) =
      Self::new_logical_device(device_extensions, device_features, &physical_device, queue_family_index)?;

    info!(
      "Selected device: [{} | {}]",
      device.physical_device().properties().device_name,
      device.api_version()
    );

    Ok(Self { device, graphics_queue })
  }

  pub fn vk(&self) -> &Arc<Device> {
    &self.device
  }

  pub fn graphics_queue(&self) -> &Arc<Queue> {
    &self.graphics_queue
  }
}

impl FoxyDevice {
  fn pick_physical_device(
    surface: &Arc<Surface>,
    instance: &Arc<Instance>,
    device_extensions: &DeviceExtensions,
    device_features: &Features,
  ) -> Result<(Arc<PhysicalDevice>, u32), RendererError> {
    let physical_devices = instance.enumerate_physical_devices()?;
    info!("Physical device count: {}", physical_devices.len());

    let (physical, queue_family_index) = physical_devices
      .filter(|p| p.supported_extensions().contains(device_extensions))
      .filter(|p| p.supported_features().contains(device_features))
      .filter_map(|p| {
        p.queue_family_properties()
          .iter()
          .enumerate()
          .position(|(i, q)| {
            q.queue_flags.intersects(QueueFlags::GRAPHICS) && p.surface_support(i as u32, surface).unwrap_or(false)
          })
          .map(|i| (p, i as u32))
      })
      .min_by_key(|(p, _)| match p.properties().device_type {
        PhysicalDeviceType::IntegratedGpu => 0,
        PhysicalDeviceType::DiscreteGpu => 1,
        PhysicalDeviceType::VirtualGpu => 2,
        PhysicalDeviceType::Cpu => 3,
        PhysicalDeviceType::Other => 4,
        _ => 5,
      })
      .ok_or_else(|| renderer_error!("failed to find valid device"))?;
    // let driver_version = Version::from(physical.properties().driver_version);

    Ok((physical, queue_family_index))
  }

  fn new_logical_device(
    device_extensions: DeviceExtensions,
    device_features: Features,
    physical_device: &Arc<PhysicalDevice>,
    queue_family_index: u32,
  ) -> Result<(Arc<Device>, Arc<Queue>), RendererError> {
    let device_info = DeviceCreateInfo {
      queue_create_infos: vec![QueueCreateInfo {
        queue_family_index,
        ..Default::default()
      }],
      enabled_extensions: device_extensions,
      enabled_features: device_features,
      ..Default::default()
    };

    let (device, mut queues) = Device::new(physical_device.clone(), device_info)?;

    let queue = queues.next().unwrap();

    Ok((device, queue))
  }
}
