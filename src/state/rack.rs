use std::collections::HashMap;

use super::{Module, ModuleId, ModuleType};

#[derive(Debug, Clone, Default)]
pub struct RackState {
    pub modules: HashMap<ModuleId, Module>,
    pub order: Vec<ModuleId>,
    pub selected: Option<usize>, // Index in order vec
    next_id: ModuleId,
}

impl RackState {
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            order: Vec::new(),
            selected: None,
            next_id: 0,
        }
    }

    pub fn add_module(&mut self, module_type: ModuleType) -> ModuleId {
        let id = self.next_id;
        self.next_id += 1;

        let module = Module::new(id, module_type);
        self.modules.insert(id, module);
        self.order.push(id);

        // Auto-select first module if none selected
        if self.selected.is_none() {
            self.selected = Some(0);
        }

        id
    }

    pub fn remove_module(&mut self, id: ModuleId) {
        if let Some(pos) = self.order.iter().position(|&mid| mid == id) {
            self.order.remove(pos);
            self.modules.remove(&id);

            // Adjust selection
            if let Some(selected_idx) = self.selected {
                if selected_idx >= self.order.len() {
                    // Selection was at or past removed item
                    self.selected = if self.order.is_empty() {
                        None
                    } else {
                        Some(self.order.len() - 1)
                    };
                }
            }
        }
    }

    pub fn selected_module(&self) -> Option<&Module> {
        self.selected
            .and_then(|idx| self.order.get(idx))
            .and_then(|id| self.modules.get(id))
    }

    pub fn selected_module_mut(&mut self) -> Option<&mut Module> {
        if let Some(idx) = self.selected {
            if let Some(&id) = self.order.get(idx) {
                return self.modules.get_mut(&id);
            }
        }
        None
    }

    pub fn move_up(&mut self) {
        if let Some(idx) = self.selected {
            if idx > 0 {
                self.order.swap(idx - 1, idx);
                self.selected = Some(idx - 1);
            }
        }
    }

    pub fn move_down(&mut self) {
        if let Some(idx) = self.selected {
            if idx < self.order.len().saturating_sub(1) {
                self.order.swap(idx, idx + 1);
                self.selected = Some(idx + 1);
            }
        }
    }

    pub fn select_next(&mut self) {
        if self.order.is_empty() {
            self.selected = None;
            return;
        }

        self.selected = match self.selected {
            None => Some(0),
            Some(idx) if idx < self.order.len() - 1 => Some(idx + 1),
            Some(idx) => Some(idx), // Stay at last item
        };
    }

    pub fn select_prev(&mut self) {
        if self.order.is_empty() {
            self.selected = None;
            return;
        }

        self.selected = match self.selected {
            None => Some(0),
            Some(0) => Some(0), // Stay at first item
            Some(idx) => Some(idx - 1),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rack_creation() {
        let rack = RackState::new();
        assert_eq!(rack.modules.len(), 0);
        assert_eq!(rack.order.len(), 0);
        assert_eq!(rack.selected, None);
    }

    #[test]
    fn test_add_module() {
        let mut rack = RackState::new();
        let id1 = rack.add_module(ModuleType::SawOsc);
        let id2 = rack.add_module(ModuleType::Lpf);

        assert_eq!(rack.modules.len(), 2);
        assert_eq!(rack.order.len(), 2);
        assert_eq!(rack.order[0], id1);
        assert_eq!(rack.order[1], id2);
        assert_eq!(rack.selected, Some(0)); // Auto-selected first
    }

    #[test]
    fn test_remove_module() {
        let mut rack = RackState::new();
        let id1 = rack.add_module(ModuleType::SawOsc);
        let id2 = rack.add_module(ModuleType::Lpf);
        let id3 = rack.add_module(ModuleType::Output);

        rack.remove_module(id2);

        assert_eq!(rack.modules.len(), 2);
        assert_eq!(rack.order.len(), 2);
        assert_eq!(rack.order[0], id1);
        assert_eq!(rack.order[1], id3);
    }

    #[test]
    fn test_remove_selected_module() {
        let mut rack = RackState::new();
        let id1 = rack.add_module(ModuleType::SawOsc);
        let id2 = rack.add_module(ModuleType::Lpf);
        let _id3 = rack.add_module(ModuleType::Output);

        rack.selected = Some(1); // Select middle module
        rack.remove_module(id2);

        assert_eq!(rack.selected, Some(1)); // Selection moves to next item
    }

    #[test]
    fn test_remove_last_module() {
        let mut rack = RackState::new();
        let id1 = rack.add_module(ModuleType::SawOsc);
        let id2 = rack.add_module(ModuleType::Lpf);

        rack.selected = Some(1); // Select last module
        rack.remove_module(id2);

        assert_eq!(rack.selected, Some(0)); // Selection adjusts to last available
    }

    #[test]
    fn test_remove_all_modules() {
        let mut rack = RackState::new();
        let id1 = rack.add_module(ModuleType::SawOsc);

        rack.remove_module(id1);

        assert_eq!(rack.selected, None);
        assert!(rack.order.is_empty());
    }

    #[test]
    fn test_selected_module() {
        let mut rack = RackState::new();
        rack.add_module(ModuleType::SawOsc);
        rack.add_module(ModuleType::Lpf);

        rack.selected = Some(0);
        let module = rack.selected_module().unwrap();
        assert_eq!(module.module_type, ModuleType::SawOsc);

        rack.selected = Some(1);
        let module = rack.selected_module().unwrap();
        assert_eq!(module.module_type, ModuleType::Lpf);
    }

    #[test]
    fn test_selected_module_mut() {
        let mut rack = RackState::new();
        rack.add_module(ModuleType::SawOsc);

        rack.selected = Some(0);
        if let Some(module) = rack.selected_module_mut() {
            module.name = "Custom Name".to_string();
        }

        let module = rack.selected_module().unwrap();
        assert_eq!(module.name, "Custom Name");
    }

    #[test]
    fn test_select_next() {
        let mut rack = RackState::new();
        rack.add_module(ModuleType::SawOsc);
        rack.add_module(ModuleType::Lpf);
        rack.add_module(ModuleType::Output);

        rack.selected = Some(0);
        rack.select_next();
        assert_eq!(rack.selected, Some(1));

        rack.select_next();
        assert_eq!(rack.selected, Some(2));

        rack.select_next();
        assert_eq!(rack.selected, Some(2)); // Stay at last
    }

    #[test]
    fn test_select_prev() {
        let mut rack = RackState::new();
        rack.add_module(ModuleType::SawOsc);
        rack.add_module(ModuleType::Lpf);
        rack.add_module(ModuleType::Output);

        rack.selected = Some(2);
        rack.select_prev();
        assert_eq!(rack.selected, Some(1));

        rack.select_prev();
        assert_eq!(rack.selected, Some(0));

        rack.select_prev();
        assert_eq!(rack.selected, Some(0)); // Stay at first
    }

    #[test]
    fn test_select_on_empty_rack() {
        let mut rack = RackState::new();

        rack.select_next();
        assert_eq!(rack.selected, None);

        rack.select_prev();
        assert_eq!(rack.selected, None);
    }

    #[test]
    fn test_move_up() {
        let mut rack = RackState::new();
        let id1 = rack.add_module(ModuleType::SawOsc);
        let id2 = rack.add_module(ModuleType::Lpf);
        let id3 = rack.add_module(ModuleType::Output);

        rack.selected = Some(1); // Select middle module
        rack.move_up();

        assert_eq!(rack.selected, Some(0));
        assert_eq!(rack.order[0], id2);
        assert_eq!(rack.order[1], id1);
        assert_eq!(rack.order[2], id3);
    }

    #[test]
    fn test_move_down() {
        let mut rack = RackState::new();
        let id1 = rack.add_module(ModuleType::SawOsc);
        let id2 = rack.add_module(ModuleType::Lpf);
        let id3 = rack.add_module(ModuleType::Output);

        rack.selected = Some(1); // Select middle module
        rack.move_down();

        assert_eq!(rack.selected, Some(2));
        assert_eq!(rack.order[0], id1);
        assert_eq!(rack.order[1], id3);
        assert_eq!(rack.order[2], id2);
    }

    #[test]
    fn test_move_up_at_top() {
        let mut rack = RackState::new();
        let id1 = rack.add_module(ModuleType::SawOsc);
        let id2 = rack.add_module(ModuleType::Lpf);

        rack.selected = Some(0);
        rack.move_up();

        assert_eq!(rack.selected, Some(0)); // Stay at top
        assert_eq!(rack.order[0], id1); // Order unchanged
        assert_eq!(rack.order[1], id2);
    }

    #[test]
    fn test_move_down_at_bottom() {
        let mut rack = RackState::new();
        let id1 = rack.add_module(ModuleType::SawOsc);
        let id2 = rack.add_module(ModuleType::Lpf);

        rack.selected = Some(1);
        rack.move_down();

        assert_eq!(rack.selected, Some(1)); // Stay at bottom
        assert_eq!(rack.order[0], id1); // Order unchanged
        assert_eq!(rack.order[1], id2);
    }
}
