use std::any::Any;

use crate::state::{Module, ModuleId, ModuleType, Param, ParamValue, RackState};
use crate::ui::{Action, Color, Graphics, InputEvent, KeyCode, Keymap, Pane, Rect, Style};

pub struct RackPane {
    keymap: Keymap,
    rack: RackState,
}

impl RackPane {
    pub fn new() -> Self {
        let mut rack = RackState::new();

        // Add some default modules to demonstrate functionality
        rack.add_module(ModuleType::SawOsc);
        rack.add_module(ModuleType::Lpf);
        rack.add_module(ModuleType::Output);

        Self {
            keymap: Keymap::new()
                .bind('q', "quit", "Quit the application")
                .bind('n', "next", "Next module")
                .bind('p', "prev", "Previous module")
                .bind('j', "next", "Next module (vim)")
                .bind('k', "prev", "Previous module (vim)")
                .bind_key(KeyCode::Down, "next", "Next module")
                .bind_key(KeyCode::Up, "prev", "Previous module")
                .bind('g', "goto_top", "Go to top")
                .bind('G', "goto_bottom", "Go to bottom")
                .bind('a', "add", "Add module")
                .bind('d', "delete", "Delete module")
                .bind('e', "edit", "Edit module")
                .bind('w', "save", "Save rack")
                .bind('o', "load", "Load rack"),
            rack,
        }
    }

    fn format_params(&self, module: &Module) -> String {
        let mut parts = Vec::new();

        // Show up to 2 key parameters
        for (i, param) in module.params.iter().take(2).enumerate() {
            if i >= 2 {
                break;
            }

            let value_str = match &param.value {
                ParamValue::Float(f) => format!("{:.1}", f),
                ParamValue::Int(i) => format!("{}", i),
                ParamValue::Bool(b) => format!("{}", b),
            };

            parts.push(format!("{}: {}", param.name, value_str));
        }

        parts.join("  ")
    }

    /// Get module data for editing (returns id, name, type_name, params)
    pub fn get_module_for_edit(&self, id: ModuleId) -> Option<(ModuleId, String, &'static str, Vec<Param>)> {
        self.rack.modules.get(&id).map(|m| {
            (m.id, m.name.clone(), m.module_type.name(), m.params.clone())
        })
    }

    /// Update a module's params
    pub fn update_module_params(&mut self, id: ModuleId, params: Vec<Param>) {
        if let Some(module) = self.rack.modules.get_mut(&id) {
            module.params = params;
        }
    }

    /// Get reference to rack state for saving
    pub fn rack(&self) -> &RackState {
        &self.rack
    }

    /// Replace rack state (for loading)
    pub fn set_rack(&mut self, rack: RackState) {
        self.rack = rack;
        // Select first module if any exist
        if !self.rack.order.is_empty() {
            self.rack.selected = Some(0);
        }
    }
}

impl Default for RackPane {
    fn default() -> Self {
        Self::new()
    }
}

impl Pane for RackPane {
    fn id(&self) -> &'static str {
        "rack"
    }

    fn handle_input(&mut self, event: InputEvent) -> Action {
        match self.keymap.lookup(&event) {
            Some("quit") => Action::Quit,
            Some("next") => {
                self.rack.select_next();
                Action::None
            }
            Some("prev") => {
                self.rack.select_prev();
                Action::None
            }
            Some("goto_top") => {
                if !self.rack.order.is_empty() {
                    self.rack.selected = Some(0);
                }
                Action::None
            }
            Some("goto_bottom") => {
                if !self.rack.order.is_empty() {
                    self.rack.selected = Some(self.rack.order.len() - 1);
                }
                Action::None
            }
            Some("add") => Action::SwitchPane("add"),
            Some("delete") => {
                if let Some(module) = self.rack.selected_module() {
                    let id = module.id;
                    self.rack.remove_module(id);
                }
                Action::None
            }
            Some("edit") => {
                if let Some(module) = self.rack.selected_module() {
                    Action::EditModule(module.id)
                } else {
                    Action::None
                }
            }
            Some("save") => Action::SaveRack,
            Some("load") => Action::LoadRack,
            _ => Action::None,
        }
    }

    fn render(&self, g: &mut dyn Graphics) {
        let (width, height) = g.size();
        let box_width = 97;
        let box_height = 29;
        let rect = Rect::centered(width, height, box_width, box_height);

        g.set_style(Style::new().fg(Color::BLACK));
        g.draw_box(rect, Some(" Rack "));

        let content_x = rect.x + 2;
        let content_y = rect.y + 2;

        // Title
        g.set_style(Style::new().fg(Color::BLACK));
        g.put_str(content_x, content_y, "Modules:");

        // Module list with viewport scrolling
        let list_y = content_y + 2;
        let max_visible = (rect.height - 7) as usize;
        let selected_idx = self.rack.selected.unwrap_or(0);

        // Calculate scroll offset to keep selection visible
        let scroll_offset = if selected_idx >= max_visible {
            selected_idx - max_visible + 1
        } else {
            0
        };

        for (i, &module_id) in self.rack.order.iter().enumerate().skip(scroll_offset) {
            let row = i - scroll_offset;
            if row >= max_visible {
                break;
            }
            let y = list_y + row as u16;

            if let Some(module) = self.rack.modules.get(&module_id) {
                let is_selected = self.rack.selected == Some(i);

                // Selection indicator
                if is_selected {
                    g.set_style(Style::new().fg(Color::WHITE).bg(Color::BLACK));
                    g.put_str(content_x, y, ">");
                } else {
                    g.set_style(Style::new().fg(Color::BLACK));
                    g.put_str(content_x, y, " ");
                }

                // Module name
                g.put_str(content_x + 2, y, &format!("{:16}", module.name));

                // Module type
                let type_name = format!("{:18}", module.module_type.name());
                g.put_str(content_x + 19, y, &type_name);

                // Parameters
                let params_str = self.format_params(module);
                if is_selected {
                    g.set_style(Style::new().fg(Color::WHITE).bg(Color::BLACK));
                } else {
                    g.set_style(Style::new().fg(Color::GRAY));
                }
                g.put_str(content_x + 38, y, &params_str);

                // Clear to end of selection if selected
                if is_selected {
                    let line_end = content_x + 38 + params_str.len() as u16;
                    for x in line_end..(rect.x + rect.width - 2) {
                        g.put_char(x, y, ' ');
                    }
                }
            }
        }

        // Scroll indicators
        if scroll_offset > 0 {
            g.set_style(Style::new().fg(Color::GRAY));
            g.put_str(rect.x + rect.width - 4, list_y, "...");
        }
        if scroll_offset + max_visible < self.rack.order.len() {
            g.set_style(Style::new().fg(Color::GRAY));
            g.put_str(rect.x + rect.width - 4, list_y + max_visible as u16 - 1, "...");
        }

        // Help text at bottom
        let help_y = rect.y + rect.height - 2;
        g.set_style(Style::new().fg(Color::GRAY));
        g.put_str(content_x, help_y, "a: add | d: delete | e: edit | q: quit");
    }

    fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    fn receive_action(&mut self, action: &Action) -> bool {
        match action {
            Action::AddModule(module_type) => {
                self.rack.add_module(*module_type);
                true
            }
            Action::UpdateModuleParams(id, params) => {
                self.update_module_params(*id, params.clone());
                true
            }
            _ => false,
        }
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
