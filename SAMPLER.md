# Sample Chopper Pane

MPC-style sample chopper for the drum machine. Load a long sample (drum break), visually chop it into slices, assign slices to the 12 drum pads.

## Files to Modify

| File | Change |
|------|--------|
| `Cargo.toml` | Add `hound = "3"` for WAV reading (waveform peaks) |
| `src/state/drum_sequencer.rs` | Add `ChopperState`, add `slice_start`/`slice_end` to `DrumPad` |
| `src/ui/pane.rs` | Add `ChopperAction` enum, `Chopper(ChopperAction)` variant to `Action`, `LoadChopperSample` to `FileSelectAction` |
| `src/panes/sample_chopper_pane.rs` | **New file** — the chopper pane |
| `src/panes/mod.rs` | Register `sample_chopper_pane` module |
| `src/panes/sequencer_pane.rs` | Rebind `c` → open chopper, move "clear pad" to `x` |
| `src/dispatch.rs` | Add `dispatch_chopper()`, update `dispatch_sequencer` for navigation, update `play_drum_hit_to_strip` calls to pass slice params |
| `src/main.rs` | Register `SampleChopperPane` |
| `src/audio/engine.rs` | Update `play_drum_hit_to_strip()` to accept and pass `sliceStart`/`sliceEnd` |

## State Changes

### `DrumPad` — add slice boundaries
```rust
pub struct DrumPad {
    pub buffer_id: Option<BufferId>,
    pub path: Option<String>,
    pub name: String,
    pub level: f32,
    pub slice_start: f32,  // NEW — 0.0-1.0, default 0.0
    pub slice_end: f32,    // NEW — 0.0-1.0, default 1.0
}
```

### `ChopperState` — new struct in `drum_sequencer.rs`
```rust
pub struct ChopperState {
    pub buffer_id: Option<BufferId>,
    pub path: Option<String>,
    pub name: String,
    pub slices: Vec<Slice>,        // reuse from sampler.rs
    pub selected_slice: usize,
    pub next_slice_id: SliceId,
    pub waveform_peaks: Vec<f32>,  // pre-computed for display (~90 values)
    pub duration_secs: f32,
}
```

### `DrumSequencerState` — add chopper field
```rust
pub chopper: Option<ChopperState>,
```

## Action Enum

```rust
pub enum ChopperAction {
    LoadSample,                    // open file browser
    LoadSampleResult(PathBuf),     // file browser callback
    AddSlice,                      // split at cursor position
    RemoveSlice,                   // delete selected slice, merge with neighbor
    AssignToPad(usize),            // assign selected slice to pad 0-11
    AutoSlice(usize),              // replace slices with N equal divisions
    PreviewSlice,                  // audition selected slice
}
```

Add to `Action` enum: `Chopper(ChopperAction)`
Add to `FileSelectAction`: `LoadChopperSample`

## Pane: `SampleChopperPane`

- **ID**: `"sample_chopper"`
- **Navigation**: pushed from sequencer via `c` key (uses `NavAction::PushPane` so Escape pops back)

### Pane State
```rust
pub struct SampleChopperPane {
    keymap: Keymap,
    cursor_pos: f32,      // 0.0-1.0 position on waveform
    auto_slice_n: usize,  // cycles through 4, 8, 12, 16
}
```
Slice selection and waveform data live on `ChopperState` (in app state), not the pane.

### Layout (97×29 box)
```
┌──────────────────── Sample Chopper ────────────────────────────────────────────────────────┐
│ break_160bpm.wav                                                        4.2s   8 slices   │
│                                                                                            │
│ ▃▅▇█▆▃▁▃▅▇█▅▃▁▂▄▆█▇▅▃▁▃▅▇█▆▃▁▃▅▇█▅▃▁▂▄▆█▇▅▃▁▃▅▇█▆▃▁▃▅▇█▅▃▁▂▄▆█▇▅▃▁▃▅▇█▆▃▁▃▅▇█▅▃▁▂▃ │
│ |   1   |   2   |   3   |   4   |   5   |   6   |   7   |   8   |                        │
│      ▲ cursor                                                                              │
│                                                                                            │
│  > 1  0.000-0.125  → Pad 1                                                                │
│    2  0.125-0.250  → Pad 2                                                                 │
│    3  0.250-0.375  → ----                                                                  │
│    4  0.375-0.500  → ----                                                                  │
│    5  0.500-0.625  → Pad 5                                                                 │
│    6  0.625-0.750  → ----                                                                  │
│    7  0.750-0.875  → ----                                                                  │
│    8  0.875-1.000  → ----                                                                  │
│                                                                                            │
│                                                                                            │
│ s:load  Enter:chop  x:del  1-9,0,-,=:assign  n:auto(8)  Space:preview  Esc:back           │
└────────────────────────────────────────────────────────────────────────────────────────────┘
```

### Key Bindings
| Key | Action | Description |
|-----|--------|-------------|
| h/Left | move_left | Move cursor left along waveform |
| l/Right | move_right | Move cursor right along waveform |
| j/Down | next_slice | Select next slice |
| k/Up | prev_slice | Select previous slice |
| Shift+Left | nudge_start | Fine-adjust selected slice start (−0.005) |
| Shift+Right | nudge_end | Fine-adjust selected slice end (+0.005) |
| Enter | chop | Split slice at cursor position |
| x | delete | Remove selected slice (merge with next) |
| 1-9, 0, -, = | assign | Assign selected slice to pad 1-12 |
| n | auto_slice | Auto-slice into N equal parts (cycles 4→8→12→16) |
| s | load | Load sample via file browser |
| Space | preview | Audition selected slice |
| Escape | back | Return to sequencer (pop pane) |

## Sequencer Pane Changes

In `sequencer_pane.rs`:
- Change `.bind('c', "clear_pad", ...)` to `.bind('x', "clear_pad", "Clear pad steps")`
- Add `.bind('c', "chopper", "Sample chopper")`
- In `handle_input`, match `"chopper"` → return `Action::Nav(NavAction::PushPane("sample_chopper"))`
- Update help line to reflect new bindings

## Dispatch Changes

### `dispatch_chopper()` in `dispatch.rs`
- **LoadSample**: configure file browser with `FileSelectAction::LoadChopperSample`, push file browser pane
- **LoadSampleResult(path)**: read WAV with `hound`, compute peaks, create `ChopperState` with one full slice, load buffer via `audio_engine.load_sample()`, store on `DrumSequencerState.chopper`
- **AddSlice**: find which slice the cursor is within, split it at cursor_pos into two slices
- **RemoveSlice**: remove selected slice, extend the previous slice's end to cover the gap
- **AssignToPad(idx)**: copy chopper's buffer_id + selected slice's start/end to `DrumPad[idx]`, set pad name from slice name, load buffer into engine if not already loaded
- **AutoSlice(n)**: clear existing slices, create N equal slices spanning 0.0-1.0
- **PreviewSlice**: call `audio_engine.play_drum_hit_to_strip()` with selected slice's start/end and the chopper's buffer_id, routed through the current drum machine strip

### Update file browser callback
Add `FileSelectAction::LoadChopperSample` handling in the file browser's select action → returns `Action::Chopper(ChopperAction::LoadSampleResult(path))`

### Update `play_drum_hit_to_strip`
Wherever drum pad hits are triggered (sequencer playback, pad keyboard), pass `pad.slice_start` and `pad.slice_end` to the engine method.

## Audio Engine Changes

### `play_drum_hit_to_strip()` signature
```rust
pub fn play_drum_hit_to_strip(
    &mut self,
    buffer_id: BufferId,
    amp: f32,
    strip_id: StripId,
    slice_start: f32,  // NEW
    slice_end: f32,     // NEW
) -> Result<(), String>
```

Pass `sliceStart` and `sliceEnd` as params to the `tuidaw_sampler_oneshot` synth (already supported by the SynthDef).

## Waveform Peak Computation

Using `hound` crate (WAV-only, lightweight):
```rust
fn compute_waveform_peaks(path: &str, num_columns: usize) -> Vec<f32> {
    let reader = hound::WavReader::open(path).unwrap();
    let samples: Vec<f32> = reader.into_samples::<f32>().filter_map(Result::ok).collect();
    let chunk_size = samples.len() / num_columns;
    (0..num_columns)
        .map(|i| {
            let start = i * chunk_size;
            let end = (start + chunk_size).min(samples.len());
            samples[start..end].iter().map(|s| s.abs()).fold(0.0f32, f32::max)
        })
        .collect()
}
```

Render using Unicode block elements: `▁▂▃▄▅▆▇█` mapped to peak amplitude 0.0-1.0.

## Verification

1. `cargo build` — compiles without errors
2. `cargo test --bin tuidaw` — existing tests pass
3. Manual test flow:
   - Create a drum machine instrument (key 1, then add)
   - Press 2 to open sequencer
   - Press `c` to open sample chopper
   - Press `s` to load a WAV file
   - Verify waveform displays
   - Press `n` to auto-slice into 8 parts
   - Press `1` through `8` to assign slices to pads 1-8
   - Press Escape to return to sequencer
   - Press Enter on steps to sequence the pads
   - Press Space to play — verify sliced playback
   - Verify `x` clears pad steps (moved from `c`)
