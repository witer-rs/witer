use windows::Win32::UI::{
  Input::{
    KeyboardAndMouse::{MapVirtualKeyW, VIRTUAL_KEY},
    *,
  },
  WindowsAndMessaging,
};

use crate::utilities::is_flag_set;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Key {
  Unknown = 0,
  // ASCII
  Tab = 9,
  Enter = 10,
  Space = 32,
  Apostrophe = 39,
  Comma = 44,
  Minus = 45,
  Period = 46,
  ForwardSlash = 47,
  _0 = 48,
  _1 = 49,
  _2 = 50,
  _3 = 51,
  _4 = 52,
  _5 = 53,
  _6 = 54,
  _7 = 55,
  _8 = 56,
  _9 = 57,
  Semicolon = 59,
  Equals = 61,
  A = 65,
  B = 66,
  C = 67,
  D = 68,
  E = 69, // ;)
  F = 70,
  G = 71,
  H = 72,
  I = 73,
  J = 74,
  K = 75,
  L = 76,
  M = 77,
  N = 78,
  O = 79,
  P = 80,
  Q = 81,
  R = 82,
  S = 83,
  T = 84,
  U = 85,
  V = 86,
  W = 87,
  X = 88,
  Y = 89,
  Z = 90,
  LeftBracket = 91,
  BackSlash = 92,
  RightBracket = 93,
  Accent = 96,
  // Non-ASCII
  Escape = 256,
  NumEnter,
  Backspace,
  Insert,
  Delete,
  Up,
  Down,
  Left,
  Right,
  PageUp,
  PageDown,
  Home,
  End,
  CapsLock,
  ScrollLock,
  NumLock,
  PrintScreen,
  Pause,
  Num0,
  Num1,
  Num2,
  Num3,
  Num4,
  Num5,
  Num6,
  Num7,
  Num8,
  Num9,
  F1,
  F2,
  F3,
  F4,
  F5,
  F6,
  F7,
  F8,
  F9,
  F10,
  F11,
  F12,
  F13,
  F14,
  F15,
  F16,
  F17,
  F18,
  F19,
  F20,
  F21,
  F22,
  F23,
  F24,
  NumPeriod,
  NumComma,
  NumPlus,
  NumMinus,
  NumDivide,
  NumMultiply,
  NumEquals,
  LeftShift,
  LeftControl,
  LeftAlt,
  LeftSuper,
  RightShift,
  RightControl,
  RightAlt,
  RightSuper,
  Menu,
  AbntC1,
  AbntC2,
  Ax,
  Convert,
  Kana,
  Kanji,
  Mail,
  MediaPlayPause,
  MediaStop,
  MediaSelect,
  MediaNextTrack,
  MediaPrevTrack,
  VolumeDown,
  VolumeUp,
  VolumeMute,
  NoConvert,
  OEM102,
  Sleep,
  NoName,
  WebBack,
  WebFavorites,
  WebForward,
  WebHome,
  WebRefresh,
  WebSearch,
  WebStop,
  Copy,
}

impl From<VIRTUAL_KEY> for Key {
  fn from(value: VIRTUAL_KEY) -> Self {
    match value {
      KeyboardAndMouse::VK_1 => Key::_1,
      KeyboardAndMouse::VK_2 => Key::_2,
      KeyboardAndMouse::VK_3 => Key::_3,
      KeyboardAndMouse::VK_4 => Key::_4,
      KeyboardAndMouse::VK_5 => Key::_5,
      KeyboardAndMouse::VK_6 => Key::_6,
      KeyboardAndMouse::VK_7 => Key::_7,
      KeyboardAndMouse::VK_8 => Key::_8,
      KeyboardAndMouse::VK_9 => Key::_9,
      KeyboardAndMouse::VK_0 => Key::_0,
      KeyboardAndMouse::VK_A => Key::A,
      KeyboardAndMouse::VK_B => Key::B,
      KeyboardAndMouse::VK_C => Key::C,
      KeyboardAndMouse::VK_D => Key::D,
      KeyboardAndMouse::VK_E => Key::E,
      KeyboardAndMouse::VK_F => Key::F,
      KeyboardAndMouse::VK_G => Key::G,
      KeyboardAndMouse::VK_H => Key::H,
      KeyboardAndMouse::VK_I => Key::I,
      KeyboardAndMouse::VK_J => Key::J,
      KeyboardAndMouse::VK_K => Key::K,
      KeyboardAndMouse::VK_L => Key::L,
      KeyboardAndMouse::VK_M => Key::M,
      KeyboardAndMouse::VK_N => Key::N,
      KeyboardAndMouse::VK_O => Key::O,
      KeyboardAndMouse::VK_P => Key::P,
      KeyboardAndMouse::VK_Q => Key::Q,
      KeyboardAndMouse::VK_R => Key::R,
      KeyboardAndMouse::VK_S => Key::S,
      KeyboardAndMouse::VK_T => Key::T,
      KeyboardAndMouse::VK_U => Key::U,
      KeyboardAndMouse::VK_V => Key::V,
      KeyboardAndMouse::VK_W => Key::W,
      KeyboardAndMouse::VK_X => Key::X,
      KeyboardAndMouse::VK_Y => Key::Y,
      KeyboardAndMouse::VK_Z => Key::Z,
      KeyboardAndMouse::VK_ESCAPE => Key::Escape,
      KeyboardAndMouse::VK_F1 => Key::F1,
      KeyboardAndMouse::VK_F2 => Key::F2,
      KeyboardAndMouse::VK_F3 => Key::F3,
      KeyboardAndMouse::VK_F4 => Key::F4,
      KeyboardAndMouse::VK_F5 => Key::F5,
      KeyboardAndMouse::VK_F6 => Key::F6,
      KeyboardAndMouse::VK_F7 => Key::F7,
      KeyboardAndMouse::VK_F8 => Key::F8,
      KeyboardAndMouse::VK_F9 => Key::F9,
      KeyboardAndMouse::VK_F10 => Key::F10,
      KeyboardAndMouse::VK_F11 => Key::F11,
      KeyboardAndMouse::VK_F12 => Key::F12,
      KeyboardAndMouse::VK_F13 => Key::F13,
      KeyboardAndMouse::VK_F14 => Key::F14,
      KeyboardAndMouse::VK_F15 => Key::F15,
      KeyboardAndMouse::VK_F16 => Key::F16,
      KeyboardAndMouse::VK_F17 => Key::F17,
      KeyboardAndMouse::VK_F18 => Key::F18,
      KeyboardAndMouse::VK_F19 => Key::F19,
      KeyboardAndMouse::VK_F20 => Key::F20,
      KeyboardAndMouse::VK_F21 => Key::F21,
      KeyboardAndMouse::VK_F22 => Key::F22,
      KeyboardAndMouse::VK_F23 => Key::F23,
      KeyboardAndMouse::VK_F24 => Key::F24,
      KeyboardAndMouse::VK_SNAPSHOT => Key::PrintScreen,
      KeyboardAndMouse::VK_SCROLL => Key::ScrollLock,
      KeyboardAndMouse::VK_PAUSE => Key::Pause,
      KeyboardAndMouse::VK_INSERT => Key::Insert,
      KeyboardAndMouse::VK_HOME => Key::Home,
      KeyboardAndMouse::VK_DELETE => Key::Delete,
      KeyboardAndMouse::VK_END => Key::End,
      KeyboardAndMouse::VK_NEXT => Key::PageDown,
      KeyboardAndMouse::VK_PRIOR => Key::PageUp,
      KeyboardAndMouse::VK_LEFT => Key::Left,
      KeyboardAndMouse::VK_UP => Key::Up,
      KeyboardAndMouse::VK_RIGHT => Key::Right,
      KeyboardAndMouse::VK_DOWN => Key::Down,
      KeyboardAndMouse::VK_BACK => Key::Backspace,
      KeyboardAndMouse::VK_RETURN => Key::Enter,
      KeyboardAndMouse::VK_SPACE => Key::Space,
      KeyboardAndMouse::VK_NUMLOCK => Key::NumLock,
      KeyboardAndMouse::VK_NUMPAD0 => Key::Num0,
      KeyboardAndMouse::VK_NUMPAD1 => Key::Num1,
      KeyboardAndMouse::VK_NUMPAD2 => Key::Num2,
      KeyboardAndMouse::VK_NUMPAD3 => Key::Num3,
      KeyboardAndMouse::VK_NUMPAD4 => Key::Num4,
      KeyboardAndMouse::VK_NUMPAD5 => Key::Num5,
      KeyboardAndMouse::VK_NUMPAD6 => Key::Num6,
      KeyboardAndMouse::VK_NUMPAD7 => Key::Num7,
      KeyboardAndMouse::VK_NUMPAD8 => Key::Num8,
      KeyboardAndMouse::VK_NUMPAD9 => Key::Num9,
      KeyboardAndMouse::VK_ADD => Key::NumPlus,
      KeyboardAndMouse::VK_SUBTRACT => Key::NumMinus,
      KeyboardAndMouse::VK_MULTIPLY => Key::NumMultiply,
      KeyboardAndMouse::VK_DIVIDE => Key::NumDivide,
      KeyboardAndMouse::VK_DECIMAL => Key::NumPeriod,
      KeyboardAndMouse::VK_ABNT_C1 => Key::AbntC1,
      KeyboardAndMouse::VK_ABNT_C2 => Key::AbntC2,
      KeyboardAndMouse::VK_OEM_7 => Key::Apostrophe,
      KeyboardAndMouse::VK_APPS => Key::Menu,
      KeyboardAndMouse::VK_OEM_AX => Key::Ax,
      KeyboardAndMouse::VK_OEM_5 => Key::BackSlash,
      KeyboardAndMouse::VK_CAPITAL => Key::CapsLock,
      KeyboardAndMouse::VK_OEM_COMMA => Key::Comma,
      KeyboardAndMouse::VK_CONVERT => Key::Convert,
      KeyboardAndMouse::VK_OEM_PLUS => Key::Equals,
      KeyboardAndMouse::VK_OEM_3 => Key::Accent,
      KeyboardAndMouse::VK_KANA => Key::Kana,
      KeyboardAndMouse::VK_KANJI => Key::Kanji,
      KeyboardAndMouse::VK_LMENU => Key::LeftAlt,
      KeyboardAndMouse::VK_OEM_4 => Key::LeftBracket,
      KeyboardAndMouse::VK_LCONTROL => Key::LeftControl,
      KeyboardAndMouse::VK_LSHIFT => Key::LeftShift,
      KeyboardAndMouse::VK_LWIN => Key::LeftSuper,
      KeyboardAndMouse::VK_LAUNCH_MAIL => Key::Mail,
      KeyboardAndMouse::VK_LAUNCH_MEDIA_SELECT => Key::MediaSelect,
      KeyboardAndMouse::VK_MEDIA_STOP => Key::MediaStop,
      KeyboardAndMouse::VK_OEM_MINUS => Key::Minus,
      KeyboardAndMouse::VK_VOLUME_MUTE => Key::VolumeMute,
      KeyboardAndMouse::VK_MEDIA_NEXT_TRACK => Key::MediaNextTrack,
      KeyboardAndMouse::VK_NONCONVERT => Key::NoConvert,
      KeyboardAndMouse::VK_OEM_102 => Key::OEM102,
      KeyboardAndMouse::VK_OEM_PERIOD => Key::Period,
      KeyboardAndMouse::VK_MEDIA_PLAY_PAUSE => Key::MediaPlayPause,
      KeyboardAndMouse::VK_MEDIA_PREV_TRACK => Key::MediaPrevTrack,
      KeyboardAndMouse::VK_RMENU => Key::RightAlt,
      KeyboardAndMouse::VK_OEM_6 => Key::RightBracket,
      KeyboardAndMouse::VK_RCONTROL => Key::RightControl,
      KeyboardAndMouse::VK_RSHIFT => Key::RightShift,
      KeyboardAndMouse::VK_RWIN => Key::RightSuper,
      KeyboardAndMouse::VK_OEM_1 => Key::Semicolon,
      KeyboardAndMouse::VK_OEM_2 => Key::ForwardSlash,
      KeyboardAndMouse::VK_SLEEP => Key::Sleep,
      KeyboardAndMouse::VK_TAB => Key::Tab,
      KeyboardAndMouse::VK_NONAME => Key::NoName,
      KeyboardAndMouse::VK_VOLUME_DOWN => Key::VolumeDown,
      KeyboardAndMouse::VK_VOLUME_UP => Key::VolumeUp,
      KeyboardAndMouse::VK_BROWSER_BACK => Key::WebBack,
      KeyboardAndMouse::VK_BROWSER_FAVORITES => Key::WebFavorites,
      KeyboardAndMouse::VK_BROWSER_FORWARD => Key::WebForward,
      KeyboardAndMouse::VK_BROWSER_HOME => Key::WebHome,
      KeyboardAndMouse::VK_BROWSER_REFRESH => Key::WebRefresh,
      KeyboardAndMouse::VK_BROWSER_SEARCH => Key::WebSearch,
      KeyboardAndMouse::VK_BROWSER_STOP => Key::WebStop,
      KeyboardAndMouse::VK_OEM_COPY => Key::Copy,
      _ => Key::Unknown,
    }
  }
}

impl From<Key> for VIRTUAL_KEY {
  fn from(value: Key) -> Self {
    match value {
      Key::_1 => KeyboardAndMouse::VK_1,
      Key::_2 => KeyboardAndMouse::VK_2,
      Key::_3 => KeyboardAndMouse::VK_3,
      Key::_4 => KeyboardAndMouse::VK_4,
      Key::_5 => KeyboardAndMouse::VK_5,
      Key::_6 => KeyboardAndMouse::VK_6,
      Key::_7 => KeyboardAndMouse::VK_7,
      Key::_8 => KeyboardAndMouse::VK_8,
      Key::_9 => KeyboardAndMouse::VK_9,
      Key::_0 => KeyboardAndMouse::VK_0,
      Key::A => KeyboardAndMouse::VK_A,
      Key::B => KeyboardAndMouse::VK_B,
      Key::C => KeyboardAndMouse::VK_C,
      Key::D => KeyboardAndMouse::VK_D,
      Key::E => KeyboardAndMouse::VK_E,
      Key::F => KeyboardAndMouse::VK_F,
      Key::G => KeyboardAndMouse::VK_G,
      Key::H => KeyboardAndMouse::VK_H,
      Key::I => KeyboardAndMouse::VK_I,
      Key::J => KeyboardAndMouse::VK_J,
      Key::K => KeyboardAndMouse::VK_K,
      Key::L => KeyboardAndMouse::VK_L,
      Key::M => KeyboardAndMouse::VK_M,
      Key::N => KeyboardAndMouse::VK_N,
      Key::O => KeyboardAndMouse::VK_O,
      Key::P => KeyboardAndMouse::VK_P,
      Key::Q => KeyboardAndMouse::VK_Q,
      Key::R => KeyboardAndMouse::VK_R,
      Key::S => KeyboardAndMouse::VK_S,
      Key::T => KeyboardAndMouse::VK_T,
      Key::U => KeyboardAndMouse::VK_U,
      Key::V => KeyboardAndMouse::VK_V,
      Key::W => KeyboardAndMouse::VK_W,
      Key::X => KeyboardAndMouse::VK_X,
      Key::Y => KeyboardAndMouse::VK_Y,
      Key::Z => KeyboardAndMouse::VK_Z,
      Key::Escape => KeyboardAndMouse::VK_ESCAPE,
      Key::F1 => KeyboardAndMouse::VK_F1,
      Key::F2 => KeyboardAndMouse::VK_F2,
      Key::F3 => KeyboardAndMouse::VK_F3,
      Key::F4 => KeyboardAndMouse::VK_F4,
      Key::F5 => KeyboardAndMouse::VK_F5,
      Key::F6 => KeyboardAndMouse::VK_F6,
      Key::F7 => KeyboardAndMouse::VK_F7,
      Key::F8 => KeyboardAndMouse::VK_F8,
      Key::F9 => KeyboardAndMouse::VK_F9,
      Key::F10 => KeyboardAndMouse::VK_F10,
      Key::F11 => KeyboardAndMouse::VK_F11,
      Key::F12 => KeyboardAndMouse::VK_F12,
      Key::F13 => KeyboardAndMouse::VK_F13,
      Key::F14 => KeyboardAndMouse::VK_F14,
      Key::F15 => KeyboardAndMouse::VK_F15,
      Key::F16 => KeyboardAndMouse::VK_F16,
      Key::F17 => KeyboardAndMouse::VK_F17,
      Key::F18 => KeyboardAndMouse::VK_F18,
      Key::F19 => KeyboardAndMouse::VK_F19,
      Key::F20 => KeyboardAndMouse::VK_F20,
      Key::F21 => KeyboardAndMouse::VK_F21,
      Key::F22 => KeyboardAndMouse::VK_F22,
      Key::F23 => KeyboardAndMouse::VK_F23,
      Key::F24 => KeyboardAndMouse::VK_F24,
      Key::PrintScreen => KeyboardAndMouse::VK_SNAPSHOT,
      Key::ScrollLock => KeyboardAndMouse::VK_SCROLL,
      Key::Pause => KeyboardAndMouse::VK_PAUSE,
      Key::Insert => KeyboardAndMouse::VK_INSERT,
      Key::Home => KeyboardAndMouse::VK_HOME,
      Key::Delete => KeyboardAndMouse::VK_DELETE,
      Key::End => KeyboardAndMouse::VK_END,
      Key::PageDown => KeyboardAndMouse::VK_NEXT,
      Key::PageUp => KeyboardAndMouse::VK_PRIOR,
      Key::Left => KeyboardAndMouse::VK_LEFT,
      Key::Up => KeyboardAndMouse::VK_UP,
      Key::Right => KeyboardAndMouse::VK_RIGHT,
      Key::Down => KeyboardAndMouse::VK_DOWN,
      Key::Backspace => KeyboardAndMouse::VK_BACK,
      Key::Enter => KeyboardAndMouse::VK_RETURN,
      Key::Space => KeyboardAndMouse::VK_SPACE,
      Key::NumLock => KeyboardAndMouse::VK_NUMLOCK,
      Key::Num0 => KeyboardAndMouse::VK_NUMPAD0,
      Key::Num1 => KeyboardAndMouse::VK_NUMPAD1,
      Key::Num2 => KeyboardAndMouse::VK_NUMPAD2,
      Key::Num3 => KeyboardAndMouse::VK_NUMPAD3,
      Key::Num4 => KeyboardAndMouse::VK_NUMPAD4,
      Key::Num5 => KeyboardAndMouse::VK_NUMPAD5,
      Key::Num6 => KeyboardAndMouse::VK_NUMPAD6,
      Key::Num7 => KeyboardAndMouse::VK_NUMPAD7,
      Key::Num8 => KeyboardAndMouse::VK_NUMPAD8,
      Key::Num9 => KeyboardAndMouse::VK_NUMPAD9,
      Key::NumPlus => KeyboardAndMouse::VK_ADD,
      Key::NumMinus => KeyboardAndMouse::VK_SUBTRACT,
      Key::NumMultiply => KeyboardAndMouse::VK_MULTIPLY,
      Key::NumDivide => KeyboardAndMouse::VK_DIVIDE,
      Key::NumPeriod => KeyboardAndMouse::VK_DECIMAL,
      Key::AbntC1 => KeyboardAndMouse::VK_ABNT_C1,
      Key::AbntC2 => KeyboardAndMouse::VK_ABNT_C2,
      Key::Apostrophe => KeyboardAndMouse::VK_OEM_7,
      Key::Menu => KeyboardAndMouse::VK_APPS,
      Key::Ax => KeyboardAndMouse::VK_OEM_AX,
      Key::BackSlash => KeyboardAndMouse::VK_OEM_5,
      Key::CapsLock => KeyboardAndMouse::VK_CAPITAL,
      Key::Comma => KeyboardAndMouse::VK_OEM_COMMA,
      Key::Convert => KeyboardAndMouse::VK_CONVERT,
      Key::Equals => KeyboardAndMouse::VK_OEM_PLUS,
      Key::Accent => KeyboardAndMouse::VK_OEM_3,
      Key::Kana => KeyboardAndMouse::VK_KANA,
      Key::Kanji => KeyboardAndMouse::VK_KANJI,
      Key::LeftAlt => KeyboardAndMouse::VK_LMENU,
      Key::LeftBracket => KeyboardAndMouse::VK_OEM_4,
      Key::LeftControl => KeyboardAndMouse::VK_LCONTROL,
      Key::LeftShift => KeyboardAndMouse::VK_LSHIFT,
      Key::LeftSuper => KeyboardAndMouse::VK_LWIN,
      Key::Mail => KeyboardAndMouse::VK_LAUNCH_MAIL,
      Key::MediaSelect => KeyboardAndMouse::VK_LAUNCH_MEDIA_SELECT,
      Key::MediaStop => KeyboardAndMouse::VK_MEDIA_STOP,
      Key::Minus => KeyboardAndMouse::VK_OEM_MINUS,
      Key::VolumeMute => KeyboardAndMouse::VK_VOLUME_MUTE,
      Key::MediaNextTrack => KeyboardAndMouse::VK_MEDIA_NEXT_TRACK,
      Key::NoConvert => KeyboardAndMouse::VK_NONCONVERT,
      Key::OEM102 => KeyboardAndMouse::VK_OEM_102,
      Key::Period => KeyboardAndMouse::VK_OEM_PERIOD,
      Key::MediaPlayPause => KeyboardAndMouse::VK_MEDIA_PLAY_PAUSE,
      Key::MediaPrevTrack => KeyboardAndMouse::VK_MEDIA_PREV_TRACK,
      Key::RightAlt => KeyboardAndMouse::VK_RMENU,
      Key::RightBracket => KeyboardAndMouse::VK_OEM_6,
      Key::RightControl => KeyboardAndMouse::VK_RCONTROL,
      Key::RightShift => KeyboardAndMouse::VK_RSHIFT,
      Key::RightSuper => KeyboardAndMouse::VK_RWIN,
      Key::Semicolon => KeyboardAndMouse::VK_OEM_1,
      Key::ForwardSlash => KeyboardAndMouse::VK_OEM_2,
      Key::Sleep => KeyboardAndMouse::VK_SLEEP,
      Key::Tab => KeyboardAndMouse::VK_TAB,
      Key::NoName => KeyboardAndMouse::VK_NONAME,
      Key::VolumeDown => KeyboardAndMouse::VK_VOLUME_DOWN,
      Key::VolumeUp => KeyboardAndMouse::VK_VOLUME_UP,
      Key::WebBack => KeyboardAndMouse::VK_BROWSER_BACK,
      Key::WebFavorites => KeyboardAndMouse::VK_BROWSER_FAVORITES,
      Key::WebForward => KeyboardAndMouse::VK_BROWSER_FORWARD,
      Key::WebHome => KeyboardAndMouse::VK_BROWSER_HOME,
      Key::WebRefresh => KeyboardAndMouse::VK_BROWSER_REFRESH,
      Key::WebSearch => KeyboardAndMouse::VK_BROWSER_SEARCH,
      Key::WebStop => KeyboardAndMouse::VK_BROWSER_STOP,
      Key::Copy => KeyboardAndMouse::VK_OEM_COPY,
      Key::NumEnter => KeyboardAndMouse::VK_RETURN,
      Key::NumComma => KeyboardAndMouse::VK_OEM_COMMA,
      Key::NumEquals => KeyboardAndMouse::VK_OEM_PLUS,
      Key::Unknown => VIRTUAL_KEY(0x00),
    }
  }
}

impl Key {
  /*
   Stolen from winit, under the Apache-2.0 license. See winit's license for more details.
  */
  pub(crate) fn from_raw(keyboard: RAWKEYBOARD) -> Option<Key> {
    let extension = {
      if is_flag_set(keyboard.Flags, WindowsAndMessaging::RI_KEY_E0 as _) {
        0xE000
      } else if is_flag_set(keyboard.Flags, WindowsAndMessaging::RI_KEY_E1 as _) {
        0xE100
      } else {
        0x0000
      }
    };
    let scancode = if keyboard.MakeCode == 0 {
      // In some cases (often with media keys) the device reports a scancode of 0 but
      // a valid virtual key. In these cases we obtain the scancode from the
      // virtual key.
      unsafe {
        MapVirtualKeyW(keyboard.VKey as u32, KeyboardAndMouse::MAPVK_VK_TO_VSC_EX) as u16
      }
    } else {
      keyboard.MakeCode | extension
    };
    if scancode == 0xE11D || scancode == 0xE02A {
      // At the hardware (or driver?) level, pressing the Pause key is equivalent to
      // pressing Ctrl+NumLock.
      // This equvalence means that if the user presses Pause, the keyboard will emit
      // two subsequent keypresses:
      // 1, 0xE11D - Which is a left Ctrl (0x1D) with an extension flag (0xE100)
      // 2, 0x0045 - Which on its own can be interpreted as Pause
      //
      // There's another combination which isn't quite an equivalence:
      // PrtSc used to be Shift+Asterisk. This means that on some keyboards, presssing
      // PrtSc (print screen) produces the following sequence:
      // 1, 0xE02A - Which is a left shift (0x2A) with an extension flag (0xE000)
      // 2, 0xE037 - Which is a numpad multiply (0x37) with an exteion flag (0xE000).
      // This on             its own it can be interpreted as PrtSc
      //
      // For this reason, if we encounter the first keypress, we simply ignore it,
      // trusting that there's going to be another event coming, from which we
      // can extract the appropriate key.
      // For more on this, read the article by Raymond Chen, titled:
      // "Why does Ctrl+ScrollLock cancel dialogs?"
      // https://devblogs.microsoft.com/oldnewthing/20080211-00/?p=23503
      return None;
    }
    let physical_key = if keyboard.VKey == KeyboardAndMouse::VK_NUMLOCK.0 {
      // Historically, the NumLock and the Pause key were one and the same physical
      // key. The user could trigger Pause by pressing Ctrl+NumLock.
      // Now these are often physically separate and the two keys can be
      // differentiated by checking the extension flag of the scancode. NumLock
      // is 0xE045, Pause is 0x0045.
      //
      // However in this event, both keys are reported as 0x0045 even on modern
      // hardware. Therefore we use the virtual key instead to determine whether
      // it's a NumLock and set the KeyCode accordingly.
      //
      // For more on this, read the article by Raymond Chen, titled:
      // "Why does Ctrl+ScrollLock cancel dialogs?"
      // https://devblogs.microsoft.com/oldnewthing/20080211-00/?p=23503
      Key::NumLock
    } else {
      Key::from(VIRTUAL_KEY(unsafe {
        MapVirtualKeyW(scancode as u32, KeyboardAndMouse::MAPVK_VSC_TO_VK_EX) as u16
      }))
    };
    if keyboard.VKey == KeyboardAndMouse::VK_SHIFT.0 {
      match physical_key {
        Key::NumPeriod
        | Key::Num0
        | Key::Num1
        | Key::Num2
        | Key::Num3
        | Key::Num4
        | Key::Num5
        | Key::Num6
        | Key::Num7
        | Key::Num8
        | Key::Num9 => {
          // On Windows, holding the Shift key makes numpad keys behave as if NumLock
          // wasn't active. The way this is exposed to applications by the system is
          // that the application receives a fake key release event for the
          // shift key at the moment when the numpad key is pressed, just
          // before receiving the numpad key as well.
          //
          // The issue is that in the raw device event (here), the fake shift release
          // event reports the numpad key as the scancode. Unfortunately, the event
          // doesn't have any information to tell whether it's the left
          // shift or the right shift that needs to get the fake release (or
          // press) event so we don't forward this event to the application
          // at all.
          //
          // For more on this, read the article by Raymond Chen, titled:
          // "The shift key overrides NumLock"
          // https://devblogs.microsoft.com/oldnewthing/20040906-00/?p=37953
          return None;
        }
        _ => (),
      }
    }

    Some(physical_key)
  }
}
