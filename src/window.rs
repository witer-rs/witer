use std::{sync::Arc, thread::JoinHandle};

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
    Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE},
    System::LibraryLoader::GetModuleHandleW,
    UI::{
      Shell::SetWindowSubclass,
      WindowsAndMessaging::{
        self,
        CreateWindowExW,
        DispatchMessageW,
        GetClientRect,
        GetMessageW,
        GetWindowRect,
        LoadCursorW,
        PeekMessageW,
        PostMessageW,
        RegisterClassExW,
        SetWindowTextW,
        TranslateMessage,
        MSG,
        WINDOW_EX_STYLE,
        WNDCLASSEXW,
      },
    },
  },
};

use self::{message::WindowMessage, stage::Stage, sync::Response};
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
    sync::ThreadMessage,
  },
  window_error,
};

pub mod callback;
pub mod input;
pub mod message;
pub mod procedure;
pub mod settings;
pub mod stage;
pub mod state;
mod sync;

/// Uses internal mutability, so passing around as an Arc is the intended use
/// case.
#[allow(unused)]
pub struct Window {
  hinstance: HINSTANCE,
  hwnd: HWND,
  state: Handle<InternalState>,
}

impl Window {
  pub const WINDOW_SUBCLASS_ID: usize = 0;

  /// Create a new window based on the settings provided.
  pub fn new(settings: WindowSettings) -> Result<Arc<Self>, WindowError> {
    let (message_sender, message_receiver) = crossbeam::channel::unbounded();
    let (response_sender, response_receiver) = crossbeam::channel::unbounded();
    let thread =
      Some(Self::window_loop(settings.clone(), message_sender, response_receiver)?);

    // block until first message sent (which will be the window opening)
    if let Message::Ready { hwnd, hinstance } = message_receiver
      .recv()
      .expect("failed to receive opened message")
    {
      // create state
      let input = Input::new();
      let state = Handle::new(InternalState {
        subclass: None,
        title: settings.title.clone().into(),
        subtitle: HSTRING::new(),
        color_mode: settings.color_mode,
        visibility: settings.visibility,
        flow: settings.flow,
        close_on_x: settings.close_on_x,
        stage: Stage::Looping,
        input,
        message: Some(Message::None),
        thread,
        message_receiver,
        response_sender,
        requested_redraw: false,
      });

      // create Self
      let window = Arc::new(Window {
        hinstance,
        hwnd,
        state,
      });

      // delay potentially revealing window to minimize "white flash" time
      window.set_color_mode(settings.color_mode);
      window.set_visibility(settings.visibility);

      Ok(window)
    } else {
      Err(window_error!("received incorrect first window message"))
    }
  }

  fn window_loop(
    settings: WindowSettings,
    message_sender: Sender<Message>,
    response_receiver: Receiver<Response>,
  ) -> WindowResult<JoinHandle<WindowResult<()>>> {
    let thread_handle = std::thread::Builder::new().name("win32".to_owned()).spawn(
      move || -> WindowResult<()> {
        fn message_pump() -> bool {
          let mut msg = MSG::default();
          if unsafe {
            PeekMessageW(&mut msg, None, 0, 0, WindowsAndMessaging::PM_REMOVE).as_bool()
          } {
            unsafe {
              TranslateMessage(&msg);
              DispatchMessageW(&msg);
            }
          }
          msg.message != WindowsAndMessaging::WM_QUIT
        }

        let hinstance: HINSTANCE = unsafe { GetModuleHandleW(None)? }.into();
        let hwnd = Self::create_hwnd(settings.title, settings.size)?;

        // create subclass ptr
        let window_data_ptr = Box::into_raw(Box::new(SubclassWindowData {
          message_sender: message_sender.clone(),
          response_receiver,
        }));

        // attach subclass ptr
        debug_assert!(unsafe {
          SetWindowSubclass(
            hwnd,
            Some(procedure::subclass_proc),
            Window::WINDOW_SUBCLASS_ID,
            window_data_ptr as usize,
          )
        }
        .as_bool());

        // Send opened message to main function
        message_sender
          .send(Message::Ready { hwnd, hinstance })
          .expect("failed to send opened message");

        while message_pump() {}

        // loop {
        //   let wait_event = unsafe {
        //     MsgWaitForMultipleObjects(
        //       Some(&[next_frame_event]),
        //       false,
        //       u32::MAX,
        //       WindowsAndMessaging::QS_ALLEVENTS,
        //     )
        //   };
        //
        //   match wait_event.0 - WAIT_OBJECT_0.0 {
        //     0 => {
        //       // handle game_event
        //       if !Self::message_pump() {
        //         break;
        //       }
        //     }
        //     _ => {
        //       // Handle message
        //       if !Self::message_pump() {
        //         break;
        //       }
        //     }
        //   }
        // }

        Ok(())
      },
    )?;

    Ok(thread_handle)
  }

  fn handle_message(&self, message: Message) -> Message {
    let stage = self.state.get().stage;

    match stage {
      Stage::Looping => {
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
      Stage::Closing => {
        if let Message::Window(WindowMessage::Closed) = &message {
          self.state.get_mut().stage = Stage::Destroyed;
        }
      }
      Stage::Destroyed => unreachable!(),
    }

    // if let Message::Window(window_message) = &message {
    //   match window_message {
    //     WindowMessage::CloseRequested => {
    //       if window.state.get().close_on_x {
    //         window.close();
    //       }
    //     }
    //     WindowMessage::Closed => {
    //       window.state.get_mut().stage = Stage::Destroyed;
    //       unsafe { PostQuitMessage(0) };
    //     }
    //     &WindowMessage::Key { key, state, .. } => {
    //       window.state.get_mut().input.update_key_state(key, state);
    //       window.state.get_mut().input.update_modifiers_state();
    //     }
    //     &WindowMessage::MouseButton { button, state, .. } => window
    //       .state
    //       .get_mut()
    //       .input
    //       .update_mouse_state(button, state),
    //     _ => (),
    //   }
    // }

    message
  }

  fn create_hwnd(title: String, size: Size) -> WindowResult<HWND> {
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
      hCursor: unsafe { LoadCursorW(None, WindowsAndMessaging::IDC_ARROW)? },
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
      Ok(hwnd)
    }
  }

  // fn set_callback(self: &Arc<Self>, callback: impl WindowCallback + 'static) {
  //   let window_data_ptr = Box::into_raw(Box::new(SubclassWindowData {
  //     window: self.clone(),
  //     callback: Box::new(callback),
  //   }));
  //
  //   unsafe {
  //     SetWindowSubclass(
  //       self.hwnd,
  //       Some(procedure::subclass_proc),
  //       Window::WINDOW_SUBCLASS_ID,
  //       window_data_ptr as usize,
  //     );
  //   }
  //
  //   self.state.get_mut().subclass = Some(Window::WINDOW_SUBCLASS_ID);
  // }

  /// Pump messages to the window procedure based on window flow type (polling
  /// or waiting).
  // pub fn run(self: &Arc<Self>, callback: impl WindowCallback + 'static) {
  //   let is_new = self.state.get().subclass.is_none();
  //   if is_new {
  //     self.set_callback(callback);
  //     // delay potentially revealing window to try to mitigate "white flash"
  //     let visibility = self.state.get().visibility;
  //     self.set_visibility(visibility);
  //     while self.pump() {}
  //   }
  // }

  pub fn next_message(&self) -> Option<Message> {
    let flow = self.state.get().flow;
    let current_stage = self.state.get().stage;
    let receiver = self.state.get().message_receiver.clone();
    let sender = self.state.get().response_sender.clone();
    sender.send(sync::Response::NextFrame).unwrap();

    let next = match current_stage {
      Stage::Looping | Stage::Closing => match flow {
        Flow::Wait => match receiver.recv() {
          Ok(message) => Some(self.handle_message(message)),
          _ => {
            error!("channel between main and window was closed!");
            self.close();
            None
          }
        },
        Flow::Poll => match receiver.try_recv() {
          Ok(message) => Some(self.handle_message(message)),
          Err(TryRecvError::Disconnected) => {
            error!("channel between main and window was closed!");
            self.close();
            None
          }
          _ => Some(Message::None),
        },
      },
      // Stage::Closing => Some(Message::None),
      Stage::Destroyed => {
        if let Some(thread) = self.state.get_mut().thread.take() {
          let _ = thread.join();
        }
        None
      }
    };

    next
  }

  // fn pump(&self) -> bool {
  //   let mut msg = MSG::default();
  //   match self.flow() {
  //     Flow::Poll => self.poll(&mut msg),
  //     Flow::Wait => self.wait(&mut msg),
  //   }
  // }
  //
  // fn poll(&self, msg: &mut MSG) -> bool {
  //   let has_message =
  //     unsafe { PeekMessageW(msg, None, 0, 0, WindowsAndMessaging::PM_REMOVE)
  // }.as_bool();   if has_message {
  //     unsafe {
  //       TranslateMessage(msg);
  //       DispatchMessageW(msg);
  //     }
  //   } else {
  //     let _ = unsafe { PostMessageW(self.hwnd, Window::MSG_EMPTY, WPARAM(0),
  // LPARAM(0)) };   }
  //
  //   if msg.message == WindowsAndMessaging::WM_QUIT {
  //     self.state.get_mut().stage = Stage::Destroyed;
  //     false
  //   } else {
  //     true
  //   }
  // }
  //
  // fn wait(&self, msg: &mut MSG) -> bool {
  //   let keeping_going = unsafe { GetMessageW(msg, None, 0, 0) }.as_bool();
  //   if keeping_going {
  //     unsafe {
  //       TranslateMessage(msg);
  //       DispatchMessageW(msg);
  //     }
  //   }
  //   if !keeping_going {
  //     self.state.get_mut().stage = Stage::Destroyed;
  //     false
  //   } else {
  //     true
  //   }
  // }

  pub fn visibility(&self) -> Visibility {
    self.state.get().visibility
  }

  pub fn set_visibility(&self, visibility: Visibility) {
    self.state.get_mut().visibility = visibility;
    self.post_message(
      ThreadMessage::ShowWindow,
      Some(match visibility {
        Visibility::Shown => 1,
        Visibility::Hidden => 0,
      }),
    );
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
      self.post_message(ThreadMessage::RequestRedraw, None);
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
    unsafe {
      let _ = SetWindowTextW(self.hwnd, &title);
    }
  }

  /// Set text to appear after the title of the window
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
    self.post_message(ThreadMessage::CloseConfirmed, None);
  }

  fn post_message(&self, message: ThreadMessage, param: Option<usize>) {
    if let Err(error) = unsafe {
      PostMessageW(
        self.hwnd,
        message as u32,
        WPARAM(param.unwrap_or_default()),
        LPARAM(0),
      )
    } {
      eprintln!("{error}");
    }
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
