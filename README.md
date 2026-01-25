# TUI DAW - Java Implementation

Terminal-based Digital Audio Workstation rewritten in Java 21.

## Prerequisites

- Java 21 or later
- Maven 3.8+
- SuperCollider (for audio)

## Build

```bash
# Install dependencies and build
mvn clean package

# Build without tests
mvn clean package -DskipTests
```

## Run

```bash
# Run with Maven
mvn exec:java

# Run the shaded JAR
java --enable-preview -jar target/tui-daw-1.0-SNAPSHOT.jar

# Run without audio (TUI only)
java --enable-preview -jar target/tui-daw-1.0-SNAPSHOT.jar --no-audio

# Connect to existing SuperCollider server
java --enable-preview -jar target/tui-daw-1.0-SNAPSHOT.jar --connect
```

## Configuration

Create `tuidaw.properties` in the working directory:

```properties
# Path to scsynth executable
scsynth.path=/Applications/SuperCollider.app/Contents/Resources/scsynth

# SuperCollider port
scsynth.port=57110

# Keybinding mode: vim, emacs, or normie
keybinding.mode=vim

# Path to SynthDef files
synthdef.path=synthdefs

# Default save file
save.path=rack.json
```

## SynthDef Files

Unlike the Clojure version which uses Overtone, this Java implementation requires
precompiled SynthDef files. Create these in SuperCollider IDE and save them to the
`synthdefs/` directory.

Example SuperCollider code to create a SynthDef:

```supercollider
(
SynthDef(\saw-osc, { |out_bus=0, freq=440, amp=0.5|
    Out.ar(out_bus, Saw.ar(freq) * amp);
}).writeDefFile("synthdefs/");
)
```

## Architecture

```
Keyboard Input
    ↓
InputHandler (key→action translation)
    ↓
Dispatcher (routes by current view)
    ↓
StateTransitions (pure) + Rack (effectful)
    ↓
RackState (immutable record)
    ↓
Renderer (draws to Lanterna screen)
```

### Key Classes

| Package | Class | Purpose |
|---------|-------|---------|
| `core` | `Dispatcher` | Routes actions by view |
| `core` | `Action` | Enum of all actions |
| `core` | `View` | Enum of views |
| `state` | `RackState` | Main immutable state record |
| `state` | `StateTransitions` | Pure state transformation functions |
| `state` | `History` | Undo/redo stacks |
| `modules` | `ModuleRegistry` | Module definitions with metadata |
| `audio` | `AudioEngine` | scsynth lifecycle |
| `audio` | `OSCClient` | JavaOSC wrapper |
| `audio` | `Rack` | Effectful module operations |
| `tui` | `TUIMain` | Main loop at 60fps |
| `tui.input` | `VimBinding` | Vim-style keybindings |
| `tui.input` | `EmacsBinding` | Emacs-style with chords |
| `tui.input` | `NormieBinding` | Arrow key / Ctrl-key bindings |
| `tui.render` | `Renderer` | View dispatch |
| `persistence` | `RackSerializer` | JSON save/load |

## Testing

```bash
# Run all tests
mvn test

# Run specific test class
mvn test -Dtest=StateTransitionsTest
```

## Differences from Clojure Version

1. **SynthDefs**: Must be precompiled (no runtime Overtone/defsynth)
2. **Persistence**: Uses JSON instead of EDN
3. **Build**: Maven instead of deps.edn
4. **Records**: Java 17+ records instead of Clojure maps
5. **Pattern matching**: Java 21 switch expressions

## Views

| View | Description |
|------|-------------|
| RACK | Module list (main view) |
| EDIT | Parameter editing |
| PATCH | Signal routing |
| SEQUENCER | Step sequencer |
| SEQ_TARGET | Target selection |
| ADD | Module type selection |

## Keybindings

### Vim Mode (default)
- `j/k` - Navigate
- `h/l` - Adjust parameter
- `e` - Edit module
- `p` - Patch view
- `s` - Sequencer
- `a` - Add module
- `d` - Delete module
- `y` - Yank (copy)
- `P` - Paste
- `u` - Undo
- `Ctrl-r` - Redo
- `/` - Search
- `:` - AI command
- `q` - Quit

### Emacs Mode
- `C-n/C-p` - Navigate
- `C-f/C-b` - Adjust parameter
- `C-x C-s` - Save
- `C-x C-c` - Quit
- `C-c p` - Patch view
- `C-c s` - Sequencer

### Normie Mode
- Arrow keys - Navigate
- Enter - Edit
- Delete - Delete module
- Ctrl-C/Ctrl-V - Copy/paste
- Ctrl-Z/Ctrl-Y - Undo/redo
- Ctrl-S - Save
