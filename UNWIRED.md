# Unwired Code Inventory

Audit of dead code, unused fields/methods, and unwired functionality.

## 1. Compiler Warnings

| Item | Location | Description | Complexity |
|------|----------|-------------|------------|
| `SessionState.midi_recording` | `state/session.rs:63` | Field is never read. Initialized but nothing accesses it. | Medium |

## 2. Entire Modules Behind `#![allow(dead_code)]`

### `state/midi_recording.rs` — Hard
Full module (`MidiRecordingState`, `RecordMode`, `MidiCcMapping`, `PitchBendConfig`, CC constants). None of these are used externally except construction in `SessionState::new()`. Requires MIDI input subsystem, a MIDI mapping pane, and integration with automation and audio engine.

### `state/automation.rs` — Medium
`AutomationState`, `AutomationLane`, `AutomationPoint`, `AutomationTarget`, `CurveType`. The `AutomationTarget` type IS used in `engine.rs` (`apply_automation`) and state is persisted. Missing: automation editing pane and playback tick loop integration.

### `state/sampler.rs` (partial) — Easy
`SamplerConfig` and `BufferId` are actively used. `SampleRegistry` and `SampleBuffer` are completely unused outside tests. Could be removed or integrated as a global sample manager.

### `state/music.rs` (partial) — Easy
`Key` and `Scale` enums are used. `snap_freq_to_scale()` has zero callers. Intended for future pitch quantization in the audio engine when snap is enabled.

### `state/custom_synthdef.rs` (partial) — Trivial
Most types are used. Unused methods: `CustomSynthDefRegistry::by_name()`, `::remove()`, `::is_empty()`, `::len()`.

## 3. Uncalled Methods

### State methods — Trivial

| Method | Location | Notes |
|--------|----------|-------|
| `AppState::collect_strip_updates()` | `state/mod.rs:86` | Batch mixer updates; engine uses per-strip instead |
| `StripState::selected_strip_mut()` | `state/strip_state.rs:61` | Dispatch indexes by strip ID, not selection index |
| `StripState::strips_with_tracks()` | `state/strip_state.rs:96` | Could be useful for piano roll track listing |
| `OscType::default_params_with_registry()` | `state/strip.rs:157` | Redundant with inline code in `add_strip()` |
| `OscType::is_custom()` | `state/strip.rs:186` | Predicate, zero callers |
| `OscType::custom_id()` | `state/strip.rs:191` | Extractor, zero callers |
| `OscType::all_with_custom()` | `state/strip.rs:205` | Add pane does this manually instead |
| `FilterType::all()` | `state/strip.rs:239` | Enumerator, zero callers |
| `EffectType::all()` | `state/strip.rs:289` | Enumerator, zero callers |
| `LfoShape::all()` | `state/strip.rs:399` | Enumerator, zero callers |
| `LfoTarget::all()` | `state/strip.rs:474` | Enumerator, zero callers |
| `PianoRollState::find_note()` | `state/piano_roll.rs:101` | `toggle_note` does its own inline search |
| `PianoRollState::notes_in_range()` | `state/piano_roll.rs:108` | Playback engine scans notes inline |
| `PianoRollState::beat_to_tick()` | `state/piano_roll.rs:133` | Inverse `tick_to_beat` IS used |

### UI framework methods — Trivial

| Method | Location | Notes |
|--------|----------|-------|
| `Frame::inner_rect()` | `ui/frame.rs:139` | Layout helper, zero callers |
| `PianoKeyboard::deactivate()` | `ui/piano_keyboard.rs:46` | Keyboard uses `handle_escape()` instead |
| `MixerPane::send_target()` | `panes/mixer_pane.rs:105` | Getter, field used internally |
| `StripEditPane::strip_id()` | `panes/strip_edit_pane.rs:95` | Getter, field accessed directly |
| `PaneManager::active_keymap()` | `ui/pane.rs:269` | Could be useful for help overlay |
| `PaneManager::pane_ids()` | `ui/pane.rs:275` | Lists all pane IDs |
| `Keymap::bind_ctrl()` | `ui/keymap.rs:118` | Only used in tests |
| `Keymap::bind_alt()` | `ui/keymap.rs:129` | Never used |
| `Keymap::bind_ctrl_key()` | `ui/keymap.rs:140` | Never used |
| `Style::underline()` | `ui/style.rs:128` | Never used |
| `Rect::right()` | `ui/graphics.rs:34` | Never used |
| `Rect::bottom()` | `ui/graphics.rs:39` | Never used |
| `Graphics::fill_rect()` | `ui/graphics.rs:70` | Implemented but never called |
| `TextInput::with_placeholder()` | `ui/widgets/text_input.rs:29` | Never used |
| `TextInput::with_value()` | `ui/widgets/text_input.rs:35` | Never used |
| `TextInput::is_focused()` | `ui/widgets/text_input.rs:55` | Never used |
| `InputEvent::key()` | `ui/input.rs:33` | Constructor, never used |
| `InputEvent::is_char()` | `ui/input.rs:42` | Never used |
| `Modifiers::none()` | `ui/input.rs:64` | Only in tests |
| `Modifiers::ctrl()` | `ui/input.rs:73` | Only in tests |

### Audio engine methods — Trivial/Easy

| Method | Location | Notes |
|--------|----------|-------|
| `AudioEngine::free_sample()` | `audio/engine.rs:1180` | Should be called when sample is unloaded |
| `AudioEngine::get_sc_bufnum()` | `audio/engine.rs:1191` | Getter, zero callers |
| `AudioEngine::is_buffer_loaded()` | `audio/engine.rs:1197` | Predicate, zero callers |
| `ModuleId` type alias | `audio/engine.rs:14` | Duplicate; `bus_allocator.rs` has its own |
| `OscClient::create_synth()` | `audio/osc_client.rs:140` | `create_synth_in_group()` used instead |
| `OscClient::alloc_buffer()` | `audio/osc_client.rs:230` | Zero callers |
| `OscClient::query_buffer()` | `audio/osc_client.rs:247` | Zero callers |
| `osc_time_immediate()` | `audio/osc_client.rs:268` | Zero callers |
| `BusAllocator::get_control_bus()` | `audio/bus_allocator.rs:69` | Only in tests |
| `BusAllocator::free_module_buses()` | `audio/bus_allocator.rs:75` | Only in tests; should be called on strip removal |

### Unused color constants — Trivial

`CORAL`, `MIDI_COLOR`, `LFO_COLOR`, `OUTPUT_COLOR`, `AUDIO_PORT`, `CONTROL_PORT`, `GATE_PORT` in `ui/style.rs`.

## 4. Action Variants Never Returned by Any Pane — Easy

All of these exist in the `Action` enum and have handler arms in `dispatch.rs` (mostly no-op stubs), but no pane ever returns them:

| Variant | Location |
|---------|----------|
| `StripAction::SetParam(StripId, String, f32)` | `ui/pane.rs:40` |
| `StripAction::AddEffect(StripId, EffectType)` | `ui/pane.rs:42` |
| `StripAction::RemoveEffect(StripId, usize)` | `ui/pane.rs:44` |
| `StripAction::MoveEffect(StripId, usize, i8)` | `ui/pane.rs:46` |
| `StripAction::SetFilter(StripId, Option<FilterType>)` | `ui/pane.rs:48` |
| `StripAction::ToggleTrack(StripId)` | `ui/pane.rs:50` |
| `PianoRollAction::MoveCursor(i8, i32)` | `ui/pane.rs:78` |
| `PianoRollAction::SetBpm(f32)` | `ui/pane.rs:87` |
| `PianoRollAction::Zoom(i8)` | `ui/pane.rs:89` |
| `PianoRollAction::ScrollOctave(i8)` | `ui/pane.rs:91` |
| `NavAction::PushPane(&'static str)` | `ui/pane.rs:28` |

## 5. Logic Gaps (Potential Bugs)

### `remove_strip` doesn't clean up automation lanes — Trivial
**Location:** `state/mod.rs:70-73`
`AppState::remove_strip()` removes from strip list and piano roll tracks but does NOT call `self.session.automation.remove_lanes_for_strip(id)`. Orphaned automation lanes will accumulate.

### `MAX_STEPS` constant unused — Trivial
**Location:** `state/drum_sequencer.rs:5`
`pub const MAX_STEPS: usize = 64` is defined but the pattern cycling code uses a hardcoded `[8, 16, 32, 64]` array.

## 6. Stale `#[allow(dead_code)]` Annotations

| Item | Location | Why stale |
|------|----------|-----------|
| `FrameEditPane::set_settings()` | `panes/frame_edit_pane.rs:47` | IS called from `on_enter()` |
| `AudioEngine::load_sample()` | `audio/engine.rs:1160` | IS called from `dispatch.rs:489,829` |
| `AudioEngine::next_bufnum` field | `audio/engine.rs:94` | Used by `load_sample` which is called |
