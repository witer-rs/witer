use std::collections::HashMap;

use self::state::KeyState;
use crate::window::input::{key::Key, mouse::MouseButton, state::ButtonState};

pub mod key;
pub mod mouse;
pub mod state;

#[derive(Debug)]
pub struct InputState {}

impl InputState {}

#[derive(Debug)]
pub struct Input {
  mouse_buttons: HashMap<MouseButton, ButtonState>,
  keys: HashMap<Key, KeyState>,
  shift: ButtonState,
  ctrl: ButtonState,
  alt: ButtonState,
  win: ButtonState,
}

impl Input {
  pub fn new() -> Self {
    let mouse_buttons = HashMap::default();
    let keys = HashMap::default();

    Self {
      mouse_buttons,
      keys,
      shift: Default::default(),
      ctrl: Default::default(),
      alt: Default::default(),
      win: Default::default(),
    }
  }

  pub fn update_key_state(&mut self, keycode: Key, new_state: KeyState) {
    if let Some(old_state) = self.keys.get_mut(&keycode) {
      *old_state = new_state;
    }
  }

  pub fn update_mouse_button_state(
    &mut self,
    button: MouseButton,
    new_state: ButtonState,
  ) {
    if let Some(old_state) = self.mouse_buttons.get_mut(&button) {
      *old_state = new_state;
    }
  }

  pub fn update_modifiers_state(
    &mut self,
  ) -> (bool, ButtonState, ButtonState, ButtonState, ButtonState) {
    let key = |keycode: Key| -> KeyState {
      self
        .keys
        .get(&keycode)
        .copied()
        .unwrap_or(KeyState::Released)
    };

    let mut changed = false;

    let old_value = self.shift;
    self.shift = if key(Key::LeftShift).is_pressed() || key(Key::RightShift).is_pressed()
    {
      ButtonState::Pressed
    } else {
      ButtonState::Released
    };
    changed |= self.shift != old_value;

    let old_value = self.ctrl;
    self.ctrl =
      if key(Key::LeftControl).is_pressed() || key(Key::RightControl).is_pressed() {
        ButtonState::Pressed
      } else {
        ButtonState::Released
      };
    changed |= self.ctrl != old_value;

    let old_value = self.alt;
    self.alt = if key(Key::LeftAlt).is_pressed() || key(Key::RightAlt).is_pressed() {
      ButtonState::Pressed
    } else {
      ButtonState::Released
    };
    changed |= self.alt != old_value;

    let old_value = self.win;
    self.win = if key(Key::LeftSuper).is_pressed() || key(Key::RightSuper).is_pressed() {
      ButtonState::Pressed
    } else {
      ButtonState::Released
    };
    changed |= self.win != old_value;

    (changed, self.shift, self.ctrl, self.alt, self.win)
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

  pub fn mouse(&self, button: MouseButton) -> ButtonState {
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
