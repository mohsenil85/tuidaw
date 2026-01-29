# Refactor Plan: Audio Engine & UI Engine

Deep dive analysis of the tuidaw codebase after multiple
iterations. Documents active bugs, iteration artifacts, and
architectural recommendations for both the audio engine and UI layer.

---

## Part 1: Active Bugs

These are things that are broken right now and should be fixed before
any structural refactoring.

### ~~1.1 Automation node index calculation ignores LFO nodes~~ FIXED

Originally quick-fixed by adding `strip.lfo.enabled` checks to
positional index calculations. Now properly fixed by the `StripNodes`
refactor (Part 3) -- `apply_automation()` uses named fields
(`nodes.filter`, `nodes.output`, `nodes.effects`) instead of
positional indexing entirely.

---

### ~~1.2 Filter resonance parameter name mismatch~~ FIXED

Changed `"res"` to `"resonance"` in the FilterResonance automation
handler to match the synthdef parameter name.

---

### ~~1.3 Sampler automation is a no-op~~ FIXED

Added `source_node: i32` and `spawn_time: Instant` to `VoiceChain`.
Both `spawn_voice()` and `spawn_sampler_voice()` now store the source
node ID. `SamplerRate` and `SamplerAmp` automation handlers use
`voice.source_node` to send `/n_set` to the correct synth node.
Voice-steal logic also updated to use `spawn_time` for proper
oldest-voice ordering instead of first-match removal.

---

### ~~1.4 `SetStripParam` action is a stub~~ FIXED

Implemented the dispatch handler to update `source_params` in state and
call `AudioEngine::set_source_param()`. The new engine method sets the
param on the persistent source node (AudioIn strips) and all active
voice source nodes (oscillator/sampler strips). No graph rebuild needed.

---

## Part 2: Iteration Artifacts

Code that works but is leftover from previous design iterations,
causing confusion or maintenance burden.

### ~~2.1 Naming mismatch between docs and code~~ FIXED

Rewrote CLAUDE.md, `docs/architecture.md`, and
`docs/ai-coding-affordances.md` to use current Strip-based naming
throughout. Removed all references to `RackState`, `RackPane`,
`ModuleId`, `render_with_state()`, and the rack/module/connection
metaphor.

### ~~2.2 `rebuild_routing()` backward-compat alias~~ FIXED

Removed the dead `rebuild_routing()` wrapper method from
`AudioEngine`.

### ~~2.3 Unused `_polyphonic` parameter~~ FIXED

Removed the `_polyphonic: bool` parameter from `spawn_voice()` and
cleaned up all call sites in `dispatch.rs` and `playback.rs`,
including the tuple types and variable extractions that carried the
unused value.

### ~~2.4 Global dead code suppression~~ FIXED

Removed the global `#![allow(dead_code)]` from `main.rs`. Removed 4
truly dead items (`piano_mode()`, `is_piano_mode()`, `selectable_count()`,
`ensure_visible()`). Added module-level `#![allow(dead_code)]` to 7
files that are entirely planned API (midi, automation, sampler,
midi_recording, custom_synthdef, music, select_list). Added ~50
targeted `#[allow(dead_code)]` annotations on intentional API surface
across 15 files (audio engine buffer methods, keymap bind variants,
color constants, action enum variants, pane accessors, etc.). Result:
`cargo check` produces zero warnings.

### ~~2.5 Piano keyboard mapping duplicated 3 times~~ FIXED

Extracted `PianoKeyboard` struct into `src/ui/piano_keyboard.rs` with
all shared state (`active`, `octave`, `layout`) and methods
(`key_to_pitch`, `handle_escape`, `octave_up/down`, `status_label`,
`activate/deactivate`). All three panes (`StripPane`, `PianoRollPane`,
`StripEditPane`) now hold a `PianoKeyboard` field and delegate to it.
Removed ~200 lines of duplicated code.

### ~~2.6 `PushPane`/`PopPane` actions defined but not implemented~~ FIXED

Implemented proper pane stack in `PaneManager` with `stack: Vec<usize>`.
`push_to()` saves current index and switches; `pop()` restores from
stack. Help pane and file browser now use push/pop for modal behavior.
See item 14.

### ~~2.7 `SemanticColor` enum defined but never used~~ FIXED

Removed the `SemanticColor` enum and its `impl` block from
`src/ui/style.rs`, and removed the re-export from `src/ui/mod.rs`.

### ~~2.8 `Keymap::merge()` exists but is never called~~ FIXED

Removed the unused `merge()` method from `src/ui/keymap.rs`.

### 2.9 `SequencerPane` is a placeholder → planned as drum sequencer

**File:** `src/panes/sequencer_pane.rs` (54 lines)

Currently has a single "quit" keybinding and renders "Coming soon..."
text. Registered in main.rs and accessible via the `3` key. See Part
4's "Proposed: Drum Sequencer" section for the full design.

### 2.10 `#[serde(skip)]` annotations on fields that aren't serialized

**Noted in:** `docs/architecture.md:139-143`

`StripState` derives `Serialize`/`Deserialize`, but persistence uses
SQLite (not serde). The `#[serde(skip)]` annotations on `mixer` and
`piano_roll` only exist to prevent compile errors from types that
don't implement `Serialize`. The serde derives themselves are unused.

---

## Part 3: Audio Engine Architecture

### Current Architecture

```
AudioEngine
  client: Option<OscClient>           -- UDP socket to scsynth
  node_map: HashMap<StripId, StripNodes> -- strip -> named node slots
  voice_chains: Vec<VoiceChain>        -- active polyphonic voices
  bus_allocator: BusAllocator          -- audio/control bus allocation
  send_node_map: HashMap<(usize, u8), i32>  -- send synth nodes
  bus_node_map: HashMap<u8, i32>       -- bus output synth nodes
  bus_audio_buses: HashMap<u8, i32>    -- mixer bus SC audio buses
```

### ~~Proposed: Structured node map~~ DONE

Replaced `node_map: HashMap<StripId, Vec<i32>>` with `HashMap<StripId,
StripNodes>`:

```rust
pub struct StripNodes {
    pub source: Option<i32>,      // AudioIn synth (None for oscillator strips)
    pub lfo: Option<i32>,         // LFO modulator
    pub filter: Option<i32>,      // Filter synth
    pub effects: Vec<i32>,        // Effect chain, in order (only enabled effects)
    pub output: i32,              // Output/mixer synth (always exists)
}
```

All methods now use named fields instead of positional
indexing. `rebuild_strip_routing()` builds individual `Option<i32>` /
`Vec<i32>` variables during synth creation, then constructs
`StripNodes` at the end. `apply_automation()` uses `nodes.output`,
`nodes.filter`, and `nodes.effects.get(enabled_idx)` directly. The
effect param automation also now correctly counts only enabled effects
before the target index, fixing a latent bug when disabled effects
preceded the target.

### ~~Proposed: Richer voice tracking~~ DONE

Added `source_node: i32` and `spawn_time: Instant` to `VoiceChain`.
`source_node` stores the oscillator or sampler synth node ID, enabling
direct `/n_set` calls for sampler automation. `spawn_time` enables
proper oldest-voice stealing via `min_by_key` instead of first-match
`position()`. Both `spawn_voice()` and `spawn_sampler_voice()` updated.
Field name `midi_node_id` kept as-is (the proposed rename to `midi_node`
was purely cosmetic).

### ~~Proposed: Configurable release cleanup~~ DONE

Replaced the hardcoded 5-second group free with envelope-aware cleanup.
`release_voice()` now takes `&StripState`, looks up the strip's
`amp_envelope.release` time, and schedules cleanup at `offset_secs +
release_time + 1.0`. The +1.0 second margin accounts for
SuperCollider's envelope grain.

### ~~Proposed: Mixer bus allocation through BusAllocator~~ DONE

Replaced hardcoded `bus_audio_base = 200` with
`bus_allocator.get_or_alloc_audio_bus()` calls using sentinel StripIds
(`u32::MAX - bus_id`). Mixer buses now share the allocator's address
space with strip buses, preventing collisions regardless of strip
count.

### ~~Proposed: Stop rebuilding the full graph for mixer changes~~ DONE

Added `update_all_strip_mixer_params(&self, state: &StripState)` which
iterates all strips and sets level/mute/pan on each strip's
`nodes.output` via OSC, without tearing down the graph. Replaced 4
`rebuild_strip_routing()` calls with this method:

- `dispatch.rs` MixerAdjustLevel handler
- `dispatch.rs` MixerToggleMute handler
- `dispatch.rs` MixerToggleSolo handler
- `main.rs` master mute toggle (`.` key)

Updates all strips in each call because master level/mute/solo affect
effective values across all strips. Topology-changing operations
(AddStrip, DeleteStrip, UpdateStrip, ConnectServer, MixerToggleSend)
still use the full `rebuild_strip_routing()`.

---

## Part 4: UI Engine Architecture

### Current Architecture

```
main.rs event loop
  AppState (owned by main.rs)
    strip: StripState
    audio_in_waveform: Option<Vec<f32>>

  PaneManager
    panes: Vec<Box<dyn Pane>>
    active_index: usize

  Pane trait:
    handle_input(&mut self, event, &AppState) -> Action
    render(&self, g, &AppState)

  dispatch::dispatch_action() mutates AppState, configures panes, calls AudioEngine
```

State is owned by `main.rs` and passed to all panes by reference. No
cloning, no `render_with_state()` workaround. Action dispatch lives
in `src/dispatch.rs`.

### ~~Proposed: Extract state from panes~~ DONE

Moved `StripState` out of `StripPane` into a top-level `AppState`
owned by `main.rs`. The `Pane` trait now passes `&AppState` to both
`handle_input()` and `render()`. Eliminated frame-by-frame cloning,
all `render_with_state()` variants, and the special-case render block
in main.rs. Action dispatch moved to `src/dispatch.rs` which operates
directly on `&mut AppState`.

Note: `as_any_mut()` is still on the Pane trait because
`dispatch_action()` uses `PaneManager::get_pane_mut::<T>()` to
configure target panes (e.g., setting strip data on `StripEditPane`
before switching to it).

### ~~Proposed: Split the Action enum~~ DONE

Split the flat 50+ variant `Action` enum into domain-specific sub-enums:
`NavAction`, `StripAction`, `MixerAction`, `PianoRollAction`,
`ServerAction`, `SessionAction`. The main `Action` enum now wraps these
via `Action::Nav(NavAction::...)`, `Action::Strip(StripAction::...)`,
etc. `dispatch.rs` restructured into domain dispatch functions
(`dispatch_strip`, `dispatch_mixer`, etc.). All 11 pane files updated.

### ~~Proposed: Extract piano keyboard utility~~ DONE

Created `src/ui/piano_keyboard.rs` with `PianoKeyboard` struct and
`PianoLayout` enum. See item 2.5 for details.

### ~~Proposed: Implement proper pane stack~~ DONE

Added `stack: Vec<usize>` to `PaneManager`. `push_to()` saves current
`active_index` onto the stack and switches; `pop()` restores from the
stack with proper `on_exit`/`on_enter` lifecycle calls. `switch_to()`
clears the stack (clean navigation resets modal state). Help pane (`?`
key) and file browser now use push/pop. See item 2.6.

### Proposed: Drum Sequencer

Replace the placeholder `SequencerPane` with a 16-step drum sequencer.
Old school hip hop machine vibes — MPC/SP-1200 workflow in the terminal.

**Core features:**

- **16-step grid, 12 pads** — each row is a pad, each column is a step.
  Toggle steps on/off with Enter. Cursor navigates the grid.
- **Sample-per-pad** — each pad loads a WAV or AIFF sample via the
  existing file browser (`PushPane("file_browser")`). Pads display the
  sample filename (truncated).
- **Transport** — play/stop the pattern. Syncs to the global BPM from
  `SessionState`. Playhead column highlights during playback.
- **Velocity per step** — Up/Down adjusts velocity of the step under
  cursor (displayed as brightness or numeric).
- **Pattern length** — default 16 steps, adjustable (8, 16, 32, 64).

**Sample chopper mode:**

- Load a longer sample, display its waveform (reuse `WAVEFORM_CHARS`
  from piano roll).
- Navigate with cursor, set start/end markers to define slices.
- Assign slices to pads — each pad plays a portion of the source sample.
- Standard audio formats: WAV, AIFF (via existing audio engine sample
  loading).

**Builds on existing infrastructure:**

- `SampleRegistry`, `BufferId`, `SamplerConfig` in `state/sampler.rs`
- File browser pane (push/pop)
- `AudioEngine::spawn_sampler_voice()` for playback
- `BusAllocator` for audio routing
- Piano roll's tick-based timing model

**State additions:**

- `DrumPattern` (steps per pad, velocities, pad→sample mapping)
- `DrumSequencerState` in `StripState` (current pattern, pad configs)
- Persistence via SQLite (new tables for patterns and pad configs)

**New actions (in a future `SequencerAction` sub-enum):**

- `ToggleStep`, `SetVelocity`, `SelectPad`, `LoadSample`,
  `SetPatternLength`, `PlayStop`, `EnterChopperMode`, `SetSlice`,
  `AssignSliceToPad`

---

## Part 5: Priority Order

### ~~Immediate fixes (bugs)~~ DONE

1. ~~**Fix automation node indexing**~~ -- quick-fixed by adding LFO
   awareness to index calculations
2. ~~**Fix `"res"` -> `"resonance"`**~~ -- corrected param name
3. ~~**Remove `rebuild_routing()` alias**~~ -- removed
4. ~~**Remove `_polyphonic` parameter**~~ -- removed from signature
   and all call sites

### ~~Short-term (audible improvements)~~ DONE

5. ~~**Structured `StripNodes` map**~~ -- replaced `HashMap<StripId,
   Vec<i32>>` with `HashMap<StripId, StripNodes>` using named fields;
   eliminated all positional index calculations
6. ~~**Stop rebuilding the full graph on mixer changes**~~ -- added
   `update_all_strip_mixer_params()` for level/mute/solo/pan; 4
   rebuild calls replaced

### ~~Short-term (remaining audible improvements)~~ DONE

7. ~~**Configurable release cleanup**~~ -- replaced hardcoded 5-second
   group free with `strip.amp_envelope.release + 1.0s` margin;
   `release_voice()` now takes `&StripState` parameter; `playback.rs`
   restructured to hoist state clone for both note-on and note-off
   blocks
8. ~~**Route mixer buses through BusAllocator**~~ -- replaced hardcoded
   `bus_audio_base = 200` formula with `bus_allocator.get_or_alloc_audio_bus()`
   calls using sentinel StripIds (`u32::MAX - bus_id`); mixer buses now
   share the allocator's address space with strip buses, preventing
   collisions

### ~~Medium-term (structural)~~ MOSTLY DONE

9. ~~**Extract `AppState` from panes**~~ -- done; `AppState` owned by
   `main.rs`, passed to all panes via `&AppState`
10. ~~**Extract piano keyboard utility**~~ -- done; `PianoKeyboard`
    in `src/ui/piano_keyboard.rs`
11. ~~**Add `source_node` to `VoiceChain`**~~ -- done; added
    `source_node` and `spawn_time` fields; sampler automation and
    oldest-voice stealing now work (bug 1.3 fixed)
12. ~~**Implement `SetStripParam` action**~~ -- done; dispatch
    handler updates state and calls `set_source_param()` on audio
    engine; works for persistent and per-voice source nodes (bug 1.4
    fixed)

### ~~Longer-term (cleanup)~~ MOSTLY DONE

13. ~~**Split Action enum** into sub-enums~~ -- done; 6 domain
    sub-enums (`NavAction`, `StripAction`, `MixerAction`,
    `PianoRollAction`, `ServerAction`, `SessionAction`); dispatch
    restructured into domain functions
14. ~~**Implement pane stack** for proper modals (relates to 2.6)~~ --
    done; `stack: Vec<usize>` in `PaneManager`; help pane and file
    browser converted to push/pop
15. ~~**Remove `#![allow(dead_code)]`**~~ -- done; removed global
    suppression from `main.rs`; 4 dead items removed; ~50 targeted
    `#[allow(dead_code)]` annotations on intentional API surface;
    7 planned-API modules given module-level allows; zero warnings
16. ~~**Update CLAUDE.md**~~ -- done; complete rewrite of CLAUDE.md,
    architecture.md, and ai-coding-affordances.md
17. ~~**Remove unused `SemanticColor`**, `Keymap::merge()`~~ -- done;
    serde derives cleanup still outstanding (2.10)
18. **Implement drum sequencer** (see Part 4 design, replaces placeholder SequencerPane)
