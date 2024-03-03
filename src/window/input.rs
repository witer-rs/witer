use std::collections::HashMap;

use strum::IntoEnumIterator;

use self::state::KeyState;
use crate::window::input::{key::Key, mouse::Mouse, state::ButtonState};

pub mod key;
pub mod mouse;
pub mod state;

#[derive(Debug)]
pub struct InputState {}

impl InputState {}

#[derive(Debug)]
pub struct Input {
  mouse_buttons: HashMap<Mouse, ButtonState>,
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
      for code in Mouse::iter() {
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

  pub fn update_key_state(&mut self, keycode: Key, state: KeyState) {
    if let Some(key_state) = self.keys.get_mut(&keycode) {
      *key_state = state;
    }
  }

  pub fn update_mouse_state(&mut self, button: Mouse, state: ButtonState) {
    if let Some(mouse_state) = self.mouse_buttons.get_mut(&button) {
      *mouse_state = state;
    }
  }

  pub fn update_modifiers_state(&mut self) {
    let key = |keycode: Key| -> KeyState {
      self
        .keys
        .get(&keycode)
        .copied()
        .unwrap_or(KeyState::Released)
    };

    self.shift = if key(Key::LeftShift).is_held() || key(Key::RightShift).is_held() {
      ButtonState::Pressed
    } else {
      ButtonState::Released
    };

    self.ctrl = if key(Key::LeftControl).is_held() || key(Key::RightControl).is_held() {
      ButtonState::Pressed
    } else {
      ButtonState::Released
    };

    self.alt = if key(Key::LeftAlt).is_held() || key(Key::RightAlt).is_held() {
      ButtonState::Pressed
    } else {
      ButtonState::Released
    };

    self.win = if key(Key::LeftSuper).is_held() || key(Key::RightSuper).is_held() {
      ButtonState::Pressed
    } else {
      ButtonState::Released
    };
  }

  // KEYBOARD

  pub fn key(&self, keycode: Key) -> KeyState {
    self
      .keys
      .get(&keycode)
      .copied()
      .unwrap_or(KeyState::Released)
  }

  // MOUSE

  pub fn mouse(&self, button: Mouse) -> ButtonState {
    self
      .mouse_buttons
      .get(&button)
      .copied()
      .unwrap_or(ButtonState::Released)
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
}

impl Default for Input {
  fn default() -> Self {
    Self::new()
  }
}
