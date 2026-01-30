#![allow(dead_code)]

use crate::ui::{InputEvent, KeyCode};

/// An item in a select list
#[derive(Clone)]
pub struct ListItem {
    pub id: String,
    pub label: String,
}

impl ListItem {
    pub fn new(id: &str, label: &str) -> Self {
        Self {
            id: id.to_string(),
            label: label.to_string(),
        }
    }
}

/// A selectable list widget
pub struct SelectList {
    items: Vec<ListItem>,
    selected: usize,
    focused: bool,
    title: String,
}

impl SelectList {
    pub fn new(title: &str) -> Self {
        Self {
            items: Vec::new(),
            selected: 0,
            focused: false,
            title: title.to_string(),
        }
    }

    pub fn with_items(mut self, items: Vec<ListItem>) -> Self {
        self.items = items;
        self
    }

    pub fn add_item(&mut self, id: &str, label: &str) {
        self.items.push(ListItem::new(id, label));
    }

    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    pub fn is_focused(&self) -> bool {
        self.focused
    }

    pub fn selected_index(&self) -> usize {
        self.selected
    }

    pub fn selected_item(&self) -> Option<&ListItem> {
        self.items.get(self.selected)
    }

    pub fn select_by_id(&mut self, id: &str) -> bool {
        if let Some(idx) = self.items.iter().position(|item| item.id == id) {
            self.selected = idx;
            true
        } else {
            false
        }
    }

    /// Handle input, returns true if the event was consumed
    pub fn handle_input(&mut self, event: &InputEvent) -> bool {
        if !self.focused || self.items.is_empty() {
            return false;
        }

        match event.key {
            KeyCode::Up | KeyCode::Char('p') | KeyCode::Char('k') => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
                true
            }
            KeyCode::Down | KeyCode::Char('n') | KeyCode::Char('j') => {
                if self.selected < self.items.len() - 1 {
                    self.selected += 1;
                }
                true
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.selected = 0;
                true
            }
            KeyCode::End | KeyCode::Char('G') => {
                self.selected = self.items.len().saturating_sub(1);
                true
            }
            _ => false,
        }
    }

}
