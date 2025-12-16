#[derive(Clone, Debug)]
pub struct KeyEvent {
    pub key: Key,
    pub text: Option<String>,
    pub modifiers: Modifiers,
    pub state: KeyState,
}

impl KeyEvent {
    /// Convert the key event to a string representation suitable for keybinding matching.
    /// Returns None for keys that don't produce meaningful input (like bare modifier presses).
    pub fn to_key_string(&self) -> Option<String> {
        // Handle Ctrl combinations
        if self.modifiers.ctrl {
            return self.key.to_base_char().map(|c| format!("C-{}", c));
        }

        // If we have UTF-8 text and it's printable, use it directly
        // This handles shifted characters like : from Shift+; automatically
        if let Some(ref t) = self.text
            && !t.is_empty()
        {
            let c = t.chars().next().unwrap();
            if !c.is_control() {
                return Some(t.clone());
            }
        }

        // Fallback: use key's character representation
        self.key.to_string_with_shift(self.modifiers.shift)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyState {
    Pressed,
    Released,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub super_: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    // Letters
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,

    // Numbers
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

    // Function keys
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

    // Navigation
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,

    // Editing
    Backspace,
    Delete,
    Insert,
    Enter,
    Tab,
    Space,

    // Modifiers (as keys)
    Shift,
    Control,
    Alt,
    Super,

    // Special
    Escape,
    Colon,
    Semicolon,
    Period,
    Comma,
    Slash,
    Backslash,
    Minus,
    Equals,
    BracketLeft,
    BracketRight,
    Quote,
    Grave,

    Unknown(u32),
}

/// Mapping of keys to their (unshifted, shifted) character representations
const KEY_CHARS: &[(Key, (&str, &str))] = &[
    // Letters - shifted is uppercase
    (Key::A, ("a", "A")),
    (Key::B, ("b", "B")),
    (Key::C, ("c", "C")),
    (Key::D, ("d", "D")),
    (Key::E, ("e", "E")),
    (Key::F, ("f", "F")),
    (Key::G, ("g", "G")),
    (Key::H, ("h", "H")),
    (Key::I, ("i", "I")),
    (Key::J, ("j", "J")),
    (Key::K, ("k", "K")),
    (Key::L, ("l", "L")),
    (Key::M, ("m", "M")),
    (Key::N, ("n", "N")),
    (Key::O, ("o", "O")),
    (Key::P, ("p", "P")),
    (Key::Q, ("q", "Q")),
    (Key::R, ("r", "R")),
    (Key::S, ("s", "S")),
    (Key::T, ("t", "T")),
    (Key::U, ("u", "U")),
    (Key::V, ("v", "V")),
    (Key::W, ("w", "W")),
    (Key::X, ("x", "X")),
    (Key::Y, ("y", "Y")),
    (Key::Z, ("z", "Z")),
    // Numbers - shifted is symbols
    (Key::Num0, ("0", ")")),
    (Key::Num1, ("1", "!")),
    (Key::Num2, ("2", "@")),
    (Key::Num3, ("3", "#")),
    (Key::Num4, ("4", "$")),
    (Key::Num5, ("5", "%")),
    (Key::Num6, ("6", "^")),
    (Key::Num7, ("7", "&")),
    (Key::Num8, ("8", "*")),
    (Key::Num9, ("9", "(")),
    // Punctuation
    (Key::Period, (".", ">")),
    (Key::Comma, (",", "<")),
    (Key::Semicolon, (";", ":")),
    (Key::Colon, (":", ":")),
    (Key::Slash, ("/", "?")),
    (Key::Backslash, ("\\", "|")),
    (Key::Minus, ("-", "_")),
    (Key::Equals, ("=", "+")),
    (Key::BracketLeft, ("[", "{")),
    (Key::BracketRight, ("]", "}")),
    (Key::Quote, ("'", "\"")),
    (Key::Grave, ("`", "~")),
    // Special keys
    (Key::Enter, ("\n", "\n")),
    (Key::Escape, ("\x1b", "\x1b")),
    (Key::Backspace, ("\x08", "\x08")),
    (Key::Tab, ("\t", "\t")),
    (Key::Space, (" ", " ")),
];

impl Key {
    /// Get the base lowercase character for this key (for Ctrl combinations)
    pub fn to_base_char(&self) -> Option<char> {
        KEY_CHARS
            .iter()
            .find(|(k, _)| k == self)
            .and_then(|(_, (base, _))| base.chars().next())
            .filter(|c| c.is_ascii_alphabetic())
    }

    /// Get the string representation of this key, considering shift state
    pub fn to_string_with_shift(&self, shifted: bool) -> Option<String> {
        KEY_CHARS
            .iter()
            .find(|(k, _)| k == self)
            .map(|(_, (base, shift))| if shifted { *shift } else { *base }.to_string())
    }

    pub fn from_keysym(keysym: u32) -> Self {
        use smithay_client_toolkit::seat::keyboard::Keysym;

        match keysym {
            x if x == Keysym::a.raw() => Key::A,
            x if x == Keysym::b.raw() => Key::B,
            x if x == Keysym::c.raw() => Key::C,
            x if x == Keysym::d.raw() => Key::D,
            x if x == Keysym::e.raw() => Key::E,
            x if x == Keysym::f.raw() => Key::F,
            x if x == Keysym::g.raw() => Key::G,
            x if x == Keysym::h.raw() => Key::H,
            x if x == Keysym::i.raw() => Key::I,
            x if x == Keysym::j.raw() => Key::J,
            x if x == Keysym::k.raw() => Key::K,
            x if x == Keysym::l.raw() => Key::L,
            x if x == Keysym::m.raw() => Key::M,
            x if x == Keysym::n.raw() => Key::N,
            x if x == Keysym::o.raw() => Key::O,
            x if x == Keysym::p.raw() => Key::P,
            x if x == Keysym::q.raw() => Key::Q,
            x if x == Keysym::r.raw() => Key::R,
            x if x == Keysym::s.raw() => Key::S,
            x if x == Keysym::t.raw() => Key::T,
            x if x == Keysym::u.raw() => Key::U,
            x if x == Keysym::v.raw() => Key::V,
            x if x == Keysym::w.raw() => Key::W,
            x if x == Keysym::x.raw() => Key::X,
            x if x == Keysym::y.raw() => Key::Y,
            x if x == Keysym::z.raw() => Key::Z,

            x if x == Keysym::_0.raw() => Key::Num0,
            x if x == Keysym::_1.raw() => Key::Num1,
            x if x == Keysym::_2.raw() => Key::Num2,
            x if x == Keysym::_3.raw() => Key::Num3,
            x if x == Keysym::_4.raw() => Key::Num4,
            x if x == Keysym::_5.raw() => Key::Num5,
            x if x == Keysym::_6.raw() => Key::Num6,
            x if x == Keysym::_7.raw() => Key::Num7,
            x if x == Keysym::_8.raw() => Key::Num8,
            x if x == Keysym::_9.raw() => Key::Num9,

            x if x == Keysym::Up.raw() => Key::Up,
            x if x == Keysym::Down.raw() => Key::Down,
            x if x == Keysym::Left.raw() => Key::Left,
            x if x == Keysym::Right.raw() => Key::Right,

            x if x == Keysym::Return.raw() => Key::Enter,
            x if x == Keysym::Escape.raw() => Key::Escape,
            x if x == Keysym::BackSpace.raw() => Key::Backspace,
            x if x == Keysym::Tab.raw() => Key::Tab,
            x if x == Keysym::space.raw() => Key::Space,

            x if x == Keysym::colon.raw() => Key::Colon,
            x if x == Keysym::semicolon.raw() => Key::Semicolon,
            x if x == Keysym::period.raw() => Key::Period,
            x if x == Keysym::comma.raw() => Key::Comma,

            _ => Key::Unknown(keysym),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PointerEvent {
    pub kind: PointerEventKind,
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerEventKind {
    Enter,
    Leave,
    Motion,
    Press(PointerButton),
    Release(PointerButton),
    Scroll { dx: i32, dy: i32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerButton {
    Left,
    Right,
    Middle,
    Other(u32),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_key_event(key: Key, text: Option<&str>, modifiers: Modifiers) -> KeyEvent {
        KeyEvent {
            key,
            text: text.map(|s| s.to_string()),
            modifiers,
            state: KeyState::Pressed,
        }
    }

    #[test]
    fn test_key_event_simple_letters() {
        let event = make_key_event(Key::A, None, Modifiers::default());
        assert_eq!(event.to_key_string(), Some("a".to_string()));

        let event = make_key_event(Key::Z, None, Modifiers::default());
        assert_eq!(event.to_key_string(), Some("z".to_string()));
    }

    #[test]
    fn test_key_event_shifted_letters() {
        let mods = Modifiers {
            shift: true,
            ..Default::default()
        };
        let event = make_key_event(Key::A, None, mods);
        assert_eq!(event.to_key_string(), Some("A".to_string()));
    }

    #[test]
    fn test_key_event_ctrl_combinations() {
        let mods = Modifiers {
            ctrl: true,
            ..Default::default()
        };

        let event = make_key_event(Key::W, None, mods);
        assert_eq!(event.to_key_string(), Some("C-w".to_string()));

        let event = make_key_event(Key::C, None, mods);
        assert_eq!(event.to_key_string(), Some("C-c".to_string()));
    }

    #[test]
    fn test_key_event_text_takes_precedence() {
        // If text is provided, it should be used directly
        let event = make_key_event(
            Key::Semicolon,
            Some(":"),
            Modifiers {
                shift: true,
                ..Default::default()
            },
        );
        assert_eq!(event.to_key_string(), Some(":".to_string()));
    }

    #[test]
    fn test_key_event_special_keys() {
        let event = make_key_event(Key::Enter, None, Modifiers::default());
        assert_eq!(event.to_key_string(), Some("\n".to_string()));

        let event = make_key_event(Key::Escape, None, Modifiers::default());
        assert_eq!(event.to_key_string(), Some("\x1b".to_string()));

        let event = make_key_event(Key::Tab, None, Modifiers::default());
        assert_eq!(event.to_key_string(), Some("\t".to_string()));

        let event = make_key_event(Key::Space, None, Modifiers::default());
        assert_eq!(event.to_key_string(), Some(" ".to_string()));
    }

    #[test]
    fn test_key_event_punctuation() {
        let event = make_key_event(Key::Period, None, Modifiers::default());
        assert_eq!(event.to_key_string(), Some(".".to_string()));

        let mods = Modifiers {
            shift: true,
            ..Default::default()
        };
        let event = make_key_event(Key::Period, None, mods);
        assert_eq!(event.to_key_string(), Some(">".to_string()));
    }

    #[test]
    fn test_key_event_numbers() {
        let event = make_key_event(Key::Num0, None, Modifiers::default());
        assert_eq!(event.to_key_string(), Some("0".to_string()));

        let mods = Modifiers {
            shift: true,
            ..Default::default()
        };
        let event = make_key_event(Key::Num1, None, mods);
        assert_eq!(event.to_key_string(), Some("!".to_string()));
    }

    #[test]
    fn test_key_event_modifier_keys_return_none() {
        let event = make_key_event(Key::Shift, None, Modifiers::default());
        assert_eq!(event.to_key_string(), None);

        let event = make_key_event(Key::Control, None, Modifiers::default());
        assert_eq!(event.to_key_string(), None);
    }

    #[test]
    fn test_key_event_unknown_returns_none() {
        let event = make_key_event(Key::Unknown(12345), None, Modifiers::default());
        assert_eq!(event.to_key_string(), None);
    }

    #[test]
    fn test_modifiers_default() {
        let mods = Modifiers::default();
        assert!(!mods.shift);
        assert!(!mods.ctrl);
        assert!(!mods.alt);
        assert!(!mods.super_);
    }

    #[test]
    fn test_key_state_equality() {
        assert_eq!(KeyState::Pressed, KeyState::Pressed);
        assert_eq!(KeyState::Released, KeyState::Released);
        assert_ne!(KeyState::Pressed, KeyState::Released);
    }

    #[test]
    fn test_pointer_button_equality() {
        assert_eq!(PointerButton::Left, PointerButton::Left);
        assert_eq!(PointerButton::Other(5), PointerButton::Other(5));
        assert_ne!(PointerButton::Left, PointerButton::Right);
    }

    #[test]
    fn test_pointer_event_kind_scroll() {
        let scroll = PointerEventKind::Scroll { dx: 10, dy: -5 };
        match scroll {
            PointerEventKind::Scroll { dx, dy } => {
                assert_eq!(dx, 10);
                assert_eq!(dy, -5);
            }
            _ => panic!("expected Scroll"),
        }
    }
}
