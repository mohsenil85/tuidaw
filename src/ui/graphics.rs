use super::Style;

/// Rectangle representing a region on screen
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Rect {
    pub const fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self { x, y, width, height }
    }

    /// Create a rect centered within a given area
    pub fn centered(area_width: u16, area_height: u16, width: u16, height: u16) -> Self {
        let x = area_width.saturating_sub(width) / 2;
        let y = area_height.saturating_sub(height) / 2;
        Self { x, y, width, height }
    }

    pub fn right(&self) -> u16 {
        self.x.saturating_add(self.width)
    }

    pub fn bottom(&self) -> u16 {
        self.y.saturating_add(self.height)
    }
}

/// Graphics abstraction for drawing to the screen
pub trait Graphics {
    /// Put a single character at the given position
    fn put_char(&mut self, x: u16, y: u16, ch: char);

    /// Put a string starting at the given position
    fn put_str(&mut self, x: u16, y: u16, s: &str);

    /// Set the current style for subsequent drawing operations
    fn set_style(&mut self, style: Style);

    /// Draw a box with optional title
    fn draw_box(&mut self, rect: Rect, title: Option<&str>);

    /// Fill a rectangle with a character
    fn fill_rect(&mut self, rect: Rect, ch: char);

    /// Get the current terminal size (width, height)
    fn size(&self) -> (u16, u16);
}
