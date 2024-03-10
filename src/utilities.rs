use std::sync::{
  atomic::{AtomicBool, Ordering},
  OnceLock,
};

use windows::{
  core::PCSTR,
  Win32::{
    Foundation::{NTSTATUS, RECT},
    System::{
      LibraryLoader::{GetProcAddress, LoadLibraryA},
      SystemInformation::OSVERSIONINFOW,
    },
    UI::WindowsAndMessaging::{
      self,
      ClipCursor,
      ShowCursor,
      WINDOW_EX_STYLE,
      WINDOW_STYLE,
    },
  },
  UI::ViewManagement::{UIColorType, UISettings},
};

use crate::window::state::{Fullscreen, Visibility};

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
    crate::utilities::get_function_impl(
      concat!($lib, '\0'),
      concat!(stringify!($func), '\0'),
    )
    .map(|f| unsafe { std::mem::transmute::<*const _, $func>(f) })
  };
}

pub fn windows_10_build_version() -> Option<u32> {
  static WIN10_BUILD_VERSION: OnceLock<Option<u32>> = OnceLock::new();
  *WIN10_BUILD_VERSION.get_or_init(|| {
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
  })
}

pub fn is_dark_mode_supported() -> bool {
  static DARK_MODE_SUPPORTED: OnceLock<bool> = OnceLock::new();
  *DARK_MODE_SUPPORTED.get_or_init(|| {
    // We won't try to do anything for windows versions < 17763
    // (Windows 10 October 2018 update)
    match windows_10_build_version() {
      Some(v) => v >= 17763,
      None => false,
    }
  })
}

pub fn is_system_dark_mode_enabled() -> bool {
  static IS_SYSTEM_DARK_MODE: OnceLock<bool> = OnceLock::new();
  *IS_SYSTEM_DARK_MODE.get_or_init(|| {
    let settings = UISettings::new().unwrap();
    let foreground = settings
      .GetColorValue(UIColorType::Foreground)
      .unwrap_or_default();
    is_color_light(&foreground)
  })
}

#[inline]
fn is_color_light(clr: &windows::UI::Color) -> bool {
  ((5 * clr.G as u32) + (2 * clr.R as u32) + clr.B as u32) > (8 * 128)
}

pub(crate) fn get_window_style(
  fullscreen: Option<Fullscreen>,
  visible: Visibility,
) -> WINDOW_STYLE {
  static NORMAL_STYLE: OnceLock<WINDOW_STYLE> = OnceLock::new();
  static BORDERLESS_STYLE: OnceLock<WINDOW_STYLE> = OnceLock::new();
  match fullscreen {
    Some(Fullscreen::Borderless) => {
      let style = BORDERLESS_STYLE.get_or_init(|| {
        WindowsAndMessaging::WS_POPUP
          | WindowsAndMessaging::WS_CLIPCHILDREN
          | WindowsAndMessaging::WS_CLIPSIBLINGS
      });
      match visible {
        Visibility::Shown => *style | WindowsAndMessaging::WS_VISIBLE,
        Visibility::Hidden => *style,
      }
    }
    None => {
      let style = NORMAL_STYLE.get_or_init(|| {
        WindowsAndMessaging::WS_OVERLAPPEDWINDOW
          | WindowsAndMessaging::WS_CLIPCHILDREN
          | WindowsAndMessaging::WS_CLIPSIBLINGS
      });
      match visible {
        Visibility::Shown => *style | WindowsAndMessaging::WS_VISIBLE,
        Visibility::Hidden => *style,
      }
    }
  }
}

pub(crate) fn get_window_ex_style(fullscreen: Option<Fullscreen>) -> WINDOW_EX_STYLE {
  static NORMAL_EX_STYLE: OnceLock<WINDOW_EX_STYLE> = OnceLock::new();
  static BORDERLESS_EX_STYLE: OnceLock<WINDOW_EX_STYLE> = OnceLock::new();
  match fullscreen {
    Some(Fullscreen::Borderless) => {
      let style =
        BORDERLESS_EX_STYLE.get_or_init(|| WindowsAndMessaging::WS_EX_APPWINDOW);
      *style
    }
    None => {
      let style = NORMAL_EX_STYLE.get_or_init(|| {
        WindowsAndMessaging::WS_EX_OVERLAPPEDWINDOW | WindowsAndMessaging::WS_EX_APPWINDOW
      });
      *style
    }
  }
}

// pub(crate) fn get_cursor_clip() -> RECT {
//   let mut rect = RECT::default();
//   if let Err(e) = unsafe { GetClipCursor(&mut rect) } {
//     error!("{e}");
//   };
//   rect
// }

pub(crate) fn set_cursor_clip(rect: Option<&RECT>) {
  if let Err(_e) = unsafe { ClipCursor(rect.map(|r| r as _)) } {
    tracing::error!("{_e}");
  }
}

// pub(crate) fn get_desktop_rect() -> RECT {
//   unsafe {
//     let left = GetSystemMetrics(WindowsAndMessaging::SM_XVIRTUALSCREEN);
//     let top = GetSystemMetrics(WindowsAndMessaging::SM_YVIRTUALSCREEN);
//     RECT {
//       left,
//       top,
//       right: left +
// GetSystemMetrics(WindowsAndMessaging::SM_CXVIRTUALSCREEN),       bottom: top
// + GetSystemMetrics(WindowsAndMessaging::SM_CYVIRTUALSCREEN),     } }
// }

pub(crate) fn set_cursor_visibility(visible: Visibility) {
  let hidden = visible == Visibility::Hidden;
  static HIDDEN: AtomicBool = AtomicBool::new(false);
  let changed = HIDDEN.swap(hidden, Ordering::SeqCst) ^ hidden;
  if changed {
    unsafe { ShowCursor(!hidden) };
  }
}