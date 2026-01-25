# Next Steps

Roadmap for tuidaw development.

## Current State

**Phase 4 complete:**
- UI engine with ratatui backend
- State types: `Module`, `ModuleType`, `Param`, `RackState`
- Action/Effect enums for command pattern
- Three panes: Rack, Add, Edit (all with viewport scrolling)
- Pane communication fully wired
- SQLite persistence with normalized schema

---

## Phase 5: Module Connections (Next)

**Status:** Not started

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
- ASCII art cables between modules?
- Tab through connection points?

---

## Phase 6: Audio Backend

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
- `u` to undo
- `Ctrl+R` to redo

---

## Completed Phases

### Phase 1: UI Foundation
- Ratatui backend with Graphics trait abstraction
- Input handling with InputSource trait
- Basic main loop

### Phase 2: State & Views
- Module, ModuleType, Param, RackState types
- Action/Effect enums
- RackPane, AddPane, EditPane

### Phase 3: Pane Communication
- AddModule action: AddPane → RackState
- EditModule/UpdateModuleParams: EditPane ↔ RackState
- Pane downcasting with as_any_mut()

### Phase 4: Persistence
- SQLite database with normalized schema
- Tables: schema_version, session, modules, module_params
- Save with `w` key, load with `o` key
- Default path: `~/.config/tuidaw/rack.tuidaw`
- Round-trip test verifies params survive save/load

---

## Immediate Priority

**Phase 5: Module Connections** - Allow signal routing between modules.
