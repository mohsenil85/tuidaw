use std::time::Duration;

/// Key codes for keyboard input
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCode {
    Char(char),
    Enter,
    Escape,
    Backspace,
    Tab,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    Delete,
    F(u8),
}

/// Modifier key state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

impl Modifiers {
    #[allow(dead_code)]
    pub const fn none() -> Self {
        Self {
            ctrl: false,
            alt: false,
            shift: false,
        }
    }

    #[allow(dead_code)]
    pub const fn ctrl() -> Self {
        Self {
            ctrl: true,
            alt: false,
            shift: false,
        }
    }
}

/// Input event from the user
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputEvent {
    pub key: KeyCode,
    pub modifiers: Modifiers,
}

impl InputEvent {
    pub const fn new(key: KeyCode, modifiers: Modifiers) -> Self {
        Self { key, modifiers }
    }

    #[allow(dead_code)]
    pub const fn key(key: KeyCode) -> Self {
        Self {
            key,
            modifiers: Modifiers::none(),
        }
    }

    /// Check if this is a specific character without modifiers
    #[allow(dead_code)]
    pub fn is_char(&self, ch: char) -> bool {
        matches!(self.key, KeyCode::Char(c) if c == ch)
            && !self.modifiers.ctrl
            && !self.modifiers.alt
    }
}

/// Trait for reading input events
pub trait InputSource {
    /// Poll for an input event with a timeout
    /// Returns None if no event is available within the timeout
    fn poll_event(&mut self, timeout: Duration) -> Option<InputEvent>;
}
