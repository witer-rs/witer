pub mod button;
pub mod keyboard;
pub mod modifier;
pub mod mouse;

use std::collections::HashMap;

use enumflags2::BitFlags;
use strum::IntoEnumIterator;

use self::button::ButtonState;
use crate::window::input::{
  button::KeyState,
  keyboard::KeyCode,
  modifier::Modifiers,
  mouse::MouseCode,
};

#[derive(Debug)]
pub struct Input {
  keys: HashMap<KeyCode, KeyState>,
  modifiers: BitFlags<Modifiers>,
  mouse_buttons: HashMap<MouseCode, ButtonState>,
}

impl Input {
  pub fn new() -> Self {
    let keys = {
      let mut map = HashMap::default();
      for code in KeyCode::iter() {
        map.insert(code, KeyState::Released);
      }
      map
    };

    let modifiers = Default::default();

    let mouse_buttons = {
      let mut map = HashMap::default();
      for code in MouseCode::iter() {
        map.insert(code, ButtonState::Released);
      }
      map
    };

    Self {
      keys,
      modifiers,
      mouse_buttons,
    }
  }

  // KEYBOARD

  pub fn key_state(&self, code: KeyCode) -> KeyState {
    self.keys.get(&code).copied().unwrap_or(KeyState::Released)
  }

  pub fn key_down(&self, code: KeyCode) -> bool {
    !matches!(self.key_state(code), KeyState::Released)
  }

  pub fn update_keyboard_state(&mut self, key_code: KeyCode, state: KeyState) {
    if let Some(key_state) = self.keys.get_mut(&key_code) {
      *key_state = state;
    }
  }

  // MOUSE

  pub fn mouse_button_state(&self, code: MouseCode) -> ButtonState {
    self
      .mouse_buttons
      .get(&code)
      .copied()
      .unwrap_or(ButtonState::Released)
  }

  pub fn mouse_button_down(&self, code: MouseCode) -> bool {
    !matches!(self.mouse_button_state(code), ButtonState::Released)
  }

  pub(crate) fn update_mouse_button_state(
    &mut self,
    mouse_code: MouseCode,
    state: ButtonState,
  ) {
    if let Some(button_state) = self.mouse_buttons.get_mut(&mouse_code) {
      *button_state = state;
    }
  }

  // MODS

  pub fn modifiers_state(&self) -> BitFlags<Modifiers> {
    self.modifiers
  }

  pub fn modifier_down(&self, modifier: Modifiers) -> bool {
    self.modifiers.contains(modifier)
  }

  pub fn modifiers_down(&self, modifiers: BitFlags<Modifiers>) -> bool {
    self.modifiers.contains(modifiers)
  }

  //   pub(crate) fn update_modifiers_state(&mut self, modifiers:
  // ModifiersState) -> BitFlags<Modifiers> {     // TODO: just swap to bit
  // manipulation to speed up. Stop being lazy, Gabriel.     for modifier in
  // Modifiers::iter() {       if modifiers.contains(modifier.into()) !=
  // self.modifiers.contains(modifier) {         self.modifiers.
  // toggle(modifier);       }
  //     }

  //     self.modifiers
  //   }
}

impl Default for Input {
  fn default() -> Self {
    Self::new()
  }
}
