use std::sync::{Arc, Condvar, Mutex};

use crossbeam::queue::SegQueue;
use windows::Win32::{
  Foundation::*,
  Graphics::Gdi::{self, RedrawWindow},
  UI::{
    Shell::{DefSubclassProc, SetWindowSubclass},
    WindowsAndMessaging::{
      self,
      DestroyWindow,
      PostQuitMessage,
      SetWindowTextW,
      ShowWindow,
      CREATESTRUCTW,
    },
  },
};

#[allow(unused)]
use super::message::{Message, WindowMessage};
use super::{
  command::Command,
  settings::{Visibility, WindowSettings},
  Window,
};
use crate::{
  handle::Handle,
  prelude::Input,
  window::{stage::Stage, state::InternalState},
};

pub struct CreateInfo {
  pub settings: WindowSettings,
  pub command_queue: Option<Arc<SegQueue<Command>>>,
  pub new_message: Option<Arc<(Mutex<bool>, Condvar)>>,
  pub next_frame: Option<Arc<(Mutex<bool>, Condvar)>>,
  pub next_message: Option<Arc<Mutex<Message>>>,
  pub window: Option<Arc<Window>>,
}

pub struct SubclassWindowData {
  pub window: Arc<Window>,
  pub command_queue: Arc<SegQueue<Command>>,
  pub new_message: Arc<(Mutex<bool>, Condvar)>,
  pub next_frame: Arc<(Mutex<bool>, Condvar)>,
  pub next_message: Arc<Mutex<Message>>,
}

pub extern "system" fn wnd_proc(
  hwnd: HWND,
  msg: u32,
  w_param: WPARAM,
  l_param: LPARAM,
) -> LRESULT {
  if msg == WindowsAndMessaging::WM_CREATE {
    let create_struct = unsafe { (l_param.0 as *mut CREATESTRUCTW).as_mut().unwrap() };
    let create_info = unsafe {
      (create_struct.lpCreateParams as *mut CreateInfo)
        .as_mut()
        .unwrap()
    };

    let command_queue = create_info.command_queue.take().unwrap();
    let new_message = create_info.new_message.take().unwrap();
    let next_frame = create_info.next_frame.take().unwrap();
    let next_message = create_info.next_message.take().unwrap();

    // create state
    let input = Input::new();
    let state = Handle::new(InternalState {
      thread: None,
      title: Default::default(),
      subtitle: Default::default(),
      color_mode: Default::default(),
      visibility: Default::default(),
      flow: Default::default(),
      close_on_x: Default::default(),
      stage: Stage::Looping,
      input,
      requested_redraw: false,
    });

    let window = Arc::new(Window {
      hinstance: create_struct.hInstance,
      hwnd,
      state,
      command_queue: command_queue.clone(),
      new_message: new_message.clone(),
      next_frame: next_frame.clone(),
      next_message: next_message.clone(),
    });

    let subclass_data = SubclassWindowData {
      window: window.clone(),
      command_queue,
      new_message,
      next_frame,
      next_message,
    };

    create_info.window = Some(window);

    // create subclass ptr
    let window_data_ptr = Box::into_raw(Box::new(subclass_data));

    // attach subclass ptr
    let result = unsafe {
      SetWindowSubclass(
        hwnd,
        Some(subclass_proc),
        Window::WINDOW_SUBCLASS_ID,
        window_data_ptr as usize,
      )
    }
    .as_bool();
    debug_assert!(result);
  }

  unsafe { WindowsAndMessaging::DefWindowProcW(hwnd, msg, w_param, l_param) }
}

pub extern "system" fn subclass_proc(
  hwnd: HWND,
  msg: u32,
  w_param: WPARAM,
  l_param: LPARAM,
  _u_id_subclass: usize,
  dw_ref_data: usize,
) -> LRESULT {
  let SubclassWindowData {
    window,
    command_queue,
    new_message,
    next_frame,
    next_message,
  }: &mut SubclassWindowData = unsafe { std::mem::transmute(dw_ref_data) };

  // ignore certain messages
  match msg {
    WindowsAndMessaging::WM_SIZING
    | WindowsAndMessaging::WM_MOVING
    | WindowsAndMessaging::WM_MOVE => {
      return unsafe { DefSubclassProc(hwnd, msg, w_param, l_param) }
    }
    _ => (),
  }

  // handle from wndproc
  let result = match msg {
    WindowsAndMessaging::WM_CLOSE => LRESULT(0),
    WindowsAndMessaging::WM_DESTROY => {
      window.state.get_mut().stage = Stage::Destroyed;
      unsafe { PostQuitMessage(0) };
      LRESULT(0)
    }
    _ => unsafe { DefSubclassProc(hwnd, msg, w_param, l_param) },
  };

  // handle command requests
  while let Some(command) = command_queue.pop() {
    match command {
      Command::Close => {
        unsafe { DestroyWindow(hwnd) }.unwrap();
        break; // hwnd will be invalid
      }
      Command::SetVisibility(visibility) => unsafe {
        ShowWindow(hwnd, match visibility {
          Visibility::Hidden => WindowsAndMessaging::SW_HIDE,
          Visibility::Shown => WindowsAndMessaging::SW_SHOW,
        });
      },
      Command::Redraw => unsafe {
        RedrawWindow(hwnd, None, None, Gdi::RDW_INTERNALPAINT);
      },
      Command::SetWindowText(text) => unsafe {
        SetWindowTextW(hwnd, &text).unwrap();
      },
    }
  }

  let message = Message::new(hwnd, msg, w_param, l_param);

  // Wait for message to be taken before overwriting
  let is_none = matches!(*next_message.lock().unwrap(), Message::None);
  if !is_none {
    wait_on_frame(next_frame, window);
  }

  // pass message to main thread
  next_message
    .lock()
    .unwrap()
    .replace(handle_message(window, message));
  signal_new_message(new_message);
  wait_on_frame(next_frame, window);

  result
}

fn signal_new_message(new_message: &mut Arc<(Mutex<bool>, Condvar)>) {
  let (lock, cvar) = new_message.as_ref();
  {
    let mut new = lock.lock().unwrap();
    *new = true;
  }
  cvar.notify_one();
}

fn wait_on_frame(next_frame: &mut Arc<(Mutex<bool>, Condvar)>, window: &Arc<Window>) {
  let (lock, cvar) = next_frame.as_ref();
  let mut next = cvar
    .wait_while(lock.lock().unwrap(), |next| !*next && !window.is_destroyed())
    .unwrap();
  *next = false;
}

fn handle_message(window: &Arc<Window>, message: Message) -> Message {
  let stage = window.state.get().stage;

  match stage {
    Stage::Looping | Stage::Closing => {
      if let Message::Window(window_message) = &message {
        match window_message {
          WindowMessage::CloseRequested => {
            if window.state.get().close_on_x {
              window.close();
            }
          }
          &WindowMessage::Key { key, state, .. } => {
            window.state.get_mut().input.update_key_state(key, state);
            window.state.get_mut().input.update_modifiers_state();
          }
          &WindowMessage::MouseButton { button, state, .. } => window
            .state
            .get_mut()
            .input
            .update_mouse_state(button, state),
          WindowMessage::Draw => {
            window.state.get_mut().requested_redraw = false;
          }
          _ => (),
        }
      }
    }
    Stage::Destroyed => (),
  }

  message
}
