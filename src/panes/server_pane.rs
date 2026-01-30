use std::any::Any;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect as RatatuiRect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::audio::ServerStatus;
use crate::state::AppState;
use crate::ui::layout_helpers::center_rect;
use crate::ui::{Action, Color, InputEvent, Keymap, Pane, ServerAction, Style};

pub struct ServerPane {
    keymap: Keymap,
    status: ServerStatus,
    message: String,
    server_running: bool,
}

impl ServerPane {
    pub fn new(keymap: Keymap) -> Self {
        Self {
            keymap,
            status: ServerStatus::Stopped,
            message: String::new(),
            server_running: false,
        }
    }

    pub fn set_status(&mut self, status: ServerStatus, message: &str) {
        self.status = status;
        self.message = message.to_string();
    }

    pub fn set_server_running(&mut self, running: bool) {
        self.server_running = running;
    }
}

impl Default for ServerPane {
    fn default() -> Self {
        Self::new(Keymap::new())
    }
}

impl Pane for ServerPane {
    fn id(&self) -> &'static str {
        "server"
    }

    fn handle_input(&mut self, event: InputEvent, _state: &AppState) -> Action {
        match self.keymap.lookup(&event) {
            Some("start") => Action::Server(ServerAction::Start),
            Some("stop") => Action::Server(ServerAction::Stop),
            Some("connect") => Action::Server(ServerAction::Connect),
            Some("disconnect") => Action::Server(ServerAction::Disconnect),
            Some("compile") => Action::Server(ServerAction::CompileSynthDefs),
            Some("load") => Action::Server(ServerAction::LoadSynthDefs),
            _ => Action::None,
        }
    }

    fn render(&self, area: RatatuiRect, buf: &mut Buffer, _state: &AppState) {
        let rect = center_rect(area, 60, 15);

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Audio Server (scsynth) ")
            .border_style(ratatui::style::Style::from(Style::new().fg(Color::GOLD)))
            .title_style(ratatui::style::Style::from(Style::new().fg(Color::GOLD)));
        let inner = block.inner(rect);
        block.render(rect, buf);

        let x = inner.x + 1;
        let label_style = ratatui::style::Style::from(Style::new().fg(Color::CYAN));

        // Server process status
        let (server_text, server_color) = if self.server_running {
            ("Running", Color::METER_LOW)
        } else {
            ("Stopped", Color::MUTE_COLOR)
        };
        let server_line = Line::from(vec![
            Span::styled("Server:     ", label_style),
            Span::styled(server_text, ratatui::style::Style::from(Style::new().fg(server_color).bold())),
        ]);
        Paragraph::new(server_line).render(
            RatatuiRect::new(x, inner.y + 1, inner.width.saturating_sub(2), 1), buf,
        );

        // Connection status
        let (status_text, status_color) = match self.status {
            ServerStatus::Stopped => ("Not connected", Color::DARK_GRAY),
            ServerStatus::Starting => ("Starting...", Color::ORANGE),
            ServerStatus::Running => ("Ready (not connected)", Color::SOLO_COLOR),
            ServerStatus::Connected => ("Connected", Color::METER_LOW),
            ServerStatus::Error => ("Error", Color::MUTE_COLOR),
        };
        let conn_line = Line::from(vec![
            Span::styled("Connection: ", label_style),
            Span::styled(status_text, ratatui::style::Style::from(Style::new().fg(status_color).bold())),
        ]);
        Paragraph::new(conn_line).render(
            RatatuiRect::new(x, inner.y + 2, inner.width.saturating_sub(2), 1), buf,
        );

        // Message
        if !self.message.is_empty() {
            let max_len = inner.width.saturating_sub(2) as usize;
            let msg: String = self.message.chars().take(max_len).collect();
            let msg_line = Line::from(Span::styled(
                msg,
                ratatui::style::Style::from(Style::new().fg(Color::SKY_BLUE)),
            ));
            Paragraph::new(msg_line).render(
                RatatuiRect::new(x, inner.y + 4, inner.width.saturating_sub(2), 1), buf,
            );
        }

        // Help text at bottom
        let help_style = ratatui::style::Style::from(Style::new().fg(Color::DARK_GRAY));
        let help_lines = [
            "s: start server  k: kill server",
            "c: connect       d: disconnect",
            "b: build synths  l: load synths",
            "F1: help  F2: instruments  F5: mixer",
        ];
        for (i, line_text) in help_lines.iter().enumerate() {
            let y = rect.y + rect.height - 5 + i as u16;
            if y >= inner.y && y < rect.y + rect.height - 1 {
                Paragraph::new(Line::from(Span::styled(*line_text, help_style)))
                    .render(RatatuiRect::new(x, y, inner.width.saturating_sub(2), 1), buf);
            }
        }
    }

    fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
