use std::ffi::{c_float, c_int, c_uint};

use windows::{
  core::Error,
  Win32::{
    Foundation::{BOOL, HINSTANCE, PROC, WIN32_ERROR},
    Graphics::{
      Gdi::{GetDC, ReleaseDC, HDC},
      OpenGL::HGLRC,
    },
    System::{LibraryLoader::GetModuleHandleW, SystemServices::APPLICATION_ERROR_MASK},
    UI::WindowsAndMessaging::{DestroyWindow, UnregisterClassW},
  },
};

use crate::{
  debug::{error::WindowError, WindowResult},
  prelude::WindowSettings,
  window::Window,
};

#[allow(unused)]
pub struct GlContext {
  pub hdc: HDC,
  // pub gl_context: glow::Context,
}

#[allow(unused)]
pub fn get_wgl_basic_functions() -> WindowResult<(
  Vec<String>,
  WglChoosePixelFormatArbT,
  WglCreateContextAttribsArbT,
  WglSwapIntervalExtT,
)> {
  use windows::Win32::Graphics::OpenGL::*;

  let pfd = PIXELFORMATDESCRIPTOR {
    dwFlags: PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL | PFD_DOUBLEBUFFER,
    iPixelType: PFD_TYPE_RGBA,
    cColorBits: 32,
    cDepthBits: 24,
    cStencilBits: 8,
    iLayerType: PFD_MAIN_PLANE.0 as u8,
    ..Default::default()
  };

  let hinstance: HINSTANCE = unsafe { GetModuleHandleW(None)? }.into();
  let (fake_hwnd, fake_wc) = Window::create_hwnd(
    WindowSettings::default()
      .with_title("fake wnd opengl")
      .with_size((1, 1)),
  )?;
  let fake_hdc = unsafe { GetDC(fake_hwnd) };
  let pixel_format_index = unsafe { ChoosePixelFormat(fake_hdc, &pfd) };
  unsafe { SetPixelFormat(fake_hdc, pixel_format_index, &pfd) }?;

  let fake_gl_context = unsafe { wglCreateContext(fake_hdc) }?;
  unsafe { wglMakeCurrent(fake_hdc, fake_gl_context) }?;

  // Begin context current

  let extensions: Vec<String> = wgl_get_extension_string_arb(fake_hdc)
    .map(|s| {
      s.split(' ')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
    })
    .unwrap_or(Vec::new());
  println!("> Extensions: {:?}", extensions);

  #[allow(non_snake_case)]
  let wglChoosePixelFormatARB: WglChoosePixelFormatArbT = unsafe {
    core::mem::transmute(wgl_get_proc_address(
      c"wglChoosePixelFormatARB".to_bytes_with_nul(),
    )?)
  };
  #[allow(non_snake_case)]
  let wglCreateContextAttribsARB: WglCreateContextAttribsArbT = unsafe {
    core::mem::transmute(wgl_get_proc_address(
      c"wglCreateContextAttribsARB".to_bytes_with_nul(),
    )?)
  };
  #[allow(non_snake_case)]
  let wglSwapIntervalEXT: WglSwapIntervalExtT = unsafe {
    core::mem::transmute(wgl_get_proc_address(c"wglSwapIntervalEXT".to_bytes_with_nul())?)
  };

  // End context current

  unsafe { wglMakeCurrent(HDC::default(), HGLRC::default()) }?;
  unsafe { wglDeleteContext(fake_gl_context) }?;

  unsafe { ReleaseDC(fake_hwnd, fake_hdc) };
  unsafe { DestroyWindow(fake_hwnd) }?;
  unsafe { UnregisterClassW(fake_wc.lpszClassName, hinstance) }?;

  Ok((
    extensions,
    wglChoosePixelFormatARB,
    wglCreateContextAttribsARB,
    wglSwapIntervalEXT,
  ))
}

// #[allow(unused)]
// pub fn create_context(hdc: HDC) -> WindowResult<glow::Context> {
//   use windows::Win32::Graphics::OpenGL::*;
//
//   // base criteria
//   let mut int_attribs = vec![
//     [WGL_DRAW_TO_WINDOW_ARB, true as _],
//     [WGL_SUPPORT_OPENGL_ARB, true as _],
//     [WGL_DOUBLE_BUFFER_ARB, true as _],
//     [WGL_PIXEL_TYPE_ARB, WGL_TYPE_RGBA_ARB],
//     [WGL_COLOR_BITS_ARB, 32],
//     [WGL_DEPTH_BITS_ARB, 24],
//     [WGL_STENCIL_BITS_ARB, 8],
//   ];
//   // if sRGB is supported, ask for that
//   if wgl_extensions.iter().any(|s| s == "WGL_EXT_framebuffer_sRGB") {
//     int_attribs.push([WGL_FRAMEBUFFER_SRGB_CAPABLE_EXT, true as _]);
//   };
//   // let's have some multisample if we can get it
//   if wgl_extensions.iter().any(|s| s == "WGL_ARB_multisample") {
//     int_attribs.push([WGL_SAMPLE_BUFFERS_ARB, 1]);
//   };
//   // finalize our list
//   int_attribs.push([0, 0]);
//   // choose a format, get the PIXELFORMATDESCRIPTOR, and set it.
//   let pix_format = unsafe {
//     do_wglChoosePixelFormatARB(wglChoosePixelFormatARB, hdc, &int_attribs,
// &[])   }
//     .unwrap();
//   let pfd = unsafe { describe_pixel_format(hdc, pix_format) }.unwrap();
//   unsafe { set_pixel_format(hdc, pix_format, &pfd) }.unwrap();
//
//   // unsafe {
//   //   glow::Context::from_loader_function_cstr(|name| {
//   //     let p = windows::Win32::System::LibraryLoader::GetProcAddress(
//   //       HMODULE(self.state.get().hinstance),
//   //       windows::core::PCSTR(name.as_ptr() as *const u8),
//   //     )
//   //       .expect("proc address was null");
//   //
//   //     p as _
//   //   })
//   // }
// }

#[allow(unused)]
fn wgl_get_proc_address(func_name: &[u8]) -> WindowResult<PROC> {
  use windows::{core::PCSTR, Win32::Graphics::OpenGL::*};

  // check that we end the slice with a \0 as expected.
  match func_name.last() {
    Some(b'\0') => (),
    _ => {
      return Err(WindowError::Error(
        "wgl function proc doesn't have terminal null char!".to_owned(),
      ));
    }
  }

  // Safety: we've checked that the end of the slice is null-terminated.
  let proc = unsafe { wglGetProcAddress(PCSTR::from_raw(func_name.as_ptr())) };
  match proc {
    // Some non-zero values can also be errors,
    // https://www.khronos.org/opengl/wiki/Load_OpenGL_Functions#Windows
    None => Err(Error::from_win32().into()),
    _ => Ok(proc),
  }
}

#[allow(unused)]
fn wgl_get_extension_string_arb(hdc: HDC) -> WindowResult<String> {
  #[allow(non_camel_case_types)]
  type wglGetExtensionsStringARB_t =
    unsafe extern "system" fn(HDC) -> *const std::ffi::c_char;

  let f: wglGetExtensionsStringARB_t = unsafe {
    std::mem::transmute(wgl_get_proc_address(
      c"wglGetExtensionsStringARB".to_bytes_with_nul(),
    )?)
  };

  let mut p: *const u8 = unsafe { f(hdc).cast() };
  if p.is_null() {
    Err(Error::from_win32().into())
  } else {
    let mut bytes = vec![];
    unsafe {
      while *p != 0 {
        bytes.push(*p);
        p = p.add(1);
      }
    }
    let string = String::from_utf8(bytes)
      .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned());

    Ok(string)
  }
}

/// Type for [wglChoosePixelFormatARB](https://www.khronos.org/registry/OpenGL/extensions/ARB/WGL_ARB_pixel_format.txt)
#[allow(unused)]
pub type WglChoosePixelFormatArbT = Option<
  unsafe extern "system" fn(
    hdc: HDC,
    pi_attrib_ilist: *const c_int,
    pf_attrib_flist: *const c_float,
    n_max_formats: u32,
    pi_formats: *mut c_int,
    n_num_formats: *mut c_uint,
  ) -> BOOL,
>;

/// Type for [wglCreateContextAttribsARB](https://www.khronos.org/registry/OpenGL/extensions/ARB/WGL_ARB_create_context.txt)
#[allow(unused)]
pub type WglCreateContextAttribsArbT = Option<
  unsafe extern "system" fn(
    h_dc: HDC,
    h_share_context: HGLRC,
    attrib_list: *const c_int,
  ) -> HGLRC,
>;

/// Type for [wglSwapIntervalEXT](https://www.khronos.org/registry/OpenGL/extensions/EXT/WGL_EXT_swap_control.txt)
#[allow(unused)]
pub type WglSwapIntervalExtT = Option<unsafe extern "system" fn(interval: c_int) -> BOOL>;

/// Arranges the data for calling a [`wglChoosePixelFormatARB_t`] procedure.
///
/// * Inputs are slices of [key, value] pairs.
/// * Input slices **can** be empty.
/// * Non-empty slices must have a zero value in the key position of the final
///   pair.
#[allow(unused)]
pub fn do_wgl_choose_pixel_format_arb(
  f: WglChoosePixelFormatArbT,
  hdc: HDC,
  int_attrs: &[[c_int; 2]],
  float_attrs: &[[c_float; 2]],
) -> WindowResult<c_int> {
  let app_err = WindowError::Win32Error(Error::from(WIN32_ERROR(APPLICATION_ERROR_MASK)));
  let i_ptr = match int_attrs.last() {
    Some([k, _v]) => {
      if *k == 0 {
        int_attrs.as_ptr()
      } else {
        return Err(app_err);
      }
    }
    None => std::ptr::null(),
  };
  let f_ptr = match float_attrs.last() {
    Some([k, _v]) => {
      if *k == 0.0 {
        float_attrs.as_ptr()
      } else {
        return Err(app_err);
      }
    }
    None => std::ptr::null(),
  };
  let mut out_format = 0;
  let mut out_format_count = 0;
  let b = unsafe {
    (f.ok_or(app_err)?)(
      hdc,
      i_ptr.cast(),
      f_ptr.cast(),
      1,
      &mut out_format,
      &mut out_format_count,
    )
  };
  if b.0 != 0 && out_format_count == 1 {
    Ok(out_format)
  } else {
    Err(Error::from_win32().into())
  }
}
