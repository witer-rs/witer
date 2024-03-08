// #![feature(c_str_literals)]
#![cfg_attr(target_os, windows)] // for now, it only supports Win32
#![deny(unsafe_op_in_unsafe_fn)]

use std::sync::OnceLock;

#[cfg(all(feature = "rwh_05", not(feature = "rwh_06")))]
pub use rwh_05 as raw_window_handle;
#[cfg(all(feature = "rwh_06", not(feature = "rwh_05")))]
pub use rwh_06 as raw_window_handle;
use windows::{
  core::PCSTR,
  Win32::{
    Foundation::NTSTATUS,
    System::{
      LibraryLoader::{GetProcAddress, LoadLibraryA},
      SystemInformation::OSVERSIONINFOW,
    },
  },
  UI::ViewManagement::{UIColorType, UISettings},
};

pub mod debug;
mod handle;
pub mod prelude;
pub mod window;

pub fn signed_lo_word(dword: i32) -> i16 {
  dword as i16
}

pub fn lo_word(dword: u32) -> u16 {
  dword as u16
}

pub fn signed_hi_word(dword: i32) -> i16 {
  (dword >> 16) as i16
}

pub fn hi_word(dword: u32) -> u16 {
  (dword >> 16) as u16
}

pub fn signed_lo_byte(word: i16) -> i8 {
  word as i8
}

pub fn lo_byte(word: u16) -> u8 {
  word as u8
}

pub fn signed_hi_byte(word: i16) -> i8 {
  (word >> 8) as i8
}

pub fn hi_byte(word: u16) -> u8 {
  (word >> 8) as u8
}

/*
  Some of the following code was taken directly from `winit` and is currently under the Apache-2.0 copyright.
  > https://github.com/rust-windowing/winit/blob/master/src/platform_impl/windows/dark_mode.rs
  Some functions are simplified to be more specific to the goals of ezwin or reduce dependencies.
  Changes were also made to adapt from the crate `windows-sys` to `windows`

  The dark mode algorithm was NOT taken from `winit`, but instead from here:
  > https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/apply-windows-themes
*/

pub(crate) fn get_function_impl(
  library: &str,
  function: &str,
) -> Option<*const std::ffi::c_void> {
  assert_eq!(library.chars().last(), Some('\0'));
  assert_eq!(function.chars().last(), Some('\0'));

  // Library names we will use are ASCII so we can use the A version to avoid
  // string conversion.
  let module = match unsafe { LoadLibraryA(PCSTR::from_raw(library.as_ptr())) } {
    Ok(module) => module,
    Err(_) => return None,
  };

  unsafe { GetProcAddress(module, PCSTR::from_raw(function.as_ptr())) }
    .map(|function_ptr| function_ptr as _)
}

macro_rules! get_function {
  ($lib:expr, $func:ident) => {
    crate::get_function_impl(concat!($lib, '\0'), concat!(stringify!($func), '\0'))
      .map(|f| unsafe { std::mem::transmute::<*const _, $func>(f) })
  };
}

pub(crate) static WIN10_BUILD_VERSION: OnceLock<Option<u32>> = OnceLock::new();
pub(crate) static DARK_MODE_SUPPORTED: OnceLock<bool> = OnceLock::new();
pub(crate) static IS_SYSTEM_DARK_MODE: OnceLock<bool> = OnceLock::new();

pub(crate) fn init_statics() {
  let _ = WIN10_BUILD_VERSION.set({
    type RtlGetVersion = unsafe extern "system" fn(*mut OSVERSIONINFOW) -> NTSTATUS;
    let handle = get_function!("ntdll.dll", RtlGetVersion);

    if let Some(rtl_get_version) = handle {
      unsafe {
        let mut vi = OSVERSIONINFOW {
          dwOSVersionInfoSize: 0,
          dwMajorVersion: 0,
          dwMinorVersion: 0,
          dwBuildNumber: 0,
          dwPlatformId: 0,
          szCSDVersion: [0; 128],
        };

        let status = (rtl_get_version)(&mut vi);

        if status.0 >= 0 && vi.dwMajorVersion == 10 && vi.dwMinorVersion == 0 {
          Some(vi.dwBuildNumber)
        } else {
          None
        }
      }
    } else {
      None
    }
  });

  let _ = DARK_MODE_SUPPORTED.set({
    // We won't try to do anything for windows versions < 17763
    // (Windows 10 October 2018 update)
    match *WIN10_BUILD_VERSION.get().unwrap() {
      Some(v) => v >= 17763,
      None => false,
    }
  });

  let _ = IS_SYSTEM_DARK_MODE.set({
    let settings = UISettings::new().unwrap();
    let foreground = settings
      .GetColorValue(UIColorType::Foreground)
      .unwrap_or_default();
    is_color_light(&foreground)
  });
}

#[inline]
fn is_color_light(clr: &windows::UI::Color) -> bool {
  ((5 * clr.G as u32) + (2 * clr.R as u32) + clr.B as u32) > (8 * 128)
}
