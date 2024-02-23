#![cfg_attr(target_os, windows)] // for now, it only supports Win32
#![deny(unsafe_op_in_unsafe_fn)]

use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

pub mod debug;
pub mod prelude;
pub mod window;

pub fn signed_lo_word(dword: i32) -> i16 {
  dword as i16
}

pub fn lo_word(dword: u32) -> u16 {
  dword as u16
}

pub fn signed_hi_word(dword: i32) -> i16 {
  (dword >> 16) as i16
}

pub fn hi_word(dword: u32) -> u16 {
  (dword >> 16) as u16
}

pub fn signed_lo_byte(word: i16) -> i8 {
  word as i8
}

pub fn lo_byte(word: u16) -> u8 {
  word as u8
}

pub fn signed_hi_byte(word: i16) -> i8 {
  (word >> 8) as i8
}

pub fn hi_byte(word: u16) -> u8 {
  (word >> 8) as u8
}

pub struct Handle<T>(Arc<RwLock<T>>);

impl<T> Clone for Handle<T> {
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }

  fn clone_from(&mut self, source: &Self) {
    *self = Self(source.0.clone());
  }
}

impl<T> Handle<T> {
  pub fn new(t: T) -> Self {
    Self(Arc::new(RwLock::new(t)))
  }

  pub fn get(&self) -> RwLockReadGuard<'_, T> {
    self.0.read().expect("lock was poisoned")
  }

  // pub fn try_get(&self) -> Result<RwLockReadGuard<'_, Device>,
  // TryLockError<RwLockReadGuard<'_, Device>>> {   self.0.try_read()
  // }

  pub fn get_mut(&self) -> RwLockWriteGuard<'_, T> {
    self.0.write().expect("lock was poisoned")
  }
}
