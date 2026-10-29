//! Stable keyboard binding model.
//!
//! Bindings use egui's **logical** key, which follows the active keyboard
//! layout (with egui falling back to the physical key for non-Latin layouts).
//! Display labels therefore match the key the user actually pressed, but a
//! saved binding can resolve to a different physical position on another
//! layout. This choice is intentional and documented in the Shortcuts dialog.

use std::fmt;

use egui::{Event, Key, Modifiers};

/// The small key set CutCraft accepts for shortcuts.
///
/// Text characters are represented by their logical keys rather than raw
/// scan codes; punctuation is included where the built-in defaults need it.
pub const SUPPORTED_KEYS: [Key; 64] = [
    Key::A,
    Key::B,
    Key::C,
    Key::D,
    Key::E,
    Key::F,
    Key::G,
    Key::H,
    Key::I,
    Key::J,
    Key::K,
    Key::L,
    Key::M,
    Key::N,
    Key::O,
    Key::P,
    Key::Q,
    Key::R,
    Key::S,
    Key::T,
    Key::U,
    Key::V,
    Key::W,
    Key::X,
    Key::Y,
    Key::Z,
    Key::Num0,
    Key::Num1,
    Key::Num2,
    Key::Num3,
    Key::Num4,
    Key::Num5,
    Key::Num6,
    Key::Num7,
    Key::Num8,
    Key::Num9,
    Key::F1,
    Key::F2,
    Key::F3,
    Key::F4,
    Key::F5,
    Key::F6,
    Key::F7,
    Key::F8,
    Key::F9,
    Key::F10,
    Key::F11,
    Key::F12,
    Key::ArrowLeft,
    Key::ArrowRight,
    Key::ArrowUp,
    Key::ArrowDown,
    Key::Home,
    Key::End,
    Key::PageUp,
    Key::PageDown,
    Key::Insert,
    Key::Delete,
    Key::Backspace,
    Key::Tab,
    Key::Enter,
    Key::Space,
    Key::Slash,
    Key::Equals,
];

/// Punctuation that can also be persisted, used by built-in defaults.
const EXTRA_SUPPORTED_KEYS: [Key; 10] = [
    Key::Minus,
    Key::Comma,
    Key::Period,
    Key::Backtick,
    Key::Semicolon,
    Key::Quote,
    Key::OpenBracket,
    Key::CloseBracket,
    Key::Plus,
    Key::Escape,
];

/// One normalized keyboard combination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShortcutBinding {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub key: Key,
}

impl ShortcutBinding {
    pub const fn new(key: Key, ctrl: bool, shift: bool, alt: bool) -> Self {
        Self {
            ctrl,
            shift,
            alt,
            key,
        }
    }

    pub fn from_key(key: Key, modifiers: &Modifiers) -> Self {
        Self {
            ctrl: modifiers.ctrl || modifiers.mac_cmd,
            shift: modifiers.shift,
            alt: modifiers.alt,
            key,
        }
    }

    /// Exact modifier matching. egui's helper ignores extra Shift/Alt presses,
    /// which would let Ctrl+K steal Ctrl+Shift+K; shortcuts must be exact.
    pub fn matches(self, key: Key, modifiers: &Modifiers) -> bool {
        self.key == key
            && self.ctrl == (modifiers.ctrl || modifiers.mac_cmd)
            && self.shift == modifiers.shift
            && self.alt == modifiers.alt
    }

    pub fn parsed_modifiers(self) -> Modifiers {
        Modifiers {
            ctrl: self.ctrl,
            shift: self.shift,
            alt: self.alt,
            ..Default::default()
        }
    }

    /// Stable persistence form, e.g. `Ctrl+Shift+K` or `Ctrl+Equals`.
    pub fn serialized(self) -> String {
        let mut parts = Vec::with_capacity(4);
        if self.ctrl {
            parts.push("Ctrl".to_owned());
        }
        if self.shift {
            parts.push("Shift".to_owned());
        }
        if self.alt {
            parts.push("Alt".to_owned());
        }
        parts.push(self.key.name().to_owned());
        parts.join("+")
    }

    /// Human-readable form shown in tooltips, e.g. `Ctrl+Shift+K`.
    pub fn label(self) -> String {
        self.keycaps().join("+")
    }

    pub fn keycaps(self) -> Vec<String> {
        let mut caps = Vec::with_capacity(4);
        if self.ctrl {
            caps.push("Ctrl".to_owned());
        }
        if self.shift {
            caps.push("Shift".to_owned());
        }
        if self.alt {
            caps.push("Alt".to_owned());
        }
        caps.push(display_key(self.key).to_owned());
        caps
    }

    pub fn parse(raw: &str) -> Result<Self, BindingParseError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(BindingParseError("empty binding".to_owned()));
        }

        let mut ctrl = false;
        let mut shift = false;
        let mut alt = false;
        let mut key = None;
        for part in trimmed.split('+').map(str::trim) {
            if part.is_empty() {
                return Err(BindingParseError(format!("invalid binding '{raw}'")));
            }
            match part.to_ascii_lowercase().as_str() {
                "ctrl" | "control" | "cmd" | "command" => {
                    if ctrl {
                        return Err(BindingParseError("duplicate Ctrl modifier".to_owned()));
                    }
                    ctrl = true;
                }
                "shift" => {
                    if shift {
                        return Err(BindingParseError("duplicate Shift modifier".to_owned()));
                    }
                    shift = true;
                }
                "alt" | "option" => {
                    if alt {
                        return Err(BindingParseError("duplicate Alt modifier".to_owned()));
                    }
                    alt = true;
                }
                _ => {
                    if key.is_some() {
                        return Err(BindingParseError(format!("invalid binding '{raw}'")));
                    }
                    key = Some(parse_key(part)?);
                }
            }
        }

        let Some(key) = key else {
            return Err(BindingParseError("binding has no key".to_owned()));
        };
        if !is_supported_key(key) {
            return Err(BindingParseError(format!(
                "'{}' cannot be used in a shortcut",
                key.name()
            )));
        }
        Ok(Self::new(key, ctrl, shift, alt))
    }
}

impl fmt::Display for ShortcutBinding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.label())
    }
}

/// Result of feeding one egui event to the capture loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureOutcome {
    Captured(ShortcutBinding),
    Cancelled,
    Reserved,
    Unsupported,
    Ignored,
}

/// Interpret one event while the dialog is recording a shortcut.
///
/// Standalone modifier presses do not produce `Event::Key` in egui and are
/// therefore ignored. Windows-key combinations never reach the app; Alt+F4 is
/// intercepted by the OS but rejected here as well for keyboard capture safety.
pub fn capture_outcome(event: &Event) -> CaptureOutcome {
    let Event::Key {
        key,
        pressed: true,
        repeat: false,
        modifiers,
        ..
    } = event
    else {
        return CaptureOutcome::Ignored;
    };

    if *key == Key::Escape {
        return CaptureOutcome::Cancelled;
    }
    if *key == Key::F4 && modifiers.alt {
        return CaptureOutcome::Reserved;
    }
    if !is_supported_key(*key) {
        return CaptureOutcome::Unsupported;
    }
    CaptureOutcome::Captured(ShortcutBinding::from_key(*key, modifiers))
}

pub fn is_supported_key(key: Key) -> bool {
    SUPPORTED_KEYS.contains(&key) || EXTRA_SUPPORTED_KEYS.contains(&key)
}

fn parse_key(raw: &str) -> Result<Key, BindingParseError> {
    let token = raw.trim().to_ascii_lowercase();
    for key in SUPPORTED_KEYS.into_iter().chain(EXTRA_SUPPORTED_KEYS) {
        if key.name().eq_ignore_ascii_case(&token) || display_key(key).eq_ignore_ascii_case(&token)
        {
            return Ok(key);
        }
    }
    match token.as_str() {
        "prime" => Ok(Key::Plus),
        "esc" => Ok(Key::Escape),
        "return" => Ok(Key::Enter),
        _ => Err(BindingParseError(format!("unknown key '{raw}'"))),
    }
}

fn display_key(key: Key) -> &'static str {
    match key {
        Key::ArrowLeft => "←",
        Key::ArrowRight => "→",
        Key::ArrowUp => "↑",
        Key::ArrowDown => "↓",
        Key::Escape => "Esc",
        Key::PageUp => "PgUp",
        Key::PageDown => "PgDn",
        Key::Equals => "=",
        Key::Minus => "−",
        Key::Plus => "+",
        Key::Slash => "/",
        Key::Comma => ",",
        Key::Period => ".",
        Key::Backtick => "`",
        Key::Semicolon => ";",
        Key::Quote => "'",
        Key::OpenBracket => "[",
        Key::CloseBracket => "]",
        _ => key.name(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingParseError(pub String);

impl fmt::Display for BindingParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for BindingParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn key_event(key: Key, modifiers: Modifiers, repeat: bool) -> Event {
        Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat,
            modifiers,
        }
    }

    #[test]
    fn serialization_round_trips_modifier_combinations() {
        for (key, ctrl, shift, alt) in [
            (Key::K, true, true, false),
            (Key::Z, true, false, false),
            (Key::Equals, true, false, false),
            (Key::ArrowLeft, false, true, false),
            (Key::F11, false, false, false),
        ] {
            let binding = ShortcutBinding::new(key, ctrl, shift, alt);
            let parsed = ShortcutBinding::parse(&binding.serialized()).expect("valid binding");
            assert_eq!(parsed, binding);
        }
    }

    #[test]
    fn modifier_matching_is_exact_and_order_independent() {
        let ctrl_k = ShortcutBinding::new(Key::K, true, false, false);
        assert!(ctrl_k.matches(
            Key::K,
            &Modifiers {
                ctrl: true,
                ..Default::default()
            }
        ));
        assert!(!ctrl_k.matches(
            Key::K,
            &Modifiers {
                ctrl: true,
                shift: true,
                ..Default::default()
            }
        ));
        assert!(!ctrl_k.matches(Key::K, &Modifiers::NONE));
        assert!(!ctrl_k.matches(
            Key::L,
            &Modifiers {
                ctrl: true,
                ..Default::default()
            }
        ));

        let combined = ShortcutBinding::new(Key::K, true, true, true);
        assert_eq!(combined.serialized(), "Ctrl+Shift+Alt+K");
    }

    #[test]
    fn keycaps_use_compact_display_labels() {
        let binding = ShortcutBinding::new(Key::ArrowLeft, true, true, false);
        assert_eq!(binding.keycaps(), vec!["Ctrl", "Shift", "←"]);
        assert_eq!(binding.label(), "Ctrl+Shift+←");
    }

    #[test]
    fn capture_rejects_reserved_unsupported_repeat_and_modifier_events() {
        assert_eq!(
            capture_outcome(&key_event(
                Key::F4,
                Modifiers {
                    alt: true,
                    ..Default::default()
                },
                false
            )),
            CaptureOutcome::Reserved
        );
        assert_eq!(
            capture_outcome(&key_event(Key::Escape, Modifiers::NONE, false)),
            CaptureOutcome::Cancelled
        );
        assert_eq!(
            capture_outcome(&key_event(Key::Space, Modifiers::NONE, true)),
            CaptureOutcome::Ignored
        );
        assert_eq!(
            capture_outcome(&key_event(
                Key::Copy,
                Modifiers {
                    ctrl: true,
                    ..Default::default()
                },
                false
            )),
            CaptureOutcome::Unsupported
        );
        assert_eq!(
            capture_outcome(&Event::ModifiersChanged(Modifiers {
                ctrl: true,
                ..Default::default()
            })),
            CaptureOutcome::Ignored
        );
    }

    #[test]
    fn capture_normalizes_ctrl_shift_and_alt() {
        let outcome = capture_outcome(&key_event(
            Key::K,
            Modifiers {
                ctrl: true,
                shift: true,
                alt: true,
                ..Default::default()
            },
            false,
        ));
        assert_eq!(
            outcome,
            CaptureOutcome::Captured(ShortcutBinding::new(Key::K, true, true, true))
        );
    }

    #[test]
    fn malformed_bindings_are_rejected() {
        for raw in [
            "",
            "Ctrl",
            "Ctrl+",
            "+K",
            "Ctrl+Shift+Ctrl+K",
            "Ctrl+Banana",
        ] {
            assert!(ShortcutBinding::parse(raw).is_err(), "{raw} should fail");
        }
        assert_eq!(
            ShortcutBinding::parse("control+shift+k").expect("aliases accepted"),
            ShortcutBinding::new(Key::K, true, true, false)
        );
    }
}
