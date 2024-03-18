use crate::raw_window_handle::RawDisplayHandle;

/// Handles interfacing with the OS clipboard.
///
/// If the "clipboard" feature is off, or we cannot connect to the OS clipboard,
/// then a fallback clipboard that just works within the same app is used
/// instead.
pub struct Clipboard {
  #[cfg(feature = "clipboard")]
  arboard: Option<arboard::Clipboard>,
  /// Fallback manual clipboard.
  clipboard: String,
}

impl Clipboard {
  /// Construct a new instance
  pub fn new(_raw_display_handle: Option<RawDisplayHandle>) -> Self {
    Self {
      #[cfg(feature = "clipboard")]
      arboard: init_arboard(),

      clipboard: Default::default(),
    }
  }

  pub fn get(&mut self) -> Option<String> {
    #[cfg(feature = "clipboard")]
    if let Some(clipboard) = &mut self.arboard {
      return match clipboard.get_text() {
        Ok(text) => Some(text),
        Err(err) => {
          tracing::error!("arboard paste error: {err}");
          None
        }
      };
    }

    Some(self.clipboard.clone())
  }

  pub fn set(&mut self, text: String) {
    #[cfg(feature = "clipboard")]
    if let Some(clipboard) = &mut self.arboard {
      if let Err(err) = clipboard.set_text(text) {
        tracing::error!("arboard copy/cut error: {err}");
      }
      return;
    }

    self.clipboard = text;
  }
}

#[cfg(feature = "clipboard")]
fn init_arboard() -> Option<arboard::Clipboard> {
  tracing::trace!("Initializing arboard clipboard…");
  match arboard::Clipboard::new() {
    Ok(clipboard) => Some(clipboard),
    Err(err) => {
      tracing::warn!("Failed to initialize arboard clipboard: {err}");
      None
    }
  }
}
