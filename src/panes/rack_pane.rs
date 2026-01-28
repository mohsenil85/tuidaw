use std::any::Any;
use std::collections::HashSet;

use crate::state::{Connection, Module, ModuleId, ModuleType, Param, ParamValue, PortDirection, PortRef, PortType, RackState};
use crate::ui::{Action, Color, Graphics, InputEvent, KeyCode, Keymap, Pane, Rect, Style};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RackMode {
    Normal,
    ConnectSource,
    ConnectDest,
}

/// Get the display color for a module type
fn module_type_color(module_type: ModuleType) -> Color {
    match module_type {
        ModuleType::Midi => Color::MIDI_COLOR,
        ModuleType::SawOsc | ModuleType::SinOsc | ModuleType::SqrOsc | ModuleType::TriOsc => {
            Color::OSC_COLOR
        }
        ModuleType::Lpf | ModuleType::Hpf | ModuleType::Bpf => Color::FILTER_COLOR,
        ModuleType::AdsrEnv => Color::ENV_COLOR,
        ModuleType::Lfo => Color::LFO_COLOR,
        ModuleType::Delay | ModuleType::Reverb => Color::FX_COLOR,
        ModuleType::Output => Color::OUTPUT_COLOR,
    }
}

/// Get the display color for a port type
fn port_type_color(port_type: PortType) -> Color {
    match port_type {
        PortType::Audio => Color::AUDIO_PORT,
        PortType::Control => Color::CONTROL_PORT,
        PortType::Gate => Color::GATE_PORT,
    }
}

#[derive(Debug, Clone)]
struct TreeRow {
    module_id: ModuleId,
    depth: u16,
    is_last: bool,
    prefix: Vec<bool>, // ancestor is_last flags for vertical line drawing
    gap_before: bool,  // blank line before this row
}

impl TreeRow {
    fn prefix_str(&self) -> String {
        if self.depth == 0 {
            return String::new();
        }
        let mut s = String::new();
        for i in 0..(self.depth as usize - 1) {
            if self.prefix[i] {
                s.push_str("    ");
            } else {
                s.push_str("│   ");
            }
        }
        if self.is_last {
            s.push_str("└── ");
        } else {
            s.push_str("├── ");
        }
        s
    }
}

pub struct RackPane {
    keymap: Keymap,
    rack: RackState,
    mode: RackMode,
    selected_port: usize,
    pending_src: Option<PortRef>,
    tree_rows: Vec<TreeRow>,
    tree_selected: usize,
}

impl RackPane {
    pub fn new() -> Self {
        let rack = RackState::new();

        let mut pane = Self {
            keymap: Keymap::new()
                .bind('q', "quit", "Quit the application")
                .bind_key(KeyCode::Down, "next", "Next module")
                .bind_key(KeyCode::Up, "prev", "Previous module")
                .bind_key(KeyCode::Home, "goto_top", "Go to top")
                .bind_key(KeyCode::End, "goto_bottom", "Go to bottom")
                .bind('a', "add", "Add module")
                .bind('d', "delete", "Delete module")
                .bind_key(KeyCode::Enter, "edit", "Edit module")
                .bind('c', "connect", "Connect modules")
                .bind('x', "disconnect", "Disconnect modules")
                .bind('w', "save", "Save rack")
                .bind('o', "load", "Load rack")
                .bind_key(KeyCode::Left, "prev_port", "Previous port")
                .bind_key(KeyCode::Right, "next_port", "Next port")
                .bind_key(KeyCode::Tab, "next_port", "Next port")
                .bind_key(KeyCode::Escape, "cancel", "Cancel"),
            rack,
            mode: RackMode::Normal,
            selected_port: 0,
            pending_src: None,
            tree_rows: Vec::new(),
            tree_selected: 0,
        };
        pane.rebuild_tree();
        pane
    }

    // --- Tree building ---

    fn rebuild_tree(&mut self) {
        self.tree_rows = self.compute_tree();
        if self.tree_rows.is_empty() {
            self.tree_selected = 0;
            self.rack.selected = None;
        } else {
            if self.tree_selected >= self.tree_rows.len() {
                self.tree_selected = self.tree_rows.len() - 1;
            }
            self.sync_rack_selected();
        }
    }

    fn sync_rack_selected(&mut self) {
        if let Some(row) = self.tree_rows.get(self.tree_selected) {
            let module_id = row.module_id;
            if let Some(pos) = self.rack.order.iter().position(|&id| id == module_id) {
                self.rack.selected = Some(pos);
            }
        }
    }

    fn compute_tree(&self) -> Vec<TreeRow> {
        let mut rows = Vec::new();
        let mut visited = HashSet::new();

        // Find Output modules as roots (preserve rack order)
        let roots: Vec<ModuleId> = self.rack.order.iter()
            .filter(|&&id| self.rack.modules.get(&id)
                .map(|m| m.module_type == ModuleType::Output)
                .unwrap_or(false))
            .copied()
            .collect();

        for (i, &root_id) in roots.iter().enumerate() {
            self.tree_dfs(root_id, 0, true, &mut vec![], &mut visited, &mut rows, i > 0);
        }

        // Unconnected modules at bottom
        let mut first_unconnected = true;
        for &id in &self.rack.order {
            if !visited.contains(&id) {
                let gap = !rows.is_empty() && first_unconnected;
                first_unconnected = false;
                visited.insert(id);
                rows.push(TreeRow {
                    module_id: id,
                    depth: 0,
                    is_last: true,
                    prefix: vec![],
                    gap_before: gap,
                });
            }
        }

        rows
    }

    fn tree_dfs(
        &self,
        module_id: ModuleId,
        depth: u16,
        is_last: bool,
        prefix: &mut Vec<bool>,
        visited: &mut HashSet<ModuleId>,
        rows: &mut Vec<TreeRow>,
        gap_before: bool,
    ) {
        if visited.contains(&module_id) {
            return;
        }
        visited.insert(module_id);

        rows.push(TreeRow {
            module_id,
            depth,
            is_last,
            prefix: prefix.clone(),
            gap_before,
        });

        // Upstream modules: things that connect TO this module
        let mut upstream: Vec<ModuleId> = self.rack.connections_to(module_id)
            .iter()
            .map(|c| c.src.module_id)
            .collect();

        // Deterministic order and dedup
        upstream.sort_by_key(|id| {
            self.rack.order.iter().position(|&oid| oid == *id).unwrap_or(usize::MAX)
        });
        upstream.dedup();

        prefix.push(is_last);
        for (i, &child_id) in upstream.iter().enumerate() {
            let child_is_last = i == upstream.len() - 1;
            self.tree_dfs(child_id, depth + 1, child_is_last, prefix, visited, rows, false);
        }
        prefix.pop();
    }

    // --- Navigation ---

    fn tree_select_next(&mut self) {
        if !self.tree_rows.is_empty() && self.tree_selected < self.tree_rows.len() - 1 {
            self.tree_selected += 1;
            self.sync_rack_selected();
        }
    }

    fn tree_select_prev(&mut self) {
        if self.tree_selected > 0 {
            self.tree_selected -= 1;
            self.sync_rack_selected();
        }
    }

    // --- Helpers ---

    fn format_params(&self, module: &Module) -> String {
        let mut parts = Vec::new();

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

    /// Get mutable reference to rack state
    pub fn rack_mut(&mut self) -> &mut RackState {
        &mut self.rack
    }

    /// Remove a module and rebuild tree
    pub fn remove_module(&mut self, id: ModuleId) {
        self.rack.remove_module(id);
        self.rebuild_tree();
    }

    /// Replace rack state (for loading)
    pub fn set_rack(&mut self, rack: RackState) {
        self.rack = rack;
        self.mode = RackMode::Normal;
        self.selected_port = 0;
        self.pending_src = None;
        self.tree_selected = 0;
        self.rebuild_tree();
    }

    /// Get the direction filter for the current connect phase
    fn connect_direction_filter(&self) -> Option<PortDirection> {
        match self.mode {
            RackMode::ConnectSource => Some(PortDirection::Output),
            RackMode::ConnectDest => Some(PortDirection::Input),
            RackMode::Normal => None,
        }
    }

    /// Get filtered ports for the selected module based on connect phase
    fn filtered_ports(&self, module: &Module) -> Vec<usize> {
        let ports = module.module_type.ports();
        if let Some(dir) = self.connect_direction_filter() {
            ports.iter().enumerate()
                .filter(|(_, p)| p.direction == dir)
                .map(|(i, _)| i)
                .collect()
        } else {
            (0..ports.len()).collect()
        }
    }

    /// Get port count for the selected module
    fn selected_module_port_count(&self) -> usize {
        self.rack
            .selected_module()
            .map(|m| self.filtered_ports(m).len())
            .unwrap_or(0)
    }

    /// Get selected port for current module
    fn get_selected_port(&self) -> Option<PortRef> {
        let module = self.rack.selected_module()?;
        let filtered = self.filtered_ports(module);
        let ports = module.module_type.ports();
        let &port_idx = filtered.get(self.selected_port)?;
        Some(PortRef::new(module.id, ports[port_idx].name))
    }

    fn handle_normal_input(&mut self, event: InputEvent) -> Action {
        match self.keymap.lookup(&event) {
            Some("quit") => Action::Quit,
            Some("next") => {
                self.tree_select_next();
                Action::None
            }
            Some("prev") => {
                self.tree_select_prev();
                Action::None
            }
            Some("goto_top") => {
                if !self.tree_rows.is_empty() {
                    self.tree_selected = 0;
                    self.sync_rack_selected();
                }
                Action::None
            }
            Some("goto_bottom") => {
                if !self.tree_rows.is_empty() {
                    self.tree_selected = self.tree_rows.len() - 1;
                    self.sync_rack_selected();
                }
                Action::None
            }
            Some("add") => Action::SwitchPane("add"),
            Some("delete") => {
                if let Some(module) = self.rack.selected_module() {
                    let id = module.id;
                    Action::DeleteModule(id)
                } else {
                    Action::None
                }
            }
            Some("edit") => {
                if let Some(module) = self.rack.selected_module() {
                    Action::EditModule(module.id)
                } else {
                    Action::None
                }
            }
            Some("connect") => {
                if self.rack.selected_module().is_some() {
                    self.mode = RackMode::ConnectSource;
                    self.selected_port = 0;
                    self.pending_src = None;
                }
                Action::None
            }
            Some("disconnect") => {
                // Delete connections from/to selected module
                if let Some(module) = self.rack.selected_module() {
                    let module_id = module.id;
                    let conn = self.rack.connections
                        .iter()
                        .find(|c| c.src.module_id == module_id || c.dst.module_id == module_id)
                        .cloned();
                    if let Some(connection) = conn {
                        return Action::RemoveConnection(connection);
                    }
                }
                Action::None
            }
            Some("save") => Action::SaveRack,
            Some("load") => Action::LoadRack,
            _ => Action::None,
        }
    }

    fn handle_connect_input(&mut self, event: InputEvent) -> Action {
        match self.keymap.lookup(&event) {
            Some("cancel") => {
                self.mode = RackMode::Normal;
                self.pending_src = None;
                self.selected_port = 0;
                Action::None
            }
            Some("next") => {
                self.tree_select_next();
                self.selected_port = 0;
                Action::None
            }
            Some("prev") => {
                self.tree_select_prev();
                self.selected_port = 0;
                Action::None
            }
            Some("next_port") => {
                let port_count = self.selected_module_port_count();
                if port_count > 0 {
                    self.selected_port = (self.selected_port + 1) % port_count;
                }
                Action::None
            }
            Some("prev_port") => {
                let port_count = self.selected_module_port_count();
                if port_count > 0 {
                    self.selected_port = if self.selected_port == 0 {
                        port_count - 1
                    } else {
                        self.selected_port - 1
                    };
                }
                Action::None
            }
            Some("edit") => {
                // Enter acts as confirm in connect mode
                if let Some(port_ref) = self.get_selected_port() {
                    match self.mode {
                        RackMode::ConnectSource => {
                            self.pending_src = Some(port_ref);
                            self.mode = RackMode::ConnectDest;
                            self.selected_port = 0;
                        }
                        RackMode::ConnectDest => {
                            if let Some(src) = self.pending_src.take() {
                                let connection = Connection::new(src, port_ref);
                                self.mode = RackMode::Normal;
                                self.selected_port = 0;
                                return Action::AddConnection(connection);
                            }
                        }
                        RackMode::Normal => {}
                    }
                }
                Action::None
            }
            _ => Action::None,
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
        match self.mode {
            RackMode::Normal => self.handle_normal_input(event),
            RackMode::ConnectSource | RackMode::ConnectDest => self.handle_connect_input(event),
        }
    }

    fn render(&self, g: &mut dyn Graphics) {
        let (width, height) = g.size();
        let box_width = 97;
        let box_height = 29;
        let rect = Rect::centered(width, height, box_width, box_height);

        let (border_color, title) = match self.mode {
            RackMode::Normal => (Color::CYAN, " Rack "),
            RackMode::ConnectSource => (Color::AUDIO_PORT, " Rack - Select Source "),
            RackMode::ConnectDest => (Color::CONTROL_PORT, " Rack - Select Destination "),
        };
        g.set_style(Style::new().fg(border_color));
        g.draw_box(rect, Some(title));

        let content_x = rect.x + 2;
        let content_y = rect.y + 2;

        // Title
        g.set_style(Style::new().fg(Color::CYAN).bold());
        g.put_str(content_x, content_y, "Modules:");

        // Determine layout based on mode
        let in_connect_mode = self.mode != RackMode::Normal;
        let max_visible = if in_connect_mode {
            ((rect.height - 10) as usize).max(3)
        } else {
            ((rect.height - 8) as usize).max(3)
        };

        let list_y = content_y + 2;

        // Count display rows (tree rows + gap lines)
        // Calculate total display height including gaps
        let mut total_display = 0usize;
        let mut row_to_display: Vec<usize> = Vec::new(); // maps tree_row index → display line
        for (i, row) in self.tree_rows.iter().enumerate() {
            if row.gap_before && i > 0 {
                total_display += 1; // blank line
            }
            row_to_display.push(total_display);
            total_display += 1;
        }

        // Calculate scroll offset based on selected item's display position
        let selected_display = row_to_display.get(self.tree_selected).copied().unwrap_or(0);
        let scroll_offset = if selected_display >= max_visible {
            selected_display - max_visible + 1
        } else {
            0
        };

        // Render tree rows
        let mut display_line = 0usize;
        for (i, row) in self.tree_rows.iter().enumerate() {
            if row.gap_before && i > 0 {
                display_line += 1; // skip a line for gap
            }

            if display_line < scroll_offset {
                display_line += 1;
                continue;
            }
            let screen_row = display_line - scroll_offset;
            if screen_row >= max_visible {
                break;
            }

            let y = list_y + screen_row as u16;
            let is_selected = i == self.tree_selected;

            if let Some(module) = self.rack.modules.get(&row.module_id) {
                let type_color = module_type_color(module.module_type);
                let prefix = row.prefix_str();
                let prefix_len = prefix.len() as u16;

                // Selection indicator
                if is_selected {
                    g.set_style(Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG).bold());
                    g.put_str(content_x, y, ">");
                } else {
                    g.set_style(Style::new().fg(Color::DARK_GRAY));
                    g.put_str(content_x, y, " ");
                }

                // Tree prefix
                if is_selected {
                    g.set_style(Style::new().fg(Color::DARK_GRAY).bg(Color::SELECTION_BG));
                } else {
                    g.set_style(Style::new().fg(Color::DARK_GRAY));
                }
                if !prefix.is_empty() {
                    g.put_str(content_x + 2, y, &prefix);
                }

                // Module name (truncate if tree is deep)
                let name_col = content_x + 2 + prefix_len;
                let name_width = 16u16.saturating_sub(prefix_len.min(12));
                if is_selected {
                    g.set_style(Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG));
                } else {
                    g.set_style(Style::new().fg(Color::WHITE));
                }
                let name_str = if module.name.len() > name_width as usize {
                    &module.name[..name_width as usize]
                } else {
                    &module.name
                };
                g.put_str(name_col, y, &format!("{:width$}", name_str, width = name_width as usize));

                // Module type (colored by category)
                let type_col = content_x + 19;
                if is_selected {
                    g.set_style(Style::new().fg(type_color).bg(Color::SELECTION_BG));
                } else {
                    g.set_style(Style::new().fg(type_color));
                }
                let type_name = format!("{:18}", module.module_type.name());
                g.put_str(type_col, y, &type_name);

                if in_connect_mode {
                    // Show ports filtered by direction for current connect phase
                    let ports = module.module_type.ports();
                    let filtered = self.filtered_ports(module);
                    let mut port_x = content_x + 38;
                    for (fi, &port_idx) in filtered.iter().enumerate() {
                        let port = &ports[port_idx];
                        let is_port_selected = is_selected && fi == self.selected_port;
                        let port_color = port_type_color(port.port_type);
                        if is_port_selected {
                            g.set_style(Style::new().fg(Color::BLACK).bg(port_color).bold());
                        } else if is_selected {
                            g.set_style(Style::new().fg(port_color).bg(Color::SELECTION_BG));
                        } else {
                            g.set_style(Style::new().fg(port_color));
                        }
                        let port_str = format!("[{}]", port.name);
                        g.put_str(port_x, y, &port_str);
                        port_x += port_str.len() as u16 + 1;
                    }
                    if is_selected {
                        g.set_style(Style::new().fg(Color::WHITE).bg(Color::SELECTION_BG));
                        for x in port_x..(rect.x + rect.width - 2) {
                            g.put_char(x, y, ' ');
                        }
                    }
                } else {
                    // Show parameters in normal mode
                    let params_str = self.format_params(module);
                    if is_selected {
                        g.set_style(Style::new().fg(Color::SKY_BLUE).bg(Color::SELECTION_BG));
                    } else {
                        g.set_style(Style::new().fg(Color::DARK_GRAY));
                    }
                    g.put_str(content_x + 38, y, &params_str);

                    if is_selected {
                        g.set_style(Style::new().bg(Color::SELECTION_BG));
                        let line_end = content_x + 38 + params_str.len() as u16;
                        for x in line_end..(rect.x + rect.width - 2) {
                            g.put_char(x, y, ' ');
                        }
                    }
                }
            }

            display_line += 1;
        }

        // Scroll indicators
        if scroll_offset > 0 {
            g.set_style(Style::new().fg(Color::ORANGE));
            g.put_str(rect.x + rect.width - 4, list_y, "...");
        }
        if scroll_offset + max_visible < total_display {
            g.set_style(Style::new().fg(Color::ORANGE));
            g.put_str(rect.x + rect.width - 4, list_y + max_visible as u16 - 1, "...");
        }

        // Show connect mode status
        let status_y = list_y + max_visible as u16 + 1;
        if in_connect_mode {
            if let Some(ref src) = self.pending_src {
                let src_name = self.rack.modules.get(&src.module_id)
                    .map(|m| m.name.as_str())
                    .unwrap_or("?");
                g.set_style(Style::new().fg(Color::AUDIO_PORT).bold());
                g.put_str(content_x, status_y, &format!("Source: {}:{}", src_name, src.port_name));
                g.set_style(Style::new().fg(Color::CONTROL_PORT));
                g.put_str(content_x, status_y + 1, "Target: (select destination...)");
            } else {
                g.set_style(Style::new().fg(Color::ORANGE));
                g.put_str(content_x, status_y, "Source: (select source port...)");
            }
        }

        // Help text at bottom
        let help_y = rect.y + rect.height - 2;
        g.set_style(Style::new().fg(Color::DARK_GRAY));
        let help_text = if in_connect_mode {
            "j/k: module | Tab/h/l: port | Enter: confirm | Esc: cancel"
        } else {
            "a: add | d: delete | e: edit | c: connect | x: disconnect | q: quit"
        };
        g.put_str(content_x, help_y, help_text);
    }

    fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    fn receive_action(&mut self, action: &Action) -> bool {
        let handled = match action {
            Action::AddModule(module_type) => {
                self.rack.add_module(*module_type);
                true
            }
            Action::UpdateModuleParams(id, params) => {
                self.update_module_params(*id, params.clone());
                true
            }
            Action::AddConnection(connection) => {
                let _ = self.rack.add_connection(connection.clone());
                true
            }
            Action::RemoveConnection(connection) => {
                self.rack.remove_connection(connection);
                true
            }
            _ => false,
        };
        if handled {
            self.rebuild_tree();
        }
        handled
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
