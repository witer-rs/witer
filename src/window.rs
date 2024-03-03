// Reference for multithreaded input processing:
//   * https://www.jendrikillner.com/post/rust-game-part-3/
//   * https://github.com/jendrikillner/RustMatch3/blob/rust-game-part-3/
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
    procedure::SubclassWindowData,
    settings::{ColorMode, Flow, Size, Visibility, WindowSettings},
    state::InternalState,
    window_message::Message,
  },
};

pub mod builder;

pub mod callback;
pub mod input;
pub mod main_message;
#[cfg(feature = "opengl")]
mod opengl;
pub mod procedure;
pub mod settings;
pub mod stage;
pub mod state;
pub mod window_message;

// pub struct MessageDispatcher {
//   messages: VecDeque<Message>,
// }

// impl MessageDispatcher {
//   pub fn dispatch(&self) {}
// }

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
  pub const WINDOW_THREAD_ID: &'static str = "window";

  pub fn run(window: &Arc<Window>) {
    if window.state.get().stage == Stage::Ready {
      // prevent re-entry
      {
        window.state.get_mut().stage = Stage::Looping;
      }

      while Window::message_pump(window) {}
    } else {
      panic!("Do not call run within callback")
    }
  }

  pub fn new<WP: WindowProcedure + 'static>(
    settings: WindowSettings,
  ) -> Result<Arc<Self>, WindowError> {
    HWND::default();
    let hinstance: HINSTANCE = unsafe { GetModuleHandleW(None)? }.into();
    let hwnd = Self::create_hwnd(settings.clone())?.0;

    // block until first message sent (which will be the window opening)
    // if let Message::Window(WindowMessage::Ready { hwnd, hinstance }) =
    // receiver.recv()? {
    let input = Input::new();

    let window = Arc::new(Window {
      hinstance,
      hwnd,
      state: Handle::new(InternalState {
        title: settings.title.into(),
        subtitle: HSTRING::new(),
        // size: settings.size,
        // inner_size,
        color_mode: settings.color_mode,
        visibility: settings.visibility,
        flow: settings.flow,
        close_on_x: settings.close_on_x,
        stage: Stage::Ready,
        // receiver,
        input,
        message: Some(Message::None),
      }),
    });

    let window_data_ptr = Box::into_raw(Box::new(SubclassWindowData {
      window: window.clone(),
      callback: Box::new(WP::new(&window)),
    }));

    unsafe {
      SetWindowSubclass(
        hwnd,
        Some(procedure::subclass_proc),
        Window::WINDOW_SUBCLASS_ID,
        window_data_ptr as usize,
      );
    }

    let color_mode = window.state.get().color_mode;
    window.set_color_mode(color_mode);
    let visibility = window.state.get().visibility;
    window.set_visibility(visibility);

    Ok(window)
  }

  fn create_hwnd(settings: WindowSettings) -> WindowResult<(HWND, WNDCLASSEXW)> {
    let hinstance: HINSTANCE = unsafe { GetModuleHandleW(None)? }.into();
    debug_assert_ne!(hinstance.0, 0);
    let title = HSTRING::from(settings.title);
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
        settings.size.width,
        settings.size.height,
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

  fn message_pump(&self) -> bool {
    let mut msg = MSG::default();
    if self.flow() == Flow::Poll {
      self.poll(&mut msg)
    } else {
      Self::wait(&mut msg)
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

  fn wait(msg: &mut MSG) -> bool {
    let keeping_going = unsafe { GetMessageW(msg, None, 0, 0) }.as_bool();
    if keeping_going {
      unsafe {
        TranslateMessage(msg);
        DispatchMessageW(msg);
      }
    }
    keeping_going
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
    matches!(self.state.get().stage, Stage::Closing | Stage::Closed)
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

// #[allow(unused)]
// pub struct Window {
//   hwnd: isize,
//   hinstance: isize,
//   state: Handle<InternalState>,
//   sync_barrier: Arc<Barrier>,
//   message_queue: Arc<Mutex<VecDeque<Message>>>,
// }

// impl Window {
//   pub const MSG_EXIT_LOOP: u32 = WM_USER + 69;
//   pub const MSG_MAIN_CLOSE_REQ: u32 = WM_USER + 11;
//   pub const WINDOW_SUBCLASS_ID: usize = 0;
//   pub const WINDOW_THREAD_ID: &'static str = "window";

//   pub fn new(settings: WindowSettings) -> Result<Self, WindowError> {
//     let sync_barrier = Arc::new(Barrier::new(2));

//     let message_queue = Arc::new(Mutex::new(VecDeque::new()));

//     let window_thread = Some(Self::window_loop(
//       settings.clone(),
//       sync_barrier.clone(),
//       message_queue.clone(),
//     )?);

//     sync_barrier.wait();

//     let (hwnd, hinstance) = loop {
//       let message = message_queue.lock().unwrap().pop_front();
//       if let Some(Message::Window(WindowMessage::Ready { hwnd, hinstance }))
// = message {         break (hwnd, hinstance);
//       }
//     };

//     // block until first message sent (which will be the window opening)
//     // if let Message::Window(WindowMessage::Ready { hwnd, hinstance }) =
//     // receiver.recv()? {
//     let input = Input::new();

//     #[cfg(all(feature = "rwh_06", not(feature = "rwh_05")))]
//     let raw_window_handle = {
//       let mut handle = Win32WindowHandle::new(
//         std::num::NonZeroIsize::new(hwnd).expect("window handle should not be
// zero"),       );
//       let hinstance = std::num::NonZeroIsize::new(hinstance)
//         .expect("instance handle should not be zero");
//       handle.hinstance = Some(hinstance);
//       RawWindowHandle::from(handle)
//     };

//     #[cfg(all(feature = "rwh_06", not(feature = "rwh_05")))]
//     let raw_display_handle = {
//       let handle = WindowsDisplayHandle::new();
//       RawDisplayHandle::from(handle)
//     };

//     let state = Handle::new(InternalState {
//       #[cfg(all(feature = "rwh_06", not(feature = "rwh_05")))]
//       raw_window_handle,
//       #[cfg(all(feature = "rwh_06", not(feature = "rwh_05")))]
//       raw_display_handle,
//       title: settings.title,
//       subtitle: String::new(),
//       // size: settings.size,
//       // inner_size,
//       color_mode: settings.color_mode,
//       visibility: settings.visibility,
//       flow: settings.flow,
//       current_stage: Stage::Looping,
//       close_on_x: settings.close_on_x,
//       is_sizing_or_moving: false,
//       is_closing: false,
//       window_thread,
//       // receiver,
//       input,
//     });

//     let window = Self {
//       hwnd,
//       hinstance,
//       state,
//       sync_barrier,
//       message_queue,
//     };

//     let color_mode = window.state.get().color_mode;
//     window.set_color_mode(color_mode);
//     let visibility = window.state.get().visibility;
//     window.set_visibility(visibility);

//     Ok(window)
//     // } else {
//     //   Err(WindowError::Error("Invalid message".into()))
//     // }
//   }

//   pub fn set_visibility(&self, visibility: Visibility) {
//     self.state.get_mut().visibility = visibility;
//     unsafe {
//       ShowWindow(HWND(self.hwnd), match visibility {
//         Visibility::Shown => SW_SHOW,
//         Visibility::Hidden => SW_HIDE,
//       });
//     }
//   }

//   pub fn set_color_mode(&self, color_mode: ColorMode) {
//     self.state.get_mut().color_mode = color_mode;
//     let dark_mode = BOOL::from(color_mode == ColorMode::Dark);
//     if let Err(error) = unsafe {
//       DwmSetWindowAttribute(
//         HWND(self.hwnd),
//         DWMWA_USE_IMMERSIVE_DARK_MODE,
//         std::ptr::addr_of!(dark_mode) as *const std::ffi::c_void,
//         std::mem::size_of::<BOOL>() as u32,
//       )
//     } {
//       error!("{error}");
//     };
//   }

//   pub fn redraw(&self) {
//     unsafe {
//       RedrawWindow(HWND(self.hwnd), None, None, RDW_INTERNALPAINT);
//     }
//   }

//   pub fn flow(&self) -> Flow {
//     self.state.get().flow
//   }

//   pub fn title(&self) -> String {
//     self.state.get().title.to_owned()
//   }

//   pub fn set_title(&self, title: impl AsRef<str>) {
//     self.state.get_mut().title = title.as_ref().to_owned();
//     let title = HSTRING::from(format!("{}{}", title.as_ref(),
// self.state.get().subtitle));     unsafe {
//       let _ = SetWindowTextW(HWND(self.hwnd), &title);
//     }
//   }

//   pub fn set_subtitle(&self, subtitle: impl AsRef<str>) {
//     self.state.get_mut().subtitle = subtitle.as_ref().to_owned();
//     let title = HSTRING::from(format!("{}{}", self.state.get().title,
// subtitle.as_ref()));     unsafe {
//       let _ = SetWindowTextW(HWND(self.hwnd), &title);
//     }
//   }

//   pub fn size(&self) -> Size {
//     let mut window_rect = RECT::default();
//     let _ =
//       unsafe { GetWindowRect(HWND(self.hwnd),
// std::ptr::addr_of_mut!(window_rect)) };     Size {
//       width: window_rect.right - window_rect.left,
//       height: window_rect.bottom - window_rect.top,
//     }
//   }

//   pub fn inner_size(&self) -> Size {
//     let mut client_rect = RECT::default();
//     let _ =
//       unsafe { GetClientRect(HWND(self.hwnd),
// std::ptr::addr_of_mut!(client_rect)) };     Size {
//       width: client_rect.right - client_rect.left,
//       height: client_rect.bottom - client_rect.top,
//     }
//   }

//   // KEYBOARD

//   pub fn key(&self, keycode: Key) -> KeyState {
//     self.state.get().input.key(keycode)
//   }

//   // MOUSE

//   pub fn mouse(&self, button: Mouse) -> ButtonState {
//     self.state.get().input.mouse(button)
//   }

//   // MODS

//   pub fn shift(&self) -> ButtonState {
//     self.state.get().input.shift()
//   }

//   pub fn ctrl(&self) -> ButtonState {
//     self.state.get().input.ctrl()
//   }

//   pub fn alt(&self) -> ButtonState {
//     self.state.get().input.alt()
//   }

//   pub fn win(&self) -> ButtonState {
//     self.state.get().input.win()
//   }

//   pub fn is_closing(&self) -> bool {
//     self.state.get_mut().is_closing
//   }

//   pub fn close(&self) {
//     self.state.get_mut().is_closing = true;
//     self.state.get_mut().current_stage = Stage::Exiting;
//   }

//   fn handle_message(&self, message: Message) -> Option<Message> {
//     match &message {
//       Message::CloseRequested => {
//         if self.state.get().close_on_x {
//           self.close();
//         }
//       }
//       Message::Window(message) => match message {
//         WindowMessage::StartedSizingOrMoving => {
//           self.state.get_mut().is_sizing_or_moving = true;
//         }
//         WindowMessage::StoppedSizingOrMoving => {
//           self.state.get_mut().is_sizing_or_moving = false;
//         }
//         // WindowMessage::Resizing { window_mode } => {
//         //   let mut state = self.state.get_mut();
//         //   state.window_mode = *window_mode;
//         // }
//         // WindowMessage::Moving { .. } => (),
//         _ => (),
//       },
//       &Message::Keyboard { key, state, .. } => {
//         self.state.get_mut().input.update_key_state(key, state);
//         self.state.get_mut().input.update_modifiers_state();
//       }
//       &Message::Mouse(MouseMessage::Button { button, state, .. }) => {
//         self
//           .state
//           .get_mut()
//           .input
//           .update_mouse_button_state(button, state);
//       }
//       _ => (),
//     }

//     Some(message)
//   }

//   fn next_message(&self, should_wait: bool) -> Option<Message> {
//     let current_stage = self.state.get_mut().current_stage;
//     // let receiver = self.state.get().receiver.clone();
//     let message = {
//       let mut message_queue = self.message_queue.lock().unwrap();
//       message_queue.pop_front()
//     };

//     let next = match current_stage {
//       Stage::Looping => {
//         match message {
//           Some(message) => self.handle_message(message),
//           None => Some(Message::None),
//         }

//         // if should_wait {
//         //   match receiver.recv() {
//         //     Ok(message) => self.handle_message(message),
//         //     _ => {
//         //       error!("channel between main and window was closed!");
//         //       self.state.get_mut().current_stage = Stage::Exiting;
//         //       Some(Message::None)
//         //     }
//         //   }
//         // } else {
//         //   match receiver.try_recv() {
//         //     Ok(message) => self.handle_message(message),
//         //     Err(TryRecvError::Disconnected) => {
//         //       error!("channel between main and window was closed!");
//         //       self.state.get_mut().current_stage = Stage::Exiting;
//         //       Some(Message::None)
//         //     }
//         //     _ => Some(Message::None),
//         //   }
//         // }
//       }
//       Stage::Exiting => {
//         self.state.get_mut().current_stage = Stage::ExitLoop;
//         Some(Message::Closing)
//       }
//       Stage::ExitLoop => {
//         #[cfg(feature = "opengl")]
//         {
//           let hwnd = self.state.get().h_wnd;
//           let hdc = self.gl_context.hdc;
//           unsafe { windows::Win32::Graphics::Gdi::ReleaseDC(HWND(hwnd), hdc)
// };         }

//         let _ = unsafe {
//           SendMessageW(HWND(self.hwnd), Self::MSG_MAIN_CLOSE_REQ, WPARAM(0),
// LPARAM(0))         };
//         if let Some(thread) = self.state.get_mut().window_thread.take() {
//           let _ = thread.join();
//         }
//         None
//       }
//     };

//     next
//   }

//   /// Waits for next window message before returning.
//   ///
//   /// Returns `None` when app is exiting.
//   ///
//   /// Use this if you want the application to only react to window events.
//   #[allow(unused)]
//   pub fn wait(&self) -> Option<Message> {
//     self.next_message(true)
//   }

//   /// Returns next window message if available, otherwise returns an empty
//   /// message immediately.
//   ///
//   /// Returns `None` when app is exiting.
//   ///
//   /// Use this if you want the application to run full tilt, as fast as
//   /// possible.
//   ///
//   /// ***Note:** the window message thread will still block until a message
// is   /// received from Windows.*
//   pub fn poll(&self) -> Option<Message> {
//     self.next_message(false)
//   }

//   fn follow_flow(&self) -> Option<Message> {
//     match self.flow() {
//       Flow::Wait => self.wait(),
//       Flow::Poll => self.poll(),
//     }
//   }

//   pub fn iter(&self) -> MessageIterator {
//     MessageIterator { window: self }
//   }

//   pub fn iter_mut(&mut self) -> MessageIteratorMut {
//     MessageIteratorMut { window: self }
//   }
// }

// pub struct MessageIterator<'a> {
//   window: &'a Window,
// }

// impl<'a> Iterator for MessageIterator<'a> {
//   type Item = Message;

//   fn next(&mut self) -> Option<Self::Item> {
//     self.window.follow_flow()
//   }
// }

// impl<'a> IntoIterator for &'a Window {
//   type IntoIter = MessageIterator<'a>;
//   type Item = Message;

//   fn into_iter(self) -> Self::IntoIter {
//     self.iter()
//   }
// }

// pub struct MessageIteratorMut<'a> {
//   window: &'a mut Window,
// }

// impl<'a> Iterator for MessageIteratorMut<'a> {
//   type Item = Message;

//   fn next(&mut self) -> Option<Self::Item> {
//     self.window.follow_flow()
//   }
// }

// impl<'a> IntoIterator for &'a mut Window {
//   type IntoIter = MessageIteratorMut<'a>;
//   type Item = Message;

//   fn into_iter(self) -> Self::IntoIter {
//     self.iter_mut()
//   }
// }

// #[cfg(all(feature = "rwh_06", not(feature = "rwh_05")))]
// impl HasWindowHandle for Window {
//   fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
//     Ok(unsafe { WindowHandle::borrow_raw(self.state.get().raw_window_handle)
// })   }
// }

// #[cfg(all(feature = "rwh_05", not(feature = "rwh_06")))]
// unsafe impl HasRawWindowHandle for Window {
//   fn raw_window_handle(&self) -> RawWindowHandle {
//     let mut handle = Win32WindowHandle::empty();
//     handle.hwnd = self.hwnd as *mut std::ffi::c_void;
//     handle.hinstance = self.hinstance as *mut std::ffi::c_void;
//     RawWindowHandle::Win32(handle)
//   }
// }

// #[cfg(all(feature = "rwh_06", not(feature = "rwh_05")))]
// impl HasDisplayHandle for Window {
//   fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
//     Ok(unsafe {
// DisplayHandle::borrow_raw(self.state.get().raw_display_handle) })   }
// }

// #[cfg(all(feature = "rwh_05", not(feature = "rwh_06")))]
// unsafe impl HasRawDisplayHandle for Window {
//   fn raw_display_handle(&self) -> RawDisplayHandle {
//     RawDisplayHandle::Windows(WindowsDisplayHandle::empty())
//   }
// }

// impl Window {
//   fn window_loop(
//     settings: WindowSettings,
//     sync_barrier: Arc<Barrier>,
//     message_queue: Arc<Mutex<VecDeque<Message>>>,
//   ) -> WindowResult<JoinHandle<WindowResult<()>>> {
//     let h_wnd = RwLock::new(HWND::default());

//     // WINDOW
//     let handle = std::thread::Builder::new()
//       .name(Self::WINDOW_THREAD_ID.to_owned())
//       .spawn(move || -> WindowResult<()> {
//         let hinstance: HINSTANCE = unsafe { GetModuleHandleW(None)? }.into();
//         *h_wnd.write().unwrap() = Self::create_hwnd(settings)?.0;

//         let window_data_ptr = Box::into_raw(Box::new(SubclassWindowData {
//           message_queue: message_queue.clone(),
//         }));

//         unsafe {
//           SetWindowSubclass(
//             *h_wnd.read().unwrap(),
//             Some(procedure::subclass_proc),
//             Window::WINDOW_SUBCLASS_ID,
//             window_data_ptr as usize,
//           );
//         }

//         {
//           let mut message_queue = message_queue.lock().unwrap();

//           // Send opened message to main function
//           message_queue.push_back(Message::Window(WindowMessage::Ready {
//             hwnd: h_wnd.read().unwrap().0,
//             hinstance: hinstance.0,
//           }));

//           sync_barrier.wait();
//         }

//         // Message pump
//         while Self::message_pump().is_some() {}

//         Ok(())
//       })?;

//     Ok(handle)
//   }

//   fn message_pump() -> Option<Message> {
//     let mut msg = MSG::default();
//     if unsafe { GetMessageW(&mut msg, None, 0, 0).as_bool() } {
//       unsafe {
//         TranslateMessage(&msg);
//         DispatchMessageW(&msg);
//       }
//       Some(Message::new(msg.hwnd, msg.message, msg.wParam, msg.lParam))
//     } else {
//       None
//     }
//   }

//   pub(crate) fn create_hwnd(
//     settings: WindowSettings,
//   ) -> WindowResult<(HWND, WNDCLASSEXW)> {
//     let hinstance: HINSTANCE = unsafe { GetModuleHandleW(None)? }.into();
//     debug_assert_ne!(hinstance.0, 0);
//     let title = HSTRING::from(settings.title);
//     let window_class = title.clone();

//     let wc = WNDCLASSEXW {
//       cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
//       style: CS_VREDRAW | CS_HREDRAW | CS_DBLCLKS | CS_OWNDC,
//       cbWndExtra: std::mem::size_of::<WNDCLASSEXW>() as i32,
//       lpfnWndProc: Some(procedure::wnd_proc),
//       hInstance: hinstance,
//       hCursor: unsafe { LoadCursorW(None, IDC_ARROW)? },
//       lpszClassName: PCWSTR(window_class.as_ptr()),
//       ..Default::default()
//     };

//     {
//       let atom = unsafe { RegisterClassExW(&wc) };
//       debug_assert_ne!(atom, 0);
//     }

//     let hwnd = unsafe {
//       CreateWindowExW(
//         WINDOW_EX_STYLE::default(),
//         &window_class,
//         &title,
//         WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN | WS_CLIPSIBLINGS,
//         CW_USEDEFAULT,
//         CW_USEDEFAULT,
//         settings.size.width,
//         settings.size.height,
//         None,
//         None,
//         hinstance,
//         None,
//       )
//     };

//     if hwnd.0 == 0 {
//       Err(WindowError::Win32Error(windows::core::Error::from_win32()))
//     } else {
//       Ok((hwnd, wc))
//     }
//   }
// }
