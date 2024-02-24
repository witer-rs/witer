use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

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

  pub fn get_mut(&self) -> RwLockWriteGuard<'_, T> {
    self.0.write().expect("lock was poisoned")
  }
}
