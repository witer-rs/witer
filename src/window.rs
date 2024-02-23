// Reference for multithreaded input processing:
//   * https://www.jendrikillner.com/post/rust-game-part-3/
//   * https://github.com/jendrikillner/RustMatch3/blob/rust-game-part-3/
use std::{
  sync::{Arc, Barrier, RwLock},
  thread::JoinHandle,
};

use crossbeam::channel::{Receiver, Sender, TryRecvError};
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
      Gdi::{RedrawWindow, RDW_ERASENOW, RDW_INVALIDATE},
    },
    System::LibraryLoader::GetModuleHandleW,
    UI::{Shell::SetWindowSubclass, WindowsAndMessaging::*},
  },
};

use self::{
  stage::Stage,
  window_message::{SizeState, WindowMessage},
};
use crate::{
  debug::{error::WindowError, WindowResult},
  window::{
    input::Input,
    procedure::SubclassWindowData,
    settings::{ColorMode, Flow, Size, Visibility, WindowSettings},
    state::WindowState,
    window_message::{KeyboardMessage, Message, MouseMessage},
  },
  Handle,
};

pub mod builder;

pub mod input;
pub mod main_message;
#[cfg(feature = "opengl")]
mod opengl;
pub mod procedure;
pub mod settings;
pub mod stage;
pub mod state;
pub mod window_message;

#[allow(unused)]
pub struct Window {
  #[cfg(all(feature = "rwh_06", not(feature = "rwh_05")))]
  raw_window_handle: RawWindowHandle,
  #[cfg(all(feature = "rwh_06", not(feature = "rwh_05")))]
  raw_display_handle: RawDisplayHandle,
  #[cfg(feature = "opengl")]
  gl_context: opengl::GlContext,

  state: Handle<WindowState>,
  window_thread: Handle<Option<JoinHandle<WindowResult<()>>>>,
  receiver: Receiver<Message>,
  current_stage: Handle<Stage>,

  barrier: Arc<Barrier>,
}

impl Window {
  pub const MSG_EXIT_LOOP: u32 = WM_USER + 69;
  pub const MSG_MAIN_CLOSE_REQ: u32 = WM_USER + 11;
  pub const WINDOW_SUBCLASS_ID: usize = 0;
  pub const WINDOW_THREAD_ID: &'static str = "window";

  pub fn new(settings: WindowSettings) -> Result<Self, WindowError> {
    let (sender, receiver) = crossbeam::channel::unbounded();
    let barrier = Arc::new(Barrier::new(2));

    let window_thread =
      Handle::new(Some(Self::window_loop(settings.clone(), sender, barrier.clone())?));

    // block until first message sent (which will be the window opening)
    if let Message::Window(WindowMessage::Ready { h_wnd, hinstance }) = receiver.recv()? {
      let input = Input::new();

      let state = Handle::new(WindowState {
        h_wnd,
        hinstance,
        title: settings.title,
        color_mode: settings.color_mode,
        visibility: settings.visibility,
        flow: settings.flow,
        input,
        size_state: SizeState::Normal,
        close_on_x: settings.close_on_x,
        sizing_or_moving: false,
      });

      #[cfg(all(feature = "rwh_06", not(feature = "rwh_05")))]
      let raw_window_handle = {
        let mut handle = Win32WindowHandle::new(
          std::num::NonZeroIsize::new(h_wnd).expect("window handle should not be zero"),
        );
        let hinstance = std::num::NonZeroIsize::new(hinstance)
          .expect("instance handle should not be zero");
        handle.hinstance = Some(hinstance);
        RawWindowHandle::from(handle)
      };

      #[cfg(all(feature = "rwh_06", not(feature = "rwh_05")))]
      let raw_display_handle = {
        let handle = WindowsDisplayHandle::new();
        RawDisplayHandle::from(handle)
      };

      #[cfg(feature = "opengl")]
      let gl_context = {
        let hdc = unsafe { windows::Win32::Graphics::Gdi::GetDC(HWND(h_wnd)) };
        opengl::GlContext {
          hdc,
          // gl_context: opengl::create_context()?,
        }
      };

      let window = Self {
        #[cfg(all(feature = "rwh_06", not(feature = "rwh_05")))]
        raw_window_handle,
        #[cfg(all(feature = "rwh_06", not(feature = "rwh_05")))]
        raw_display_handle,
        #[cfg(feature = "opengl")]
        gl_context,
        state,
        window_thread,
        receiver,
        current_stage: Handle::new(Stage::Looping),
        barrier,
      };

      let color_mode = window.state.get().color_mode;
      window.set_color_mode(color_mode);
      let visibility = window.state.get().visibility;
      window.set_visibility(visibility);

      Ok(window)
    } else {
      Err(WindowError::Error("Invalid message".into()))
    }
  }

  pub fn set_visibility(&self, visibility: Visibility) {
    self.state.get_mut().visibility = visibility;
    unsafe {
      ShowWindow(HWND(self.state.get().h_wnd), match visibility {
        Visibility::Shown => SW_SHOW,
        Visibility::Hidden => SW_HIDE,
      });
    }
  }

  pub fn set_color_mode(&self, color_mode: ColorMode) {
    self.state.get_mut().color_mode = color_mode;
    let dark_mode = BOOL::from(color_mode == ColorMode::Dark);
    if let Err(error) = unsafe {
      DwmSetWindowAttribute(
        HWND(self.state.get().h_wnd),
        DWMWA_USE_IMMERSIVE_DARK_MODE,
        std::ptr::addr_of!(dark_mode) as *const std::ffi::c_void,
        std::mem::size_of::<BOOL>() as u32,
      )
    } {
      error!("{error}");
    };
  }

  pub fn redraw(&self) {
    unsafe {
      let h_wnd = self.state.get().h_wnd;
      RedrawWindow(HWND(h_wnd), None, None, RDW_INVALIDATE | RDW_ERASENOW);
    }
  }

  pub fn flow(&self) -> Flow {
    self.state.get().flow
  }

  pub fn title(&self) -> String {
    self.state.get().title.to_owned()
  }

  pub fn set_title(&self, title: &str) {
    unsafe {
      let _ = SetWindowTextW(HWND(self.state.get().h_wnd), &HSTRING::from(title));
    }
  }

  pub fn size(&self) -> Size {
    let mut window_rect = RECT::default();
    let _ = unsafe {
      GetWindowRect(HWND(self.state.get().h_wnd), std::ptr::addr_of_mut!(window_rect))
    };
    Size {
      width: window_rect.right - window_rect.left,
      height: window_rect.bottom - window_rect.top,
    }
  }

  pub fn inner_size(&self) -> Size {
    let mut client_rect = RECT::default();
    let _ = unsafe {
      GetClientRect(HWND(self.state.get().h_wnd), std::ptr::addr_of_mut!(client_rect))
    };
    Size {
      width: client_rect.right - client_rect.left,
      height: client_rect.bottom - client_rect.top,
    }
  }

  pub fn close(&self) {
    *self.current_stage.get_mut() = Stage::Exiting;
  }

  fn handle_message(&self, message: Message) -> Option<Message> {
    self.state.get_mut().sizing_or_moving = matches!(
      message,
      Message::Window(WindowMessage::Resizing { .. } | WindowMessage::Moving { .. })
    );

    match message {
      Message::CloseRequested => {
        if self.state.get().close_on_x {
          self.close();
        }
      }
      Message::Window(WindowMessage::Resizing { size_state }) => {
        self.state.get_mut().size_state = size_state;
      }
      Message::Window(WindowMessage::Moving { .. }) => {}
      Message::Keyboard(KeyboardMessage::Key {
        key_code, state, ..
      }) => {
        self
          .state
          .get_mut()
          .input
          .update_keyboard_state(key_code, state);
      }
      Message::Mouse(MouseMessage::Button {
        mouse_code, state, ..
      }) => {
        self
          .state
          .get_mut()
          .input
          .update_mouse_button_state(mouse_code, state);
      }
      _ => {}
    }

    Some(message)
  }

  fn next_message(&self, should_wait: bool) -> Option<Message> {
    if self.state.get().sizing_or_moving {
      self.barrier.wait();
      self.state.get_mut().sizing_or_moving = false;
    }

    let current_stage = *self.current_stage.get();

    match current_stage {
      Stage::Looping => {
        if should_wait {
          match self.receiver.recv() {
            Ok(message) => self.handle_message(message),
            _ => {
              error!("channel between main and window was closed!");
              *self.current_stage.get_mut() = Stage::Exiting;
              Some(Message::None)
            }
          }
        } else {
          match self.receiver.try_recv() {
            Ok(message) => self.handle_message(message),
            Err(TryRecvError::Disconnected) => {
              error!("channel between main and window was closed!");
              *self.current_stage.get_mut() = Stage::Exiting;
              Some(Message::None)
            }
            _ => Some(Message::None),
          }
        }
      }
      Stage::Exiting => {
        *self.current_stage.get_mut() = Stage::ExitLoop;
        Some(Message::Closing)
      }
      Stage::ExitLoop => {
        #[cfg(feature = "opengl")]
        {
          let hwnd = self.state.get().h_wnd;
          let hdc = self.gl_context.hdc;
          unsafe { windows::Win32::Graphics::Gdi::ReleaseDC(HWND(hwnd), hdc) };
        }

        let _ = unsafe {
          PostMessageW(
            HWND(self.state.get().h_wnd),
            Self::MSG_MAIN_CLOSE_REQ,
            WPARAM(0),
            LPARAM(0),
          )
        };
        if let Some(thread) = self.window_thread.get_mut().take() {
          let _ = thread.join();
        }
        None
      }
    }
  }

  /// Waits for next window message before returning.
  ///
  /// Returns `None` when app is exiting.
  ///
  /// Use this if you want the application to only react to window events.
  #[allow(unused)]
  pub fn wait(&self) -> Option<Message> {
    self.next_message(true)
  }

  /// Returns next window message if available, otherwise returns an empty
  /// message immediately.
  ///
  /// Returns `None` when app is exiting.
  ///
  /// Use this if you want the application to run full tilt, as fast as
  /// possible.
  ///
  /// ***Note:** the window message thread will still block until a message is
  /// received from Windows.*
  pub fn poll(&self) -> Option<Message> {
    self.next_message(false)
  }

  fn follow_flow(&self) -> Option<Message> {
    match self.flow() {
      Flow::Wait => self.wait(),
      Flow::Poll => self.poll(),
    }
  }

  pub fn iter(&self) -> MessageIterator {
    MessageIterator { window: self }
  }

  pub fn iter_mut(&mut self) -> MessageIteratorMut {
    MessageIteratorMut { window: self }
  }
}

pub struct MessageIterator<'a> {
  window: &'a Window,
}

impl<'a> Iterator for MessageIterator<'a> {
  type Item = Message;

  fn next(&mut self) -> Option<Self::Item> {
    self.window.follow_flow()
  }
}

impl<'a> IntoIterator for &'a Window {
  type IntoIter = MessageIterator<'a>;
  type Item = Message;

  fn into_iter(self) -> Self::IntoIter {
    self.iter()
  }
}

pub struct MessageIteratorMut<'a> {
  window: &'a mut Window,
}

impl<'a> Iterator for MessageIteratorMut<'a> {
  type Item = Message;

  fn next(&mut self) -> Option<Self::Item> {
    self.window.follow_flow()
  }
}

impl<'a> IntoIterator for &'a mut Window {
  type IntoIter = MessageIteratorMut<'a>;
  type Item = Message;

  fn into_iter(self) -> Self::IntoIter {
    self.iter_mut()
  }
}

#[cfg(all(feature = "rwh_06", not(feature = "rwh_05")))]
impl HasWindowHandle for Window {
  fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
    Ok(unsafe { WindowHandle::borrow_raw(self.raw_window_handle) })
  }
}

#[cfg(all(feature = "rwh_05", not(feature = "rwh_06")))]
unsafe impl HasRawWindowHandle for Window {
  fn raw_window_handle(&self) -> RawWindowHandle {
    let mut handle = Win32WindowHandle::empty();
    handle.hwnd = self.state.get().h_wnd as *mut std::ffi::c_void;
    handle.hinstance = self.state.get().hinstance as *mut std::ffi::c_void;
    RawWindowHandle::Win32(handle)
  }
}

#[cfg(all(feature = "rwh_06", not(feature = "rwh_05")))]
impl HasDisplayHandle for Window {
  fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
    Ok(unsafe { DisplayHandle::borrow_raw(self.raw_display_handle) })
  }
}

#[cfg(all(feature = "rwh_05", not(feature = "rwh_06")))]
unsafe impl HasRawDisplayHandle for Window {
  fn raw_display_handle(&self) -> RawDisplayHandle {
    RawDisplayHandle::Windows(WindowsDisplayHandle::empty())
  }
}

impl Window {
  fn window_loop(
    settings: WindowSettings,
    sender: Sender<Message>,
    barrier: Arc<Barrier>,
  ) -> WindowResult<JoinHandle<WindowResult<()>>> {
    let h_wnd = RwLock::new(HWND::default());

    // WINDOW
    let handle = std::thread::Builder::new()
      .name(Self::WINDOW_THREAD_ID.to_owned())
      .spawn(move || -> WindowResult<()> {
        let hinstance: HINSTANCE = unsafe { GetModuleHandleW(None)? }.into();
        *h_wnd.write().unwrap() = Self::create_hwnd(settings)?.0;

        let window_data_ptr = Box::into_raw(Box::new(SubclassWindowData {
          sender: sender.clone(),
          barrier,
        }));

        unsafe {
          SetWindowSubclass(
            *h_wnd.read().unwrap(),
            Some(procedure::subclass_proc),
            Window::WINDOW_SUBCLASS_ID,
            window_data_ptr as usize,
          );
        }

        // Send opened message to main function
        sender.send(Message::Window(WindowMessage::Ready {
          h_wnd: h_wnd.read().unwrap().0,
          hinstance: hinstance.0,
        }))?;

        // Message pump
        while let Some(message) = Self::message_pump() {
          #[allow(clippy::single_match)]
          match message {
            Message::Other {
              message: Window::MSG_MAIN_CLOSE_REQ,
              ..
            } => {
              let _ = unsafe { DestroyWindow(*h_wnd.read().unwrap()) };
              break;
            }
            _ => {}
          }
        }

        Ok(())
      })?;

    Ok(handle)
  }

  fn message_pump() -> Option<Message> {
    let mut msg = MSG::default();
    if unsafe { GetMessageW(&mut msg, None, 0, 0).as_bool() } {
      unsafe {
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
      }
      Some(Message::new(msg.hwnd, msg.message, msg.wParam, msg.lParam))
    } else {
      None
    }
  }

  pub(crate) fn create_hwnd(
    settings: WindowSettings,
  ) -> WindowResult<(HWND, WNDCLASSEXW)> {
    let hinstance: HINSTANCE = unsafe { GetModuleHandleW(None)? }.into();
    debug_assert_ne!(hinstance.0, 0);
    let title = HSTRING::from(settings.title);
    let window_class = title.clone();

    let wc = WNDCLASSEXW {
      cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
      style: CS_VREDRAW | CS_HREDRAW | CS_DBLCLKS | CS_OWNDC,
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
        WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN | WS_CLIPSIBLINGS,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        settings.size.width,
        settings.size.height,
        None,
        None,
        hinstance,
        None,
      )
    };

    if hwnd.0 == 0 {
      match unsafe { GetLastError() } {
        Ok(()) => Err(WindowError::Error("HWND was null".to_owned())),
        Err(error) => Err(error.into()),
      }
    } else {
      Ok((hwnd, wc))
    }
  }
}
