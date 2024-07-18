use std::{
  collections::VecDeque,
  sync::{mpsc::SyncSender, Arc, Condvar, Mutex},
  thread::JoinHandle,
};

use cursor_icon::CursorIcon;
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
    Graphics::{
      Dwm::{self, DwmSetWindowAttribute},
      Gdi::{
        self,
        EnumDisplayMonitors,
        MonitorFromPoint,
        MonitorFromWindow,
        HDC,
        HMONITOR,
      },
    },
    System::LibraryLoader::GetModuleHandleW,
    UI::{
      HiDpi::{
        AdjustWindowRectExForDpi,
        SetProcessDpiAwarenessContext,
        DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE,
        DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
      },
      WindowsAndMessaging::{
        self,
        CreateWindowExW,
        DispatchMessageW,
        GetClientRect,
        GetCursorPos,
        GetMessageW,
        GetWindowRect,
        LoadCursorW,
        RegisterClassExW,
        TranslateMessage,
        MSG,
        WNDCLASSEXW,
      },
    },
  },
};

use self::{
  command::Command,
  data::{CursorMode, Fullscreen, PhysicalSize, Position},
  message::LoopMessage,
  settings::WindowBuilder,
  stage::Stage,
};
use crate::{
  error::WindowError,
  prelude::{ButtonState, Key, KeyState, MouseButton},
  utilities::{
    get_window_ex_style,
    get_window_style,
    hwnd_dpi,
    is_dark_mode_supported,
    is_system_dark_mode_enabled,
    Monitor,
  },
  window::{
    data::{Flow, Internal, PhysicalPosition, Size, SyncData, Theme, Visibility},
    frame::Style,
    input::Input,
    message::Message,
    procedure::CreateInfo,
    settings::WindowSettings,
  },
};

mod command;
pub mod cursor;
pub mod data;
pub mod frame;
pub mod input;
pub mod message;
pub mod monitor;
pub mod procedure;
pub mod settings;
pub mod stage;

/// Main window class. Uses internal mutability. Window is destroyed on drop. Cloning does not create a new window,
/// but instead clones the smart pointer handle to the same window.
#[allow(unused)]
#[derive(Clone)]
pub struct Window(Arc<Internal>);

impl Window {
  pub const WINDOW_SUBCLASS_ID: usize = 0;

  /// Create a new [`WindowBuilder`] to set up a [`Window`].
  ///
  /// [`WindowBuilder::with_size`] is relative to the whole window frame,
  /// not just the client area. I recommend spawning the window
  /// hidden and using [`Window::set_inner_size`] if you need a
  /// specific size for the client area.
  pub fn builder() -> WindowBuilder {
    WindowBuilder::default()
  }

  pub(crate) fn new(
    title: impl Into<String>,
    size: impl Into<Size>,
    position: impl Into<Option<Position>>,
    settings: WindowSettings,
  ) -> Result<Self, WindowError> {
    let title: String = title.into();
    let size: Size = size.into();
    let position: Option<Position> = position.into();

    tracing::trace!("[`{}`]: creating window", &title);

    let sync = SyncData {
      new_message: Arc::new((Mutex::new(false), Condvar::new())),
      next_frame: Arc::new((Mutex::new(true), Condvar::new())),
      skip_wait: Arc::new(Mutex::new(true)),
    };

    let create_info = CreateInfo {
      title: title.clone(),
      size,
      position,
      settings: settings.clone(),
      class_atom: 0,
      window: None,
      message: Arc::new(Mutex::new(None)),
      sync: sync.clone(),
      style: Style {
        visibility: settings.visibility,
        decorations: settings.decorations,
        fullscreen: settings.fullscreen,
        resizeable: settings.resizeable,
        minimized: false,
        maximized: false,
        active: false,
        focused: false,
      },
    };

    let (window_sender, window_receiver) = std::sync::mpsc::sync_channel(0);

    let thread = Some(Self::window_loop(window_sender, create_info)?);

    tracing::trace!("[`{}`]: waiting for window loop to hand back window", &title);

    let window = window_receiver.recv().unwrap();

    tracing::trace!("[`{}`]: received window from window loop", &title);

    window.0.set_thread(thread);

    tracing::trace!("[`{}`]: created window", &title);

    Ok(window)
  }

  fn window_loop(
    window_sender: SyncSender<Self>,
    create_info: CreateInfo,
  ) -> Result<JoinHandle<Result<(), WindowError>>, WindowError> {
    let thread_handle = std::thread::Builder::new()
      .name("window".to_owned())
      .spawn(move || -> Result<(), WindowError> {
        let title = create_info.title.clone();
        // let flow = create_info.settings.flow;
        let window = Self::create_hwnd(create_info)?;

        tracing::trace!("[`{}`]: sending window back to main thread", title);
        window_sender.send(window).expect("failed to send window");

        tracing::trace!("[`{}`]: pumping messages", title);

        while Self::message_pump() {}

        tracing::trace!("[`{}`]: joining main thread", title);
        Ok(())
      })?;

    Ok(thread_handle)
  }

  fn create_hwnd(mut create_info: CreateInfo) -> Result<Self, WindowError> {
    tracing::trace!("[`{}`]: creating window class", &create_info.title);

    let hinstance: HINSTANCE = unsafe { GetModuleHandleW(None)? }.into();
    debug_assert!(!hinstance.is_invalid());
    let title = HSTRING::from(create_info.title.clone());
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

    tracing::trace!("[`{}`]: registering window class", &create_info.title);

    {
      create_info.class_atom = unsafe { RegisterClassExW(&wc) };
      debug_assert_ne!(create_info.class_atom, 0);
    }

    tracing::trace!("[`{}`]: creating window handle", &create_info.title);

    if unsafe {
      SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)
    }
    .is_err()
    {
      unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE) }
        .unwrap();
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
    }?;

    tracing::trace!("[`{}`]: window handle created", &create_info.title);

    if hwnd.is_invalid() {
      Err(WindowError::Win32Error(windows::core::Error::from_win32()))
    } else {
      let window = create_info.window.take().unwrap();

      Ok(window)
    }
  }

  fn message_pump() -> bool {
    let mut msg = MSG::default();
    if unsafe { GetMessageW(&mut msg, None, 0, 0) }.as_bool() {
      unsafe {
        let _ = TranslateMessage(&msg);
        DispatchMessageW(&msg);
      }
      true
    } else {
      false
    }
  }

  fn take_message(&self) -> Option<Message> {
    let flow = self.0.data.lock().unwrap().flow;
    if let Flow::Wait = flow {
      let should_wait = self.0.message.lock().unwrap().is_none();
      if should_wait {
        let (lock, cvar) = self.0.sync.new_message.as_ref();
        let mut new = cvar.wait_while(lock.lock().unwrap(), |new| !*new).unwrap();
        *new = false;
      }
    }

    self
      .0
      .message
      .lock()
      .unwrap()
      .take()
      .or(Some(Message::Loop(LoopMessage::Empty)))
  }

  fn next_message(&self) -> Option<Message> {
    self.0.sync.signal_next_frame();

    let current_stage = self.0.data.lock().unwrap().stage;
    let next = match current_stage {
      Stage::Setup | Stage::Ready | Stage::Destroyed => None,
      Stage::Looping | Stage::Closing => {
        let message = self.take_message();
        match message {
          Some(Message::CloseRequested) => {
            let x = self.0.data.lock().unwrap().close_on_x;
            if x {
              self.close();
            }
          }
          Some(Message::Loop(LoopMessage::Exit)) => {
            *self.0.sync.skip_wait.lock().unwrap() = true;
            self.0.data.lock().unwrap().stage = Stage::ExitLoop;
          }
          _ => (),
        }
        message
      }
      Stage::ExitLoop => {
        tracing::trace!("[`{}`]: exiting loop", self.title());
        None
      }
    };

    next
  }

  /// Request the window be closed
  pub fn close(&self) {
    if self.is_closing() {
      return; // already closing
    }
    tracing::trace!("[`{}`]: closing window", self.title());
    self.0.data.lock().unwrap().stage = Stage::Closing;
    Command::Exit.post(self.0.hwnd);
  }

  // GETTERS

  pub fn is_closing(&self) -> bool {
    self.0.is_closing()
  }

  pub fn visibility(&self) -> Visibility {
    self.0.data.lock().unwrap().style.visibility
  }

  pub fn theme(&self) -> Theme {
    self.0.data.lock().unwrap().theme
  }

  pub fn flow(&self) -> Flow {
    self.0.data.lock().unwrap().flow
  }

  pub fn title(&self) -> String {
    self.0.data.lock().unwrap().title.to_string()
  }

  pub fn subtitle(&self) -> String {
    self.0.data.lock().unwrap().subtitle.to_string()
  }

  pub fn outer_size(&self) -> PhysicalSize {
    let mut window_rect = RECT::default();
    let _ = unsafe { GetWindowRect(HWND(self.0.hwnd as _), &mut window_rect) };
    PhysicalSize {
      width: (window_rect.right - window_rect.left) as u32,
      height: (window_rect.bottom - window_rect.top) as u32,
    }
  }

  pub fn inner_size(&self) -> PhysicalSize {
    let mut client_rect = RECT::default();
    let _ = unsafe { GetClientRect(HWND(self.0.hwnd as _), &mut client_rect) };
    PhysicalSize {
      width: (client_rect.right - client_rect.left) as u32,
      height: (client_rect.bottom - client_rect.top) as u32,
    }
  }

  pub fn outer_position(&self) -> PhysicalPosition {
    let mut window_rect = RECT::default();
    let _ = unsafe { GetWindowRect(HWND(self.0.hwnd as _), &mut window_rect) };
    PhysicalPosition {
      x: window_rect.left,
      y: window_rect.top,
    }
  }

  pub fn inner_position(&self) -> PhysicalPosition {
    let mut window_rect = RECT::default();
    let _ = unsafe { GetClientRect(HWND(self.0.hwnd as _), &mut window_rect) };
    PhysicalPosition {
      x: window_rect.left,
      y: window_rect.top,
    }
  }

  pub fn fullscreen(&self) -> Option<Fullscreen> {
    self.0.data.lock().unwrap().style.fullscreen
  }

  pub fn cursor_screen_position(&self) -> PhysicalPosition {
    let mut pt = POINT::default();
    let _ = unsafe { GetCursorPos(std::ptr::addr_of_mut!(pt)) };
    PhysicalPosition { x: pt.x, y: pt.y }
  }

  pub fn has_focus(&self) -> bool {
    let style = &self.0.data.lock().unwrap().style;
    style.focused && style.active
  }

  pub fn scale_factor(&self) -> f64 {
    self.0.data.lock().unwrap().scale_factor
  }

  unsafe extern "system" fn monitor_enum_proc(
    hmonitor: HMONITOR,
    _hdc: HDC,
    _place: *mut RECT,
    data: LPARAM,
  ) -> BOOL {
    let monitors = data.0 as *mut VecDeque<HMONITOR>;
    unsafe { (*monitors).push_back(hmonitor) };
    true.into() // continue enumeration
  }

  pub fn available_monitors(&self) -> VecDeque<Monitor> {
    let mut monitors: VecDeque<HMONITOR> = VecDeque::new();
    unsafe {
      EnumDisplayMonitors(
        HDC::default(),
        None,
        Some(Self::monitor_enum_proc),
        LPARAM(&mut monitors as *mut _ as _),
      );
    }

    monitors.into_iter().map(Monitor::new).collect()
  }

  pub fn current_monitor(&self) -> Monitor {
    let hmonitor =
      unsafe { MonitorFromWindow(HWND(self.0.hwnd as _), Gdi::MONITOR_DEFAULTTONEAREST) };
    Monitor::new(hmonitor)
  }

  pub fn primary_monitor(&self) -> Monitor {
    const ORIGIN: POINT = POINT { x: 0, y: 0 };
    let hmonitor = unsafe { MonitorFromPoint(ORIGIN, Gdi::MONITOR_DEFAULTTOPRIMARY) };
    Monitor::new(hmonitor)
  }

  pub fn key(&self, keycode: Key) -> KeyState {
    self.0.data.lock().unwrap().input.key(keycode)
  }

  pub fn mouse(&self, button: MouseButton) -> ButtonState {
    self.0.data.lock().unwrap().input.mouse(button)
  }

  pub fn shift(&self) -> ButtonState {
    self.0.data.lock().unwrap().input.shift()
  }

  pub fn ctrl(&self) -> ButtonState {
    self.0.data.lock().unwrap().input.ctrl()
  }

  pub fn alt(&self) -> ButtonState {
    self.0.data.lock().unwrap().input.alt()
  }

  pub fn win(&self) -> ButtonState {
    self.0.data.lock().unwrap().input.win()
  }

  pub fn is_minimized(&self) -> bool {
    self.0.data.lock().unwrap().style.minimized
  }

  pub fn is_maximized(&self) -> bool {
    self.0.data.lock().unwrap().style.maximized
  }

  // SETTERS

  fn force_set_cursor_icon(&self, cursor_icon: CursorIcon) {
    // self.state.write_lock().position = position;
    Command::SetCursorIcon(cursor_icon).post(self.0.hwnd);
  }

  pub fn set_cursor_icon(&self, cursor_icon: CursorIcon) {
    let selected_icon = self.0.data.lock().unwrap().cursor.selected_icon;
    if selected_icon == cursor_icon {
      return;
    }
    self.force_set_cursor_icon(cursor_icon)
  }

  fn force_set_outer_position(&self, position: Position) {
    // self.state.write_lock().position = position;
    Command::SetPosition(position).post(self.0.hwnd);
  }

  pub fn set_outer_position(&self, position: Position) {
    let scale_factor = self.0.data.lock().unwrap().scale_factor;
    if position.as_physical(scale_factor) == self.outer_position() {
      return;
    }
    self.force_set_outer_position(position)
  }

  fn force_set_outer_size(&self, size: Size) {
    // self.state.write_lock().size = size;
    Command::SetSize(size).post(self.0.hwnd);
  }

  pub fn set_outer_size(&self, size: impl Into<Size>) {
    let size = size.into();
    let scale_factor = self.0.data.lock().unwrap().scale_factor;
    if size.as_physical(scale_factor) == self.outer_size() {
      return;
    }
    self.force_set_outer_size(size)
  }

  fn force_set_inner_size(&self, size: Size) {
    let scale_factor = self.0.data.lock().unwrap().scale_factor;
    let physical_size = size.as_physical(scale_factor);
    let style = self.0.data.lock().unwrap().style.clone();
    let mut window_rect = RECT {
      top: 0,
      left: 0,
      right: physical_size.width as i32,
      bottom: physical_size.height as i32,
    };
    unsafe {
      AdjustWindowRectExForDpi(
        &mut window_rect,
        get_window_style(&style),
        false,
        get_window_ex_style(&style),
        hwnd_dpi(HWND(self.0.hwnd as _)),
      )
    }
    .unwrap();

    let adjusted_size = PhysicalSize {
      width: (window_rect.right - window_rect.left) as u32,
      height: (window_rect.bottom - window_rect.top) as u32,
    };

    Command::SetSize(adjusted_size.into()).post(self.0.hwnd);
  }

  pub fn set_inner_size(&self, size: impl Into<Size>) {
    let size = size.into();
    let scale_factor = self.0.data.lock().unwrap().scale_factor;
    if size.as_physical(scale_factor) == self.inner_size() {
      return;
    }
    self.force_set_inner_size(size)
  }

  fn force_set_visibility(&self, visibility: Visibility) {
    self.0.data.lock().unwrap().style.visibility = visibility;
    Command::SetVisibility(visibility).post(self.0.hwnd);
  }

  pub fn set_visibility(&self, visibility: Visibility) {
    if visibility == self.0.data.lock().unwrap().style.visibility {
      return;
    }
    self.force_set_visibility(visibility)
  }

  fn force_set_decorations(&self, visibility: Visibility) {
    self.0.data.lock().unwrap().style.decorations = visibility;
    Command::SetDecorations(visibility).post(self.0.hwnd);
  }

  pub fn set_decorations(&self, visibility: Visibility) {
    if visibility == self.0.data.lock().unwrap().style.decorations {
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

    self.0.data.lock().unwrap().theme = theme;
    let dark_mode = BOOL::from(theme == Theme::Dark);
    if let Err(_error) = unsafe {
      DwmSetWindowAttribute(
        HWND(self.0.hwnd as _),
        Dwm::DWMWA_USE_IMMERSIVE_DARK_MODE,
        std::ptr::addr_of!(dark_mode) as *const std::ffi::c_void,
        std::mem::size_of::<BOOL>() as u32,
      )
    } {
      tracing::error!("{_error}");
    };
  }

  pub fn set_theme(&self, theme: Theme) {
    if theme == self.0.data.lock().unwrap().theme {
      return;
    }
    self.force_set_theme(theme)
  }

  fn force_set_fullscreen(&self, fullscreen: Option<Fullscreen>) {
    self.0.data.lock().unwrap().style.fullscreen = fullscreen;
    Command::SetFullscreen(fullscreen).post(self.0.hwnd);
  }

  pub fn set_fullscreen(&self, fullscreen: Option<Fullscreen>) {
    if fullscreen == self.0.data.lock().unwrap().style.fullscreen {
      return;
    }
    self.force_set_fullscreen(fullscreen)
  }

  fn force_set_title(&self, title: impl AsRef<str>) {
    self.0.data.lock().unwrap().title = title.as_ref().into();
    let title = HSTRING::from(format!(
      "{}{}",
      title.as_ref(),
      self.0.data.lock().unwrap().subtitle
    ));
    Command::SetWindowText(title).post(self.0.hwnd);
  }

  /// Set the title of the window
  pub fn set_title(&self, title: impl AsRef<str>) {
    if title.as_ref() == self.0.data.lock().unwrap().title {
      return;
    }
    self.force_set_title(title)
  }

  fn force_set_cursor_mode(&self, cursor_mode: CursorMode) {
    self.0.data.lock().unwrap().cursor.mode = cursor_mode;
    Command::SetCursorMode(cursor_mode).post(self.0.hwnd);
  }

  pub fn set_cursor_mode(&self, cursor_mode: CursorMode) {
    if cursor_mode == self.0.data.lock().unwrap().cursor.mode {
      return;
    }
    self.force_set_cursor_mode(cursor_mode)
  }

  fn force_set_cursor_visibility(&self, cursor_visibility: Visibility) {
    self.0.data.lock().unwrap().cursor.visibility = cursor_visibility;
    Command::SetCursorVisibility(cursor_visibility).post(self.0.hwnd);
  }

  pub fn set_cursor_visibility(&self, cursor_visibility: Visibility) {
    if cursor_visibility == self.0.data.lock().unwrap().cursor.visibility {
      return;
    }
    self.force_set_cursor_visibility(cursor_visibility)
  }

  fn force_set_subtitle(&self, subtitle: impl AsRef<str>) {
    self.0.data.lock().unwrap().subtitle = subtitle.as_ref().into();
    let title = HSTRING::from(format!(
      "{}{}",
      self.0.data.lock().unwrap().title,
      subtitle.as_ref()
    ));
    Command::SetWindowText(title).post(self.0.hwnd);
  }

  /// Set text to appear after the title of the window
  pub fn set_subtitle(&self, subtitle: impl AsRef<str>) {
    if subtitle.as_ref() == self.0.data.lock().unwrap().subtitle {
      return;
    }
    self.force_set_subtitle(subtitle)
  }

  fn force_request_redraw(&self) {
    self.0.data.lock().unwrap().requested_redraw = true;
    Command::Redraw.post(self.0.hwnd);
  }

  /// Request a new Draw event
  pub fn request_redraw(&self) {
    if self.0.data.lock().unwrap().requested_redraw {
      return;
    }
    self.force_request_redraw()
  }

  #[cfg(all(feature = "rwh_06", not(feature = "rwh_05")))]
  pub fn raw_window_handle(&self) -> RawWindowHandle {
    let mut handle = Win32WindowHandle::new(
      std::num::NonZeroIsize::new(self.0.hwnd as _)
        .expect("window handle should not be zero"),
    );
    let hinstance = std::num::NonZeroIsize::new(self.0.hinstance as _)
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
    handle.hwnd = self.0.hwnd.0 as *mut std::ffi::c_void;
    handle.hinstance = self.0.hinstance.0 as *mut std::ffi::c_void;
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
  fn iter(&self) -> MessageIterator {
    let current_stage = self.0.data.lock().unwrap().stage;
    match current_stage {
      Stage::Ready => {
        tracing::trace!(
          "[`{}`]: preparing to immutably iterate over messages",
          self.title()
        );
        self.0.data.lock().unwrap().stage = Stage::Looping;
      }
      Stage::ExitLoop => {
        tracing::error!(
          "[`{}`]: attempted to iterate over window already in the ExitLoop stage",
          self.title()
        )
      }
      _ => tracing::warn!(
        "[`{}`]: iterating over window which wasn't in the Ready stage",
        self.title()
      ),
    }
    MessageIterator { window: self }
  }

  fn iter_mut(&mut self) -> MessageIteratorMut {
    let current_stage = self.0.data.lock().unwrap().stage;
    match current_stage {
      Stage::Ready => {
        tracing::trace!(
          "[`{}`]: preparing to mutably iterate over messages",
          self.title()
        );
        self.0.data.lock().unwrap().stage = Stage::Looping;
      }
      Stage::ExitLoop => {
        tracing::error!(
          "[`{}`]: attempted to iterate over window already in the ExitLoop stage",
          self.title()
        )
      }
      _ => tracing::warn!(
        "[`{}`]: iterating over window which wasn't in the Ready stage",
        self.title()
      ),
    }
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

#[cfg(feature = "egui")]
impl Window {
  pub fn create_egui_state(
    &self,
    egui_ctx: egui::Context,
    viewport_id: egui::ViewportId,
    max_texture_size: Option<usize>,
  ) -> crate::compat::egui::State {
    crate::compat::egui::State::new(
      egui_ctx,
      viewport_id,
      self,
      Some(self.scale_factor() as f32),
      max_texture_size,
    )
  }
}
