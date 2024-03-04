use std::sync::Arc;

#[cfg(all(feature = "rwh_05", not(feature = "rwh_06")))]
use rwh_05::{
  HasRawDisplayHandle,
  HasRawWindowHandle,
  RawDisplayHandle,
  RawWindowHandle,
  Win32WindowHandle,
  WindowsDisplayHandle,
};
#[cfg(all(feature = "rwh_06", not(feature = "rwh_05")))]
use rwh_06::{
  DisplayHandle,
  HandleError,
  HasDisplayHandle,
  HasWindowHandle,
  RawDisplayHandle,
  RawWindowHandle,
  Win32WindowHandle,
  WindowHandle,
  WindowsDisplayHandle,
};
use tracing::*;
use windows::{
  core::{HSTRING, PCWSTR},
  Win32::{
    Foundation::*,
    Graphics::{
      Dwm::{DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE},
      Gdi::{RedrawWindow, RDW_INTERNALPAINT},
    },
    System::LibraryLoader::GetModuleHandleW,
    UI::{
      Shell::SetWindowSubclass,
      WindowsAndMessaging::{self, *},
    },
  },
};

use self::{callback::WindowProcedure, stage::Stage};
use crate::{
  debug::{error::WindowError, WindowResult},
  handle::Handle,
  prelude::{ButtonState, Key, KeyState, Mouse},
  window::{
    input::Input,
    message::Message,
    procedure::SubclassWindowData,
    settings::{ColorMode, Flow, Size, Visibility, WindowSettings},
    state::InternalState,
  },
};

pub mod builder;

pub mod callback;
pub mod input;
pub mod message;
pub mod procedure;
pub mod settings;
pub mod stage;
pub mod state;

#[allow(unused)]
pub struct Window {
  hinstance: HINSTANCE,
  hwnd: HWND,
  state: Handle<InternalState>,
}

impl Window {
  pub const MSG_EMPTY: u32 = WM_USER + 10;
  pub const MSG_STAGE_EXIT_LOOP: u32 = WM_USER + 11;
  pub const WINDOW_SUBCLASS_ID: usize = 0;

  pub fn new(settings: WindowSettings) -> Result<Arc<Self>, WindowError> {
    let input = Input::new();
    let state = Handle::new(InternalState {
      title: settings.title.clone().into(),
      subtitle: HSTRING::new(),
      color_mode: settings.color_mode,
      visibility: settings.visibility,
      flow: settings.flow,
      close_on_x: settings.close_on_x,
      stage: Stage::Ready,
      input,
      message: Some(Message::None),
    });

    HWND::default();
    let hinstance: HINSTANCE = unsafe { GetModuleHandleW(None)? }.into();
    let hwnd = Self::create_hwnd(settings.title, settings.size)?.0;

    let window = Arc::new(Window {
      hinstance,
      hwnd,
      state,
    });
    
    window.set_color_mode(settings.color_mode);

    Ok(window)
  }

  pub fn run(self: &Arc<Self>, wndproc: impl WindowProcedure + 'static) {
    // prevent re-entry
    if self.state.get().stage == Stage::Ready {
      {
        self.state.get_mut().stage = Stage::Looping;
      }

      self.set_subclass(wndproc);

      // delay potentially revealing window to try to mitigate "white flash"
      let visibility = self.state.get().visibility;
      self.set_visibility(visibility);

      while Window::message_pump(self) {}

      self.state.get_mut().stage = Stage::Destroyed;
    } else {
      panic!("Do not call run within callback")
    }
  }

  fn create_hwnd(title: String, size: Size) -> WindowResult<(HWND, WNDCLASSEXW)> {
    let hinstance: HINSTANCE = unsafe { GetModuleHandleW(None)? }.into();
    debug_assert_ne!(hinstance.0, 0);
    let title = HSTRING::from(title);
    let window_class = title.clone();

    let wc = WNDCLASSEXW {
      cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
      style: WindowsAndMessaging::CS_VREDRAW
        | WindowsAndMessaging::CS_HREDRAW
        | WindowsAndMessaging::CS_DBLCLKS
        | WindowsAndMessaging::CS_OWNDC,
      cbWndExtra: std::mem::size_of::<WNDCLASSEXW>() as i32,
      lpfnWndProc: Some(procedure::wnd_proc),
      hInstance: hinstance,
      hCursor: unsafe { LoadCursorW(None, IDC_ARROW)? },
      lpszClassName: PCWSTR(window_class.as_ptr()),
      ..Default::default()
    };

    {
      let atom = unsafe { RegisterClassExW(&wc) };
      debug_assert_ne!(atom, 0);
    }

    let hwnd = unsafe {
      CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        &window_class,
        &title,
        WindowsAndMessaging::WS_OVERLAPPEDWINDOW
          | WindowsAndMessaging::WS_CLIPCHILDREN
          | WindowsAndMessaging::WS_CLIPSIBLINGS,
        WindowsAndMessaging::CW_USEDEFAULT,
        WindowsAndMessaging::CW_USEDEFAULT,
        size.width,
        size.height,
        None,
        None,
        hinstance,
        None,
      )
    };

    if hwnd.0 == 0 {
      Err(WindowError::Win32Error(windows::core::Error::from_win32()))
    } else {
      Ok((hwnd, wc))
    }
  }

  fn set_subclass(self: &Arc<Self>, wndproc: impl WindowProcedure + 'static) {
    let window_data_ptr = Box::into_raw(Box::new(SubclassWindowData {
      window: self.clone(),
      wndproc: Box::new(wndproc),
    }));

    unsafe {
      SetWindowSubclass(
        self.hwnd,
        Some(procedure::subclass_proc),
        Window::WINDOW_SUBCLASS_ID,
        window_data_ptr as usize,
      );
    }
  }

  fn message_pump(&self) -> bool {
    let mut msg = MSG::default();
    if self.flow() == Flow::Poll {
      self.poll(&mut msg)
    } else {
      self.wait(&mut msg)
    }
  }

  fn poll(&self, msg: &mut MSG) -> bool {
    let has_message =
      unsafe { PeekMessageW(msg, None, 0, 0, WindowsAndMessaging::PM_REMOVE) }.as_bool();
    if has_message {
      unsafe {
        TranslateMessage(msg);
        DispatchMessageW(msg);
      }
    } else {
      let _ = unsafe { PostMessageW(self.hwnd, Window::MSG_EMPTY, WPARAM(0), LPARAM(0)) };
    }

    msg.message != WindowsAndMessaging::WM_QUIT
  }

  fn wait(&self, msg: &mut MSG) -> bool {
    let keeping_going = unsafe { GetMessageW(msg, None, 0, 0) }.as_bool();
    if keeping_going {
      unsafe {
        TranslateMessage(msg);
        DispatchMessageW(msg);
      }
    }
    keeping_going
  }

  pub fn visibility(&self) -> Visibility {
    self.state.get().visibility
  }

  pub fn set_visibility(&self, visibility: Visibility) {
    self.state.get_mut().visibility = visibility;
    unsafe {
      ShowWindow(self.hwnd, match visibility {
        Visibility::Shown => SW_SHOW,
        Visibility::Hidden => SW_HIDE,
      });
    }
  }

  pub fn color_mode(&self) -> ColorMode {
    self.state.get().color_mode
  }

  pub fn set_color_mode(&self, color_mode: ColorMode) {
    self.state.get_mut().color_mode = color_mode;
    let dark_mode = BOOL::from(color_mode == ColorMode::Dark);
    if let Err(error) = unsafe {
      DwmSetWindowAttribute(
        self.hwnd,
        DWMWA_USE_IMMERSIVE_DARK_MODE,
        std::ptr::addr_of!(dark_mode) as *const std::ffi::c_void,
        std::mem::size_of::<BOOL>() as u32,
      )
    } {
      error!("{error}");
    };
  }

  pub fn request_redraw(&self) {
    unsafe {
      RedrawWindow(self.hwnd, None, None, RDW_INTERNALPAINT);
    }
  }

  pub fn flow(&self) -> Flow {
    self.state.get().flow
  }

  pub fn title(&self) -> String {
    self.state.get().title.to_string()
  }

  pub fn subtitle(&self) -> String {
    self.state.get().subtitle.to_string()
  }

  pub fn set_title(&self, title: impl AsRef<str>) {
    self.state.get_mut().title = title.as_ref().into();
    let title = HSTRING::from(format!("{}{}", title.as_ref(), self.state.get().subtitle));
    unsafe {
      let _ = SetWindowTextW(self.hwnd, &title);
    }
  }

  pub fn set_subtitle(&self, subtitle: impl AsRef<str>) {
    self.state.get_mut().subtitle = subtitle.as_ref().into();
    let title = HSTRING::from(format!("{}{}", self.state.get().title, subtitle.as_ref()));
    unsafe {
      let _ = SetWindowTextW(self.hwnd, &title);
    }
  }

  pub fn size(&self) -> Size {
    let mut window_rect = RECT::default();
    let _ = unsafe { GetWindowRect(self.hwnd, std::ptr::addr_of_mut!(window_rect)) };
    Size {
      width: window_rect.right - window_rect.left,
      height: window_rect.bottom - window_rect.top,
    }
  }

  pub fn inner_size(&self) -> Size {
    let mut client_rect = RECT::default();
    let _ = unsafe { GetClientRect(self.hwnd, std::ptr::addr_of_mut!(client_rect)) };
    Size {
      width: client_rect.right - client_rect.left,
      height: client_rect.bottom - client_rect.top,
    }
  }

  // KEYBOARD

  pub fn key(&self, keycode: Key) -> KeyState {
    self.state.get().input.key(keycode)
  }

  // MOUSE

  pub fn mouse(&self, button: Mouse) -> ButtonState {
    self.state.get().input.mouse(button)
  }

  // MODS

  pub fn shift(&self) -> ButtonState {
    self.state.get().input.shift()
  }

  pub fn ctrl(&self) -> ButtonState {
    self.state.get().input.ctrl()
  }

  pub fn alt(&self) -> ButtonState {
    self.state.get().input.alt()
  }

  pub fn win(&self) -> ButtonState {
    self.state.get().input.win()
  }

  pub fn is_closing(&self) -> bool {
    matches!(self.state.get().stage, Stage::Closing | Stage::Destroyed)
  }

  pub fn close(&self) {
    self.state.get_mut().stage = Stage::Closing;

    unsafe { DestroyWindow(self.hwnd) }.expect("failed to destroy window");
  }

  #[cfg(all(feature = "rwh_06", not(feature = "rwh_05")))]
  pub fn raw_window_handle(&self) -> RawWindowHandle {
    let mut handle = Win32WindowHandle::new(
      std::num::NonZeroIsize::new(self.hwnd.0).expect("window handle should not be zero"),
    );
    let hinstance = std::num::NonZeroIsize::new(self.hinstance.0)
      .expect("instance handle should not be zero");
    handle.hinstance = Some(hinstance);
    RawWindowHandle::from(handle)
  }

  #[cfg(all(feature = "rwh_06", not(feature = "rwh_05")))]
  pub fn raw_display_handle(&self) -> RawDisplayHandle {
    let handle = WindowsDisplayHandle::new();
    RawDisplayHandle::from(handle)
  }
}

#[cfg(all(feature = "rwh_06", not(feature = "rwh_05")))]
impl HasWindowHandle for Window {
  fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
    Ok(unsafe { WindowHandle::borrow_raw(self.raw_window_handle()) })
  }
}

#[cfg(all(feature = "rwh_05", not(feature = "rwh_06")))]
unsafe impl HasRawWindowHandle for Window {
  fn raw_window_handle(&self) -> RawWindowHandle {
    let mut handle = Win32WindowHandle::empty();
    handle.hwnd = self.hwnd.0 as *mut std::ffi::c_void;
    handle.hinstance = self.hinstance.0 as *mut std::ffi::c_void;
    RawWindowHandle::Win32(handle)
  }
}

#[cfg(all(feature = "rwh_06", not(feature = "rwh_05")))]
impl HasDisplayHandle for Window {
  fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
    Ok(unsafe { DisplayHandle::borrow_raw(self.raw_display_handle()) })
  }
}

#[cfg(all(feature = "rwh_05", not(feature = "rwh_06")))]
unsafe impl HasRawDisplayHandle for Window {
  fn raw_display_handle(&self) -> RawDisplayHandle {
    RawDisplayHandle::Windows(WindowsDisplayHandle::empty())
  }
}
