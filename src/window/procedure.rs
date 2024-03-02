use std::sync::Arc;

use windows::Win32::{
  Foundation::*,
  UI::{
    Shell::DefSubclassProc,
    WindowsAndMessaging::{self, DestroyWindow},
  },
};

#[allow(unused)]
use super::window_message::{Message, WindowMessage};
use super::{stage::Stage, window_message::MouseMessage, Window, WindowProcedure};

pub struct SubclassWindowData {
  pub window: Arc<Window>,
  pub dispatcher: Box<dyn WindowProcedure>,
}

pub extern "system" fn wnd_proc(
  hwnd: HWND,
  msg: u32,
  w_param: WPARAM,
  l_param: LPARAM,
) -> LRESULT {
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
  let SubclassWindowData { window, dispatcher }: &mut SubclassWindowData =
    unsafe { std::mem::transmute(dw_ref_data) };

  let message = Message::new(hwnd, msg, w_param, l_param);
  if message != Message::Ignored {
    let current_stage = window.state.get_mut().current_stage;

    match current_stage {
      Stage::Looping => {
        dispatcher.callback(window, handle_message(window, message));
      }
      Stage::Closing => {
        window.state.get_mut().current_stage = Stage::Destroyed;

        dispatcher.callback(window, Message::Closing);
      }
      Stage::Destroyed => {
        window.state.get_mut().current_stage = Stage::ExitLoop;

        #[cfg(feature = "opengl")]
        {
          let hwnd = self.state.get().h_wnd;
          let hdc = self.gl_context.hdc;
          unsafe { windows::Win32::Graphics::Gdi::ReleaseDC(HWND(hwnd), hdc) };
        }

        unsafe { DestroyWindow(hwnd) }.expect("failed to destroy window");

        dispatcher.callback(window, Message::Destroyed);
      }
      Stage::ExitLoop => (),
    }
  }

  match msg {
    WindowsAndMessaging::WM_CLOSE => LRESULT(0),
    WindowsAndMessaging::WM_DESTROY => {
      unsafe {
        WindowsAndMessaging::PostQuitMessage(0);
      }
      LRESULT(0)
    }
    _ => unsafe { DefSubclassProc(hwnd, msg, w_param, l_param) },
  }
}

fn handle_message(window: &Arc<Window>, message: Message) -> Message {
  match &message {
    Message::CloseRequested => {
      if window.state.get().close_on_x {
        window.close();
      }
    }
    Message::Window(message) => match message {
      WindowMessage::StartedSizingOrMoving => {
        window.state.get_mut().is_sizing_or_moving = true;
      }
      WindowMessage::StoppedSizingOrMoving => {
        window.state.get_mut().is_sizing_or_moving = false;
      }
      // WindowMessage::Resizing { window_mode } => {
      //   let mut state = self.state.get_mut();
      //   state.window_mode = *window_mode;
      // }
      // WindowMessage::Moving { .. } => (),
      _ => (),
    },
    &Message::Keyboard { key, state, .. } => {
      window.state.get_mut().input.update_key_state(key, state);
      window.state.get_mut().input.update_modifiers_state();
    }
    &Message::Mouse(MouseMessage::Button { button, state, .. }) => {
      window
        .state
        .get_mut()
        .input
        .update_mouse_button_state(button, state);
    }
    _ => (),
  }

  message
}
