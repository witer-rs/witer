pub mod key;
pub mod modifier;
pub mod mouse;
pub mod state;

use std::collections::HashMap;

use strum::IntoEnumIterator;

use self::state::KeyState;
use crate::window::input::{key::Key, mouse::Button, state::ButtonState};

#[derive(Debug)]
pub struct Input {
  mouse_buttons: HashMap<Button, ButtonState>,
  keys: HashMap<Key, KeyState>,
  shift: ButtonState,
  ctrl: ButtonState,
  alt: ButtonState,
  win: ButtonState,
}

impl Input {
  pub fn new() -> Self {
    let mouse_buttons = {
      let mut map = HashMap::default();
      for code in Button::iter() {
        map.insert(code, ButtonState::Released);
      }
      map
    };

    let keys = {
      let mut map = HashMap::default();
      for code in Key::iter() {
        map.insert(code, KeyState::Released);
      }
      map
    };

    Self {
      mouse_buttons,
      keys,
      shift: Default::default(),
      ctrl: Default::default(),
      alt: Default::default(),
      win: Default::default(),
    }
  }

  // KEYBOARD

  pub fn key(&self, keycode: Key) -> KeyState {
    self
      .keys
      .get(&keycode)
      .copied()
      .unwrap_or(KeyState::Released)
  }

  pub(crate) fn update_key_state(&mut self, keycode: Key, state: KeyState) {
    if let Some(key_state) = self.keys.get_mut(&keycode) {
      *key_state = state;
    }
  }

  // MOUSE

  pub fn mouse(&self, button: Button) -> ButtonState {
    self
      .mouse_buttons
      .get(&button)
      .copied()
      .unwrap_or(ButtonState::Released)
  }

  pub(crate) fn update_mouse_button_state(&mut self, button: Button, state: ButtonState) {
    if let Some(mouse_state) = self.mouse_buttons.get_mut(&button) {
      *mouse_state = state;
    }
  }

  // MODS

  pub fn shift(&self) -> ButtonState {
    self.shift
  }

  pub fn ctrl(&self) -> ButtonState {
    self.ctrl
  }

  pub fn alt(&self) -> ButtonState {
    self.alt
  }

  pub fn win(&self) -> ButtonState {
    self.win
  }

  pub(crate) fn update_modifiers_state(&mut self) {
    self.shift =
      if self.key(Key::LeftShift).is_held() || self.key(Key::RightShift).is_held() {
        ButtonState::Pressed
      } else {
        ButtonState::Released
      };

    self.ctrl =
      if self.key(Key::LeftControl).is_held() || self.key(Key::RightControl).is_held() {
        ButtonState::Pressed
      } else {
        ButtonState::Released
      };

    self.alt = if self.key(Key::LeftAlt).is_held() || self.key(Key::RightAlt).is_held() {
      ButtonState::Pressed
    } else {
      ButtonState::Released
    };

    self.win =
      if self.key(Key::LeftSuper).is_held() || self.key(Key::RightSuper).is_held() {
        ButtonState::Pressed
      } else {
        ButtonState::Released
      };
  }
}

impl Default for Input {
  fn default() -> Self {
    Self::new()
  }
}
