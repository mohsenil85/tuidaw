/// RGB Color representation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    // ANSI color constants
    pub const BLACK: Color = Color::new(0, 0, 0);
    pub const WHITE: Color = Color::new(255, 255, 255);
    pub const RED: Color = Color::new(255, 0, 0);
    pub const GREEN: Color = Color::new(0, 255, 0);
    pub const BLUE: Color = Color::new(0, 0, 255);
    pub const YELLOW: Color = Color::new(255, 255, 0);
    pub const CYAN: Color = Color::new(0, 255, 255);
    pub const MAGENTA: Color = Color::new(255, 0, 255);
    pub const GRAY: Color = Color::new(128, 128, 128);
    pub const DARK_GRAY: Color = Color::new(64, 64, 64);
}

/// Text style with foreground, background, and attributes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Style {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub bold: bool,
    pub underline: bool,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            fg: None,
            bg: None,
            bold: false,
            underline: false,
        }
    }
}

impl Style {
    pub const fn new() -> Self {
        Self {
            fg: None,
            bg: None,
            bold: false,
            underline: false,
        }
    }

    pub const fn fg(mut self, color: Color) -> Self {
        self.fg = Some(color);
        self
    }

    pub const fn bg(mut self, color: Color) -> Self {
        self.bg = Some(color);
        self
    }

    pub const fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    pub const fn underline(mut self) -> Self {
        self.underline = true;
        self
    }
}

/// Semantic color categories for theming
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticColor {
    Text,
    TextMuted,
    Selected,
    SelectedText,
    Border,
    BorderFocused,
    Background,
    Error,
    Warning,
    Success,
}

impl SemanticColor {
    /// Get the default color for this semantic category
    pub fn default_color(&self) -> Color {
        match self {
            SemanticColor::Text => Color::WHITE,
            SemanticColor::TextMuted => Color::GRAY,
            SemanticColor::Selected => Color::BLUE,
            SemanticColor::SelectedText => Color::WHITE,
            SemanticColor::Border => Color::GRAY,
            SemanticColor::BorderFocused => Color::WHITE,
            SemanticColor::Background => Color::BLACK,
            SemanticColor::Error => Color::RED,
            SemanticColor::Warning => Color::YELLOW,
            SemanticColor::Success => Color::GREEN,
        }
    }
}
