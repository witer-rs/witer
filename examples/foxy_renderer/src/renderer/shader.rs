mod gradient {
  use vulkano_shaders::shader;

  shader! {
    ty: "compute",
    path: "assets/foxy_renderer/shaders/gradient.comp",
    linalg_type: "nalgebra",
  }
}
