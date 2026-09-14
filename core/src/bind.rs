//! What a hotkey is, before any of it touches Windows.
//!
//! Binds are captured from the low-level hook rather than from the webview,
//! because a browser keydown cannot see the mouse side buttons and those are
//! the ones people actually bind a zoom to.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    X1,
    X2,
}

impl MouseButton {
    pub fn label(self) -> &'static str {
        match self {
            MouseButton::Left => "LMB",
            MouseButton::Right => "RMB",
            MouseButton::Middle => "MMB",
            MouseButton::X1 => "M4",
            MouseButton::X2 => "M5",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Trigger {
    Key { vk: u32 },
    Mouse { button: MouseButton },
}

impl Trigger {
    pub fn label(self) -> String {
        match self {
            Trigger::Mouse { button } => button.label().to_string(),
            Trigger::Key { vk } => key_label(vk),
        }
    }
}

/// How holding the trigger behaves.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Active only while the trigger is down.
    #[default]
    Hold,
    /// Each press flips the state.
    Toggle,
}

/// What a bind drives. Actions are addressed per target window except for
/// `Master` and `Panic`, which are global by definition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Action {
    Resize,
    BlackBars,
    Scope,
    Crosshair,
    LayerZoom,
    /// Disables and re-enables the Resize bind without opening the app.
    Master,
    /// Restores every window tlk-grid has touched.
    Panic,
}

impl Action {
    /// Master and Panic have to work while the app is in the background and
    /// must never eat the key — otherwise a stuck state cannot be escaped.
    pub fn is_global(self) -> bool {
        matches!(self, Action::Master | Action::Panic)
    }

    pub fn may_swallow(self) -> bool {
        !self.is_global()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bind {
    pub action: Action,
    pub trigger: Trigger,
    #[serde(default)]
    pub mode: Mode,
    /// Keep the key from reaching the game. Ignored for global actions.
    #[serde(default = "yes")]
    pub swallow: bool,
}

fn yes() -> bool {
    true
}

impl Bind {
    pub fn new(action: Action, trigger: Trigger) -> Self {
        Self {
            action,
            trigger,
            mode: Mode::default(),
            swallow: action.may_swallow(),
        }
    }

    pub fn swallows(&self) -> bool {
        self.swallow && self.action.may_swallow()
    }
}

/// The default emergency reset, spelled out so both the hook and the guide
/// agree on it.
pub const PANIC_KEY: u32 = 0x77; // VK_F8

pub fn panic_bind() -> Bind {
    Bind {
        action: Action::Panic,
        trigger: Trigger::Key { vk: PANIC_KEY },
        mode: Mode::Toggle,
        swallow: false,
    }
}

/// Human-readable name for a virtual-key code, for the bind field and the
/// profile file. Unknown codes keep their hex so a profile never loses a bind.
pub fn key_label(vk: u32) -> String {
    let named = match vk {
        0x08 => "Backspace",
        0x09 => "Tab",
        0x0D => "Enter",
        0x13 => "Pause",
        0x14 => "CapsLock",
        0x1B => "Esc",
        0x20 => "Space",
        0x21 => "PageUp",
        0x22 => "PageDown",
        0x23 => "End",
        0x24 => "Home",
        0x25 => "Left",
        0x26 => "Up",
        0x27 => "Right",
        0x28 => "Down",
        0x2C => "PrintScreen",
        0x2D => "Insert",
        0x2E => "Delete",
        0x5B => "LWin",
        0x5C => "RWin",
        0x5D => "Menu",
        0x90 => "NumLock",
        0x91 => "ScrollLock",
        0xA0 => "LShift",
        0xA1 => "RShift",
        0xA2 => "LCtrl",
        0xA3 => "RCtrl",
        0xA4 => "LAlt",
        0xA5 => "RAlt",
        0xBA => ";",
        0xBB => "=",
        0xBC => ",",
        0xBD => "-",
        0xBE => ".",
        0xBF => "/",
        0xC0 => "`",
        0xDB => "[",
        0xDC => "\\",
        0xDD => "]",
        0xDE => "'",
        _ => "",
    };
    if !named.is_empty() {
        return named.to_string();
    }
    match vk {
        0x30..=0x39 => char::from(vk as u8).to_string(),
        0x41..=0x5A => char::from(vk as u8).to_string(),
        0x60..=0x69 => format!("Num{}", vk - 0x60),
        0x6A => "Num*".to_string(),
        0x6B => "Num+".to_string(),
        0x6D => "Num-".to_string(),
        0x6E => "Num.".to_string(),
        0x6F => "Num/".to_string(),
        0x70..=0x87 => format!("F{}", vk - 0x6F),
        _ => format!("VK 0x{vk:02X}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mouse_side_buttons_read_the_way_players_name_them() {
        assert_eq!(MouseButton::X1.label(), "M4");
        assert_eq!(MouseButton::X2.label(), "M5");
        assert_eq!(MouseButton::Right.label(), "RMB");
    }

    #[test]
    fn function_and_letter_keys_are_named() {
        assert_eq!(key_label(0x77), "F8");
        assert_eq!(key_label(0x70), "F1");
        assert_eq!(key_label(0x41), "A");
        assert_eq!(key_label(0x39), "9");
        assert_eq!(key_label(0x62), "Num2");
    }

    #[test]
    fn an_unknown_code_survives_as_hex_rather_than_vanishing() {
        assert_eq!(key_label(0xFE), "VK 0xFE");
    }

    #[test]
    fn the_master_bind_can_never_swallow_its_key() {
        let mut bind = Bind::new(
            Action::Master,
            Trigger::Mouse {
                button: MouseButton::X1,
            },
        );
        assert!(!bind.swallows());
        bind.swallow = true;
        assert!(!bind.swallows(), "a global action must stay pass-through");
    }

    #[test]
    fn a_resize_bind_swallows_by_default() {
        let bind = Bind::new(
            Action::Resize,
            Trigger::Mouse {
                button: MouseButton::Right,
            },
        );
        assert!(bind.swallows());
        assert_eq!(bind.mode, Mode::Hold);
    }

    #[test]
    fn f8_is_the_emergency_reset() {
        assert_eq!(panic_bind().trigger.label(), "F8");
        assert!(!panic_bind().swallows());
    }
}
