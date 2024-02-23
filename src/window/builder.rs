// use super::Window;
// use crate::debug::error::WindowError;
// use crate::window::control_flow::Flow;
// use crate::window::settings::{ColorMode, Size, Visibility, WindowSettings};

// I'm not supporting the builder because it's a pain in the behind to keep
// updated

// #[derive(Debug, Clone)]
// pub struct HasTitle(pub &'static str);
// pub struct MissingTitle;
//
// #[derive(Debug, Clone)]
// pub struct HasSize(pub Size);
// pub struct MissingSize;
//
// #[derive(Debug, Clone)]
// pub struct WindowCreateInfo<Title, Size> {
//   pub title: Title,
//   pub size: Size,
//   pub color_mode: ColorMode,
//   pub visibility: Visibility,
//   pub flow: Flow,
// }
//
// pub struct WindowBuilder<Title, Size> {
//   create_info: WindowCreateInfo<Title, Size>,
// }
//
// impl WindowBuilder<MissingTitle, MissingSize> {
//   pub fn new() -> Self {
//     Self::default()
//   }
// }
//
// impl Default for WindowBuilder<MissingTitle, MissingSize> {
//   fn default() -> Self {
//     Self {
//       create_info: WindowCreateInfo {
//         title: MissingTitle,
//         size: MissingSize,
//         color_mode: ColorMode::Dark,
//         visibility: Visibility::Shown,
//       },
//     }
//   }
// }
//
// impl<Size> WindowBuilder<MissingTitle, Size> {
//   pub fn with_title(self, title: &'static str) -> WindowBuilder<HasTitle,
// Size> {     WindowBuilder {
//       create_info: WindowCreateInfo {
//         title: HasTitle(title),
//         size: self.create_info.size,
//         color_mode: self.create_info.color_mode,
//         visibility: self.create_info.visibility,
//       },
//     }
//   }
// }
//
// impl<Title> WindowBuilder<Title, MissingSize> {
//   pub fn with_size(self, size: impl Into<Size>) -> WindowBuilder<Title,
// HasSize> {     WindowBuilder {
//       create_info: WindowCreateInfo {
//         title: self.create_info.title,
//         size: HasSize(size.into()),
//         color_mode: self.create_info.color_mode,
//         visibility: self.create_info.visibility,
//       },
//     }
//   }
// }
//
// impl<Title, Size> WindowBuilder<Title, Size> {
//   pub fn with_color_mode(self, color_mode: ColorMode) -> Self {
//     Self {
//       create_info: WindowCreateInfo {
//         title: self.create_info.title,
//         size: self.create_info.size,
//         color_mode,
//         visibility: self.create_info.visibility,
//       },
//     }
//   }
//
//   pub fn with_visibility(self, visibility: Visibility) -> Self {
//     Self {
//       create_info: WindowCreateInfo {
//         title: self.create_info.title,
//         size: self.create_info.size,
//         color_mode: self.create_info.color_mode,
//         visibility,
//       },
//     }
//   }
//
//   pub fn with_flow(self, flow: Flow) -> Self {
//     Self {
//       create_info: WindowCreateInfo {
//         title: self.create_info.title,
//         size: self.create_info.size,
//         color_mode: self.create_info.color_mode,
//         visibility,
//       },
//     }
//   }
// }
//
// impl WindowBuilder<HasTitle, HasSize> {
//   pub fn build(self) -> Result<Window, WindowError> {
//     Window::new(WindowSettings {
//       size: self.create_info.size.0,
//       title: self.create_info.title.0,
//       color_mode: self.create_info.color_mode,
//       visibility: self.create_info.visibility,
//       flow:
//     })
//   }
// }
