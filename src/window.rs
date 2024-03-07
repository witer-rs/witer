use std::{
  sync::{Arc, Condvar, Mutex},
  thread::JoinHandle,
};

use crossbeam::{channel::Sender, queue::SegQueue};
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
    Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE},
    System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::{
      self,
      CreateWindowExW,
      DispatchMessageW,
      GetClientRect,
      GetMessageW,
      GetWindowRect,
      LoadCursorW,
      PostMessageW,
      RegisterClassExW,
      TranslateMessage,
      MSG,
      WINDOW_EX_STYLE,
      WNDCLASSEXW,
    },
  },
};

use self::{command::Command, message::WindowMessage, stage::Stage};
use crate::{
  debug::{error::WindowError, WindowResult},
  handle::Handle,
  prelude::{ButtonState, Key, KeyState, Mouse},
  window::{
    input::Input,
    message::Message,
    procedure::CreateInfo,
    settings::{ColorMode, Flow, Size, Visibility, WindowSettings},
    state::InternalState,
  },
};

mod command;
pub mod input;
pub mod message;
pub mod procedure;
pub mod settings;
pub mod stage;
pub mod state;

/// Uses internal mutability, so passing around as an Arc is the intended use
/// case.
#[allow(unused)]
pub struct Window {
  hinstance: HINSTANCE,
  hwnd: HWND,
  state: Handle<InternalState>,
  command_queue: Arc<SegQueue<Command>>,
}

impl Window {
  pub const WINDOW_SUBCLASS_ID: usize = 0;

  /// Create a new window based on the settings provided.
  pub fn new(settings: WindowSettings) -> Result<Arc<Self>, WindowError> {
    let (tx, rx) = crossbeam::channel::bounded(0);
    let new_message = Arc::new((Mutex::new(false), Condvar::new()));
    let next_frame = Arc::new((Mutex::new(false), Condvar::new()));
    let next_message = Arc::new(Mutex::new(Message::None));
    let command_queue = Arc::new(SegQueue::new());

    let thread = Some(Self::window_loop(
      settings.clone(),
      tx,
      command_queue.clone(),
      new_message.clone(),
      next_frame.clone(),
      next_message.clone(),
    )?);

    // block until first message sent (which will be the window opening)
    let window = rx.recv().expect("failed to receive opened message");
    {
      let mut state = window.state.get_mut();
      state.thread = thread;
      state.title = settings.title.into();
      state.color_mode = settings.color_mode;
      state.visibility = settings.visibility;
      state.flow = settings.flow;
      state.close_on_x = settings.close_on_x;
    }

    // // delay potentially revealing window to minimize "white flash" time
    window.set_color_mode(settings.color_mode);
    window.set_visibility(settings.visibility);

    Ok(window)
  }

  fn window_loop(
    settings: WindowSettings,
    tx: Sender<Arc<Window>>,
    command_queue: Arc<SegQueue<Command>>,
    new_message: Arc<(Mutex<bool>, Condvar)>,
    next_frame: Arc<(Mutex<bool>, Condvar)>,
    next_message: Arc<Mutex<Message>>,
  ) -> WindowResult<JoinHandle<WindowResult<()>>> {
    let thread_handle = std::thread::Builder::new().name("win32".to_owned()).spawn(
      move || -> WindowResult<()> {
        fn message_pump() -> bool {
          let mut msg = MSG::default();
          if unsafe { GetMessageW(&mut msg, None, 0, 0).as_bool() } {
            unsafe {
              TranslateMessage(&msg);
              DispatchMessageW(&msg);
            }
            true
          } else {
            false
          }
        }

        let window = Self::create_hwnd(
          settings,
          command_queue,
          new_message,
          next_frame,
          next_message,
        )?;

        // Send opened message to main function
        tx.send(window).expect("failed to send opened message");

        while message_pump() {}

        Ok(())
      },
    )?;

    Ok(thread_handle)
  }

  fn handle_message(&self, message: Message) -> Message {
    let stage = self.state.get().stage;

    match stage {
      Stage::Looping | Stage::Closing => {
        if let Message::Window(window_message) = &message {
          match window_message {
            WindowMessage::CloseRequested => {
              if self.state.get().close_on_x {
                self.close();
              }
            }
            &WindowMessage::Key { key, state, .. } => {
              self.state.get_mut().input.update_key_state(key, state);
              self.state.get_mut().input.update_modifiers_state();
            }
            &WindowMessage::MouseButton { button, state, .. } => {
              self.state.get_mut().input.update_mouse_state(button, state)
            }
            WindowMessage::Draw => {
              self.state.get_mut().requested_redraw = false;
            }
            _ => (),
          }
        }
      }
      // Stage::Closing => {
      //   if let Message::Window(WindowMessage::Closed) = &message {
      //     self.state.get_mut().stage = Stage::Destroyed;
      //   }
      // }
      Stage::Destroyed => (),
    }

    message
  }

  fn create_hwnd(
    settings: WindowSettings,
    command_queue: Arc<SegQueue<Command>>,
    new_message: Arc<(Mutex<bool>, Condvar)>,
    next_frame: Arc<(Mutex<bool>, Condvar)>,
    next_message: Arc<Mutex<Message>>,
  ) -> WindowResult<Arc<Window>> {
    let hinstance: HINSTANCE = unsafe { GetModuleHandleW(None)? }.into();
    debug_assert_ne!(hinstance.0, 0);
    let size = settings.size;
    let title = HSTRING::from(settings.title.clone());
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
      hCursor: unsafe { LoadCursorW(None, WindowsAndMessaging::IDC_ARROW)? },
      lpszClassName: PCWSTR(window_class.as_ptr()),
      ..Default::default()
    };

    {
      let atom = unsafe { RegisterClassExW(&wc) };
      debug_assert_ne!(atom, 0);
    }

    let mut create_info = CreateInfo {
      settings,
      command_queue: Some(command_queue),
      new_message: Some(new_message),
      next_frame: Some(next_frame),
      next_message: Some(next_message),
      window: None,
    };

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
        Some(std::ptr::addr_of_mut!(create_info) as _),
      )
    };

    if hwnd.0 == 0 {
      Err(WindowError::Win32Error(windows::core::Error::from_win32()))
    } else {
      Ok(create_info.window.take().unwrap())
    }
  }

  fn signal_next_frame(&self) {
    let next_frame = self.state.get().next_frame.clone();
    let (lock, cvar) = next_frame.as_ref();
    {
      let mut next = lock.lock().unwrap();
      *next = true;
    }
    cvar.notify_one();
  }

  pub fn next_message(&self) -> Option<Message> {
    let flow = self.state.get().flow;
    let current_stage = self.state.get().stage;

    self.signal_next_frame();

    let next_message = self.state.get().next_message.clone();

    let next = match current_stage {
      Stage::Looping | Stage::Closing => match flow {
        Flow::Wait => {
          let new_message = self.state.get().new_message.clone();
          let (lock, cvar) = new_message.as_ref();
          let mut new = cvar.wait_while(lock.lock().unwrap(), |new| !*new).unwrap();
          *new = false;

          let message = next_message.lock().unwrap().take();
          Some(self.handle_message(message))
        }
        Flow::Poll => {
          let message = next_message.lock().unwrap().take();
          Some(self.handle_message(message))
        }
      },
      // Stage::Closing => Some(Message::None),
      Stage::Destroyed => {
        let thread = self.state.get_mut().thread.take();
        if let Some(thread) = thread {
          let _ = thread.join();
        }
        None
      }
    };

    next
  }

  pub fn visibility(&self) -> Visibility {
    self.state.get().visibility
  }

  pub fn set_visibility(&self, visibility: Visibility) {
    self.state.get_mut().visibility = visibility;
    self.request(Command::SetVisibility(visibility));
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
    let requested_redraw = self.state.get().requested_redraw;
    if !requested_redraw {
      self.state.get_mut().requested_redraw = true;
      self.request(Command::Redraw);
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

  /// Set the title of the window
  pub fn set_title(&self, title: impl AsRef<str>) {
    self.state.get_mut().title = title.as_ref().into();
    let title = HSTRING::from(format!("{}{}", title.as_ref(), self.state.get().subtitle));
    self.request(Command::SetWindowText(title));
  }

  /// Set text to appear after the title of the window
  pub fn set_subtitle(&self, subtitle: impl AsRef<str>) {
    self.state.get_mut().subtitle = subtitle.as_ref().into();
    let title = HSTRING::from(format!("{}{}", self.state.get().title, subtitle.as_ref()));
    self.request(Command::SetWindowText(title));
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

  pub fn is_destroyed(&self) -> bool {
    self.state.get().stage == Stage::Destroyed
  }

  pub fn close(&self) {
    if self.is_closing() {
      return; // already closing
    }

    self.request(Command::Close);
    self.state.get_mut().stage = Stage::Closing;
  }

  fn request(&self, command: Command) {
    if self.is_destroyed() {
      return; // hwnd will be invalid
    }

    let err_str = format!("failed to post command `{command:?}`");

    self.command_queue.push(command);

    unsafe { PostMessageW(self.hwnd, WindowsAndMessaging::WM_APP, WPARAM(0), LPARAM(0)) }
      .unwrap_or_else(|_| panic!("{}", err_str));
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

impl Window {
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
    self.window.next_message()
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
    self.window.next_message()
  }
}

impl<'a> IntoIterator for &'a mut Window {
  type IntoIter = MessageIteratorMut<'a>;
  type Item = Message;

  fn into_iter(self) -> Self::IntoIter {
    self.iter_mut()
  }
}
