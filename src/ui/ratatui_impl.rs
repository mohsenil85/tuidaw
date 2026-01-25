use std::io::{self, Stdout};
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyEvent, KeyCode as CrosstermKeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    buffer::Buffer,
    layout::Rect as RatatuiRect,
    style::{Color as RatatuiColor, Modifier, Style as RatatuiStyle},
    widgets::{Block, Borders, Widget},
    Terminal,
};

use super::{Graphics, InputEvent, InputSource, KeyCode, Modifiers, Rect, Style};

/// Ratatui-based terminal backend
pub struct RatatuiBackend {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl RatatuiBackend {
    /// Create a new ratatui backend (does not start terminal mode)
    pub fn new() -> io::Result<Self> {
        let backend = CrosstermBackend::new(io::stdout());
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }

    /// Enter raw mode and alternate screen
    pub fn start(&mut self) -> io::Result<()> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
        self.terminal.clear()?;
        Ok(())
    }

    /// Leave raw mode and alternate screen
    pub fn stop(&mut self) -> io::Result<()> {
        disable_raw_mode()?;
        execute!(io::stdout(), LeaveAlternateScreen)?;
        Ok(())
    }

    /// Begin a new frame for drawing
    pub fn begin_frame(&self) -> io::Result<RatatuiFrame> {
        let size = self.terminal.size()?;
        Ok(RatatuiFrame {
            buffer: Buffer::empty(size),
            size: (size.width, size.height),
            current_style: Style::default(),
        })
    }

    /// End the current frame and render to screen
    pub fn end_frame(&mut self, frame: RatatuiFrame) -> io::Result<()> {
        self.terminal.draw(|f| {
            let area = f.size();
            f.render_widget(BufferWidget(frame.buffer), area);
        })?;
        Ok(())
    }
}

/// A frame for drawing operations
pub struct RatatuiFrame {
    buffer: Buffer,
    size: (u16, u16),
    current_style: Style,
}

impl RatatuiFrame {
    fn convert_style(&self, style: &Style) -> RatatuiStyle {
        let mut rs = RatatuiStyle::default();
        if let Some(fg) = style.fg {
            rs = rs.fg(RatatuiColor::Rgb(fg.r, fg.g, fg.b));
        }
        if let Some(bg) = style.bg {
            rs = rs.bg(RatatuiColor::Rgb(bg.r, bg.g, bg.b));
        }
        if style.bold {
            rs = rs.add_modifier(Modifier::BOLD);
        }
        if style.underline {
            rs = rs.add_modifier(Modifier::UNDERLINED);
        }
        rs
    }
}

impl Graphics for RatatuiFrame {
    fn put_char(&mut self, x: u16, y: u16, ch: char) {
        if x < self.size.0 && y < self.size.1 {
            let style = self.convert_style(&self.current_style);
            self.buffer.get_mut(x, y).set_char(ch).set_style(style);
        }
    }

    fn put_str(&mut self, x: u16, y: u16, s: &str) {
        let style = self.convert_style(&self.current_style);
        let mut current_x = x;
        for ch in s.chars() {
            if current_x >= self.size.0 {
                break;
            }
            if y < self.size.1 {
                self.buffer.get_mut(current_x, y).set_char(ch).set_style(style);
            }
            current_x += 1;
        }
    }

    fn set_style(&mut self, style: Style) {
        self.current_style = style;
    }

    fn draw_box(&mut self, rect: Rect, title: Option<&str>) {
        let ratatui_rect = RatatuiRect::new(rect.x, rect.y, rect.width, rect.height);
        let mut block = Block::default().borders(Borders::ALL);
        if let Some(t) = title {
            block = block.title(t);
        }
        let style = self.convert_style(&self.current_style);
        block = block.border_style(style).title_style(style);
        block.render(ratatui_rect, &mut self.buffer);
    }

    fn fill_rect(&mut self, rect: Rect, ch: char) {
        let style = self.convert_style(&self.current_style);
        for y in rect.y..rect.bottom().min(self.size.1) {
            for x in rect.x..rect.right().min(self.size.0) {
                self.buffer.get_mut(x, y).set_char(ch).set_style(style);
            }
        }
    }

    fn size(&self) -> (u16, u16) {
        self.size
    }
}

impl InputSource for RatatuiBackend {
    fn poll_event(&mut self, timeout: Duration) -> Option<InputEvent> {
        if event::poll(timeout).ok()? {
            if let Event::Key(key_event) = event::read().ok()? {
                return Some(convert_key_event(key_event));
            }
        }
        None
    }
}

fn convert_key_event(event: KeyEvent) -> InputEvent {
    let key = match event.code {
        CrosstermKeyCode::Char(c) => KeyCode::Char(c),
        CrosstermKeyCode::Enter => KeyCode::Enter,
        CrosstermKeyCode::Esc => KeyCode::Escape,
        CrosstermKeyCode::Backspace => KeyCode::Backspace,
        CrosstermKeyCode::Tab => KeyCode::Tab,
        CrosstermKeyCode::Up => KeyCode::Up,
        CrosstermKeyCode::Down => KeyCode::Down,
        CrosstermKeyCode::Left => KeyCode::Left,
        CrosstermKeyCode::Right => KeyCode::Right,
        CrosstermKeyCode::Home => KeyCode::Home,
        CrosstermKeyCode::End => KeyCode::End,
        CrosstermKeyCode::PageUp => KeyCode::PageUp,
        CrosstermKeyCode::PageDown => KeyCode::PageDown,
        CrosstermKeyCode::Insert => KeyCode::Insert,
        CrosstermKeyCode::Delete => KeyCode::Delete,
        CrosstermKeyCode::F(n) => KeyCode::F(n),
        _ => KeyCode::Char('\0'),
    };

    let modifiers = Modifiers {
        ctrl: event.modifiers.contains(KeyModifiers::CONTROL),
        alt: event.modifiers.contains(KeyModifiers::ALT),
        shift: event.modifiers.contains(KeyModifiers::SHIFT),
    };

    InputEvent::new(key, modifiers)
}

/// Widget that renders a pre-built buffer
struct BufferWidget(Buffer);

impl Widget for BufferWidget {
    fn render(self, area: RatatuiRect, buf: &mut Buffer) {
        for y in area.y..area.y.saturating_add(area.height) {
            for x in area.x..area.x.saturating_add(area.width) {
                if x < self.0.area.width && y < self.0.area.height {
                    *buf.get_mut(x, y) = self.0.get(x, y).clone();
                }
            }
        }
    }
}
