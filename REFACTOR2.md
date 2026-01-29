# Refactor Plan: Inbox Items

Triaged from `inbox.txt`. Each item is analyzed against the current codebase
and given a concrete task description. Items marked √ in the original inbox
are noted as done. Format follows REFACTOR1.md.

---

## 1. Command-line argument parsing

**Inbox:** `bin/wrapper script, command line handling`

**Status:** Not started

**What this means:** The binary currently launches directly into the TUI
with no CLI argument handling. `main()` in `src/main.rs` calls `run()`
immediately with no argument parsing. There's no `--help`, `--version`,
`--file <path>`, or any other command-line interface.

**Proposed:**

Add `clap` as a dependency and parse CLI arguments before entering the
event loop. Minimum viable flags:

```
tuidaw                          # launch with default session
tuidaw <file.tuidaw>            # open specific session file
tuidaw --new                    # start fresh (no auto-load)
tuidaw --help                   # usage info
tuidaw --version                # version string
```

Implementation:
1. Add `clap` to `Cargo.toml`
2. Define a `Cli` struct with derive macros in `main.rs` (or a new
   `src/cli.rs`)
3. Parse args before `run()`, pass relevant config (file path, flags)
   into the event loop
4. Wire `--file` into `StripState::load()` path override

---

## 2. Global shortcuts: save, load, export, import

**Inbox:** `global shortcuts, save, load, export import.`

**Status:** Partially done

**What this means:** Some global shortcuts exist (`Ctrl-S` save, `Ctrl-L`
load in `main.rs:56-72`), but export/import functionality doesn't exist
yet. There's no UI flow for "Save As", "Open File", or
exporting/importing individual strips or effects.

**Proposed:**

1. **Save As (`Ctrl-Shift-S`):** Open file browser in save mode, write
   session to chosen path
2. **Open (`Ctrl-O`):** Open file browser to pick a `.tuidaw`/`.sqlite`
   file, load it
3. **Export strip:** Select a strip, export its config (source, filter,
   effects, envelope, LFO) to a portable format (JSON or SQLite subset)
4. **Import strip:** Browse for exported strip file, add it to the
   current session

This overlaps with item 14 (exporting/importing effects, instruments).
Implement the file browser save-mode first, then build export/import on
top of it.

**Files:** `src/main.rs` (global key handlers), `src/panes/file_browser_pane.rs`
(needs save-mode), `src/state/persistence.rs` (export/import methods),
`src/ui/pane.rs` (new Action variants)

---

## 3. Database migrations

**Inbox:** `migrations`

**Status:** Not started

**What this means:** The SQLite persistence layer (`src/state/persistence.rs`)
creates tables with `CREATE TABLE IF NOT EXISTS` but has no migration
system. When the schema changes (new columns, renamed tables, new
tables), old `.tuidaw` files become incompatible or silently lose data.

**Proposed:**

Add a schema versioning and migration system:

1. Add a `schema_version` table (single row, integer version)
2. On load, check version and run migrations sequentially
3. Each migration is a function: `fn migrate_v1_to_v2(conn: &Connection)`
4. Keep migrations in `src/state/migrations.rs` or inline in
   `persistence.rs`
5. Bump version on every schema-changing release

This is important before any persistence format changes (like adding
export/import or new state fields) to avoid breaking existing user sessions.

**Files:** `src/state/persistence.rs`, new `src/state/migrations.rs`

---

## 4. Broken Frame settings screen

**Inbox:** `broken Frame settings screen`

**Status:** Bug — not started

**What this means:** The `FrameEditPane` (`src/panes/frame_edit_pane.rs`)
has several issues:

1. **Escape goes to `"rack"`** (line 214) instead of returning to the
   previous pane. This is a hardcoded `SwitchPane("rack")` that should
   use `PopPane` or navigate back to wherever the user came from.
2. **Confirm behavior is inconsistent:** Enter on BPM/Tuning enters text
   edit mode, but Enter on Key/Scale/TimeSig/Snap immediately fires
   `UpdateSession` and presumably should return to the previous pane.
   The user has to press Escape to leave after editing non-numeric
   fields.
3. **No visible "save and return" flow:** After adjusting values with
   Left/Right, there's no obvious way to commit changes except pressing
   Enter on a non-numeric field. The session state changes made via
   Left/Right are stored locally but may not propagate if the user
   presses Escape.

**Proposed:**

1. Track the originating pane and return to it on Escape/confirm
   (or use the pane stack from REFACTOR1.md item 2.6)
2. Make Enter always commit the current session state and return
3. Make Escape discard uncommitted changes and return
4. Add Left/Right changes to auto-commit (live preview) or show a
   "modified" indicator

**Files:** `src/panes/frame_edit_pane.rs`, `src/ui/pane.rs` (PopPane
implementation)

---

## ~~5. "Jump back"~~ DONE

**Inbox:** `√"jump back"`

Already implemented. Backtick/tilde navigate back/forward through pane
history via `Frame.back_view` / `Frame.forward_view` in `main.rs`.

---

## 6. Move console log to Server pane

**Inbox:** `console at bottom not updating, move that to server screen`

**Status:** Not started

**What this means:** The `Frame` struct (`src/ui/frame.rs`) maintains a
`messages: VecDeque<String>` console that renders the last few lines at
the bottom of every screen. It's reportedly not updating properly, and
the user wants all logging moved to the Server pane instead.

**Proposed:**

1. **Remove the bottom console rendering** from `Frame::render()` /
   `main.rs` render block — no more global 4-line log at the bottom
   of every pane
2. **Add a scrollable log viewer** to `ServerPane`
   (`src/panes/server_pane.rs`) that displays all `Frame.messages`
3. Keep `Frame::push_message()` API intact so all subsystems can still
   log; the messages just won't render globally
4. Reclaim the 4 lines of screen real estate for pane content (adjust
   `box_height` constants in panes that account for the console area)
5. The master meter at bottom-right can remain as a minimal status
   indicator

**Files:** `src/ui/frame.rs` (remove console render), `src/main.rs`
(adjust render layout), `src/panes/server_pane.rs` (add log viewer),
possibly all panes (adjust height constants)

---

## 7. Keybinding consistency

**Inbox:** `consistency in keybindings (eg a= add everywhere) (add shortcuts)`

**Status:** Not started

**What this means:** Keybindings are inconsistent across panes. For
example, `a` means "add strip" in StripPane but may do nothing or
something different in other panes. Common actions like add, delete,
edit, navigate should use the same keys everywhere they apply.

**Proposed:**

1. Audit all pane keymaps and document current bindings in a matrix
2. Establish a consistent binding convention:
   - `a` = add/create (new strip, new note, new effect, etc.)
   - `d` = delete (strip, note, effect)
   - `e` or `Enter` = edit/open detail view
   - `j`/`k` or Up/Down = navigate list
   - `/` = piano keyboard mode (where applicable)
   - `?` = help (already global)
   - `Escape` = back/cancel
3. Add missing shortcuts where they make sense (e.g., `a` in MixerPane
   to add a strip)
4. Update `docs/keybindings.md` with the unified convention

**Files:** All panes in `src/panes/`, `docs/keybindings.md`

---

## ~~8. OSC screen, all on one screen~~ DONE

**Inbox:** `√osc screen, all on one screen`

Already implemented. Strip editing consolidated into a single
`StripEditPane`.

---

## 9. Custom synths and VSTs

**Inbox:** `custom synths, vst's`

**Status:** Partially started (custom synthdefs exist)

**What this means:** Custom SynthDef import is already implemented
(`src/state/custom_synthdef.rs`, `src/scd_parser.rs`,
`src/panes/file_browser_pane.rs` for `.scd` import). The inbox item
likely refers to:

1. **Expanding custom synthdef support:** Better UI for managing custom
   synths, parameter discovery, preset saving
2. **VST/plugin hosting:** This would be a major feature — hosting VST2/VST3
   plugins inside the DAW. This requires a plugin host library (e.g.,
   `vst-rs` or `clap-host`) and significant audio architecture changes
   since the current engine runs everything through SuperCollider OSC.

**Proposed (phased):**

**Phase 1 — Custom synthdef polish:**
- Add a "Custom Synths" management screen (list, rename, delete imported
  synthdefs)
- Show discovered parameters with ranges and defaults
- Allow parameter mapping in the strip editor

**Phase 2 — VST support (future):**
- Research `vst-rs` or CLAP plugin hosting
- Would require a local audio processing path alongside the SC OSC path
- This is a large architectural change; document requirements and
  feasibility first before implementing

**Files:** `src/state/custom_synthdef.rs`, `src/panes/strip_edit_pane.rs`,
new pane for synth management

---

## 10. Piano roll fixes

**Inbox:** `Piano roll fixes, remvoe bpm, midi-0 wrong name`

**Status:** Bug — not started

**What this means:** The PianoRollPane (`src/panes/piano_roll_pane.rs`)
has at least two specific issues:

1. **Remove BPM display:** BPM is shown somewhere in the piano roll
   UI but shouldn't be (it belongs in the session/frame settings, not
   cluttering the piano roll). Or, BPM editing controls in the piano
   roll should be removed since they now live in FrameEditPane.
2. **MIDI note 0 wrong name:** The lowest MIDI note (0) is displaying
   an incorrect note name. MIDI note 0 should be C-1 (or C0 depending
   on convention). The `pitch_to_name()` or equivalent function likely
   has an off-by-one or formatting error for the boundary case.

**Proposed:**

1. Find and remove BPM display/controls from the piano roll pane
2. Fix the note name lookup for MIDI note 0 — check `pitch_to_name()`
   or the note label rendering in the piano roll's vertical axis
3. Audit other boundary MIDI notes (127, etc.) for similar issues

**Files:** `src/panes/piano_roll_pane.rs`, possibly `src/state/music.rs`
(if note naming lives there)

---

## 11. Remove help text along the bottom

**Inbox:** `remove helps along bottom every where`

**Status:** Not started

**What this means:** Most panes render a hardcoded help line at the
bottom of their box (e.g., FrameEditPane line 277-284 renders
`"Left/Right: adjust | Enter: type/confirm | Esc: cancel"`, HomePane
line 124 renders `"[1-3] Jump  [Enter] Select  [q] Quit"`). This
clutters the UI and takes up space.

**Proposed:**

1. Remove all inline help text from pane `render()` methods
2. The existing `?` key already opens context-sensitive help via
   `HelpPane`, which reads the active pane's keymap — this is the
   proper help system
3. Optionally, add a subtle `? for help` indicator in one corner
   (maybe in the frame chrome) so users know help is available

**Files:** All panes in `src/panes/` (search for help text in `render()`
methods), `src/ui/frame.rs` (optional help indicator)

---

## 12. Fix strip deletion + rename Strips screen

**Inbox:** `delete an osc doesn't work. rename "strips" screen to oscillators`

**Status:** Bug + enhancement — not started

**What this means:** Two issues:

1. **Delete strip broken:** Pressing `d` in StripPane dispatches
   `Action::DeleteStrip(strip_id)` which calls
   `state.strip.remove_strip()` and `audio_engine.rebuild_strip_routing()`.
   Something in this chain is failing — possibly the strip index goes
   stale after deletion, or the audio graph rebuild crashes, or the
   selected index isn't adjusted after removing a strip.
2. **Rename "Strips" to "Oscillators":** The StripPane (pane ID `"strip"`)
   likely shows a title like "Strips" but the user wants it called
   "Oscillators" to better reflect the musical purpose. Note: HomePane
   also references this as "Rack" (line 25: `pane_id: "rack"`) which is
   an outdated name — StripPane's actual ID is `"strip"`.

**Proposed:**

1. Debug the delete flow: add the strip, verify `remove_strip()` removes
   it from state, verify the audio rebuild succeeds, verify the selected
   index is clamped after deletion
2. Rename the StripPane title from "Strips" (or "Rack") to "Oscillators"
   in the render method
3. Update HomePane to show "Oscillators" instead of "Rack" and fix
   `pane_id: "rack"` to `pane_id: "strip"` (or whatever the correct ID is)

**Files:** `src/panes/strip_pane.rs`, `src/dispatch.rs` (DeleteStrip
handler), `src/state/strip_state.rs` (remove_strip), `src/panes/home_pane.rs`

---

## 13. Handle small terminal and resize

**Inbox:** `terminal too small when starting up, handle resize`

**Status:** Not started

**What this means:** The TUI renders with fixed-size boxes (e.g.,
`Rect::centered(width, height, box_width, 29)` with height 29 being
standard). If the terminal is smaller than the expected size on startup,
rendering breaks or panics. Terminal resize events may also not be
handled properly.

**Proposed:**

1. **Minimum size check:** On startup and on resize, check terminal
   dimensions against a minimum (e.g., 80x24). If too small, show a
   centered message: `"Terminal too small (need 80x24, have WxH)"`
   instead of rendering panes
2. **Resize handling:** Ensure the ratatui backend processes
   `Event::Resize` events. The main loop should re-render on resize.
   Check that `RatatuiBackend` calls `terminal.autoresize()` or
   equivalent
3. **Responsive layouts:** For panes using fixed `box_width`/`box_height`,
   clamp to available terminal size with `min(box_width, term_width - 2)`
4. **Graceful degradation:** If terminal is marginally too small, hide
   optional elements (help text, console) rather than crashing

**Files:** `src/main.rs` (event loop, resize handling),
`src/ui/ratatui_impl.rs` (backend), `src/ui/graphics.rs` (Rect clamping)

---

## 14. Export/import effects and instruments

**Inbox:** `exporting/importing effects, instruments`

**Status:** Not started

**What this means:** Overlaps with item 2 (global shortcuts). Users want
to save individual strips or effect chains to files and load them into
other sessions. Currently, only full-session save/load exists via SQLite.

**Proposed:**

1. **Export strip:** Serialize a single `Strip` (source, filter, effects,
   LFO, envelope, mixer settings) to a standalone file (JSON or
   mini-SQLite)
2. **Import strip:** Deserialize from file and add to current session
   with a new `StripId`
3. **Export effect chain:** Save just the effects list from a strip
4. **Import effect chain:** Apply an exported effect chain to an existing
   strip
5. **UI flow:** Use the file browser pane in save/load mode with
   appropriate `FileSelectAction` variants
6. Add `Action::ExportStrip(StripId)` and `Action::ImportStrip` variants

**Files:** `src/state/persistence.rs` or new `src/state/export.rs`,
`src/ui/pane.rs` (new actions), `src/dispatch.rs` (handlers),
`src/panes/file_browser_pane.rs` (save mode)

---

## 15. Documentation cleanup

**Inbox:** `docs cleanup`

**Status:** Partially done (CLAUDE.md and architecture docs were rewritten
in REFACTOR1)

**What this means:** The `docs/` directory has accumulated documentation
from multiple iterations. Some docs may be outdated, redundant, or
inconsistent with the current codebase.

**Proposed:**

1. Audit each file in `docs/`:
   - `architecture.md` — recently updated, verify still accurate
   - `audio-routing.md` — check against current `AudioEngine` code
   - `keybindings.md` — update with current bindings
   - `ai-coding-affordances.md` — recently updated
   - `sc-engine-architecture.md` — verify against current engine modules
   - `polyphonic-voice-allocation.md` — verify against `VoiceChain` changes
   - `custom-synthdef-plan.md` — compare plan vs implementation
   - `sqlite-persistence.md` — marked "partially outdated", update or remove
   - `ai-integration.md` — check if still relevant
2. Remove or archive any docs that describe planned-but-abandoned features
3. Ensure `CLAUDE.md` doc references are all valid

**Files:** All files in `docs/`

---

## 16. LFO modulation targets

**Inbox:** `LFO mods (todo)`

**Status:** Partially implemented

**What this means:** The `LfoTarget` enum (`src/state/strip.rs:368-528`)
defines 15 modulation targets, but only `FilterCutoff` is actually wired
up in the audio engine. The other 14 targets (FilterResonance, Amplitude,
Pitch, Pan, PulseWidth, SampleRate, DelayTime, DelayFeedback, ReverbMix,
GateRate, SendLevel, Detune, Attack, Release) are defined in the enum
but do nothing when selected.

**Proposed:**

For each target, implementation requires:
1. Add a `*_mod_in` control-rate input to the relevant SuperCollider
   SynthDef (e.g., `amp_mod_in` for Amplitude)
2. Wire the LFO bus to that input in
   `AudioEngine::rebuild_strip_routing()` when the target is selected
3. Test that the modulation actually affects the parameter

Priority order (most musically useful first):
1. **Amplitude** — tremolo effect
2. **Pitch** — vibrato effect
3. **Pan** — auto-pan effect
4. **FilterResonance** — already partially documented
5. **PulseWidth** — PWM synthesis
6. **DelayTime** / **DelayFeedback** — modulated delay effects
7. Remaining targets as needed

**Files:** `src/audio/engine.rs` (routing), SuperCollider SynthDef files,
`src/state/strip.rs` (already defined)

---

## 17. Better UI/input primitives

**Inbox:** `better pane/ui/text input primitives`

**Status:** Not started

**What this means:** The current UI primitives are minimal:
- `TextInput` (`src/ui/widgets/text_input.rs`) — single-line text input
- `SelectList` (`src/ui/widgets/`) — basic list selection
- `Graphics` trait — low-level `put_char`, `put_str`, `draw_box`

Each pane manually positions elements with absolute coordinates. There's
no layout system, no multi-line text input, no dropdown/combo box, no
tabs, no scrollable containers.

**Proposed:**

1. **Numeric input widget:** Specialized input for float/int values with
   arrow-key increment, min/max clamping, and format string
2. **Multi-line text input:** For longer text fields
3. **Scrollable list widget:** Generic scrollable list with selection,
   used across panes (replace bespoke scroll logic in StripPane,
   MixerPane, etc.)
4. **Layout helpers:** Row/column layout functions that auto-position
   elements instead of manual `(x, y)` math
5. **Form widget:** Label + value pairs with field navigation (extract
   the pattern used in FrameEditPane and StripEditPane)

**Files:** `src/ui/widgets/` (new widgets), `src/ui/graphics.rs` (layout
helpers)

---

## 18. Ctrl-L to force re-render

**Inbox:** `CTL-L to re render`

**Status:** Not started

**What this means:** `Ctrl-L` is a standard terminal convention for
clearing and redrawing the screen (like in vim, less, etc.). Currently,
`Ctrl-L` is bound to "load" (`main.rs`). The user wants `Ctrl-L` to
force a full terminal redraw instead, which is useful when the terminal
gets corrupted by external output or resize artifacts.

**Proposed:**

1. Rebind `Ctrl-L` from "load" to "force redraw"
2. Move "load" to a different shortcut (e.g., `Ctrl-O` for "open")
3. Implement force redraw by calling `terminal.clear()` on the ratatui
   backend, which will cause the next frame to fully repaint
4. Add the clear/redraw method to `RatatuiBackend`

**Files:** `src/main.rs` (keybinding change), `src/ui/ratatui_impl.rs`
(clear method)

---

## 19. UI themes

**Inbox:** `ui themes`

**Status:** Not started

**What this means:** All colors are currently hardcoded as constants in
`src/ui/style.rs` (30+ named constants like `Color::CYAN`,
`Color::SELECTION_BG`, `Color::MIDI_COLOR`, etc.). There's no way to
change the color scheme. Note: a `SemanticColor` enum was previously
defined but removed as unused (REFACTOR1 item 2.7).

**Proposed:**

1. **Define a `Theme` struct** with semantic color slots:
   ```rust
   struct Theme {
       bg: Color,
       fg: Color,
       accent: Color,
       selection_bg: Color,
       selection_fg: Color,
       muted: Color,
       error: Color,
       // ... etc
   }
   ```
2. **Ship 2-3 built-in themes:** Default (current colors), Light, High
   Contrast
3. **Store active theme** in `AppState` or `Frame`
4. **Replace direct `Color::*` usage** in panes with theme lookups
5. **Theme switcher** in session settings or via a keybinding

This is a large change touching every pane's `render()` method.
Consider doing it incrementally: define the Theme struct and make new
code use it, then migrate existing panes one at a time.

**Files:** `src/ui/style.rs` (Theme struct), `src/state/mod.rs` (store
theme), all panes (migrate color references)

---

## 20. Stale LSP diagnostics in dev tooling

**Inbox:** `stale diagnostics?`

**Status:** Not started — tooling/dev-environment issue

**What this means:** When Claude Code edits files in this codebase, the
LSP (rust-analyzer via cclsp MCP) sometimes reports stale diagnostics
for recently-edited files. The diagnostics reflect the pre-edit state
of the file, leading to false errors and confusion.

**Proposed:**

This is a development tooling issue, not a codebase change:

1. **Investigate cclsp refresh behavior:** Determine if cclsp (configured
   in `.mcp.json` and `cclsp.json`) properly notifies rust-analyzer of
   file changes after edits
2. **Consider tree-sitter MCP:** A tree-sitter-based MCP server could
   provide faster, file-local syntax analysis that doesn't depend on
   rust-analyzer's full recompilation cycle
3. **Workaround:** Use `mcp__cclsp__restart_server` to force a refresh
   when diagnostics seem stale
4. **Upstream fix:** Report the staleness issue to the cclsp project if
   it's a bug in the file-watching mechanism

**Files:** `.mcp.json`, `cclsp.json` (MCP configuration)

---

## 21. Logging interface

**Inbox:** `logging interface`

**Status:** Not started

**What this means:** The application has no structured logging. Debug
output goes through `Frame::push_message()` (user-facing console) or
`eprintln!` (lost in the TUI). There's no log levels, no file logging,
no way to debug issues in production.

**Proposed:**

1. **Add `log` + `env_logger` (or `tracing`)** as dependencies
2. **Configure file-based logging:** Write to `~/.config/tuidaw/tuidaw.log`
   or a path specified via CLI flag (item 1)
3. **Replace `eprintln!` calls** with `log::error!`, `log::warn!`, etc.
4. **Keep `Frame::push_message()`** for user-visible messages (these are
   distinct from debug logs)
5. **Log levels:** Default to `warn` in normal use, `debug` with
   `--verbose` flag
6. **Log key events:** OSC messages sent/received, voice allocation,
   file operations, errors

**Files:** `Cargo.toml` (add deps), `src/main.rs` (init logger),
throughout codebase (replace ad-hoc logging)

---

## 22. Unit tests

**Inbox:** `"comfortable amount of unit tests"`

**Status:** ~41 tests exist (`cargo test --bin tuidaw`)

**What this means:** The codebase has some tests but coverage is likely
uneven. Critical subsystems that handle state mutation, audio routing,
and persistence should have thorough test coverage.

**Proposed:**

Priority areas for new tests:

1. **`dispatch.rs`** — The central action handler. Test that each action
   variant produces the expected state mutation. Currently 900+ lines
   with complex logic.
2. **`persistence.rs`** — Round-trip tests: save a `StripState`, load it
   back, verify equality. Test migration paths (item 3).
3. **`AudioEngine` node calculations** — Test `StripNodes` construction,
   bus allocation, voice stealing logic (without needing a live SC
   server). May require extracting pure functions from engine methods.
4. **Piano roll** — Note placement, deletion, quantization, track
   management
5. **Music theory** — `Key`, `Scale`, pitch calculations, note naming
6. **Keymap** — Verify bindings resolve correctly, no conflicts

Target: enough tests that refactoring any module gives confidence
nothing broke. Focus on logic-heavy code, not rendering.

**Files:** Test modules within each `src/` file, or `tests/` directory
for integration tests

---

## 23. ESC exits piano/insert mode directly

**Inbox:** `esc takes you directly out of piano mode (or insert mode)`

**Status:** Bug/enhancement — not started

**What this means:** When piano keyboard mode is active (via `/` in
StripPane, StripEditPane, or PianoRollPane), pressing Escape may not
deactivate it cleanly. The `PianoKeyboard` struct
(`src/ui/piano_keyboard.rs`) has `handle_escape()` and `deactivate()`
methods, but the pane's input handler may route Escape to other
actions (like "go back to previous pane") before checking piano mode.

**Proposed:**

1. In every pane that uses `PianoKeyboard`, check `piano.is_active()`
   first in `handle_input()`. If active and Escape is pressed, call
   `piano.deactivate()` and return `Action::None` — do NOT propagate
   Escape further
2. Same logic for any "insert mode" (text editing in StripEditPane):
   Escape should exit insert mode, not leave the pane
3. Establish a priority chain: insert mode > piano mode > pane navigation
4. Audit all three panes (StripPane, StripEditPane, PianoRollPane) for
   consistent Escape handling

**Files:** `src/panes/strip_pane.rs`, `src/panes/strip_edit_pane.rs`,
`src/panes/piano_roll_pane.rs`, `src/ui/piano_keyboard.rs`

---

## 24. Insert mode touch-ups

**Inbox:** `touch ups around insert mode`

**Status:** Not started

**What this means:** "Insert mode" in the piano roll (and possibly strip
editor) where keyboard input is captured for note entry or text editing
has rough edges. Likely issues include:

1. **Visual indicator:** Not always clear when you're in insert mode vs
   normal mode (no mode line or cursor change)
2. **Mode transitions:** Entering/exiting insert mode may have edge cases
   (stuck in mode, double-toggle, etc.)
3. **Key conflicts:** Some keys may do different things in insert vs
   normal mode, causing confusion

**Proposed:**

1. Add a clear **mode indicator** in the pane header or status area
   (e.g., `-- INSERT --`, `-- PIANO --`, `-- NORMAL --`)
2. Audit mode transitions in all panes that have insert mode
3. Ensure consistent enter/exit behavior (Enter to start editing, Escape
   to stop, across all panes)
4. Test edge cases: switching panes while in insert mode, resize while
   in insert mode, etc.

**Files:** `src/panes/strip_edit_pane.rs`, `src/panes/piano_roll_pane.rs`,
`src/ui/piano_keyboard.rs`

---

## ~~25. Pane markup language~~ SKIPPED

**Inbox:** `pane markup language`

Skipped per user decision — too aspirational for current scope.

---

## 26. Remove HomePane

**Inbox:** `remove homepane`

**Status:** Not started

**What this means:** The HomePane (`src/panes/home_pane.rs`) is a simple
3-item menu (Rack, Mixer, Server) that serves as a landing screen. Since
number keys (1-5) already provide direct navigation to any pane, the
home screen is redundant. It also uses outdated naming ("Rack" instead
of the current "Strip"/"Oscillators" terminology).

**Proposed:**

1. **Remove `HomePane`** from `src/panes/home_pane.rs` and `src/panes/mod.rs`
2. **Remove registration** from `main.rs`
3. **Change default pane** on startup from `"home"` to `"strip"` (or
   whichever pane the user lands on)
4. Remove any `SwitchPane("home")` references (FrameEditPane's cancel
   goes to `"rack"` which may alias to home)
5. Clean up `"rack"` pane ID references — several places use `"rack"`
   but the actual StripPane ID is `"strip"`

**Files:** `src/panes/home_pane.rs` (delete), `src/panes/mod.rs`,
`src/main.rs` (registration and default pane), `src/panes/frame_edit_pane.rs`
(cancel target), anywhere referencing `"home"` or `"rack"` pane IDs

---

## 27. File picker scroll wrapping

**Inbox:** `file picker, scroll to bottom, does not loop back to top`

**Status:** Bug — not started

**What this means:** The FileBrowserPane (`src/panes/file_browser_pane.rs`)
allows navigating a directory listing with Up/Down. When you reach the
bottom of the list and press Down, it stops instead of wrapping back to
the top. Similarly, pressing Up at the top doesn't wrap to the bottom.

**Proposed:**

1. In the FileBrowserPane's navigation handler, use modular arithmetic
   for the selection index:
   ```rust
   // Down at bottom wraps to top
   self.selected = (self.selected + 1) % self.entries.len();
   // Up at top wraps to bottom
   self.selected = (self.selected + self.entries.len() - 1) % self.entries.len();
   ```
2. Apply the same wrap-around logic to any other list-based panes where
   it makes sense (StripPane, MixerPane, etc.) for consistency (relates
   to item 7, keybinding consistency)

**Files:** `src/panes/file_browser_pane.rs`

---

## 28. Arrangement view (zoomed-out piano roll)

**Status:** Not started — new feature

**What this means:** A GarageBand-style arrangement/timeline view that
shows all tracks horizontally with a zoomed-out perspective. Instead of
showing individual MIDI notes (like the piano roll does), it shows
**regions** — colored blocks representing where MIDI data exists on each
track. This makes it easy to see the song structure at a glance, loop
sections, and move MIDI data between instruments.

The current `SequencerPane` (key `3`) is a placeholder ("Coming soon...")
— this is the natural home for the arrangement view.

**Toggle:** `'` (single quote) toggles between the arrangement view and
the piano roll view (key `2`), providing a quick zoom-in/zoom-out
workflow.

### Visual Design

```
┌─ Arrangement ─────────────────────────────────────────────────────────────────┐
│  1       2       3       4       5       6       7       8                     │
│  ┊───────┊───────┊───────┊───────┊───────┊───────┊───────┊─────── (bar ruler) │
├──────────┬────────────────────────────────────────────────────────────────────┤
│ saw-0    │ ████████████  ░░░░░░  ████████████████████████                     │
│          │ ~~automation curve~~                                               │
│ sin-1    │       ██████████████████████  ░░░░░░░░░░░░                         │
│          │                                                                    │
│ sampl-2  │ ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓  ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓                 │
│          │                                                                    │
│ audio-3  │▁▃▅▇█▇▅▃▁▁▃▅▇█▇▅▃▁  ▁▃▅▇█▇▅▃▁▁▃▅▇█▇▅▃▁▁▃▅▇                     │
│          │                                                                    │
│          ▼ (playhead)                                                         │
│  ┊═══════┊═══════┊ (loop region highlight)                                   │
└──────────┴────────────────────────────────────────────────────────────────────┘
```

Where:
- `████` = MIDI region (colored per strip, shows duration of note
  activity, not individual notes)
- `▓▓▓▓` = Sampler region (different color/pattern)
- `▁▃▅▇█` = Waveform (AudioIn strips, miniature amplitude display)
- `~~` = Automation curve (small line using braille/block characters)
- Empty space = no data in that time range

### Data Model

**MIDI regions are first-class stored objects**, not derived from note
data. This matches how real DAWs (GarageBand, Ableton, Logic) work —
regions/clips are discrete containers that hold notes. The hierarchy
is: Track → Regions → Notes.

Deriving regions by gap-merging notes would break down for: explicit
user-defined boundaries, empty placeholder regions, naming/coloring
individual regions, copy/paste as discrete objects, splitting/merging
as user actions, and stable identity for cross-track moves.

```rust
pub type RegionId = u32;

pub struct MidiRegion {
    pub id: RegionId,
    pub strip_id: StripId,        // which track this region belongs to
    pub start_tick: u32,          // region start (independent of first note)
    pub end_tick: u32,            // region end (independent of last note)
    pub name: Option<String>,     // user-assigned label
    pub color: Option<Color>,     // color override (default: strip color)
    pub muted: bool,              // mute region without deleting
    pub notes: Vec<Note>,         // notes within this region (ticks relative to region start)
}
```

**SQLite table:**

```sql
CREATE TABLE midi_regions (
    id          INTEGER PRIMARY KEY,
    strip_id    INTEGER NOT NULL,
    start_tick  INTEGER NOT NULL,
    end_tick    INTEGER NOT NULL,
    name        TEXT,
    color       INTEGER,
    muted       INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (strip_id) REFERENCES strips(id)
);
```

Notes get a `region_id` foreign key:

```sql
ALTER TABLE notes ADD COLUMN region_id INTEGER REFERENCES midi_regions(id);
```

**Migration path:** Existing notes (which currently belong directly to
a Track) get auto-wrapped into one region per track on first load.
The gap-merging algorithm becomes a one-time migration tool, not the
runtime model.

**Waveform data** uses `AppState.audio_in_waveform` (already exists for
AudioIn strips). May need to cache waveforms per-track rather than just
the active one. AudioIn strips would have waveform regions (same table,
different rendering) rather than MIDI regions.

**Automation data** reads from `state.strip.automation.lanes` —
`AutomationLane.points` provides the (tick, value) pairs for rendering
a miniature envelope curve under each track.

### Track Layout

- **Left sidebar** (10 chars): Strip name (truncated), source type icon
- **Timeline area** (remaining width): Horizontal bar ruler + track
  content
- **Track height**: 2 lines per track (1 for regions/waveform, 1 for
  automation or padding). Expandable to 3+ lines for detailed automation
- **Vertical scrolling**: If more tracks than fit, scroll with Up/Down
  or j/k
- **Horizontal scrolling**: Arrow Left/Right move the viewport along
  the timeline

### Zoom Levels

Wider than the piano roll's finest zoom. Suggested levels:

| Level | Resolution         | Use case                  |
|-------|--------------------|---------------------------|
| 1     | 1 char = 1/4 beat  | Close-up (near piano roll)|
| 2     | 1 char = 1 beat    | Default                   |
| 3     | 1 char = 1 bar     | Song overview             |
| 4     | 1 char = 4 bars    | Full song at a glance     |

Zoom with `z`/`x` (matching piano roll convention).

### Color Scheme

Each strip gets a distinct color for its regions. Suggested palette:

- Oscillator strips (Saw/Sin/Sqr/Tri): blues and cyans
- Sampler strips: greens
- AudioIn strips: oranges/yellows (waveform rendered with block chars)
- Automation curves: magenta/pink (overlaid on track row)
- Loop region: highlighted background (e.g., `Color::SELECTION_BG`)
- Playhead: green vertical line (matching piano roll)

### Keybindings

```
'              Toggle to/from piano roll (zoomed-in view)
Arrow L/R      Scroll timeline
Arrow U/D      Select track (vertical navigation)
j/k            Select track (vim-style)
z/x            Zoom in/out (time axis)
Space          Play/Stop
L              Toggle loop
[ / ]          Set loop start/end at cursor position
Enter          Jump to piano roll at selected track + cursor time
< / >          Move selected region to previous/next track (reattach
               MIDI data to a different strip)
```

### Interactions (future)

These can be implemented incrementally:

1. **Region selection:** Highlight a region, show its start/end/duration
   in a status line
2. **Region move:** Move a region's notes to a different time position
   (shift all note ticks)
3. **Region copy/duplicate:** Copy a region's notes to another position
4. **Cross-track move:** Move a region from one strip's track to another
   (item `< / >` above). This means reassigning `Note.strip_id` in the
   piano roll
5. **Loop-from-region:** Select a region, press `L` to set loop
   boundaries to match the region's extent
6. **Split/merge regions:** Split a region at the cursor, merge adjacent
   regions

### Implementation Plan

**Phase 1 — Read-only arrangement view:**
1. Create `src/panes/arrangement_pane.rs` implementing `Pane`
2. Replace `SequencerPane` registration with `ArrangementPane` (key `3`)
3. Render bar ruler, track labels, and MIDI regions as colored blocks
4. Render playhead and loop markers
5. Add `'` keybinding in both arrangement and piano roll panes for
   toggling between them
6. Horizontal/vertical scrolling and zoom

**Phase 2 — Waveform and automation display:**
7. Render miniature waveforms for AudioIn tracks
8. Render automation curves under tracks (braille or block characters)
9. Per-strip color assignment

**Phase 3 — Interactive editing:**
10. Region selection and status display
11. Cross-track region move (`< / >`)
12. Loop-from-region
13. Region copy/duplicate

**Files:** New `src/panes/arrangement_pane.rs`, `src/panes/mod.rs`,
`src/main.rs` (register pane, add `'` global toggle), `src/ui/pane.rs`
(if new Action variants needed), `src/panes/piano_roll_pane.rs` (add `'`
toggle keybinding)

---

## Priority Order

### Bugs (fix first)
1. **Item 4** — Broken Frame settings screen
2. **Item 12** — Delete strip doesn't work + rename
3. **Item 10** — Piano roll fixes (BPM display, MIDI-0 name)
4. **Item 27** — File picker scroll wrapping
5. **Item 23** — ESC exits piano/insert mode directly

### Quick wins
6. **Item 26** — Remove HomePane
7. **Item 11** — Remove help text at bottom
8. **Item 18** — Ctrl-L to force re-render

### Infrastructure
9. **Item 1** — CLI argument parsing
10. **Item 3** — Database migrations
11. **Item 21** — Logging interface
12. **Item 6** — Move console to Server pane

### Polish
13. **Item 7** — Keybinding consistency audit
14. **Item 24** — Insert mode touch-ups
15. **Item 13** — Handle small terminal + resize
16. **Item 22** — Unit tests

### Features
17. **Item 28** — Arrangement view (phase 1: read-only)
18. **Item 2** — Global shortcuts: save as, open, export, import
19. **Item 14** — Export/import effects, instruments
20. **Item 16** — LFO modulation targets
21. **Item 17** — Better UI/input primitives
22. **Item 9** — Custom synths, VST support
23. **Item 28** — Arrangement view (phases 2-3: waveforms, automation, editing)

### Long-term
24. **Item 19** — UI themes
25. **Item 15** — Documentation cleanup
26. **Item 20** — Stale LSP diagnostics (dev tooling)
