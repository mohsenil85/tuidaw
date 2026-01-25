# Next Steps

Roadmap for tuidaw development after Phase 2 completion.

## Current State

Phase 2 complete:
- UI engine with ratatui backend
- State types: `Module`, `ModuleType`, `Param`, `RackState`
- Action/Effect enums for command pattern
- Three panes: Rack, Add, Edit (all with viewport scrolling)

## Phase 3: Pane Communication

**Status:** Not started

The panes exist but don't communicate. Wire them up so user actions flow through properly.

### Task 3.1: Add Module Flow

When user presses Enter in AddPane, the selected module type should be added to the rack.

**Current behavior:**
- AddPane shows module types
- Enter just calls `Action::SwitchPane("rack")`
- No module is actually added

**Needed:**
- AddPane needs to communicate selected `ModuleType` back
- RackPane needs to receive it and call `rack.add_module(type)`

**Options:**
1. New action: `Action::AddModule(ModuleType)` - handle in main loop
2. Shared state: `Arc<Mutex<RackState>>` accessible to both panes
3. Message passing: Return value from pane switch

**Recommendation:** Option 1 - Add `AddModule(ModuleType)` to ui::Action enum. Main loop handles it by finding RackPane and calling add_module.

### Task 3.2: Edit Module Flow

When user presses 'e' in RackPane, EditPane should open with that module's params.

**Current behavior:**
- 'e' returns `Action::None` (placeholder)
- EditPane exists with hardcoded test params
- No connection between selected module and EditPane

**Needed:**
- RackPane needs to pass selected module data to EditPane
- EditPane changes need to flow back to RackState
- On Escape, params should be saved (or discarded?)

**Options:**
1. Recreate EditPane each time with current module's params
2. EditPane holds reference/id to module, fetches params on open
3. Clone params into EditPane, sync back on close

**Recommendation:** Option 3 - Clone params, sync on close. Simpler ownership.

### Task 3.3: Delete Confirmation (Optional)

Currently 'd' deletes immediately. Maybe add confirmation?

---

## Phase 4: Module Connections

**Status:** Future

Modules need signal routing (osc → filter → output).

### Concepts

```
┌─────────┐     ┌─────────┐     ┌─────────┐
│ SawOsc  │────▶│   Lpf   │────▶│ Output  │
└─────────┘     └─────────┘     └─────────┘
```

### Data Model

```rust
struct Connection {
    from_module: ModuleId,
    from_port: &'static str,  // "out", "audio", etc.
    to_module: ModuleId,
    to_port: &'static str,    // "in", "cutoff", etc.
}

struct RackState {
    modules: HashMap<ModuleId, Module>,
    connections: Vec<Connection>,
    // ...
}
```

### UI Considerations

- How to visualize connections in TUI?
- Dedicated "connect mode" with cursor?
- ASCII art cables?

---

## Phase 5: Audio Backend

**Status:** Future

Integrate with audio engine for actual sound output.

### Options

1. **SuperCollider** - OSC control, powerful synthesis
2. **cpal + fundsp** - Pure Rust, simpler
3. **Jack** - Pro audio, Linux-focused

### Architecture

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   TUI App   │────▶│   Effects   │────▶│ Audio Engine│
│  (ratatui)  │     │   Queue     │     │ (SC/cpal)   │
└─────────────┘     └─────────────┘     └─────────────┘
```

The `Effect` enum already has:
- `CreateSynth { module_id }`
- `FreeSynth { module_id }`
- `SetParam { module_id, param, value }`

---

## Phase 6: Persistence

**Status:** Future

Save and load rack configurations.

### Format

JSON or RON for human-readable configs:

```json
{
  "modules": [
    {"id": 1, "type": "SawOsc", "name": "saw-1", "params": {...}},
    {"id": 2, "type": "Lpf", "name": "lpf-1", "params": {...}}
  ],
  "connections": [
    {"from": 1, "to": 2}
  ]
}
```

### Features

- `Ctrl+S` to save
- Auto-save on quit?
- Load from command line arg
- Recent files?

---

## Phase 7: Undo/Redo

**Status:** Future

Command history for undoing changes.

### Architecture

```rust
struct History {
    undo_stack: Vec<Command>,
    redo_stack: Vec<Command>,
}

enum Command {
    AddModule { id: ModuleId, module_type: ModuleType },
    RemoveModule { id: ModuleId, module: Module },
    SetParam { module_id: ModuleId, param: String, old: f32, new: f32 },
    // ...
}
```

Each command knows how to undo itself.

---

## Immediate Priority

**Phase 3.1: Add Module Flow** - Get AddPane actually adding modules to the rack.
