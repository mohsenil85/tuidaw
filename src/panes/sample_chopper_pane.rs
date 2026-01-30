use std::any::Any;

use crate::panes::FileBrowserPane;
use crate::state::AppState;
use crate::ui::{
    Action, ChopperAction, Color, FileSelectAction, Graphics, InputEvent, Keymap, NavAction, Pane, Rect, Style,
};

pub struct SampleChopperPane {
    keymap: Keymap,
    cursor_pos: f32, // 0.0-1.0
    auto_slice_n: usize,
    file_browser: FileBrowserPane,
}

impl SampleChopperPane {
    pub fn new(keymap: Keymap, file_browser_keymap: Keymap) -> Self {
        Self {
            keymap,
            cursor_pos: 0.5,
            auto_slice_n: 4,
            file_browser: FileBrowserPane::new(file_browser_keymap),
        }
    }

    fn selected_drum_sequencer<'a>(&self, state: &'a AppState) -> Option<&'a crate::state::drum_sequencer::DrumSequencerState> {
        state.instruments.selected_instrument()
            .and_then(|i| i.drum_sequencer.as_ref())
    }

    fn get_chopper_state<'a>(&self, state: &'a AppState) -> Option<&'a crate::state::drum_sequencer::ChopperState> {
        self.selected_drum_sequencer(state)
            .and_then(|d| d.chopper.as_ref())
    }

    fn should_show_file_browser(&self, state: &AppState) -> bool {
        self.selected_drum_sequencer(state)
            .map(|d| d.chopper.is_none())
            .unwrap_or(false)
    }
}

impl Default for SampleChopperPane {
    fn default() -> Self {
        Self::new(Keymap::new(), Keymap::new())
    }
}

impl Pane for SampleChopperPane {
    fn id(&self) -> &'static str {
        "sample_chopper"
    }

    fn handle_input(&mut self, event: InputEvent, state: &AppState) -> Action {
        if self.should_show_file_browser(state) {
            return self.file_browser.handle_input(event, state);
        }

        match self.keymap.lookup(&event) {
            Some("move_left") => {
                self.cursor_pos = (self.cursor_pos - 0.01).max(0.0);
                Action::Chopper(ChopperAction::MoveCursor(-1)) // Also update state if needed, but we track locally too
            }
            Some("move_right") => {
                self.cursor_pos = (self.cursor_pos + 0.01).min(1.0);
                Action::Chopper(ChopperAction::MoveCursor(1))
            }
            Some("next_slice") => Action::Chopper(ChopperAction::SelectSlice(1)),
            Some("prev_slice") => Action::Chopper(ChopperAction::SelectSlice(-1)),
            Some("nudge_start") => Action::Chopper(ChopperAction::NudgeSliceStart(-0.005)),
            Some("nudge_end") => Action::Chopper(ChopperAction::NudgeSliceEnd(0.005)),
            Some("chop") => {
                Action::Chopper(ChopperAction::AddSlice(self.cursor_pos))
            }
            Some("delete") => Action::Chopper(ChopperAction::RemoveSlice),
            Some("auto_slice") => {
                let n = self.auto_slice_n;
                self.auto_slice_n = match n {
                    4 => 8,
                    8 => 12,
                    12 => 16,
                    _ => 4,
                };
                Action::Chopper(ChopperAction::AutoSlice(n))
            }
            Some("commit") => Action::Chopper(ChopperAction::CommitAll),
            Some("load") => Action::Chopper(ChopperAction::LoadSample),
            Some("preview") => Action::Chopper(ChopperAction::PreviewSlice),
            Some("back") => Action::Nav(NavAction::PopPane),
            Some(action) if action.starts_with("assign_") => {
                if let Ok(idx) = action[7..].parse::<usize>() {
                    Action::Chopper(ChopperAction::AssignToPad(idx - 1))
                } else {
                    Action::None
                }
            }
            _ => Action::None,
        }
    }

    fn render(&self, g: &mut dyn Graphics, state: &AppState) {
        let (width, height) = g.size();
        let box_width = 97;
        let box_height = 29;
        let rect = Rect::centered(width, height, box_width, box_height);

        if let Some(drum_seq) = self.selected_drum_sequencer(state) {
            if drum_seq.chopper.is_none() {
                self.file_browser.render(g, state);
                return;
            }
        } else {
            g.draw_box(rect, Some(" Sample Chopper "));
            g.set_style(Style::new().fg(Color::DARK_GRAY));
            g.put_str(
                rect.x + 2,
                rect.y + 2,
                "No drum machine instrument selected. Press 1 to add one.",
            );
            return;
        }

        g.draw_box(rect, Some(" Sample Chopper "));

        // Get chopper state
        let chopper = match self.get_chopper_state(state) {
            Some(c) => c,
            None => {
                g.set_style(Style::new().fg(Color::DARK_GRAY));
                g.put_str(rect.x + 2, rect.y + 2, "No sample loaded.");
                return;
            }
        };

        let content_x = rect.x + 2;
        let content_y = rect.y + 2;

        // Header info
        let filename = chopper.path.as_ref()
            .map(|p| std::path::Path::new(p).file_name().unwrap_or_default().to_string_lossy())
            .unwrap_or("No Sample".into());
        g.set_style(Style::new().fg(Color::CYAN).bold());
        g.put_str(content_x, content_y, &filename);
        
        g.set_style(Style::new().fg(Color::DARK_GRAY));
        let info = format!("{:.1}s   {} slices", chopper.duration_secs, chopper.slices.len());
        g.put_str(rect.x + rect.width - 2 - info.len() as u16, content_y, &info);

        // Waveform
        let wave_y = content_y + 2;
        let wave_height = 8;
        let wave_width = (rect.width - 4) as usize;

        // Draw waveform peaks
        // If we have peaks, map them to width
        if !chopper.waveform_peaks.is_empty() {
            g.set_style(Style::new().fg(Color::GREEN));
            // Simple resampling to fit wave_width
            let peaks = &chopper.waveform_peaks;
            for i in 0..wave_width {
                // map i to index in peaks
                let peak_idx = (i as f32 / wave_width as f32 * peaks.len() as f32) as usize;
                if let Some(&val) = peaks.get(peak_idx) {
                    // Draw vertical bar centered
                    let bar_h = (val * wave_height as f32) as u16;
                    let center_y = wave_y + wave_height / 2;
                    let top = center_y.saturating_sub(bar_h / 2);
                    let bottom = center_y.saturating_add(bar_h / 2);
                    for y in top..=bottom {
                        g.put_char(content_x + i as u16, y, '│');
                    }
                }
            }
        } else {
            g.put_str(content_x, wave_y + wave_height/2, "(No waveform data)");
        }

        // Draw slices
        let slice_y_start = wave_y;
        let slice_y_end = wave_y + wave_height;
        
        for (i, slice) in chopper.slices.iter().enumerate() {
            let start_x = (slice.start * wave_width as f32) as u16;
            let end_x = (slice.end * wave_width as f32) as u16;
            let center_x = (start_x + end_x) / 2;
            
            // Draw slice boundaries
            g.set_style(Style::new().fg(Color::DARK_GRAY));
            if i > 0 { // Don't draw over left edge
                for y in slice_y_start..=slice_y_end {
                    g.put_char(content_x + start_x, y, '|');
                }
            }

            // Highlight selected slice
            if i == chopper.selected_slice {
                g.set_style(Style::new().bg(Color::SELECTION_BG));
                // Fill background of selected slice
                for x in start_x..end_x {
                    // avoid drawing over boundary
                    if x >= wave_width as u16 { break; }
                    g.put_char(content_x + x, slice_y_end + 1, ' ');
                }
                g.set_style(Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG));
                let label = format!("{}", i + 1);
                g.put_str(content_x + center_x.saturating_sub(label.len() as u16 / 2), slice_y_end + 1, &label);
            } else {
                g.set_style(Style::new().fg(Color::DARK_GRAY));
                let label = format!("{}", i + 1);
                if end_x - start_x > label.len() as u16 {
                     g.put_str(content_x + center_x.saturating_sub(label.len() as u16 / 2), slice_y_end + 1, &label);
                }
            }
        }

        // Draw cursor
        let cursor_screen_x = (self.cursor_pos * wave_width as f32) as u16;
        g.set_style(Style::new().fg(Color::YELLOW));
        for y in slice_y_start..=slice_y_end {
            g.put_char(content_x + cursor_screen_x, y, '┆');
        }
        g.put_char(content_x + cursor_screen_x, slice_y_end + 2, '▲');


        // List slices
        let list_y = slice_y_end + 4;
        for i in 0..8 { // Show first 8 slices or scroll? Fixed 8 for now as per mockup
            if i >= chopper.slices.len() { break; }
            let slice = &chopper.slices[i];
            let y = list_y + i as u16;
            
            if i == chopper.selected_slice {
                g.set_style(Style::new().fg(Color::WHITE).bold());
                g.put_str(content_x, y, ">");
            } else {
                g.set_style(Style::new().fg(Color::DARK_GRAY));
                g.put_str(content_x, y, " ");
            }
            
            g.set_style(Style::new().fg(if i == chopper.selected_slice { Color::WHITE } else { Color::GRAY }));
            g.put_str(content_x + 2, y, &format!("{:<2} {:.3}-{:.3}", i + 1, slice.start, slice.end));
            
            // Check if assigned to any pad
            // This requires access to DrumSequencerState which we have via AppState
            // But we need to look up which pads use this buffer + slice range? 
            // Or just trust the user assigned it.
            // Let's check pads
            if let Some(inst) = state.instruments.selected_instrument() {
                if let Some(ds) = &inst.drum_sequencer {
                    for (pad_idx, pad) in ds.pads.iter().enumerate() {
                        if pad.buffer_id == chopper.buffer_id && 
                           (pad.slice_start - slice.start).abs() < 0.001 &&
                           (pad.slice_end - slice.end).abs() < 0.001 {
                               g.put_str(content_x + 25, y, &format!("→ Pad {}", pad_idx + 1));
                           }
                    }
                }
            }
        }

        // Footer help
        let help_y = rect.y + rect.height - 2;
        g.set_style(Style::new().fg(Color::DARK_GRAY));
        g.put_str(content_x, help_y, "Enter:chop ,:commit x:del n:auto 1-0:assign Space:preview s:load Esc:back");
    }

    fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn on_enter(&mut self, state: &AppState) {
        if self.should_show_file_browser(state) {
            self.file_browser.open_for(FileSelectAction::LoadChopperSample, None);
        }
    }
}
