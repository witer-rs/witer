#![cfg(feature = "egui")]

//! [`witer`] port of [`egui`] bindings for [`winit`].
//!
//! The library translates winit events to egui, handled copy/paste,
//! updates the cursor, open links clicked in egui, etc.

use cursor_icon::CursorIcon;
pub use egui;
use egui::{
  Pos2,
  Rect,
  Vec2,
  ViewportBuilder,
  ViewportCommand,
  ViewportId,
  ViewportInfo,
};
pub use window_settings::WindowSettings;

use self::window::message::{CursorMoveKind, Focus};
use crate::{prelude::*, raw_window_handle::HasDisplayHandle};

pub mod clipboard;
mod window_settings;

pub fn screen_size_in_pixels(window: &Window) -> egui::Vec2 {
  let size = window.inner_size();
  egui::vec2(size.width as f32, size.height as f32)
}

/// Calculate the `pixels_per_point` for a given window, given the current egui
/// zoom factor
pub fn pixels_per_point(egui_ctx: &egui::Context, window: &Window) -> f32 {
  let native_pixels_per_point = window.scale_factor() as f32;
  let egui_zoom_factor = egui_ctx.zoom_factor();
  egui_zoom_factor * native_pixels_per_point
}

// ----------------------------------------------------------------------------

#[must_use]
#[derive(Clone, Copy, Debug, Default)]
pub struct EventResponse {
  /// If true, egui consumed this event, i.e. wants exclusive use of this event
  /// (e.g. a mouse click on an egui window, or entering text into a text
  /// field).
  ///
  /// For instance, if you use egui for a game, you should only
  /// pass on the events to your game when [`Self::consumed`] is `false.
  ///
  /// Note that egui uses `tab` to move focus between elements, so this will
  /// always be `true` for tabs.
  pub consumed: bool,

  /// Do we need an egui refresh because of this event?
  pub repaint: bool,
}

// ----------------------------------------------------------------------------

/// Handles the integration between egui and a winit Window.
///
/// Instantiate one of these per viewport/window.
pub struct State {
  /// Shared clone.
  egui_ctx: egui::Context,

  viewport_id: ViewportId,
  start_time: web_time::Instant,
  egui_input: egui::RawInput,
  pointer_pos_in_points: Option<egui::Pos2>,
  any_pointer_button_down: bool,
  current_cursor_icon: Option<egui::CursorIcon>,

  clipboard: clipboard::Clipboard,

  /// If `true`, mouse inputs will be treated as touches.
  /// Useful for debugging touch support in egui.
  ///
  /// Creates duplicate touches, if real touch inputs are coming.
  simulate_touch_screen: bool,

  /// Is Some(…) when a touch is being translated to a pointer.
  ///
  /// Only one touch will be interpreted as pointer at any time.
  pointer_touch_id: Option<u64>,

  /// track ime state
  input_method_editor_started: bool,

  allow_ime: bool,
}

impl State {
  /// Construct a new instance
  pub fn new(
    egui_ctx: egui::Context,
    viewport_id: ViewportId,
    display_target: &dyn HasDisplayHandle,
    native_pixels_per_point: Option<f32>,
    max_texture_side: Option<usize>,
  ) -> Self {
    let egui_input = egui::RawInput {
      focused: false, // winit will tell us when we have focus
      ..Default::default()
    };

    let mut slf = Self {
      egui_ctx,
      viewport_id,
      start_time: web_time::Instant::now(),
      egui_input,
      pointer_pos_in_points: None,
      any_pointer_button_down: false,
      current_cursor_icon: None,

      clipboard: clipboard::Clipboard::new(
        display_target.display_handle().ok().map(|h| h.as_raw()),
      ),

      simulate_touch_screen: false,
      pointer_touch_id: None,

      input_method_editor_started: false,

      allow_ime: false,
    };

    slf
      .egui_input
      .viewports
      .entry(ViewportId::ROOT)
      .or_default()
      .native_pixels_per_point = native_pixels_per_point;

    if let Some(max_texture_side) = max_texture_side {
      slf.set_max_texture_side(max_texture_side);
    }
    slf
  }

  /// Call this once a graphics context has been created to update the maximum
  /// texture dimensions that egui will use.
  pub fn set_max_texture_side(&mut self, max_texture_side: usize) {
    self.egui_input.max_texture_side = Some(max_texture_side);
  }

  /// Fetches text from the clipboard and returns it.
  pub fn clipboard_text(&mut self) -> Option<String> {
    self.clipboard.get()
  }

  /// Places the text onto the clipboard.
  pub fn set_clipboard_text(&mut self, text: String) {
    self.clipboard.set(text);
  }

  /// Returns [`false`] or the last value that [`Window::set_ime_allowed()`] was
  /// called with, used for debouncing.
  pub fn allow_ime(&self) -> bool {
    self.allow_ime
  }

  /// Set the last value that [`Window::set_ime_allowed()`] was called with.
  pub fn set_allow_ime(&mut self, allow: bool) {
    self.allow_ime = allow;
  }

  #[inline]
  pub fn egui_ctx(&self) -> &egui::Context {
    &self.egui_ctx
  }

  /// The current input state.
  /// This is changed by [`Self::on_window_event`] and cleared by
  /// [`Self::take_egui_input`].
  #[inline]
  pub fn egui_input(&self) -> &egui::RawInput {
    &self.egui_input
  }

  /// The current input state.
  /// This is changed by [`Self::on_window_event`] and cleared by
  /// [`Self::take_egui_input`].
  #[inline]
  pub fn egui_input_mut(&mut self) -> &mut egui::RawInput {
    &mut self.egui_input
  }

  /// Prepare for a new frame by extracting the accumulated input,
  ///
  /// as well as setting [the time](egui::RawInput::time) and [screen
  /// rectangle](egui::RawInput::screen_rect).
  ///
  /// You need to set [`egui::RawInput::viewports`] yourself though.
  /// Use [`update_viewport_info`] to update the info for each
  /// viewport.
  pub fn take_egui_input(&mut self, window: &Window) -> egui::RawInput {
    self.egui_input.time = Some(self.start_time.elapsed().as_secs_f64());

    // On Windows, a minimized window will have 0 width and height.
    // See: https://github.com/rust-windowing/winit/issues/208
    // This solves an issue where egui window positions would be changed when
    // minimizing on Windows.
    let screen_size_in_pixels = screen_size_in_pixels(window);
    let screen_size_in_points =
      screen_size_in_pixels / pixels_per_point(&self.egui_ctx, window);

    self.egui_input.screen_rect = (screen_size_in_points.x > 0.0
      && screen_size_in_points.y > 0.0)
      .then(|| Rect::from_min_size(Pos2::ZERO, screen_size_in_points));

    // Tell egui which viewport is now active:
    self.egui_input.viewport_id = self.viewport_id;

    self
      .egui_input
      .viewports
      .entry(self.viewport_id)
      .or_default()
      .native_pixels_per_point = Some(window.scale_factor() as f32);

    self.egui_input.take()
  }

  /// Call this when there is a new event.
  ///
  /// The result can be found in [`Self::egui_input`] and be extracted with
  /// [`Self::take_egui_input`].
  pub fn on_window_event(&mut self, window: &Window, message: &Message) -> EventResponse {
    match message {
      Message::ScaleFactorChanged(scale_factor) => {
        let native_pixels_per_point = *scale_factor as f32;

        self
          .egui_input
          .viewports
          .entry(self.viewport_id)
          .or_default()
          .native_pixels_per_point = Some(native_pixels_per_point);

        EventResponse {
          repaint: true,
          consumed: false,
        }
      }
      Message::MouseButton { state, button, .. } => {
        self.on_mouse_button_input(*button, *state);
        EventResponse {
          repaint: true,
          consumed: self.egui_ctx.wants_pointer_input(),
        }
      }
      Message::MouseWheel { delta_x, delta_y } => {
        self.on_mouse_wheel(window, *delta_x, *delta_y);
        EventResponse {
          repaint: true,
          consumed: self.egui_ctx.wants_pointer_input(),
        }
      }
      Message::CursorMove {
        kind: CursorMoveKind::Left,
        ..
      } => {
        self.pointer_pos_in_points = None;
        self.egui_input.events.push(egui::Event::PointerGone);
        EventResponse {
          repaint: true,
          consumed: false,
        }
      }
      Message::CursorMove { position, .. } => {
        self.on_cursor_moved(window, *position);
        EventResponse {
          repaint: true,
          consumed: self.egui_ctx.is_using_pointer(),
        }
      }
      // WindowEvent::TouchpadPressure {device_id, pressure, stage, ..  } => {} // TODO
      // Message::Touch(touch) => {
      //   self.on_touch(window, touch);
      //   let consumed = match touch.phase {
      //     winit::event::TouchPhase::Started
      //     | winit::event::TouchPhase::Ended
      //     | winit::event::TouchPhase::Cancelled => self.egui_ctx.wants_pointer_input(),
      //     winit::event::TouchPhase::Moved => self.egui_ctx.is_using_pointer(),
      //   };
      //   EventResponse {
      //     repaint: true,
      //     consumed,
      //   }
      // }

      // WindowEvent::Ime(ime) => {
      //   // on Mac even Cmd-C is pressed during ime, a `c` is pushed to Preedit.
      //   // So no need to check is_mac_cmd.
      //   //
      //   // How winit produce `Ime::Enabled` and `Ime::Disabled` differs in MacOS
      //   // and Windows.
      //   //
      //   // - On Windows, before and after each Commit will produce an Enable/Disabled
      //   // event.
      //   // - On MacOS, only when user explicit enable/disable ime. No Disabled
      //   // after Commit.
      //   //
      //   // We use input_method_editor_started to manually insert CompositionStart
      //   // between Commits.
      //   match ime {
      //     winit::event::Ime::Enabled | winit::event::Ime::Disabled => (),
      //     winit::event::Ime::Commit(text) => {
      //       self.input_method_editor_started = false;
      //       self
      //         .egui_input
      //         .events
      //         .push(egui::Event::CompositionEnd(text.clone()));
      //     }
      //     winit::event::Ime::Preedit(text, Some(_)) => {
      //       if !self.input_method_editor_started {
      //         self.input_method_editor_started = true;
      //         self.egui_input.events.push(egui::Event::CompositionStart);
      //       }
      //       self
      //         .egui_input
      //         .events
      //         .push(egui::Event::CompositionUpdate(text.clone()));
      //     }
      //     winit::event::Ime::Preedit(_, None) => {}
      //   };

      //   EventResponse {
      //     repaint: true,
      //     consumed: self.egui_ctx.wants_keyboard_input(),
      //   }
      // }
      Message::Key { key, state, .. } => {
        self.on_keyboard_input(key, state);

        // When pressing the Tab key, egui focuses the first focusable element, hence
        // Tab always consumes.
        let consumed = self.egui_ctx.wants_keyboard_input() || key == &Key::Tab;
        EventResponse {
          repaint: true,
          consumed,
        }
      }
      Message::Text(text) => {
        self.on_text_input(text);
        let consumed = self.egui_ctx.wants_keyboard_input();
        EventResponse {
          repaint: true,
          consumed,
        }
      } 
      Message::Focus(focus) => {
        self.egui_input.focused = *focus == Focus::Gained;
        // We will not be given a KeyboardInput event when the modifiers are released
        // while the window does not have focus. Unset all modifier state to be
        // safe.
        self.egui_input.modifiers = egui::Modifiers::default();
        self
          .egui_input
          .events
          .push(egui::Event::WindowFocused(self.egui_input.focused));
        EventResponse {
          repaint: true,
          consumed: false,
        }
      }
      // WindowEvent::HoveredFile(path) => {
      //   self.egui_input.hovered_files.push(egui::HoveredFile {
      //     path: Some(path.clone()),
      //     ..Default::default()
      //   });
      //   EventResponse {
      //     repaint: true,
      //     consumed: false,
      //   }
      // }
      // WindowEvent::HoveredFileCancelled => {
      //   self.egui_input.hovered_files.clear();
      //   EventResponse {
      //     repaint: true,
      //     consumed: false,
      //   }
      // }
      // WindowEvent::DroppedFile(path) => {
      //   self.egui_input.hovered_files.clear();
      //   self.egui_input.dropped_files.push(egui::DroppedFile {
      //     path: Some(path.clone()),
      //     ..Default::default()
      //   });
      //   EventResponse {
      //     repaint: true,
      //     consumed: false,
      //   }
      // }
      Message::ModifiersChanged { shift, ctrl, alt, win } => {
        self.egui_input.modifiers.alt = alt.is_pressed();
        self.egui_input.modifiers.ctrl = ctrl.is_pressed();
        self.egui_input.modifiers.shift = shift.is_pressed();
        self.egui_input.modifiers.mac_cmd = false;
        self.egui_input.modifiers.command = false;

        EventResponse {
          repaint: true,
          consumed: false,
        }
      }

      // Things that may require repaint:
      Message::Paint
      // | WindowEvent::Occluded(_)
      | Message::Resized(_)
      | Message::Moved(_)
      // | WindowEvent::ThemeChanged(_)
      // | WindowEvent::TouchpadPressure { .. }
      | Message::CloseRequested => EventResponse {
        repaint: true,
        consumed: false,
      },

      // Things we completely ignore:
        Message::Loop(_) |
        Message::RawInput(_) |
        Message::Created { .. } |
        Message::BoundsChanged { .. } |
        Message::Command |
        Message::SystemCommand => EventResponse {
        repaint: false,
        consumed: false,
      },

      // Things we completely ignore:
      // WindowEvent::ActivationTokenDone { .. }
      // | WindowEvent::AxisMotion { .. }
      // | WindowEvent::SmartMagnify { .. }
      // | WindowEvent::TouchpadRotate { .. } => EventResponse {
      //   repaint: false,
      //   consumed: false,
      // },

      // WindowEvent::TouchpadMagnify { delta, .. } => {
      //   // Positive delta values indicate magnification (zooming in).
      //   // Negative delta values indicate shrinking (zooming out).
      //   let zoom_factor = (*delta as f32).exp();
      //   self.egui_input.events.push(egui::Event::Zoom(zoom_factor));
      //   EventResponse {
      //     repaint: true,
      //     consumed: self.egui_ctx.wants_pointer_input(),
      //   }
      // }
    }
  }

  pub fn on_mouse_motion(&mut self, delta: (f64, f64)) {
    self.egui_input.events.push(egui::Event::PointerMoved(Pos2 {
      x: delta.0 as f32,
      y: delta.1 as f32,
    }));
  }

  fn on_mouse_button_input(&mut self, button: MouseButton, state: ButtonState) {
    if let Some(pos) = self.pointer_pos_in_points {
      if let Some(button) = translate_mouse_button(button) {
        let pressed = state == ButtonState::Pressed;

        self.egui_input.events.push(egui::Event::PointerButton {
          pos,
          button,
          pressed,
          modifiers: self.egui_input.modifiers,
        });

        if self.simulate_touch_screen {
          if pressed {
            self.any_pointer_button_down = true;

            self.egui_input.events.push(egui::Event::Touch {
              device_id: egui::TouchDeviceId(0),
              id: egui::TouchId(0),
              phase: egui::TouchPhase::Start,
              pos,
              force: None,
            });
          } else {
            self.any_pointer_button_down = false;

            self.egui_input.events.push(egui::Event::PointerGone);

            self.egui_input.events.push(egui::Event::Touch {
              device_id: egui::TouchDeviceId(0),
              id: egui::TouchId(0),
              phase: egui::TouchPhase::End,
              pos,
              force: None,
            });
          };
        }
      }
    }
  }

  fn on_cursor_moved(&mut self, window: &Window, pos_in_pixels: PhysicalPosition) {
    let pixels_per_point = pixels_per_point(&self.egui_ctx, window);

    let pos_in_points = egui::pos2(
      pos_in_pixels.x as f32 / pixels_per_point,
      pos_in_pixels.y as f32 / pixels_per_point,
    );
    self.pointer_pos_in_points = Some(pos_in_points);

    if self.simulate_touch_screen {
      if self.any_pointer_button_down {
        self
          .egui_input
          .events
          .push(egui::Event::PointerMoved(pos_in_points));

        self.egui_input.events.push(egui::Event::Touch {
          device_id: egui::TouchDeviceId(0),
          id: egui::TouchId(0),
          phase: egui::TouchPhase::Move,
          pos: pos_in_points,
          force: None,
        });
      }
    } else {
      self
        .egui_input
        .events
        .push(egui::Event::PointerMoved(pos_in_points));
    }
  }

  // fn on_touch(&mut self, window: &Window, touch: &winit::event::Touch) {
  //   let pixels_per_point = pixels_per_point(&self.egui_ctx, window);

  //   // Emit touch event
  //   self.egui_input.events.push(egui::Event::Touch {
  //     device_id:
  // egui::TouchDeviceId(egui::epaint::util::hash(touch.device_id)),
  //     id: egui::TouchId::from(touch.id),
  //     phase: match touch.phase {
  //       winit::event::TouchPhase::Started => egui::TouchPhase::Start,
  //       winit::event::TouchPhase::Moved => egui::TouchPhase::Move,
  //       winit::event::TouchPhase::Ended => egui::TouchPhase::End,
  //       winit::event::TouchPhase::Cancelled => egui::TouchPhase::Cancel,
  //     },
  //     pos: egui::pos2(
  //       touch.location.x as f32 / pixels_per_point,
  //       touch.location.y as f32 / pixels_per_point,
  //     ),
  //     force: match touch.force {
  //       Some(winit::event::Force::Normalized(force)) => Some(force as f32),
  //       Some(winit::event::Force::Calibrated {
  //         force,
  //         max_possible_force,
  //         ..
  //       }) => Some((force / max_possible_force) as f32),
  //       None => None,
  //     },
  //   });
  //   // If we're not yet translating a touch or we're translating this very
  //   // touch …
  //   if self.pointer_touch_id.is_none() || self.pointer_touch_id.unwrap() ==
  // touch.id {     // … emit PointerButton resp. PointerMoved events to emulate
  // mouse     match touch.phase {
  //       winit::event::TouchPhase::Started => {
  //         self.pointer_touch_id = Some(touch.id);
  //         // First move the pointer to the right location
  //         self.on_cursor_moved(window, touch.location);
  //         self.on_mouse_button_input(
  //           winit::event::ElementState::Pressed,
  //           winit::event::MouseButton::Left,
  //         );
  //       }
  //       winit::event::TouchPhase::Moved => {
  //         self.on_cursor_moved(window, touch.location);
  //       }
  //       winit::event::TouchPhase::Ended => {
  //         self.pointer_touch_id = None;
  //         self.on_mouse_button_input(
  //           winit::event::ElementState::Released,
  //           winit::event::MouseButton::Left,
  //         );
  //         // The pointer should vanish completely to not get any
  //         // hover effects
  //         self.pointer_pos_in_points = None;
  //         self.egui_input.events.push(egui::Event::PointerGone);
  //       }
  //       winit::event::TouchPhase::Cancelled => {
  //         self.pointer_touch_id = None;
  //         self.pointer_pos_in_points = None;
  //         self.egui_input.events.push(egui::Event::PointerGone);
  //       }
  //     }
  //   }
  // }

  fn on_mouse_wheel(&mut self, window: &Window, delta_x: f32, delta_y: f32) {
    let pixels_per_point = pixels_per_point(&self.egui_ctx, window);

    {
      let (unit, delta) = (
        egui::MouseWheelUnit::Point,
        egui::vec2(delta_x as f32, delta_y as f32) / pixels_per_point,
      );
      let modifiers = self.egui_input.modifiers;
      self.egui_input.events.push(egui::Event::MouseWheel {
        unit,
        delta,
        modifiers,
      });
    }

    let delta = egui::vec2(delta_x as f32, delta_y as f32) / pixels_per_point;

    if self.egui_input.modifiers.ctrl || self.egui_input.modifiers.command {
      // Treat as zoom instead:
      let factor = (delta.y / 200.0).exp();
      self.egui_input.events.push(egui::Event::Zoom(factor));
    } else if self.egui_input.modifiers.shift {
      // Treat as horizontal scrolling.
      // Note: one Mac we already get horizontal scroll events when shift is down.
      self
        .egui_input
        .events
        .push(egui::Event::Scroll(egui::vec2(delta.x + delta.y, 0.0)));
    } else {
      self.egui_input.events.push(egui::Event::Scroll(delta));
    }
  }

  fn on_keyboard_input(&mut self, key: &Key, state: &KeyState) {
    let pressed = state.is_pressed();

    let physical_key = key_from_key_code(key);
    let logical_key = key_from_winit_key(key);

    if let Some(logical_key) = logical_key {
      if pressed {
        if is_cut_command(self.egui_input.modifiers, logical_key) {
          self.egui_input.events.push(egui::Event::Cut);
          return;
        } else if is_copy_command(self.egui_input.modifiers, logical_key) {
          self.egui_input.events.push(egui::Event::Copy);
          return;
        } else if is_paste_command(self.egui_input.modifiers, logical_key) {
          if let Some(contents) = self.clipboard.get() {
            let contents = contents.replace("\r\n", "\n");
            if !contents.is_empty() {
              self.egui_input.events.push(egui::Event::Paste(contents));
            }
          }
          return;
        }
      }

      self.egui_input.events.push(egui::Event::Key {
        key: logical_key,
        physical_key,
        pressed,
        repeat: false, // egui will fill this in for us!
        modifiers: self.egui_input.modifiers,
      });
    }

    // if let Some(text) = &text {}
  }

  fn on_text_input(&mut self, text: &String) {
    // Make sure there is text, and that it is not control characters
    // (e.g. delete is sent as "\u{f728}" on macOS).
    if !text.is_empty() && text.chars().all(is_printable_char) {
      // On some platforms we get here when the user presses Cmd-C (copy), ctrl-W,
      // etc. We need to ignore these characters that are side-effects of
      // commands. Also make sure the key is pressed (not released). On Linux,
      // text might contain some data even when the key is released.
      let is_cmd = self.egui_input.modifiers.ctrl
        || self.egui_input.modifiers.command
        || self.egui_input.modifiers.mac_cmd;
      if !is_cmd {
        self
          .egui_input
          .events
          .push(egui::Event::Text(text.to_string()));
      }
    }
  }

  /// Call with the output given by `egui`.
  ///
  /// This will, if needed:
  /// * update the cursor
  /// * copy text to the clipboard
  /// * open any clicked urls
  /// * update the IME
  /// *
  pub fn handle_platform_output(
    &mut self,
    window: &Window,
    platform_output: egui::PlatformOutput,
  ) {
    let egui::PlatformOutput {
      cursor_icon,
      open_url,
      copied_text,
      events: _,                    // handled elsewhere
      mutable_text_under_cursor: _, // only used in eframe web
      ime,
      #[cfg(feature = "accesskit")]
      accesskit_update,
    } = platform_output;

    self.set_cursor_icon(window, cursor_icon);

    if let Some(open_url) = open_url {
      open_url_in_browser(&open_url.url);
    }

    if !copied_text.is_empty() {
      self.clipboard.set(copied_text);
    }

    // let allow_ime = ime.is_some();
    // if self.allow_ime != allow_ime {
    //   self.allow_ime = allow_ime;
    //   // crate::profile_scope!("set_ime_allowed");
    //   window.set_ime_allowed(allow_ime);
    // }

    // if let Some(ime) = ime {
    //   let rect = ime.rect;
    //   let pixels_per_point = pixels_per_point(&self.egui_ctx, window);
    //   crate::profile_scope!("set_ime_cursor_area");
    //   window.set_ime_cursor_area(
    //     winit::dpi::PhysicalPosition {
    //       x: pixels_per_point * rect.min.x,
    //       y: pixels_per_point * rect.min.y,
    //     },
    //     winit::dpi::PhysicalSize {
    //       width: pixels_per_point * rect.width(),
    //       height: pixels_per_point * rect.height(),
    //     },
    //   );
    // }

    // #[cfg(feature = "accesskit")]
    // if let Some(accesskit) = self.accesskit.as_ref() {
    //   if let Some(update) = accesskit_update {
    //     crate::profile_scope!("accesskit");
    //     accesskit.update_if_active(|| update);
    //   }
    // }
  }

  fn set_cursor_icon(&mut self, window: &Window, cursor_icon: egui::CursorIcon) {
    if self.current_cursor_icon == Some(cursor_icon) {
      // Prevent flickering near frame boundary when Windows OS tries to control
      // cursor icon for window resizing. On other platforms: just early-out to
      // save CPU.
      return;
    }

    let is_pointer_in_window = self.pointer_pos_in_points.is_some();
    if is_pointer_in_window {
      self.current_cursor_icon = Some(cursor_icon);

      if let Some(winit_cursor_icon) = translate_cursor(cursor_icon) {
        window.set_cursor_visibility(Visibility::Shown);
        // window.set_cursor_icon(winit_cursor_icon);
      } else {
        window.set_cursor_visibility(Visibility::Hidden);
      }
    } else {
      // Remember to set the cursor again once the cursor returns to the screen:
      self.current_cursor_icon = None;
    }
  }
}

/// Update the given viewport info with the current state of the window.
///
/// Call before [`State::take_egui_input`].
pub fn update_viewport_info(
  viewport_info: &mut ViewportInfo,
  egui_ctx: &egui::Context,
  window: &Window,
) {
  let pixels_per_point = pixels_per_point(egui_ctx, window);

  let has_a_position = !window.is_minimized();

  let inner_pos_px = if has_a_position {
    let p = window.inner_position();
    Some((p.x as f32, p.y as f32).into())
  } else {
    None
  };

  let outer_pos_px = if has_a_position {
    let p = window.outer_position();
    Some((p.x as f32, p.y as f32).into())
  } else {
    None
  };

  let inner_size_px = if has_a_position {
    let size = window.inner_size();
    Some(Vec2::new(size.width as f32, size.height as f32))
  } else {
    None
  };

  let outer_size_px = if has_a_position {
    let size = window.outer_size();
    Some(Vec2::new(size.width as f32, size.height as f32))
  } else {
    None
  };

  let inner_rect_px = if let (Some(pos), Some(size)) = (inner_pos_px, inner_size_px) {
    Some(Rect::from_min_size(pos, size))
  } else {
    None
  };

  let outer_rect_px = if let (Some(pos), Some(size)) = (outer_pos_px, outer_size_px) {
    Some(Rect::from_min_size(pos, size))
  } else {
    None
  };

  let inner_rect = inner_rect_px.map(|r| r / pixels_per_point);
  let outer_rect = outer_rect_px.map(|r| r / pixels_per_point);

  let monitor_size = {
    // crate::profile_scope!("monitor_size");
    let monitor = window.current_monitor();
    let size = monitor.size().as_logical(pixels_per_point.into());
    Some(egui::vec2(size.width as f32, size.height as f32))
  };

  viewport_info.focused = Some(window.has_focus());
  viewport_info.fullscreen = Some(window.fullscreen().is_some());
  viewport_info.inner_rect = inner_rect;
  viewport_info.monitor_size = monitor_size;
  viewport_info.native_pixels_per_point = Some(window.scale_factor() as f32);
  viewport_info.outer_rect = outer_rect;
  viewport_info.title = Some(window.title());

  if cfg!(target_os = "windows") {
    // It's tempting to do this, but it leads to a deadlock on Mac when running
    // `cargo run -p custom_window_frame`.
    // See https://github.com/emilk/egui/issues/3494
    viewport_info.maximized = Some(window.is_maximized());
    viewport_info.minimized = Some(window.is_minimized());
  }
}

fn open_url_in_browser(_url: &str) {
  #[cfg(feature = "webbrowser")]
  if let Err(err) = webbrowser::open(_url) {
    log::warn!("Failed to open url: {}", err);
  }

  #[cfg(not(feature = "webbrowser"))]
  {
    tracing::warn!("Cannot open url - feature \"links\" not enabled.");
  }
}

/// Winit sends special keys (backspace, delete, F1, …) as characters.
/// Ignore those.
/// We also ignore '\r', '\n', '\t'.
/// Newlines are handled by the `Key::Enter` event.
fn is_printable_char(chr: char) -> bool {
  let is_in_private_use_area = '\u{e000}' <= chr && chr <= '\u{f8ff}'
    || '\u{f0000}' <= chr && chr <= '\u{ffffd}'
    || '\u{100000}' <= chr && chr <= '\u{10fffd}';

  !is_in_private_use_area && !chr.is_ascii_control()
}

fn is_cut_command(modifiers: egui::Modifiers, keycode: egui::Key) -> bool {
  keycode == egui::Key::Cut
    || (modifiers.command && keycode == egui::Key::X)
    || (cfg!(target_os = "windows") && modifiers.shift && keycode == egui::Key::Delete)
}

fn is_copy_command(modifiers: egui::Modifiers, keycode: egui::Key) -> bool {
  keycode == egui::Key::Copy
    || (modifiers.command && keycode == egui::Key::C)
    || (cfg!(target_os = "windows") && modifiers.ctrl && keycode == egui::Key::Insert)
}

fn is_paste_command(modifiers: egui::Modifiers, keycode: egui::Key) -> bool {
  keycode == egui::Key::Paste
    || (modifiers.command && keycode == egui::Key::V)
    || (cfg!(target_os = "windows") && modifiers.shift && keycode == egui::Key::Insert)
}

fn translate_mouse_button(button: MouseButton) -> Option<egui::PointerButton> {
  match button {
    MouseButton::Left => Some(egui::PointerButton::Primary),
    MouseButton::Right => Some(egui::PointerButton::Secondary),
    MouseButton::Middle => Some(egui::PointerButton::Middle),
    MouseButton::Back => Some(egui::PointerButton::Extra1),
    MouseButton::Forward => Some(egui::PointerButton::Extra2),
    MouseButton::Unknown => None,
  }
}

fn key_from_winit_key(key: &Key) -> Option<egui::Key> {
  match key {
    Key::Unknown /*| winit::keyboard::Key::Dead(_)*/ => None,
    named_key => key_from_named_key(*named_key),
    // Key::Character(str) => egui::Key::from_name(str.as_str()),
  }
}

fn key_from_named_key(named_key: Key) -> Option<egui::Key> {
  // use egui::Key;
  // use winit::keyboard::NamedKey;

  Some(match named_key {
    Key::Enter => egui::Key::Enter,
    Key::Tab => egui::Key::Tab,
    Key::Down => egui::Key::ArrowDown,
    Key::Left => egui::Key::ArrowLeft,
    Key::Right => egui::Key::ArrowRight,
    Key::Up => egui::Key::ArrowUp,
    Key::End => egui::Key::End,
    Key::Home => egui::Key::Home,
    Key::PageDown => egui::Key::PageDown,
    Key::PageUp => egui::Key::PageUp,
    Key::Backspace => egui::Key::Backspace,
    Key::Delete => egui::Key::Delete,
    Key::Insert => egui::Key::Insert,
    Key::Escape => egui::Key::Escape,
    Key::Copy => egui::Key::Copy,

    Key::Space => egui::Key::Space,

    Key::F1 => egui::Key::F1,
    Key::F2 => egui::Key::F2,
    Key::F3 => egui::Key::F3,
    Key::F4 => egui::Key::F4,
    Key::F5 => egui::Key::F5,
    Key::F6 => egui::Key::F6,
    Key::F7 => egui::Key::F7,
    Key::F8 => egui::Key::F8,
    Key::F9 => egui::Key::F9,
    Key::F10 => egui::Key::F10,
    Key::F11 => egui::Key::F11,
    Key::F12 => egui::Key::F12,
    Key::F13 => egui::Key::F13,
    Key::F14 => egui::Key::F14,
    Key::F15 => egui::Key::F15,
    Key::F16 => egui::Key::F16,
    Key::F17 => egui::Key::F17,
    Key::F18 => egui::Key::F18,
    Key::F19 => egui::Key::F19,
    Key::F20 => egui::Key::F20,
    _ => {
      tracing::trace!("Unknown key: {named_key:?}");
      return None;
    }
  })
}

fn key_from_key_code(key: &crate::window::input::key::Key) -> Option<egui::Key> {
  use egui::Key;

  use crate::window::input::key::Key as KeyCode;

  Some(match key {
    KeyCode::Down => Key::ArrowDown,
    KeyCode::Left => Key::ArrowLeft,
    KeyCode::Right => Key::ArrowRight,
    KeyCode::Up => Key::ArrowUp,

    KeyCode::Escape => Key::Escape,
    KeyCode::Tab => Key::Tab,
    KeyCode::Backspace => Key::Backspace,
    KeyCode::Enter | KeyCode::NumEnter => Key::Enter,

    KeyCode::Insert => Key::Insert,
    KeyCode::Delete => Key::Delete,
    KeyCode::Home => Key::Home,
    KeyCode::End => Key::End,
    KeyCode::PageUp => Key::PageUp,
    KeyCode::PageDown => Key::PageDown,

    // Punctuation
    KeyCode::Space => Key::Space,
    KeyCode::Comma => Key::Comma,
    KeyCode::Period => Key::Period,
    // KeyCode::Colon => Key::Colon, // NOTE: there is no physical colon key on an
    // american keyboard
    KeyCode::Semicolon => Key::Semicolon,
    KeyCode::BackSlash => Key::Backslash,
    KeyCode::ForwardSlash | KeyCode::NumDivide => Key::Slash,
    KeyCode::LeftBracket => Key::OpenBracket,
    KeyCode::RightBracket => Key::CloseBracket,
    KeyCode::Accent => Key::Backtick,

    KeyCode::Copy => Key::Copy,
    KeyCode::Minus | KeyCode::NumMinus => Key::Minus,
    KeyCode::NumPlus => Key::Plus,
    KeyCode::Equals => Key::Equals,

    KeyCode::_0 | KeyCode::Num0 => Key::Num0,
    KeyCode::_1 | KeyCode::Num1 => Key::Num1,
    KeyCode::_2 | KeyCode::Num2 => Key::Num2,
    KeyCode::_3 | KeyCode::Num3 => Key::Num3,
    KeyCode::_4 | KeyCode::Num4 => Key::Num4,
    KeyCode::_5 | KeyCode::Num5 => Key::Num5,
    KeyCode::_6 | KeyCode::Num6 => Key::Num6,
    KeyCode::_7 | KeyCode::Num7 => Key::Num7,
    KeyCode::_8 | KeyCode::Num8 => Key::Num8,
    KeyCode::_9 | KeyCode::Num9 => Key::Num9,

    KeyCode::A => Key::A,
    KeyCode::B => Key::B,
    KeyCode::C => Key::C,
    KeyCode::D => Key::D,
    KeyCode::E => Key::E,
    KeyCode::F => Key::F,
    KeyCode::G => Key::G,
    KeyCode::H => Key::H,
    KeyCode::I => Key::I,
    KeyCode::J => Key::J,
    KeyCode::K => Key::K,
    KeyCode::L => Key::L,
    KeyCode::M => Key::M,
    KeyCode::N => Key::N,
    KeyCode::O => Key::O,
    KeyCode::P => Key::P,
    KeyCode::Q => Key::Q,
    KeyCode::R => Key::R,
    KeyCode::S => Key::S,
    KeyCode::T => Key::T,
    KeyCode::U => Key::U,
    KeyCode::V => Key::V,
    KeyCode::W => Key::W,
    KeyCode::X => Key::X,
    KeyCode::Y => Key::Y,
    KeyCode::Z => Key::Z,

    KeyCode::F1 => Key::F1,
    KeyCode::F2 => Key::F2,
    KeyCode::F3 => Key::F3,
    KeyCode::F4 => Key::F4,
    KeyCode::F5 => Key::F5,
    KeyCode::F6 => Key::F6,
    KeyCode::F7 => Key::F7,
    KeyCode::F8 => Key::F8,
    KeyCode::F9 => Key::F9,
    KeyCode::F10 => Key::F10,
    KeyCode::F11 => Key::F11,
    KeyCode::F12 => Key::F12,
    KeyCode::F13 => Key::F13,
    KeyCode::F14 => Key::F14,
    KeyCode::F15 => Key::F15,
    KeyCode::F16 => Key::F16,
    KeyCode::F17 => Key::F17,
    KeyCode::F18 => Key::F18,
    KeyCode::F19 => Key::F19,
    KeyCode::F20 => Key::F20,

    _ => {
      return None;
    }
  })
}

fn translate_cursor(cursor_icon: egui::CursorIcon) -> Option<CursorIcon> {
  match cursor_icon {
    egui::CursorIcon::None => None,

    egui::CursorIcon::Alias => Some(CursorIcon::Alias),
    egui::CursorIcon::AllScroll => Some(CursorIcon::AllScroll),
    egui::CursorIcon::Cell => Some(CursorIcon::Cell),
    egui::CursorIcon::ContextMenu => Some(CursorIcon::ContextMenu),
    egui::CursorIcon::Copy => Some(CursorIcon::Copy),
    egui::CursorIcon::Crosshair => Some(CursorIcon::Crosshair),
    egui::CursorIcon::Default => Some(CursorIcon::Default),
    egui::CursorIcon::Grab => Some(CursorIcon::Grab),
    egui::CursorIcon::Grabbing => Some(CursorIcon::Grabbing),
    egui::CursorIcon::Help => Some(CursorIcon::Help),
    egui::CursorIcon::Move => Some(CursorIcon::Move),
    egui::CursorIcon::NoDrop => Some(CursorIcon::NoDrop),
    egui::CursorIcon::NotAllowed => Some(CursorIcon::NotAllowed),
    egui::CursorIcon::PointingHand => Some(CursorIcon::Pointer),
    egui::CursorIcon::Progress => Some(CursorIcon::Progress),

    egui::CursorIcon::ResizeHorizontal => Some(CursorIcon::EwResize),
    egui::CursorIcon::ResizeNeSw => Some(CursorIcon::NeswResize),
    egui::CursorIcon::ResizeNwSe => Some(CursorIcon::NwseResize),
    egui::CursorIcon::ResizeVertical => Some(CursorIcon::NsResize),

    egui::CursorIcon::ResizeEast => Some(CursorIcon::EResize),
    egui::CursorIcon::ResizeSouthEast => Some(CursorIcon::SeResize),
    egui::CursorIcon::ResizeSouth => Some(CursorIcon::SResize),
    egui::CursorIcon::ResizeSouthWest => Some(CursorIcon::SwResize),
    egui::CursorIcon::ResizeWest => Some(CursorIcon::WResize),
    egui::CursorIcon::ResizeNorthWest => Some(CursorIcon::NwResize),
    egui::CursorIcon::ResizeNorth => Some(CursorIcon::NResize),
    egui::CursorIcon::ResizeNorthEast => Some(CursorIcon::NeResize),
    egui::CursorIcon::ResizeColumn => Some(CursorIcon::ColResize),
    egui::CursorIcon::ResizeRow => Some(CursorIcon::RowResize),

    egui::CursorIcon::Text => Some(CursorIcon::Text),
    egui::CursorIcon::VerticalText => Some(CursorIcon::VerticalText),
    egui::CursorIcon::Wait => Some(CursorIcon::Wait),
    egui::CursorIcon::ZoomIn => Some(CursorIcon::ZoomIn),
    egui::CursorIcon::ZoomOut => Some(CursorIcon::ZoomOut),
  }
}

// Helpers for egui Viewports
// ---------------------------------------------------------------------------

// pub fn process_viewport_commands(
//   egui_ctx: &egui::Context,
//   info: &mut ViewportInfo,
//   commands: impl IntoIterator<Item = ViewportCommand>,
//   window: &Window,
//   is_viewport_focused: bool,
//   screenshot_requested: &mut bool,
// ) {
//   for command in commands {
//     process_viewport_command(
//       egui_ctx,
//       window,
//       command,
//       info,
//       is_viewport_focused,
//       screenshot_requested,
//     );
//   }
// }

// fn process_viewport_command(
//   egui_ctx: &egui::Context,
//   window: &Window,
//   command: ViewportCommand,
//   info: &mut ViewportInfo,
//   is_viewport_focused: bool,
//   screenshot_requested: &mut bool,
// ) {
//   use winit::window::ResizeDirection;

//   // log::trace!("Processing ViewportCommand::{command:?}");

//   let pixels_per_point = pixels_per_point(egui_ctx, window);

//   match command {
//     ViewportCommand::Close => {
//       info.events.push(egui::ViewportEvent::Close);
//     }
//     ViewportCommand::CancelClose => {
//       // Need to be handled elsewhere
//     }
//     ViewportCommand::StartDrag => {
//       // If `is_viewport_focused` is not checked on x11 the input will be permanently
//       // taken until the app is killed!

//       // TODO: check that the left mouse-button was pressed down recently,
//       // or we will have bugs on Windows.
//       // See https://github.com/emilk/egui/pull/1108
//       if is_viewport_focused {
//         if let Err(err) = window.drag_window() {
//           log::warn!("{command:?}: {err}");
//         }
//       }
//     }
//     ViewportCommand::InnerSize(size) => {
//       let width_px = pixels_per_point * size.x.max(1.0);
//       let height_px = pixels_per_point * size.y.max(1.0);
//       if window
//         .request_inner_size(PhysicalSize::new(width_px, height_px))
//         .is_some()
//       {
//         log::debug!("ViewportCommand::InnerSize ignored by winit");
//       }
//     }
//     ViewportCommand::BeginResize(direction) => {
//       if let Err(err) = window.drag_resize_window(match direction {
//         egui::viewport::ResizeDirection::North => ResizeDirection::North,
//         egui::viewport::ResizeDirection::South => ResizeDirection::South,
//         egui::viewport::ResizeDirection::East => ResizeDirection::East,
//         egui::viewport::ResizeDirection::West => ResizeDirection::West,
//         egui::viewport::ResizeDirection::NorthEast => ResizeDirection::NorthEast,
//         egui::viewport::ResizeDirection::SouthEast => ResizeDirection::SouthEast,
//         egui::viewport::ResizeDirection::NorthWest => ResizeDirection::NorthWest,
//         egui::viewport::ResizeDirection::SouthWest => ResizeDirection::SouthWest,
//       }) {
//         log::warn!("{command:?}: {err}");
//       }
//     }
//     ViewportCommand::Title(title) => {
//       window.set_title(&title);
//     }
//     ViewportCommand::Transparent(v) => window.set_transparent(v),
//     ViewportCommand::Visible(v) => window.set_visible(v),
//     ViewportCommand::OuterPosition(pos) => {
//       window.set_outer_position(PhysicalPosition::new(
//         pixels_per_point * pos.x,
//         pixels_per_point * pos.y,
//       ));
//     }
//     ViewportCommand::MinInnerSize(s) => {
//       window.set_min_inner_size(
//         (s.is_finite() && s != Vec2::ZERO)
//           .then_some(PhysicalSize::new(pixels_per_point * s.x, pixels_per_point * s.y)),
//       );
//     }
//     ViewportCommand::MaxInnerSize(s) => {
//       window.set_max_inner_size(
//         (s.is_finite() && s != Vec2::INFINITY)
//           .then_some(PhysicalSize::new(pixels_per_point * s.x, pixels_per_point * s.y)),
//       );
//     }
//     ViewportCommand::ResizeIncrements(s) => {
//       window.set_resize_increments(
//         s.map(|s| PhysicalSize::new(pixels_per_point * s.x, pixels_per_point * s.y)),
//       );
//     }
//     ViewportCommand::Resizable(v) => window.set_resizable(v),
//     ViewportCommand::EnableButtons {
//       close,
//       minimized,
//       maximize,
//     } => window.set_enabled_buttons(
//       if close {
//         WindowButtons::CLOSE
//       } else {
//         WindowButtons::empty()
//       } | if minimized {
//         WindowButtons::MINIMIZE
//       } else {
//         WindowButtons::empty()
//       } | if maximize {
//         WindowButtons::MAXIMIZE
//       } else {
//         WindowButtons::empty()
//       },
//     ),
//     ViewportCommand::Minimized(v) => {
//       window.set_minimized(v);
//       info.minimized = Some(v);
//     }
//     ViewportCommand::Maximized(v) => {
//       window.set_maximized(v);
//       info.maximized = Some(v);
//     }
//     ViewportCommand::Fullscreen(v) => {
//       window.set_fullscreen(v.then_some(winit::window::Fullscreen::Borderless(None)));
//     }
//     ViewportCommand::Decorations(v) => window.set_decorations(v),
//     ViewportCommand::WindowLevel(l) => window.set_window_level(match l {
//       egui::viewport::WindowLevel::AlwaysOnBottom => WindowLevel::AlwaysOnBottom,
//       egui::viewport::WindowLevel::AlwaysOnTop => WindowLevel::AlwaysOnTop,
//       egui::viewport::WindowLevel::Normal => WindowLevel::Normal,
//     }),
//     ViewportCommand::Icon(icon) => {
//       let winit_icon = icon.and_then(|icon| to_winit_icon(&icon));
//       window.set_window_icon(winit_icon);
//     }
//     ViewportCommand::IMERect(rect) => {
//       window.set_ime_cursor_area(
//         PhysicalPosition::new(
//           pixels_per_point * rect.min.x,
//           pixels_per_point * rect.min.y,
//         ),
//         PhysicalSize::new(
//           pixels_per_point * rect.size().x,
//           pixels_per_point * rect.size().y,
//         ),
//       );
//     }
//     ViewportCommand::IMEAllowed(v) => window.set_ime_allowed(v),
//     ViewportCommand::IMEPurpose(p) => window.set_ime_purpose(match p {
//       egui::viewport::IMEPurpose::Password => winit::window::ImePurpose::Password,
//       egui::viewport::IMEPurpose::Terminal => winit::window::ImePurpose::Terminal,
//       egui::viewport::IMEPurpose::Normal => winit::window::ImePurpose::Normal,
//     }),
//     ViewportCommand::Focus => {
//       if !window.has_focus() {
//         window.focus_window();
//       }
//     }
//     ViewportCommand::RequestUserAttention(a) => {
//       window.request_user_attention(match a {
//         egui::UserAttentionType::Reset => None,
//         egui::UserAttentionType::Critical => {
//           Some(winit::window::UserAttentionType::Critical)
//         }
//         egui::UserAttentionType::Informational => {
//           Some(winit::window::UserAttentionType::Informational)
//         }
//       });
//     }
//     ViewportCommand::SetTheme(t) => window.set_theme(match t {
//       egui::SystemTheme::Light => Some(winit::window::Theme::Light),
//       egui::SystemTheme::Dark => Some(winit::window::Theme::Dark),
//       egui::SystemTheme::SystemDefault => None,
//     }),
//     ViewportCommand::ContentProtected(v) => window.set_content_protected(v),
//     ViewportCommand::CursorPosition(pos) => {
//       if let Err(err) = window.set_cursor_position(PhysicalPosition::new(
//         pixels_per_point * pos.x,
//         pixels_per_point * pos.y,
//       )) {
//         log::warn!("{command:?}: {err}");
//       }
//     }
//     ViewportCommand::CursorGrab(o) => {
//       if let Err(err) = window.set_cursor_grab(match o {
//         egui::viewport::CursorGrab::None => CursorGrabMode::None,
//         egui::viewport::CursorGrab::Confined => CursorGrabMode::Confined,
//         egui::viewport::CursorGrab::Locked => CursorGrabMode::Locked,
//       }) {
//         log::warn!("{command:?}: {err}");
//       }
//     }
//     ViewportCommand::CursorVisible(v) => window.set_cursor_visible(v),
//     ViewportCommand::MousePassthrough(passthrough) => {
//       if let Err(err) = window.set_cursor_hittest(!passthrough) {
//         log::warn!("{command:?}: {err}");
//       }
//     }
//     ViewportCommand::Screenshot => {
//       *screenshot_requested = true;
//     }
//   }
// }

// /// Build and intitlaize a window.
// ///
// /// Wrapper around `create_winit_window_builder` and
// /// `apply_viewport_builder_to_window`.
// pub fn create_window<T>(
//   egui_ctx: &egui::Context,
//   event_loop: &EventLoopWindowTarget<T>,
//   viewport_builder: &ViewportBuilder,
// ) -> Result<Window, winit::error::OsError> {
//   let window_builder =
//     create_winit_window_builder(egui_ctx, event_loop, viewport_builder.clone());
//   let window = {
//     crate::profile_scope!("WindowBuilder::build");
//     window_builder.build(event_loop)?
//   };
//   apply_viewport_builder_to_window(egui_ctx, &window, viewport_builder);
//   Ok(window)
// }

// pub fn create_winit_window_builder<T>(
//   egui_ctx: &egui::Context,
//   event_loop: &EventLoopWindowTarget<T>,
//   viewport_builder: ViewportBuilder,
// ) -> winit::window::WindowBuilder {
//   // We set sizes and positions in egui:s own ui points, which depends on the egui
//   // zoom_factor and the native pixels per point, so we need to know that here.
//   // We don't know what monitor the window will appear on though, but
//   // we'll try to fix that after the window is created in the vall to
//   // `apply_viewport_builder_to_window`.
//   let native_pixels_per_point = event_loop
//     .primary_monitor()
//     .or_else(|| event_loop.available_monitors().next())
//     .map_or_else(
//       || {
//         // log::debug!("Failed to find a monitor - assuming native_pixels_per_point of 1.0");
//         1.0
//       },
//       |m| m.scale_factor() as f32,
//     );
//   let zoom_factor = egui_ctx.zoom_factor();
//   let pixels_per_point = zoom_factor * native_pixels_per_point;

//   let ViewportBuilder {
//     title,
//     position,
//     inner_size,
//     min_inner_size,
//     max_inner_size,
//     fullscreen,
//     maximized,
//     resizable,
//     transparent,
//     decorations,
//     icon,
//     active,
//     visible,
//     close_button,
//     minimize_button,
//     maximize_button,
//     window_level,

//     // macOS:
//     fullsize_content_view: _fullsize_content_view,
//     title_shown: _title_shown,
//     titlebar_buttons_shown: _titlebar_buttons_shown,
//     titlebar_shown: _titlebar_shown,

//     // Windows:
//     drag_and_drop: _drag_and_drop,
//     taskbar: _taskbar,

//     // wayland:
//     app_id: _app_id,

//     mouse_passthrough: _, // handled in `apply_viewport_builder_to_window`
//   } = viewport_builder;

//   let mut window_builder = winit::window::WindowBuilder::new()
//     .with_title(title.unwrap_or_else(|| "egui window".to_owned()))
//     .with_transparent(transparent.unwrap_or(false))
//     .with_decorations(decorations.unwrap_or(true))
//     .with_resizable(resizable.unwrap_or(true))
//     .with_visible(visible.unwrap_or(true))
//     .with_maximized(maximized.unwrap_or(false))
//     .with_window_level(match window_level.unwrap_or_default() {
//       egui::viewport::WindowLevel::AlwaysOnBottom => WindowLevel::AlwaysOnBottom,
//       egui::viewport::WindowLevel::AlwaysOnTop => WindowLevel::AlwaysOnTop,
//       egui::viewport::WindowLevel::Normal => WindowLevel::Normal,
//     })
//     .with_fullscreen(
//       fullscreen.and_then(|e| e.then_some(winit::window::Fullscreen::Borderless(None))),
//     )
//     .with_enabled_buttons({
//       let mut buttons = WindowButtons::empty();
//       if minimize_button.unwrap_or(true) {
//         buttons |= WindowButtons::MINIMIZE;
//       }
//       if maximize_button.unwrap_or(true) {
//         buttons |= WindowButtons::MAXIMIZE;
//       }
//       if close_button.unwrap_or(true) {
//         buttons |= WindowButtons::CLOSE;
//       }
//       buttons
//     })
//     .with_active(active.unwrap_or(true));

//   if let Some(size) = inner_size {
//     window_builder = window_builder.with_inner_size(PhysicalSize::new(
//       pixels_per_point * size.x,
//       pixels_per_point * size.y,
//     ));
//   }

//   if let Some(size) = min_inner_size {
//     window_builder = window_builder.with_min_inner_size(PhysicalSize::new(
//       pixels_per_point * size.x,
//       pixels_per_point * size.y,
//     ));
//   }

//   if let Some(size) = max_inner_size {
//     window_builder = window_builder.with_max_inner_size(PhysicalSize::new(
//       pixels_per_point * size.x,
//       pixels_per_point * size.y,
//     ));
//   }

//   if let Some(pos) = position {
//     window_builder = window_builder.with_position(PhysicalPosition::new(
//       pixels_per_point * pos.x,
//       pixels_per_point * pos.y,
//     ));
//   }

//   if let Some(icon) = icon {
//     let winit_icon = to_winit_icon(&icon);
//     window_builder = window_builder.with_window_icon(winit_icon);
//   }

//   #[cfg(all(feature = "wayland", target_os = "linux"))]
//   if let Some(app_id) = _app_id {
//     use winit::platform::wayland::WindowBuilderExtWayland as _;
//     window_builder = window_builder.with_name(app_id, "");
//   }

//   #[cfg(target_os = "windows")]
//   {
//     use winit::platform::windows::WindowBuilderExtWindows as _;
//     if let Some(enable) = _drag_and_drop {
//       window_builder = window_builder.with_drag_and_drop(enable);
//     }
//     if let Some(show) = _taskbar {
//       window_builder = window_builder.with_skip_taskbar(!show);
//     }
//   }

//   #[cfg(target_os = "macos")]
//   {
//     use winit::platform::macos::WindowBuilderExtMacOS as _;
//     window_builder = window_builder
//       .with_title_hidden(!_title_shown.unwrap_or(true))
//       .with_titlebar_buttons_hidden(!_titlebar_buttons_shown.unwrap_or(true))
//       .with_titlebar_transparent(!_titlebar_shown.unwrap_or(true))
//       .with_fullsize_content_view(_fullsize_content_view.unwrap_or(false));
//   }

//   window_builder
// }

// fn to_winit_icon(icon: &egui::IconData) -> Option<winit::window::Icon> {
//   if icon.is_empty() {
//     None
//   } else {
//     match winit::window::Icon::from_rgba(icon.rgba.clone(), icon.width, icon.height) {
//       Ok(winit_icon) => Some(winit_icon),
//       Err(err) => {
//         log::warn!("Invalid IconData: {err}");
//         None
//       }
//     }
//   }
// }

// /// Applies what `create_winit_window_builder` couldn't
// pub fn apply_viewport_builder_to_window(
//   egui_ctx: &egui::Context,
//   window: &Window,
//   builder: &ViewportBuilder,
// ) {
//   if let Some(mouse_passthrough) = builder.mouse_passthrough {
//     if let Err(err) = window.set_cursor_hittest(!mouse_passthrough) {
//       log::warn!("set_cursor_hittest failed: {err}");
//     }
//   }

//   {
//     // In `create_winit_window_builder` we didn't know
//     // on what monitor the window would appear, so we didn't know
//     // how to translate egui ui point to native physical pixels.
//     // Now we do know:

//     let pixels_per_point = pixels_per_point(egui_ctx, window);

//     if let Some(size) = builder.inner_size {
//       if window
//         .request_inner_size(PhysicalSize::new(
//           pixels_per_point * size.x,
//           pixels_per_point * size.y,
//         ))
//         .is_some()
//       {
//         log::debug!("Failed to set window size");
//       }
//     }
//     if let Some(size) = builder.min_inner_size {
//       window.set_min_inner_size(Some(PhysicalSize::new(
//         pixels_per_point * size.x,
//         pixels_per_point * size.y,
//       )));
//     }
//     if let Some(size) = builder.max_inner_size {
//       window.set_max_inner_size(Some(PhysicalSize::new(
//         pixels_per_point * size.x,
//         pixels_per_point * size.y,
//       )));
//     }
//     if let Some(pos) = builder.position {
//       let pos = PhysicalPosition::new(pixels_per_point * pos.x, pixels_per_point * pos.y);
//       window.set_outer_position(pos);
//     }
//   }
// }

// // ---------------------------------------------------------------------------

// /// Short and fast description of an event.
// /// Useful for logging and profiling.
// pub fn short_generic_event_description<T>(
//   event: &winit::event::Event<T>,
// ) -> &'static str {
//   use winit::event::{DeviceEvent, Event, StartCause};

//   match event {
//     Event::AboutToWait => "Event::AboutToWait",
//     Event::LoopExiting => "Event::LoopExiting",
//     Event::Suspended => "Event::Suspended",
//     Event::Resumed => "Event::Resumed",
//     Event::MemoryWarning => "Event::MemoryWarning",
//     Event::UserEvent(_) => "UserEvent",
//     Event::DeviceEvent { event, .. } => match event {
//       DeviceEvent::Added { .. } => "DeviceEvent::Added",
//       DeviceEvent::Removed { .. } => "DeviceEvent::Removed",
//       DeviceEvent::MouseMotion { .. } => "DeviceEvent::MouseMotion",
//       DeviceEvent::MouseWheel { .. } => "DeviceEvent::MouseWheel",
//       DeviceEvent::Motion { .. } => "DeviceEvent::Motion",
//       DeviceEvent::Button { .. } => "DeviceEvent::Button",
//       DeviceEvent::Key { .. } => "DeviceEvent::Key",
//     },
//     Event::NewEvents(start_cause) => match start_cause {
//       StartCause::ResumeTimeReached { .. } => "NewEvents::ResumeTimeReached",
//       StartCause::WaitCancelled { .. } => "NewEvents::WaitCancelled",
//       StartCause::Poll => "NewEvents::Poll",
//       StartCause::Init => "NewEvents::Init",
//     },
//     Event::WindowEvent { event, .. } => short_window_event_description(event),
//   }
// }

// /// Short and fast description of an event.
// /// Useful for logging and profiling.
// pub fn short_window_event_description(event: &winit::event::WindowEvent) -> &'static str {
//   use winit::event::WindowEvent;

//   match event {
//     WindowEvent::ActivationTokenDone { .. } => "WindowEvent::ActivationTokenDone",
//     WindowEvent::Resized { .. } => "WindowEvent::Resized",
//     WindowEvent::Moved { .. } => "WindowEvent::Moved",
//     WindowEvent::CloseRequested { .. } => "WindowEvent::CloseRequested",
//     WindowEvent::Destroyed { .. } => "WindowEvent::Destroyed",
//     WindowEvent::DroppedFile { .. } => "WindowEvent::DroppedFile",
//     WindowEvent::HoveredFile { .. } => "WindowEvent::HoveredFile",
//     WindowEvent::HoveredFileCancelled { .. } => "WindowEvent::HoveredFileCancelled",
//     WindowEvent::Focused { .. } => "WindowEvent::Focused",
//     WindowEvent::KeyboardInput { .. } => "WindowEvent::KeyboardInput",
//     WindowEvent::ModifiersChanged { .. } => "WindowEvent::ModifiersChanged",
//     WindowEvent::Ime { .. } => "WindowEvent::Ime",
//     WindowEvent::CursorMoved { .. } => "WindowEvent::CursorMoved",
//     WindowEvent::CursorEntered { .. } => "WindowEvent::CursorEntered",
//     WindowEvent::CursorLeft { .. } => "WindowEvent::CursorLeft",
//     WindowEvent::MouseWheel { .. } => "WindowEvent::MouseWheel",
//     WindowEvent::MouseInput { .. } => "WindowEvent::MouseInput",
//     WindowEvent::TouchpadMagnify { .. } => "WindowEvent::TouchpadMagnify",
//     WindowEvent::RedrawRequested { .. } => "WindowEvent::RedrawRequested",
//     WindowEvent::SmartMagnify { .. } => "WindowEvent::SmartMagnify",
//     WindowEvent::TouchpadRotate { .. } => "WindowEvent::TouchpadRotate",
//     WindowEvent::TouchpadPressure { .. } => "WindowEvent::TouchpadPressure",
//     WindowEvent::AxisMotion { .. } => "WindowEvent::AxisMotion",
//     WindowEvent::Touch { .. } => "WindowEvent::Touch",
//     WindowEvent::ScaleFactorChanged { .. } => "WindowEvent::ScaleFactorChanged",
//     WindowEvent::ThemeChanged { .. } => "WindowEvent::ThemeChanged",
//     WindowEvent::Occluded { .. } => "WindowEvent::Occluded",
//   }
// }
