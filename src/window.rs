use std::{
  sync::{Arc, Condvar, Mutex},
  thread::JoinHandle,
};

use crossbeam::{
  channel::{Receiver, Sender, TryRecvError},
  queue::SegQueue,
};
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
use windows::{
  core::{HSTRING, PCWSTR},
  Win32::{
    Foundation::*,
    Graphics::Dwm::{self, DwmSetWindowAttribute},
    System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::{
      self,
      CreateWindowExW,
      DispatchMessageW,
      GetClientRect,
      GetCursorPos,
      GetMessageW,
      GetWindowRect,
      LoadCursorW,
      PostMessageW,
      RegisterClassExW,
      TranslateMessage,
      MSG,
      WNDCLASSEXW,
    },
  },
};

use self::{
  command::Command,
  message::LoopMessage,
  procedure::SyncData,
  stage::Stage,
  state::{CursorMode, Fullscreen, PhysicalSize, Position, StyleInfo},
};
use crate::{
  debug::{error::WindowError, WindowResult},
  handle::Handle,
  prelude::{ButtonState, Key, KeyState, Mouse},
  utilities::{
    get_window_ex_style,
    get_window_style,
    is_dark_mode_supported,
    is_system_dark_mode_enabled,
  },
  window::{
    input::Input,
    message::Message,
    procedure::CreateInfo,
    settings::WindowSettings,
    state::{Flow, InternalState, PhysicalPosition, Size, Theme, Visibility},
  },
};

mod command;
pub mod input;
pub mod message;
pub mod procedure;
pub mod settings;
pub mod stage;
pub mod state;

/// Main window class. Uses internal mutability. Window is destroyed on drop.
#[allow(unused)]
pub struct Window {
  hinstance: HINSTANCE,
  hwnd: HWND,
  state: Handle<InternalState>,
  sync: SyncData,
  command_queue: Arc<SegQueue<Command>>,
  message_receiver: Receiver<Message>,
}

/// Window is destroyed on drop.
impl Drop for Window {
  fn drop(&mut self) {
    self.request(Command::Destroy);
    let thread = self.state.get_mut().thread.take();
    if let Some(thread) = thread {
      let _ = thread.join();
    }
  }
}

impl Window {
  pub const WINDOW_SUBCLASS_ID: usize = 0;

  /// Create a new window based on the settings provided.
  pub fn new(settings: WindowSettings) -> Result<Self, WindowError> {
    let (message_sender, message_receiver) = crossbeam::channel::unbounded();

    let sync = SyncData {
      new_message: Arc::new((Mutex::new(false), Condvar::new())),
      next_frame: Arc::new((Mutex::new(false), Condvar::new())),
    };

    let create_info = CreateInfo {
      settings: settings.clone(),
      window: None,
      sync: sync.clone(),
      command_queue: Arc::new(SegQueue::new()),
      message_sender,
      message_receiver,
      style: StyleInfo {
        visibility: settings.visibility,
        decorations: settings.decorations,
        fullscreen: settings.fullscreen,
        resizeable: settings.resizeable,
      },
    };

    let (window_sender, window_receiver) = crossbeam::channel::bounded(0);

    let thread = Some(Self::window_loop(window_sender, create_info)?);

    let window = window_receiver.recv().unwrap();

    window.state.get_mut().thread = thread;
    if let Some(position) = settings.position {
      window.force_set_outer_position(position);
    }
    window.force_set_inner_size(settings.size);
    window.force_set_decorations(settings.decorations);
    window.force_set_theme(settings.theme);
    window.force_set_visibility(settings.visibility);
    window.force_set_fullscreen(settings.fullscreen);

    Ok(window)
  }

  fn window_loop(
    window_sender: Sender<Self>,
    create_info: CreateInfo,
  ) -> WindowResult<JoinHandle<WindowResult<()>>> {
    let thread_handle = std::thread::Builder::new().name("win32".to_owned()).spawn(
      move || -> WindowResult<()> {
        let sync = create_info.sync.clone();
        let message_sender = create_info.message_sender.clone();
        let (window, state) = Self::create_hwnd(create_info)?;

        window_sender
          .send(window)
          .expect("failed to send opened message");

        while Self::message_pump(&sync, &message_sender, &state) {}

        Ok(())
      },
    )?;

    Ok(thread_handle)
  }

  fn create_hwnd(
    mut create_info: CreateInfo,
  ) -> WindowResult<(Self, Handle<InternalState>)> {
    let hinstance: HINSTANCE = unsafe { GetModuleHandleW(None)? }.into();
    debug_assert_ne!(hinstance.0, 0);
    // let size = create_info.settings.size;
    // let position = create_info.settings.position;
    let title = HSTRING::from(create_info.settings.title.clone());
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
        get_window_ex_style(&create_info.style),
        &window_class,
        &title,
        get_window_style(&create_info.style) & !WindowsAndMessaging::WS_VISIBLE,
        WindowsAndMessaging::CW_USEDEFAULT,
        WindowsAndMessaging::CW_USEDEFAULT,
        WindowsAndMessaging::CW_USEDEFAULT,
        WindowsAndMessaging::CW_USEDEFAULT,
        None,
        None,
        hinstance,
        Some(std::ptr::addr_of_mut!(create_info) as _),
      )
    };

    if hwnd.0 == 0 {
      Err(WindowError::Win32Error(windows::core::Error::from_win32()))
    } else {
      let (window, state) = create_info.window.take().unwrap();

      Ok((window, state))
    }
  }

  fn message_pump(
    sync: &SyncData,
    message_sender: &Sender<Message>,
    state: &Handle<InternalState>,
  ) -> bool {
    if !message_sender.is_empty() {
      sync.wait_on_frame(|| state.get().stage == Stage::ExitLoop);
    }

    // pass message to main thread
    if let Err(_e) = message_sender.try_send(Message::Loop(message::LoopMessage::Wait)) {
      tracing::error!("{_e}");
      state.get_mut().stage = Stage::ExitLoop;
    }
    sync.signal_new_message();
    sync.wait_on_frame(|| state.get().stage == Stage::ExitLoop);

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

  fn take_message(&self) -> Option<Message> {
    let flow = self.state.get().flow;
    if let Flow::Wait = flow {
      let (lock, cvar) = self.sync.new_message.as_ref();
      let mut new = cvar.wait_while(lock.lock().unwrap(), |new| !*new).unwrap();
      *new = false;
    }

    // let msg = self.sync.next_message.lock().unwrap().take();
    self.message_receiver.try_recv().map_or_else(
      |e| match e {
        TryRecvError::Empty => Some(Message::Loop(LoopMessage::Empty)),
        TryRecvError::Disconnected => None,
      },
      Some,
    )
  }

  pub fn next_message(&self) -> Option<Message> {
    let current_stage = self.state.get().stage;

    self.sync.signal_next_frame();

    let next = match current_stage {
      Stage::Looping => {
        let message = self.take_message();
        if let Some(Message::CloseRequested) = message {
          let x = self.state.get().close_on_x;
          if x {
            self.close();
          }
        }
        message
      }
      Stage::Closing => {
        let _ = self.take_message();
        self.state.get_mut().stage = Stage::ExitLoop;
        Some(Message::Loop(message::LoopMessage::ExitLoop))
      }
      Stage::ExitLoop => None,
    };

    next
  }

  // GETTERS

  pub fn visibility(&self) -> Visibility {
    self.state.get().style.visibility
  }

  pub fn theme(&self) -> Theme {
    self.state.get().theme
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

  pub fn outer_size(&self) -> PhysicalSize {
    let mut window_rect = RECT::default();
    let _ = unsafe { GetWindowRect(self.hwnd, &mut window_rect) };
    PhysicalSize {
      width: (window_rect.right - window_rect.left) as u32,
      height: (window_rect.bottom - window_rect.top) as u32,
    }
  }

  pub fn inner_size(&self) -> PhysicalSize {
    let mut client_rect = RECT::default();
    let _ = unsafe { GetClientRect(self.hwnd, &mut client_rect) };
    PhysicalSize {
      width: (client_rect.right - client_rect.left) as u32,
      height: (client_rect.bottom - client_rect.top) as u32,
    }
  }

  pub fn outer_position(&self) -> PhysicalPosition {
    let mut window_rect = RECT::default();
    let _ = unsafe { GetWindowRect(self.hwnd, &mut window_rect) };
    PhysicalPosition {
      x: window_rect.left,
      y: window_rect.top,
    }
  }

  pub fn inner_position(&self) -> PhysicalPosition {
    let mut window_rect = RECT::default();
    let _ = unsafe { GetClientRect(self.hwnd, &mut window_rect) };
    PhysicalPosition {
      x: window_rect.left,
      y: window_rect.top,
    }
  }

  pub fn fullscreen(&self) -> Option<Fullscreen> {
    let state = self.state.get();
    state.style.fullscreen
  }

  pub fn cursor_screen_position(&self) -> PhysicalPosition {
    let mut pt = POINT::default();
    let _ = unsafe { GetCursorPos(std::ptr::addr_of_mut!(pt)) };
    PhysicalPosition { x: pt.x, y: pt.y }
  }

  pub fn scale_factor(&self) -> f64 {
    self.state.get().scale_factor
  }

  pub fn key(&self, keycode: Key) -> KeyState {
    let state = self.state.get();
    state.input.key(keycode)
  }

  pub fn mouse(&self, button: Mouse) -> ButtonState {
    let state = self.state.get();
    state.input.mouse(button)
  }

  pub fn shift(&self) -> ButtonState {
    let state = self.state.get();
    state.input.shift()
  }

  pub fn ctrl(&self) -> ButtonState {
    let state = self.state.get();
    state.input.ctrl()
  }

  pub fn alt(&self) -> ButtonState {
    let state = self.state.get();
    state.input.alt()
  }

  pub fn win(&self) -> ButtonState {
    let state = self.state.get();
    state.input.win()
  }

  pub fn is_closing(&self) -> bool {
    let state = self.state.get();
    state.is_closing()
  }

  // SETTERS

  fn force_set_outer_position(&self, position: Position) {
    self.state.get_mut().position = position;
    self.request(Command::SetPosition(position));
  }

  pub fn set_outer_position(&self, position: Position) {
    if position == self.state.get().position {
      return;
    }
    self.force_set_outer_position(position)
  }

  fn force_set_inner_size(&self, size: Size) {
    self.state.get_mut().size = size;
    self.request(Command::SetSize(size));
  }

  pub fn set_inner_size(&self, size: Size) {
    if size == self.state.get().size {
      return;
    }
    self.force_set_inner_size(size)
  }

  fn force_set_visibility(&self, visibility: Visibility) {
    self.state.get_mut().style.visibility = visibility;
    self.request(Command::SetVisibility(visibility));
  }

  pub fn set_visibility(&self, visibility: Visibility) {
    if visibility == self.state.get().style.visibility {
      return;
    }
    self.force_set_visibility(visibility)
  }

  fn force_set_decorations(&self, visibility: Visibility) {
    self.state.get_mut().style.decorations = visibility;
    self.request(Command::SetDecorations(visibility));
  }

  pub fn set_decorations(&self, visibility: Visibility) {
    if visibility == self.state.get().style.decorations {
      return;
    }
    self.force_set_decorations(visibility)
  }

  fn force_set_theme(&self, theme: Theme) {
    let theme = match theme {
      Theme::Auto => {
        if is_system_dark_mode_enabled() {
          Theme::Dark
        } else {
          Theme::Light
        }
      }
      Theme::Dark => {
        if is_dark_mode_supported() {
          Theme::Dark
        } else {
          Theme::Light
        }
      }
      Theme::Light => Theme::Light,
    };

    self.state.get_mut().theme = theme;
    let dark_mode = BOOL::from(theme == Theme::Dark);
    if let Err(_error) = unsafe {
      DwmSetWindowAttribute(
        self.hwnd,
        Dwm::DWMWA_USE_IMMERSIVE_DARK_MODE,
        std::ptr::addr_of!(dark_mode) as *const std::ffi::c_void,
        std::mem::size_of::<BOOL>() as u32,
      )
    } {
      tracing::error!("{_error}");
    };
  }

  pub fn set_theme(&self, theme: Theme) {
    if theme == self.state.get().theme {
      return;
    }
    self.force_set_theme(theme)
  }

  fn force_set_fullscreen(&self, fullscreen: Option<Fullscreen>) {
    self.state.get_mut().style.fullscreen = fullscreen;
    self.request(Command::SetFullscreen(fullscreen));
  }

  pub fn set_fullscreen(&self, fullscreen: Option<Fullscreen>) {
    if fullscreen == self.state.get().style.fullscreen {
      return;
    }
    self.force_set_fullscreen(fullscreen)
  }

  fn force_set_title(&self, title: impl AsRef<str>) {
    self.state.get_mut().title = title.as_ref().into();
    let title = HSTRING::from(format!("{}{}", title.as_ref(), self.state.get().subtitle));
    self.request(Command::SetWindowText(title));
  }

  /// Set the title of the window
  pub fn set_title(&self, title: impl AsRef<str>) {
    if title.as_ref() == self.state.get().title {
      return;
    }
    self.force_set_title(title)
  }

  fn force_set_cursor_mode(&self, cursor_mode: CursorMode) {
    self.state.get_mut().cursor_mode = cursor_mode;
    self.request(Command::SetCursorMode(cursor_mode));
  }

  pub fn set_cursor_mode(&self, cursor_mode: CursorMode) {
    if cursor_mode == self.state.get().cursor_mode {
      return;
    }
    self.force_set_cursor_mode(cursor_mode)
  }

  fn force_set_cursor_visibility(&self, cursor_visibility: Visibility) {
    self.state.get_mut().cursor_visibility = cursor_visibility;
    self.request(Command::SetCursorVisibility(cursor_visibility));
  }

  pub fn set_cursor_visibility(&self, cursor_visibility: Visibility) {
    if cursor_visibility == self.state.get().cursor_visibility {
      return;
    }
    self.force_set_cursor_visibility(cursor_visibility)
  }

  fn force_set_subtitle(&self, subtitle: impl AsRef<str>) {
    self.state.get_mut().subtitle = subtitle.as_ref().into();
    let title = HSTRING::from(format!("{}{}", self.state.get().title, subtitle.as_ref()));
    self.request(Command::SetWindowText(title));
  }

  /// Set text to appear after the title of the window
  pub fn set_subtitle(&self, subtitle: impl AsRef<str>) {
    if subtitle.as_ref() == self.state.get().subtitle {
      return;
    }
    self.force_set_subtitle(subtitle)
  }

  fn force_request_redraw(&self) {
    self.state.get_mut().requested_redraw = true;
    self.request(Command::Redraw);
  }

  /// Request a new Draw event
  pub fn request_redraw(&self) {
    if self.state.get().requested_redraw {
      return;
    }
    self.force_request_redraw()
  }

  /// Request the window be closed
  pub fn close(&self) {
    if self.is_closing() {
      return; // already closing
    }
    self.state.get_mut().stage = Stage::Closing;
  }

  fn request(&self, command: Command) {
    let err_str = format!("failed to post command `{command:?}`");

    self.command_queue.push(command);

    unsafe { PostMessageW(self.hwnd, WindowsAndMessaging::WM_APP, WPARAM(0), LPARAM(0)) }
      .unwrap_or_else(|e| panic!("{}: {e}", err_str));
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
