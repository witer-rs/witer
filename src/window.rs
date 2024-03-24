use std::{
  collections::VecDeque,
  sync::{Arc, Barrier, Condvar, Mutex},
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

// use windows::{
//   core::{HSTRING, PCWSTR},
//   Win32::{
//     Foundation::*,
//     Graphics::{
//       Dwm::{self, DwmSetWindowAttribute},
//       Gdi::{
//         self,
//         EnumDisplayMonitors,
//         MonitorFromPoint,
//         MonitorFromWindow,
//         HDC,
//         HMONITOR,
//       },
//     },
//     System::LibraryLoader::GetModuleHandleW,
//     UI::{
//       HiDpi::AdjustWindowRectExForDpi,
//       WindowsAndMessaging::{
//         self,
//         CreateWindowExW,
//         DispatchMessageW,
//         GetClientRect,
//         GetCursorPos,
//         GetMessageW,
//         GetWindowRect,
//         LoadCursorW,
//         PostMessageW,
//         RegisterClassExW,
//         TranslateMessage,
//         MSG,
//         WNDCLASSEXW,
//       },
//     },
//   },
// };
use self::{
  command::Command,
  // message::LoopMessage,
  procedure::SyncData,
  settings::WindowBuilder,
  stage::Stage,
  // state::{CursorMode, Fullscreen, PhysicalSize, Position, StyleInfo},
};
use crate::{
  error::{WindowError, WindowResult},
  handle::Handle,
  // prelude::{ButtonState, Key, KeyState, MouseButton},
  // utilities::{
  //   get_window_ex_style,
  //   get_window_style,
  //   hwnd_dpi,
  //   is_dark_mode_supported,
  //   is_system_dark_mode_enabled,
  //   Monitor,
  // },
  window::{
    // input::Input,
    // message::Message,
    settings::WindowSettings,
    // state::{Flow, InternalState, PhysicalPosition, Size, Theme, Visibility},
  },
};

mod command;
// pub mod input;
pub mod message;
pub mod procedure;
pub mod settings;
pub mod stage;
pub mod state;

use winit::{
  dpi::{PhysicalPosition, PhysicalSize, Position, Size},
  event::{Event, WindowEvent},
  event_loop::{
    ControlFlow,
    EventLoop,
    EventLoopBuilder,
    EventLoopProxy,
    EventLoopWindowTarget,
  },
  platform::windows::EventLoopBuilderExtWindows,
  window::{
    CursorGrabMode,
    Fullscreen,
    Theme,
    Window as WinitWindow,
    WindowBuilder as WinitWindowBuilder,
  },
};

#[derive(Debug, Clone)]
pub enum WiterEvent<T: 'static> {
  Command(Command),
  User(T),
}

struct CreateInfo {
  pub title: String,
  pub size: Size,
  pub position: Option<Position>,
  pub settings: WindowSettings,
  pub sync: SyncData,
  pub command_queue: Arc<SegQueue<Command>>,
  pub message_sender: Sender<Event<WiterEvent<()>>>,
}

/// Main window class. Uses internal mutability. Window is destroyed on drop.
#[allow(unused)]
pub struct Window {
  // hinstance: HINSTANCE,
  // hwnd: HWND,
  winit: Arc<WinitWindow>,
  proxy: EventLoopProxy<WiterEvent<()>>,
  thread: Arc<Mutex<Option<JoinHandle<WindowResult<()>>>>>,
  sync: SyncData,
  command_queue: Arc<SegQueue<Command>>,
  message_receiver: Receiver<Event<WiterEvent<()>>>,
  // input: Arc<Mutex<Input>>,
  stage: Arc<Mutex<Stage>>,
}

/// Window is destroyed on drop.
impl Drop for Window {
  fn drop(&mut self) {
    // self.request(Command::Destroy);
    self.sync.exit_sync.wait();
    let thread = self.thread.lock().unwrap().take();
    if let Some(thread) = thread {
      thread.join().unwrap();
    }
  }
}

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
    let (message_sender, message_receiver) = crossbeam::channel::unbounded();

    let sync = SyncData {
      new_message: Arc::new((Mutex::new(false), Condvar::new())),
      next_frame: Arc::new((Mutex::new(false), Condvar::new())),
      exit_sync: Arc::new(Barrier::new(2)),
    };
    let command_queue = Arc::new(SegQueue::new());

    let title: String = title.into();
    let size: Size = size.into();
    let position: Option<Position> = position.into();

    let create_info = CreateInfo {
      title,
      size,
      position,
      settings: settings.clone(),
      sync: sync.clone(),
      command_queue: command_queue.clone(),
      message_sender,
    };

    let (window_sender, window_receiver) = crossbeam::channel::bounded(0);

    let thread = Some(Self::window_loop(window_sender, create_info)?);

    let (winit, proxy) = window_receiver.recv().unwrap();

    let window = Self {
      winit,
      proxy,
      thread: Arc::new(Mutex::new(thread)),
      sync,
      command_queue,
      message_receiver,
      // input: Arc::new(Mutex::new(Input::new())),
      stage: Arc::new(Mutex::new(Stage::Looping)),
    };

    if let Some(position) = position {
      window.force_set_outer_position(position);
    }
    window.force_set_inner_size(size);
    window.force_set_decorated(settings.decorations);
    window.force_set_theme(settings.theme);
    window.force_set_visible(settings.visible);
    window.force_set_fullscreen(settings.fullscreen);

    Ok(window)
  }

  fn window_loop(
    window_sender: Sender<(Arc<WinitWindow>, EventLoopProxy<WiterEvent<()>>)>,
    create_info: CreateInfo,
  ) -> WindowResult<JoinHandle<WindowResult<()>>> {
    let thread_handle = std::thread::Builder::new()
      .name("window".to_owned())
      .spawn(move || -> WindowResult<()> {
        let sync = create_info.sync.clone();
        let sync_exit = create_info.sync.clone();
        let message_sender = create_info.message_sender.clone();
        let event_loop = EventLoopBuilder::<WiterEvent<()>>::with_user_event()
          .with_any_thread(true)
          .build()
          .unwrap();

        let proxy = event_loop.create_proxy();

        let winit = Arc::new(WinitWindowBuilder::new().build(&event_loop).unwrap());
        event_loop.set_control_flow(create_info.settings.flow);

        window_sender
          .send((winit.clone(), proxy))
          .expect("failed to send window to main thread");

        // let mut processing_command = false;
        event_loop
          .run(move |event, elwt| {
            // if processing_command {
            //   return;
            // }

            // #[allow(unused_assignments)]
            // while let Some(command) = create_info.command_queue.pop() {
            //   // not sure if this is necessary with the winit version, but this
            // prevents   // recursively entering the event_loop and executing
            // commands out of order.   processing_command = true;

            //   Self::process_command(&winit, elwt, command);

            //   processing_command = false;
            // }

            if let Event::UserEvent(WiterEvent::Command(command)) = &event {
              match command {
                Command::Close => {
                  elwt.exit();
                }
                Command::Redraw => {
                  winit.request_redraw();
                }
                Command::SetVisibility(visibility) => {
                  winit.set_visible(*visibility);
                }
                Command::SetDecorations(decorations) => {}
                Command::SetWindowText(text) => {}
                Command::SetSize(size) => {}
                Command::SetPosition(position) => {}
                Command::SetFullscreen(fullscreen) => {}
                Command::SetCursorMode(mode) => {}
                Command::SetCursorVisibility(visibility) => {}
              }
            }

            if !message_sender.is_empty() {
              sync.wait_on_frame(|| elwt.exiting());
            }

            if let Event::WindowEvent {
              event: WindowEvent::CloseRequested,
              ..
            } = event
            {
              if create_info.settings.close_on_x {
                elwt.exit();
              }
            }

            message_sender.try_send(event).unwrap();
            sync.signal_new_message();
            sync.wait_on_frame(|| elwt.exiting());
          })
          .unwrap();

        // wait for window to be dropped before exiting the thread to prevent the window
        // from prematurely closing
        sync_exit.exit_sync.wait();

        Ok(())
      })?;

    Ok(thread_handle)
  }

  fn process_command(
    winit: &Arc<WinitWindow>,
    elwt: &EventLoopWindowTarget<WiterEvent<()>>,
    command: Command,
  ) {
    match command {
      Command::Close => {
        elwt.exit();
      }
      Command::Redraw => {
        winit.request_redraw();
      }
      Command::SetVisibility(visibility) => {
        winit.set_visible(visibility);
      }
      Command::SetDecorations(decorations) => {}
      Command::SetWindowText(text) => {}
      Command::SetSize(size) => {}
      Command::SetPosition(position) => {}
      Command::SetFullscreen(fullscreen) => {}
      Command::SetCursorMode(mode) => {}
      Command::SetCursorVisibility(visibility) => {}
    }
  }

  // fn message_pump(
  //   sync: &SyncData,
  //   message_sender: &Sender<Message>,
  //   state: &Handle<InternalState>,
  // ) -> bool {
  //   if !message_sender.is_empty() {
  //     sync.wait_on_frame(|| state.read_lock().stage == Stage::ExitLoop);
  //   }

  //   // pass message to main thread
  //   if let Err(_e) =
  // message_sender.try_send(Message::Loop(message::LoopMessage::Wait)) {
  //     tracing::error!("{_e}");
  //     state.write_lock().stage = Stage::ExitLoop;
  //   }
  //   sync.signal_new_message();
  //   sync.wait_on_frame(|| state.read_lock().stage == Stage::ExitLoop);

  //   let mut msg = MSG::default();
  //   if unsafe { GetMessageW(&mut msg, None, 0, 0).as_bool() } {
  //     unsafe {
  //       TranslateMessage(&msg);
  //       DispatchMessageW(&msg);
  //     }
  //     true
  //   } else {
  //     false
  //   }
  // }

  // fn take_message(&self) -> Option<Message> {
  //   let flow = self.state.read_lock().flow;
  //   if let Flow::Wait = flow {
  //     let (lock, cvar) = self.sync.new_message.as_ref();
  //     let mut new = cvar.wait_while(lock.lock().unwrap(), |new|
  // !*new).unwrap();     *new = false;
  //   }

  //   // let msg = self.sync.next_message.lock().unwrap().take();
  //   self.message_receiver.try_recv().map_or_else(
  //     |e| match e {
  //       TryRecvError::Empty => Some(Message::Loop(LoopMessage::Empty)),
  //       TryRecvError::Disconnected => None,
  //     },
  //     Some,
  //   )
  // }

  pub fn next_event(&self) -> Option<Event<WiterEvent<()>>> {
    let current_stage = *self.stage.lock().unwrap();

    self.sync.signal_next_frame();

    let next = match current_stage {
      Stage::Looping => {
        let message = self.message_receiver.recv().unwrap();
        if let Event::WindowEvent {
          event: WindowEvent::CloseRequested,
          ..
        } = message
        {
          // let x = self.state.read_lock().close_on_x;
          // if x {
          self.close();
          // }
        }
        Some(message)
      }
      Stage::Closing => {
        let message = self.message_receiver.recv().unwrap();
        *self.stage.lock().unwrap() = Stage::ExitLoop;
        Some(message)
      }
      Stage::ExitLoop => None,
    };

    next
  }

  // GETTERS

  pub fn visible(&self) -> bool {
    self.winit.is_visible().unwrap()
  }

  pub fn theme(&self) -> Option<Theme> {
    self.winit.theme()
  }

  pub fn title(&self) -> String {
    "stub".to_owned()
  }

  pub fn subtitle(&self) -> String {
    "stub".to_owned()
  }

  pub fn outer_size(&self) -> PhysicalSize<u32> {
    self.winit.outer_size()
  }

  pub fn inner_size(&self) -> PhysicalSize<u32> {
    self.winit.inner_size()
  }

  pub fn outer_position(&self) -> PhysicalPosition<i32> {
    self.winit.outer_position().unwrap()
  }

  pub fn inner_position(&self) -> PhysicalPosition<i32> {
    self.winit.inner_position().unwrap()
  }

  pub fn fullscreen(&self) -> Option<Fullscreen> {
    self.winit.fullscreen()
  }

  pub fn scale_factor(&self) -> f64 {
    self.winit.scale_factor()
  }

  pub fn available_monitors(
    &self,
  ) -> impl Iterator<Item = winit::monitor::MonitorHandle> {
    self.winit.available_monitors()
  }

  pub fn current_monitor(&self) -> Option<winit::monitor::MonitorHandle> {
    self.winit.current_monitor()
  }

  pub fn primary_monitor(&self) -> Option<winit::monitor::MonitorHandle> {
    self.winit.primary_monitor()
  }

  // pub fn key(&self, keycode: Key) -> KeyState {
  //   let state = self.state.read_lock();
  //   state.input.key(keycode)
  // }

  // pub fn mouse(&self, button: MouseButton) -> ButtonState {
  //   let state = self.state.read_lock();
  //   state.input.mouse(button)
  // }

  // pub fn shift(&self) -> ButtonState {
  //   let state = self.state.read_lock();
  //   state.input.shift()
  // }

  // pub fn ctrl(&self) -> ButtonState {
  //   let state = self.state.read_lock();
  //   state.input.ctrl()
  // }

  // pub fn alt(&self) -> ButtonState {
  //   let state = self.state.read_lock();
  //   state.input.alt()
  // }

  // pub fn win(&self) -> ButtonState {
  //   let state = self.state.read_lock();
  //   state.input.win()
  // }

  pub fn is_closing(&self) -> bool {
    let stage = self.stage.lock().unwrap();
    matches!(*stage, Stage::Closing | Stage::ExitLoop)
  }

  // SETTERS

  fn force_set_outer_position(&self, position: Position) {
    // self.state.write_lock().position = position;
    self.request(Command::SetPosition(position));
  }

  pub fn set_outer_position(&self, position: Position) {
    // let scale_factor = self.state.read_lock().scale_factor;
    // if position.as_physical(scale_factor) == self.outer_position() {
    //   return;
    // }
    self.force_set_outer_position(position)
  }

  // fn force_set_outer_size(&self, size: Size) {
  //   // self.state.write_lock().size = size;
  //   self.request(Command::SetSize(size));
  // }

  // pub fn set_outer_size(&self, size: impl Into<Size>) {
  //   let size = size.into();
  //   // let scale_factor = self.state.read_lock().scale_factor;
  //   // if size.as_physical(scale_factor) == self.outer_size() {
  //   //   return;
  //   // }
  //   self.force_set_outer_size(size)
  // }

  fn force_set_inner_size(&self, size: Size) {
    // let scale_factor = self.state.read_lock().scale_factor;
    // let physical_size = size.as_physical(scale_factor);
    // let style = self.state.read_lock().style;
    // let mut window_rect = RECT {
    //   top: 0,
    //   left: 0,
    //   right: physical_size.width as i32,
    //   bottom: physical_size.height as i32,
    // };
    // unsafe {
    //   AdjustWindowRectExForDpi(
    //     &mut window_rect,
    //     get_window_style(&style),
    //     false,
    //     get_window_ex_style(&style),
    //     hwnd_dpi(self.hwnd),
    //   )
    // }
    // .unwrap();

    // let adjusted_size = PhysicalSize {
    //   width: (window_rect.right - window_rect.left) as u32,
    //   height: (window_rect.bottom - window_rect.top) as u32,
    // };

    self.request(Command::SetSize(size));
  }

  pub fn set_inner_size(&self, size: impl Into<Size>) {
    let size = size.into();
    // let scale_factor = self.state.read_lock().scale_factor;
    // if size.as_physical(scale_factor) == self.inner_size() {
    //   return;
    // }
    self.force_set_inner_size(size)
  }

  fn force_set_visible(&self, visible: bool) {
    // self.state.write_lock().style.visibility = visibility;
    self.request(Command::SetVisibility(visible));
  }

  pub fn set_visible(&self, visible: bool) {
    // if visibility == self.state.read_lock().style.visibility {
    //   return;
    // }
    self.force_set_visible(visible)
  }

  fn force_set_decorated(&self, decorated: bool) {
    // self.state.write_lock().style.decorations = visibility;
    self.request(Command::SetDecorations(decorated));
  }

  pub fn set_decorated(&self, decorated: bool) {
    // if visibility == self.state.read_lock().style.decorations {
    //   return;
    // }
    self.force_set_decorated(decorated)
  }

  fn force_set_theme(&self, theme: Option<Theme>) {
    self.winit.set_theme(theme)
    // let theme = match theme {
    //   Theme::Auto => {
    //     if is_system_dark_mode_enabled() {
    //       Theme::Dark
    //     } else {
    //       Theme::Light
    //     }
    //   }
    //   Theme::Dark => {
    //     if is_dark_mode_supported() {
    //       Theme::Dark
    //     } else {
    //       Theme::Light
    //     }
    //   }
    //   Theme::Light => Theme::Light,
    // };

    // self.state.write_lock().theme = theme;
    // let dark_mode = BOOL::from(theme == Theme::Dark);
    // if let Err(_error) = unsafe {
    //   DwmSetWindowAttribute(
    //     self.hwnd,
    //     Dwm::DWMWA_USE_IMMERSIVE_DARK_MODE,
    //     std::ptr::addr_of!(dark_mode) as *const std::ffi::c_void,
    //     std::mem::size_of::<BOOL>() as u32,
    //   )
    // } {
    //   tracing::error!("{_error}");
    // };
  }

  pub fn set_theme(&self, theme: Option<Theme>) {
    // if theme == self.state.read_lock().theme {
    //   return;
    // }
    self.force_set_theme(theme)
  }

  fn force_set_fullscreen(&self, fullscreen: Option<Fullscreen>) {
    // self.state.write_lock().style.fullscreen = fullscreen;
    self.request(Command::SetFullscreen(fullscreen));
  }

  pub fn set_fullscreen(&self, fullscreen: Option<Fullscreen>) {
    // if fullscreen == self.state.read_lock().style.fullscreen {
    //   return;
    // }
    self.force_set_fullscreen(fullscreen)
  }

  fn force_set_title(&self, title: impl AsRef<str>) {
    // self.state.write_lock().title = title.as_ref().into();
    // let title =
    //   HSTRING::from(format!("{}{}", title.as_ref(),
    // self.state.read_lock().subtitle));
    self.request(Command::SetWindowText(title.as_ref().to_owned()));
  }

  /// Set the title of the window
  pub fn set_title(&self, title: impl AsRef<str>) {
    // if title.as_ref() == self.state.read_lock().title {
    //   return;
    // }
    self.force_set_title(title)
  }

  fn force_set_cursor_mode(&self, cursor_mode: CursorGrabMode) {
    // self.state.write_lock().cursor.mode = cursor_mode;
    self.request(Command::SetCursorMode(cursor_mode));
  }

  pub fn set_cursor_mode(&self, cursor_mode: CursorGrabMode) {
    // if cursor_mode == self.state.read_lock().cursor.mode {
    //   return;
    // }
    self.force_set_cursor_mode(cursor_mode)
  }

  fn force_set_cursor_visible(&self, cursor_visible: bool) {
    // self.state.write_lock().cursor.visibility = cursor_visibility;
    self.request(Command::SetCursorVisibility(cursor_visible));
  }

  pub fn set_cursor_visibility(&self, cursor_visible: bool) {
    // if cursor_visibility == self.state.read_lock().cursor.visibility {
    //   return;
    // }
    self.force_set_cursor_visible(cursor_visible)
  }

  // fn force_set_subtitle(&self, subtitle: impl AsRef<str>) {
  //   self.state.write_lock().subtitle = subtitle.as_ref().into();
  //   let title =
  //     HSTRING::from(format!("{}{}", self.state.read_lock().title,
  // subtitle.as_ref()));   self.request(Command::SetWindowText(title));
  // }

  // /// Set text to appear after the title of the window
  // pub fn set_subtitle(&self, subtitle: impl AsRef<str>) {
  //   if subtitle.as_ref() == self.state.read_lock().subtitle {
  //     return;
  //   }
  //   self.force_set_subtitle(subtitle)
  // }

  fn force_request_redraw(&self) {
    // self.state.write_lock().requested_redraw = true;
    self.request(Command::Redraw);
  }

  /// Request a new Draw event
  pub fn request_redraw(&self) {
    // if self.state.read_lock().requested_redraw {
    //   return;
    // }
    self.force_request_redraw()
  }

  /// Request the window be closed
  pub fn close(&self) {
    if self.is_closing() {
      return; // already closing
    }
    *self.stage.lock().unwrap() = Stage::Closing;
    self.request(Command::Close);
  }

  fn request(&self, command: Command) {
    // let err_str = format!("failed to post command `{command:?}`");

    // self.command_queue.push(command);

    // unsafe { PostMessageW(self.hwnd, WindowsAndMessaging::WM_APP, WPARAM(0),
    // LPARAM(0)) }   .unwrap_or_else(|e| panic!("{}: {e}", err_str));
    if let Err(e) = self.proxy.send_event(WiterEvent::Command(command)) {
      tracing::error!("{e}");
    }
  }

  #[cfg(all(feature = "rwh_06", not(feature = "rwh_05")))]
  pub fn raw_window_handle(&self) -> RawWindowHandle {
    use rwh_06::HasRawWindowHandle;

    self.winit.raw_window_handle().unwrap()
  }

  #[cfg(all(feature = "rwh_06", not(feature = "rwh_05")))]
  pub fn raw_display_handle(&self) -> RawDisplayHandle {
    use rwh_06::HasRawDisplayHandle;

    self.winit.raw_display_handle().unwrap()
  }
}

#[cfg(all(feature = "rwh_06", not(feature = "rwh_05")))]
impl HasWindowHandle for Window {
  fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
    self.winit.window_handle()
  }
}

#[cfg(all(feature = "rwh_05", not(feature = "rwh_06")))]
unsafe impl HasRawWindowHandle for Window {
  fn raw_window_handle(&self) -> RawWindowHandle {
    use rwh_05::HasRawWindowHandle;

    self.winit.raw_window_handle()
  }
}

#[cfg(all(feature = "rwh_06", not(feature = "rwh_05")))]
impl HasDisplayHandle for Window {
  fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
    self.winit.display_handle()
  }
}

#[cfg(all(feature = "rwh_05", not(feature = "rwh_06")))]
unsafe impl HasRawDisplayHandle for Window {
  fn raw_display_handle(&self) -> RawDisplayHandle {
    use rwh_05::HasRawDisplayHandle;

    self.winit.raw_display_handle()
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
  type Item = Event<WiterEvent<()>>;

  fn next(&mut self) -> Option<Self::Item> {
    self.window.next_event()
  }
}

impl<'a> IntoIterator for &'a Window {
  type IntoIter = MessageIterator<'a>;
  type Item = Event<WiterEvent<()>>;

  fn into_iter(self) -> Self::IntoIter {
    self.iter()
  }
}

pub struct MessageIteratorMut<'a> {
  window: &'a mut Window,
}

impl<'a> Iterator for MessageIteratorMut<'a> {
  type Item = Event<WiterEvent<()>>;

  fn next(&mut self) -> Option<Self::Item> {
    self.window.next_event()
  }
}

impl<'a> IntoIterator for &'a mut Window {
  type IntoIter = MessageIteratorMut<'a>;
  type Item = Event<WiterEvent<()>>;

  fn into_iter(self) -> Self::IntoIter {
    self.iter_mut()
  }
}
