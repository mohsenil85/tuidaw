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
    style::{Color as RatatuiColor, Style as RatatuiStyle},
    widgets::Widget,
    Terminal,
};

use super::{InputEvent, InputSource, KeyCode, Modifiers};

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

    /// Begin a new frame for drawing (black background)
    pub fn begin_frame(&self) -> io::Result<RatatuiFrame> {
        let size = self.terminal.size()?;
        let area = RatatuiRect::new(0, 0, size.width, size.height);
        let mut buffer = Buffer::empty(area);
        // Fill entire buffer with black background
        let bg_style = RatatuiStyle::default().bg(RatatuiColor::Rgb(0, 0, 0));
        for y in 0..area.height {
            for x in 0..area.width {
                if let Some(cell) = buffer.cell_mut((x, y)) {
                    cell.set_style(bg_style);
                }
            }
        }
        Ok(RatatuiFrame {
            buffer,
            size: (area.width, area.height),
        })
    }

    /// End the current frame and render to screen
    pub fn end_frame(&mut self, frame: RatatuiFrame) -> io::Result<()> {
        self.terminal.draw(|f| {
            let area = f.area();
            f.render_widget(BufferWidget(frame.buffer), area);
        })?;
        Ok(())
    }
}

/// A frame for drawing operations
pub struct RatatuiFrame {
    buffer: Buffer,
    size: (u16, u16),
}

impl RatatuiFrame {
    /// Get mutable access to the underlying buffer
    pub fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffer
    }

    /// Get the full terminal area as a ratatui Rect
    pub fn area(&self) -> RatatuiRect {
        RatatuiRect::new(0, 0, self.size.0, self.size.1)
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
                    if let (Some(src), Some(dst)) = (self.0.cell((x, y)), buf.cell_mut((x, y))) {
                        *dst = src.clone();
                    }
                }
            }
        }
    }
}
