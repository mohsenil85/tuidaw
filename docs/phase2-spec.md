# Phase 2 Specification

Detailed specs for parallel implementation by agents.

## Task 1: State Types (`src/state/`)

### Files to Create

**`src/state/mod.rs`**
```rust
mod module;
mod rack;

pub use module::{Module, ModuleId, ModuleType, Param, ParamValue};
pub use rack::RackState;
```

**`src/state/module.rs`**
```rust
pub type ModuleId = u32;

#[derive(Debug, Clone, PartialEq)]
pub enum ModuleType {
    // Oscillators
    SawOsc,
    SinOsc,
    SqrOsc,
    TriOsc,

    // Filters
    Lpf,
    Hpf,
    Bpf,

    // Envelopes
    AdsrEnv,

    // Modulation
    Lfo,

    // Effects
    Delay,
    Reverb,

    // Output
    Output,
}

impl ModuleType {
    pub fn name(&self) -> &'static str { ... }
    pub fn default_params(&self) -> Vec<Param> { ... }
    pub fn all_types() -> Vec<ModuleType> { ... }
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: &'static str,
    pub value: ParamValue,
    pub min: f32,
    pub max: f32,
}

#[derive(Debug, Clone)]
pub enum ParamValue {
    Float(f32),
    Int(i32),
    Bool(bool),
}

#[derive(Debug, Clone)]
pub struct Module {
    pub id: ModuleId,
    pub module_type: ModuleType,
    pub name: String,        // e.g., "saw-1", "lpf-2"
    pub params: Vec<Param>,
}
```

**`src/state/rack.rs`**
```rust
use std::collections::HashMap;
use super::{Module, ModuleId};

#[derive(Debug, Clone, Default)]
pub struct RackState {
    pub modules: HashMap<ModuleId, Module>,
    pub order: Vec<ModuleId>,           // Display order
    pub selected: Option<usize>,        // Index in order
    pub next_id: ModuleId,
}

impl RackState {
    pub fn new() -> Self { ... }
    pub fn add_module(&mut self, module_type: ModuleType) -> ModuleId { ... }
    pub fn remove_module(&mut self, id: ModuleId) { ... }
    pub fn selected_module(&self) -> Option<&Module> { ... }
    pub fn selected_module_mut(&mut self) -> Option<&mut Module> { ... }
    pub fn move_up(&mut self) { ... }
    pub fn move_down(&mut self) { ... }
}
```

### Tests Required
- Create module, verify default params
- Add/remove modules from rack
- Selection navigation
- Move module up/down in order

---

## Task 2: Action & Effect Enums (`src/core/`)

### Files to Create

**`src/core/mod.rs`**
```rust
mod action;
mod effect;

pub use action::Action;
pub use effect::Effect;
```

**`src/core/action.rs`**
```rust
use crate::state::ModuleType;

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    // Navigation
    MoveUp,
    MoveDown,
    SelectNext,      // n
    SelectPrev,      // p

    // Module operations
    AddModule(ModuleType),
    DeleteSelected,
    EditSelected,

    // Parameter editing
    ParamIncrement,
    ParamDecrement,
    ParamSet(f32),
    NextParam,
    PrevParam,

    // View switching
    OpenAddView,
    OpenEditView,
    CloseView,       // Escape
    Confirm,         // Enter

    // System
    Quit,
    Save,
    Undo,
    Redo,
}
```

**`src/core/effect.rs`**
```rust
use std::path::PathBuf;
use crate::state::ModuleId;

#[derive(Debug, Clone)]
pub enum Effect {
    // Audio (future)
    CreateSynth { module_id: ModuleId },
    FreeSynth { module_id: ModuleId },
    SetParam { module_id: ModuleId, param: String, value: f32 },

    // Persistence
    Save,
    Load { path: PathBuf },

    // System
    Quit,
}
```

---

## Task 3: Rack View Pane

Integrate with existing pane system. Replace DemoPane as the main view.

**`src/panes/rack_pane.rs`**
```rust
use crate::ui::{Pane, Action as PaneAction, Keymap, Graphics, ...};
use crate::state::RackState;
use crate::core::Action;

pub struct RackPane {
    keymap: Keymap,
    state: RackState,
}
```

### Keymap
| Key | Action | Description |
|-----|--------|-------------|
| `q` | Quit | Exit app |
| `n`/`j`/Down | SelectNext | Next module |
| `p`/`k`/Up | SelectPrev | Prev module |
| `a` | OpenAddView | Add module |
| `d` | DeleteSelected | Delete module |
| `e`/Enter | EditSelected | Edit params |
| `g` | First | Go to first |
| `G` | Last | Go to last |

### UI Layout
```
┌ Rack ─────────────────────────────────────────────────────────────────────────┐
│                                                                               │
│  Modules:                                                                     │
│                                                                               │
│  > saw-1         SawOsc       freq: 440.0  amp: 0.5                          │
│    lpf-1         Lpf          cutoff: 1000.0  res: 0.5                       │
│    out-1         Output                                                       │
│                                                                               │
│                                                                               │
│                                                                               │
│  a: add | d: delete | e: edit | q: quit                                      │
└───────────────────────────────────────────────────────────────────────────────┘
```

---

## Task 4: Add View Pane

Modal pane for selecting module type to add.

**`src/panes/add_pane.rs`**

### UI Layout
```
┌ Add Module ───────────────────────────────────────────────────────────────────┐
│                                                                               │
│  Select module type:                                                          │
│                                                                               │
│  Oscillators:                                                                 │
│  > SawOsc        Sawtooth oscillator                                         │
│    SinOsc        Sine oscillator                                             │
│    SqrOsc        Square oscillator                                           │
│                                                                               │
│  Filters:                                                                     │
│    Lpf           Low-pass filter                                             │
│    Hpf           High-pass filter                                            │
│                                                                               │
│  Output:                                                                      │
│    Output        Audio output                                                │
│                                                                               │
│  Enter: add | Escape: cancel                                                 │
└───────────────────────────────────────────────────────────────────────────────┘
```

### Behavior
- Enter adds selected module, returns to Rack
- Escape cancels, returns to Rack
- n/p navigates list

---

## Task 5: Edit View Pane

Modal pane for editing module parameters.

**`src/panes/edit_pane.rs`**

### UI Layout
```
┌ Edit: saw-1 (SawOsc) ─────────────────────────────────────────────────────────┐
│                                                                               │
│  Parameters:                                                                  │
│                                                                               │
│  > freq      [━━━━━━━━━━━━━━━━━━━━━━━●━━━━━━━]  440.0 Hz                     │
│    amp       [━━━━━━━━━━●━━━━━━━━━━━━━━━━━━━━]  0.5                          │
│    detune    [━━━━━━━━━━━━━━━━━━━━●━━━━━━━━━━]  0.0                          │
│                                                                               │
│                                                                               │
│  Left/Right: adjust | n/p: next/prev param | Escape: done                    │
└───────────────────────────────────────────────────────────────────────────────┘
```

### Behavior
- Left/Right adjusts value (coarse)
- Shift+Left/Right adjusts value (fine) - optional
- n/p or Up/Down selects param
- Escape returns to Rack

---

## Integration Notes

### Pane Switching
- RackPane is the main pane (id: "rack")
- AddPane is modal (id: "add") - pushed on stack
- EditPane is modal (id: "edit") - pushed on stack
- Use `Action::PushPane("add")` / `Action::PopPane`

### State Ownership
- `RackState` lives in `RackPane`
- AddPane returns selected `ModuleType` via action
- EditPane borrows module data, returns param changes

### File Structure
```
src/
├── main.rs
├── state/
│   ├── mod.rs
│   ├── module.rs
│   └── rack.rs
├── core/
│   ├── mod.rs
│   ├── action.rs
│   └── effect.rs
├── panes/
│   ├── mod.rs
│   ├── rack_pane.rs
│   ├── add_pane.rs
│   └── edit_pane.rs
└── ui/
    └── (existing)
```

---

## Parallel Execution Plan

**Can run in parallel:**
- Task 1 (State types) - no dependencies
- Task 2 (Action/Effect) - no dependencies

**Sequential after above:**
- Task 3 (Rack view) - depends on 1, 2
- Task 4 (Add view) - depends on 1, 2, 3
- Task 5 (Edit view) - depends on 1, 2, 3

**Suggested approach:**
1. Fork 2 Sonnets for Tasks 1 & 2 simultaneously
2. After both complete, fork 1 Sonnet for Task 3
3. After Task 3, fork 2 Sonnets for Tasks 4 & 5 simultaneously
