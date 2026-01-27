# Architecture

## Ownership & Borrow Patterns

### The Central Problem

All application state lives in `RackState`, which is owned by
`RackPane`. But other panes need to read (and sometimes influence)
that state. Rust's borrow checker prevents holding two `&mut`
references to `PaneManager` simultaneously, so you can't get
`&RackPane` and `&mut MixerPane` at the same time.

### The render_with_state Workaround

Panes that need external state implement a public
`render_with_state()` method alongside the trait's `render()`:

```rust
// In MixerPane
impl Pane for MixerPane {
    fn render(&self, g: &mut dyn Graphics) {
        // Fallback: renders "use render_with_state" message
    }
}

impl MixerPane {
    pub fn render_with_state(&self, g: &mut dyn Graphics, rack: &RackState) {
        // Actual rendering using rack.mixer
    }
}
```

In `main.rs`, the render section clones the needed state, drops the
borrow, then passes it:

```rust
let active_id = panes.active().id();
if active_id == "mixer" {
    let rack_state = panes.get_pane_mut::<RackPane>("rack").map(|r| r.rack().clone());
    if let Some(rack) = rack_state {
        if let Some(mixer_pane) = panes.get_pane_mut::<MixerPane>("mixer") {
            mixer_pane.render_with_state(&mut frame, &rack);
        }
    }
} else if active_id == "piano_roll" {
    let pr_state = panes.get_pane_mut::<RackPane>("rack").map(|r| r.rack().piano_roll.clone());
    if let Some(pr) = pr_state {
        if let Some(pr_pane) = panes.get_pane_mut::<PianoRollPane>("piano_roll") {
            pr_pane.render_with_state(&mut frame, &pr);
        }
    }
} else {
    panes.render(&mut frame);
}
```

**Cost:** Cloning state every frame. `RackState` derives `Clone` (and
so do `MixerState`, `PianoRollState`). For current data sizes this is
negligible, but could become a concern with large note data.

**Alternative considered:** Moving state out of `RackPane` into a
separate `AppState` passed to all panes. This would be cleaner but
requires refactoring the `Pane` trait to accept state, which touches
every pane.

### The Action Dispatch Borrow Pattern

Same problem arises when handling actions that need data from one pane
to act on another:

```rust
// WRONG: two simultaneous mutable borrows
let pitch = panes.get_pane_mut::<PianoRollPane>("piano_roll").unwrap().cursor_pitch();
panes.get_pane_mut::<RackPane>("rack").unwrap().rack_mut().piano_roll.toggle_note(...);

// RIGHT: extract data first, then act
if let Some(pr_pane) = panes.get_pane_mut::<PianoRollPane>("piano_roll") {
    let pitch = pr_pane.cursor_pitch();
    let tick = pr_pane.cursor_tick();
    // pr_pane borrow dropped here (shadowed by next get_pane_mut)
    if let Some(rack_pane) = panes.get_pane_mut::<RackPane>("rack") {
        rack_pane.rack_mut().piano_roll.toggle_note(track, pitch, tick, dur, vel);
    }
}
```

The key insight: each `get_pane_mut` call takes `&mut self` on
`PaneManager`. You must let the first borrow go out of scope (or
shadow it) before taking a second one.

## Persistence

### Current State

Persistence uses SQLite via the `rusqlite` crate. Files have the
`.tuidaw` extension.

**What's persisted** (in `RackState::save()` / `load()`):
- Module list (id, type, name, position)
- Module parameters (name, value, min, max, type)
- Connections (src module/port → dst module/port)
- Session metadata (next_module_id)

**What's NOT persisted** (marked `#[serde(skip)]`):
- `MixerState` — channel levels, mute, solo, bus config, sends
- `PianoRollState` — tracks, notes, BPM, time signature, transport
- `selected` — UI cursor position in rack

### Schema vs Reality

The `docs/sqlite-persistence.md` design doc describes an ambitious
schema with tables for mixer channels, buses, sequencer tracks, steps,
musical settings, click settings, UI state, and undo history. The
actual implementation is much simpler — only `session`, `modules`,
`module_params`, and `connections` tables exist.

Tables defined in the doc but **not yet implemented**:
- `mixer_channels` / `mixer_buses` / `mixer_master`
- `tracks` / `steps` (sequencer — now piano roll)
- `musical_settings` / `click_settings`
- `ui_state`
- `undo_history`
- `presets` / `rack_templates`

The doc also references Java classes (the project was originally Java,
now Rust). Ignore the Java-specific sections.

### Adding Persistence for New State

To persist `MixerState` or `PianoRollState`:

1. Add new tables in the `save()` method's `execute_batch` call
2. Write insertion logic after the existing module/param/connection
   inserts
3. Add loading logic in `load()` after existing module loading
4. Remove the `#[serde(skip)]` annotation (or keep it — serde isn't
   used for SQLite, it's a leftover from when JSON was the format)

The `#[serde(skip)]` annotations on `mixer` and `piano_roll` in
`RackState` exist because `RackState` derives
`Serialize`/`Deserialize`, but those traits aren't actually used for
persistence anymore (SQLite replaced JSON). The skip just prevents
compile errors from types that don't implement Serialize.

### Piano Roll Persistence (TODO)

The piano roll needs two new tables:

```sql
CREATE TABLE IF NOT EXISTS piano_roll_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    bpm REAL NOT NULL DEFAULT 120.0,
    time_sig_num INTEGER NOT NULL DEFAULT 4,
    time_sig_den INTEGER NOT NULL DEFAULT 4,
    ticks_per_beat INTEGER NOT NULL DEFAULT 480,
    loop_start INTEGER NOT NULL DEFAULT 0,
    loop_end INTEGER NOT NULL DEFAULT 1920
);

CREATE TABLE IF NOT EXISTS piano_roll_notes (
    track_module_id INTEGER NOT NULL,
    tick INTEGER NOT NULL,
    duration INTEGER NOT NULL,
    pitch INTEGER NOT NULL,
    velocity INTEGER NOT NULL,
    PRIMARY KEY (track_module_id, tick, pitch)
);
```

Tracks don't need a separate table — they're derived from MIDI modules
in the rack (auto-assignment in `add_module`).

## Playback Engine

The playback engine lives in the main event loop (not in a separate
thread). Each frame (~16ms at 60fps):

1. Compute elapsed real time since last frame
2. Convert to ticks: `seconds * (bpm / 60) * ticks_per_beat`
3. Advance playhead, handle loop wrapping
4. Scan all tracks for notes starting in the elapsed tick range
5. Send note-on bundles with sub-frame timestamps via OSC
6. Decrement active note durations, send note-off bundles when expired

Notes are sent as OSC bundles with NTP timetags so SuperCollider can
schedule them at sample-accurate times, eliminating frame-rate
jitter. All params for a note-on (freq, vel, gate) are packed into a
single bundle so they arrive atomically.

### Timing Precision

- Frame rate: ~60fps (16ms poll interval)
- Tick resolution: 480 ticks per beat (standard MIDI resolution)
- Sub-frame scheduling: each note's offset within the frame is
  computed and sent as a future timetag
- Worst case timing error: bounded by system clock precision, not
  frame rate

### Polyphony Limitation

Currently, each MIDI module is a single synth node on
SuperCollider. This means each module is monophonic — a new note-on
will retrigger the same synth. True polyphony would require allocating
multiple synth nodes per module and managing voice assignment.

 Each MIDI module in the rack maps to exactly one SuperCollider synth
  node. When you send a note-on, you're setting freq, vel, and gate on
  that single node. If you send a second note-on before the first note
  finishes, it doesn't create a second voice — it just retriggers the
  same synth at the new pitch. The old note is gone.

  This means if you place overlapping notes in the piano roll (e.g., a
  C and an E starting at the same tick on the same track), only one
  will sound. And if a long note is still sustaining when the next
  note starts, it cuts off.

  Real polyphony would require a voice allocator — when a note-on
  arrives, allocate a new synth node (or reuse a freed one), and when
  note-off arrives, free that specific node. Something like:

  Note C4 on → spawn synth node 1001 (C4) Note E4 on → spawn synth
  node 1002 (E4) Note C4 off → free node 1001 Note G4 on → spawn synth
  node 1003 (G4) (or reuse 1001) Note E4 off → free node 1002

  In SuperCollider terms, each note-on would be an /s_new (create
  synth) and each note-off would be either /n_set gate 0 (letting the
  envelope release) or /n_free (immediate kill). The engine would need
  to track a pool of active nodes per MIDI module rather than one
  static node.

  The scope of the change:
  - AudioEngine needs a voice_map: HashMap<(ModuleId, u8), Vec<i32>>
    mapping (module, pitch) to active node IDs
  - Note-on: /s_new with the module's synthdef, into the correct
    group, with the right bus assignments
  - Note-off: /n_set gate 0 on the matching node, then free after
    release
  - Voice limit: cap at N voices per module (e.g., 16) and steal the
    oldest if exceeded
  - rebuild_routing() currently creates one node per module — it would
  need to handle the initial node differently (or not pre-create MIDI
  module nodes at all, letting the piano roll spawn them on demand)

  It's a meaningful refactor of the audio engine but doesn't touch the
  UI or state layer at all.
