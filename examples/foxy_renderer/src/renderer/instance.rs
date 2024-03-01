use std::sync::Arc;

use ezwin::window::Window;
use itertools::Itertools;
use vulkano::{
  instance::{Instance, InstanceCreateInfo, InstanceExtensions},
  swapchain::Surface,
  Version,
  VulkanLibrary,
};

use super::debug::Debug;
use crate::{error::RendererError, renderer_error};

#[derive(Clone)]
pub struct FoxyInstance {
  _library: Arc<VulkanLibrary>,
  instance: Arc<Instance>,
  _debug: Arc<Debug>,
}

impl FoxyInstance {
  pub const ENABLE_VALIDATION_LAYERS: bool = cfg!(debug_assertions);
  const VALIDATION_LAYERS: &'static [&'static str] = &["VK_LAYER_KHRONOS_validation"];

  pub fn new(window: &Window) -> Result<Self, RendererError> {
    let library = VulkanLibrary::new()?;
    let instance = Self::new_instance(library.clone(), window)?;
    let debug = Debug::new(instance.clone())?;

    Ok(Self {
      _debug: debug,
      instance,
      _library: library,
    })
  }

  pub fn vk(&self) -> &Arc<Instance> {
    &self.instance
  }

  fn new_instance(library: Arc<VulkanLibrary>, window: &Window) -> Result<Arc<Instance>, RendererError> {
    let (requested_layers, requested_extensions) = Self::request_layers_and_extensions(&library, window)?;

    let instance_create_info = InstanceCreateInfo {
      enabled_layers: requested_layers,
      enabled_extensions: requested_extensions,
      engine_name: Some("Foxy Framework".to_owned()),
      engine_version: Version::major_minor(1, 0),
      ..InstanceCreateInfo::application_from_cargo_toml()
    };

    let instance = Instance::new(library, instance_create_info)?;

    Ok(instance)
  }

  fn request_layers_and_extensions(
    library: &VulkanLibrary,
    window: &Window,
  ) -> Result<(Vec<String>, InstanceExtensions), RendererError> {
    let supported_layers = library.layer_properties()?;
    let supported_layers = supported_layers.map(|l| l.name().to_owned()).collect_vec();
    // debug!("Supported layers:\n{:#?}", supported_layers);

    // Layers ----------------------

    let mut requested_layers = Self::VALIDATION_LAYERS.iter().map(|l| (*l).to_owned()).collect_vec();

    let mut missing_layers = Vec::new();
    for layer in &requested_layers {
      if !supported_layers.contains(layer) {
        missing_layers.push(layer);
      }
    }

    if !missing_layers.is_empty() {
      return Err(renderer_error!(
        "not all requested layers are supported on this device:\nMissing: {missing_layers:?}"
      ));
    }

    if !Self::ENABLE_VALIDATION_LAYERS {
      requested_layers.clear();
    }

    // Extensions ------------------

    let supported_extensions = library.supported_extensions();
    // debug!("Supported instance extensions:\n{:#?}", supported_extensions);

    let mut requested_extensions = Surface::required_extensions(window);
    requested_extensions = requested_extensions.union(&InstanceExtensions {
      khr_surface: true,
      ext_debug_utils: true,
      ..Default::default()
    });

    if !supported_extensions.contains(&requested_extensions) {
      return Err(renderer_error!(
        "not all requested instance extensions are supported on this device:\nMissing: {:?}",
        supported_extensions.difference(&requested_extensions)
      ));
    }

    Ok((requested_layers, requested_extensions))
  }
}
