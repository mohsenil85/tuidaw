# AI Coding Affordances

Things that make this codebase easier or harder for AI agents (Claude
Code, Copilot, Cursor, etc.) to work with. Each section describes a
friction point encountered during real AI-assisted development and the
fix applied or recommended.

## 1. API Surface Documentation

### Problem

AI agents infer method names from conventions. When a type has
`bind_key()`, `bind_ctrl()`, and `bind_alt()`, the agent will
confidently call `bind_shift_key()` — which doesn't exist. Similarly,
`Color::new(r, g, b)` gets guessed as `Color::rgb(r, g, b)`. Each
wrong guess costs a compile-check round-trip.

### What helps

Doc comments on key types that enumerate the full API surface:

```rust
/// Keymap builder for pane input handling.
///
/// Available bind methods:
/// - `bind(char, action, desc)` — character key (no modifiers)
/// - `bind_key(KeyCode, action, desc)` — special key (arrows, F-keys, etc.)
/// - `bind_ctrl(char, action, desc)` — Ctrl + character
/// - `bind_alt(char, action, desc)` — Alt + character
/// - `bind_ctrl_key(KeyCode, action, desc)` — Ctrl + special key
///
/// No shift variant exists. For shift detection, check
/// `event.modifiers.shift` manually before keymap lookup.
pub struct Keymap { ... }
```

```rust
/// RGB color. Use named constants (Color::WHITE, Color::PINK, etc.)
/// or construct with Color::new(r, g, b).
///
/// No Color::rgb() alias exists.
pub struct Color { ... }
```

The key principle: **list what exists, and explicitly note what
doesn't** when the gap is surprising. An AI won't grep for "what
methods does this struct NOT have" — but it will read a doc comment at
the top of the struct.

### Where to apply

Any type with a builder or factory pattern where the AI might guess
plausible-but-nonexistent methods:

- `Keymap` — bind variants
- `Color` — constructors
- `Style` — modifier chain methods (fg, bg, bold, etc.)
- `Rect` — constructors (new, centered — no `from_size`, no `zero`)
- `Graphics` trait — drawing methods (put_str, put_char, draw_box,
  set_style, size — no `fill`, no `clear_rect`)

## 2. Borrow Pattern Cookbook

### Problem

The main.rs event loop repeatedly needs data from one pane to act on
another. Rust's borrow checker prevents two simultaneous `&mut`
references to `PaneManager`. An AI agent will write the intuitive
(wrong) version, hit a compile error, and spend a turn figuring out
the workaround.

### The pattern

**Extract, drop, use:**

```rust
// WRONG — two simultaneous &mut borrows of `panes`
let data = panes.get_pane_mut::<PaneA>("a").unwrap().get_data();
panes.get_pane_mut::<PaneB>("b").unwrap().use_data(data);

// RIGHT — first borrow ends before second begins
let data = {
    let pane_a = panes.get_pane_mut::<PaneA>("a").unwrap();
    pane_a.get_data()  // Copy/clone the data out
};  // pane_a borrow dropped here
if let Some(pane_b) = panes.get_pane_mut::<PaneB>("b") {
    pane_b.use_data(data);
}
```

Or equivalently with `if let` shadowing (the style used in this
codebase):

```rust
if let Some(pane_a) = panes.get_pane_mut::<PaneA>("a") {
    let data = pane_a.get_data();
    // pane_a borrow is shadowed/dropped by the next get_pane_mut
    if let Some(pane_b) = panes.get_pane_mut::<PaneB>("b") {
        pane_b.use_data(data);
    }
}
```

**This works because** `if let` bindings in Rust drop at the end of
their block, and a second `if let` on the same variable shadows the
first binding. The key constraint: `data` must be owned (copied or
cloned), not a reference into `pane_a`.

### Where this applies

Every `Action::PianoRoll*` handler that reads cursor state from
`PianoRollPane` and writes to `RackState` via `RackPane`. Every
`render_with_state` call (clone state, then pass). Any future
cross-pane interaction.

## 3. Build Verification

### Problem

AI agents need fast feedback on whether code compiles. `cargo build`
works but isn't the fastest option, and there's no documentation
saying "run this to check."

### What helps

A note in CLAUDE.md (done):

```markdown
## Build & Test
cargo build              # compile
cargo test --bin tuidaw  # unit tests (55 tests)
```

For even faster feedback, `cargo check` skips codegen and only runs
the compiler frontend. It catches all type errors, borrow errors, and
missing imports in roughly half the time of `cargo build`.

A `.cargo/config.toml` alias would make this discoverable:

```toml
[alias]
ck = "check"
```

Then `cargo ck` works as a quick verify. Not critical — AI agents will
run `cargo build` regardless — but shaves seconds per iteration.

### Pre-commit hook (optional)

A git pre-commit hook running `cargo check` would catch broken commits
before they happen. Useful for both humans and AI agents:

```bash
#!/bin/sh
# .git/hooks/pre-commit
cargo check --quiet 2>&1
```

The AI agent doesn't directly run hooks, but if it tries to commit
broken code, the hook blocks it and the agent sees the error — a
safety net.

## 4. The render_with_state Convention

### Problem

New panes that need data from `RackState` silently fail — their
`Pane::render()` method gets called with no external state, and the
developer doesn't realize they need to add a special case in main.rs.
An AI agent adding a new pane will implement `render()` and wonder why
it has no data.

### What helps

A comment on the `Pane` trait explaining the convention:

```rust
/// UI pane trait. All panes implement this for input handling and rendering.
///
/// ## External State
///
/// Some panes need data from RackState (which is owned by RackPane).
/// Because PaneManager only allows one &mut borrow at a time, these
/// panes implement a separate `render_with_state()` method and get
/// special-cased in the render block in main.rs.
///
/// If your pane needs rack/mixer/piano_roll state:
/// 1. Add a `pub fn render_with_state(&self, g, state)` method
/// 2. Make `render()` a no-op fallback
/// 3. Add a branch in main.rs's render section (search for "active_id")
///
/// Current panes using this pattern: MixerPane, PianoRollPane
pub trait Pane { ... }
```

This tells the AI agent exactly what to do without having to reverse-
engineer the pattern from main.rs. The "search for" hint gives a
concrete grep target.

### Longer term

The root cause is that state ownership is tangled with pane ownership.
A cleaner architecture would pass `&AppState` to every pane's
`render()` call, eliminating the special cases entirely. But that's a
larger refactor — the comment is the pragmatic fix.

## General Principles

Things that help AI agents work faster on any codebase:

1. **Document what doesn't exist** when it's a plausible guess. "No
   `Color::rgb()`" saves more time than documenting `Color::new()`.

2. **Show patterns at the call site.** A comment in main.rs saying
   "// Pattern: extract data, drop borrow, then use" right above the
   first occurrence makes it copy-pasteable.

3. **Name conventions explicitly.** "All panes use
   `Rect::centered(w, h, 97, 29)`" prevents a new pane from using
   full-screen coordinates and overwriting the frame.

4. **Keep CLAUDE.md updated.** It's the first file the agent reads.
   Every new convention, API, or gotcha should be added there. It
   costs nothing to maintain and saves multiple round-trips per
   session.

5. **Compile errors are cheap; wrong behavior is expensive.** The AI
   agent can fix compile errors in one turn. But if it writes code that
   compiles but silently doesn't work (like a `render()` that never
   gets called because `render_with_state()` is needed), it might not
   catch the problem at all.
