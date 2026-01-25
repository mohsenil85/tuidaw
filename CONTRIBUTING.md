# Contributing to TUI DAW (Java)

Guide for humans and AI agents working on this codebase.

## Quick Start

```bash
mvn compile        # Verify changes compile (runs in pre-commit hook)
mvn test           # Run full test suite
mvn javadoc:javadoc  # Generate API docs at target/site/apidocs/
```

## Architecture Overview

```
KeyStroke → InputHandler → Action → Dispatcher → ViewDispatcher → StateTransitions → RackState
                                         ↓
                                  EffectRequest → Rack (audio)
```

| Layer | Responsibility | Side Effects? |
|-------|----------------|---------------|
| `tui/input/` | Key → Action translation | No |
| `core/Dispatcher` | Routes to view dispatchers | No (delegates) |
| `core/dispatchers/` | View-specific action handling | No (returns new state) |
| `state/StateTransitions` | Pure state transformations | No |
| `audio/Rack` | Audio engine operations | Yes (OSC to scsynth) |

## Common Tasks

### Add a New View

1. **Add to View enum** (`core/View.java`):
```java
public enum View {
    RACK, EDIT, PATCH, ADD, SEQUENCER, SEQ_TARGET, HELP,
    MY_VIEW  // ← add here
}
```

2. **Create dispatcher** (`core/dispatchers/MyViewDispatcher.java`):
```java
public class MyViewDispatcher implements ViewDispatcher {
    @Override
    public RackState dispatch(Action action, RackState state, History history,
                              Consumer<EffectRequest> effectHandler) {
        return switch (action) {
            case MOVE_DOWN -> StateTransitions.myViewMoveDown(state);
            case CANCEL -> StateTransitions.setView(state, View.RACK);
            default -> state;
        };
    }
}
```

3. **Register dispatcher** (`core/Dispatcher.java:initializeDispatchers()`):
```java
dispatchers.put(View.MY_VIEW, new MyViewDispatcher());
```

4. **Create renderer** (`tui/render/MyViewRenderer.java`):
```java
public class MyViewRenderer {
    public static void render(TextGraphics g, RackState state) {
        DrawUtils.drawBox(g, 0, 0, 40, 10, "My View");
        // render content...
    }
}
```

5. **Wire renderer** (`tui/render/RenderDispatcher.java`):
```java
case MY_VIEW -> MyViewRenderer.render(g, state);
```

### Add a New Action

1. **Add to Action enum** (`core/Action.java`):
```java
public enum Action {
    // ... existing actions ...
    MY_ACTION
}
```

2. **Add keybinding** (all three files in `tui/input/`):
```java
// VimBinding.java
case 'm' -> Action.MY_ACTION;

// EmacsBinding.java
if (ctrl && ch == 'm') return Action.MY_ACTION;

// NormieBinding.java
case F7 -> { return Action.MY_ACTION; }
```

3. **Handle in dispatcher** (appropriate `*ViewDispatcher.java`):
```java
case MY_ACTION -> handleMyAction(state, history, effectHandler);
```

### Add a New Module Type

1. **Add to ModuleType enum** (`modules/ModuleType.java`):
```java
public enum ModuleType {
    // ... existing types ...
    MY_SYNTH;

    public String getIdPrefix() {
        return switch (this) {
            // ...
            case MY_SYNTH -> "mysyn";
        };
    }
}
```

2. **Register module** (`modules/ModuleRegistry.java`):
```java
modules.put(ModuleType.MY_SYNTH, new ModuleDefinition(
    "my_synth",           // synthdef name (must match .scsyndef)
    List.of("out"),       // audio outputs
    List.of("in"),        // audio inputs
    List.of("freq", "amp"), // control inputs
    Map.of(               // parameter defaults
        "freq", new ParamDefinition(440.0, 20.0, 20000.0, ParamCurve.EXPONENTIAL),
        "amp", new ParamDefinition(0.5, 0.0, 1.0, ParamCurve.LINEAR)
    )
));
```

3. **Create synthdef** (`synthdefs/compile.scd`):
```supercollider
SynthDef(\my_synth, { |out_bus=0, in_bus=0, freq=440, amp=0.5|
    Out.ar(out_bus, SinOsc.ar(freq) * amp);
}).writeDefFile;
```

4. **Recompile synthdefs**:
```bash
cd synthdefs && sclang compile.scd
```

### Add State Transition

All pure state changes go in `state/StateTransitions.java`:

```java
/**
 * Example: Move selection down in my view.
 *
 * @param state current state
 * @return state with updated selection
 */
public static RackState myViewMoveDown(RackState state) {
    int current = state.myViewCursor();
    int max = state.myViewItems().size() - 1;
    int next = Math.min(current + 1, max);
    return state.withMyViewCursor(next);
}
```

## Testing Patterns

### Unit Test Structure

```java
@Nested
@DisplayName("MyFeature")
class MyFeatureTest {

    @Test
    @DisplayName("should do X when Y")
    void shouldDoXWhenY() {
        // Given
        RackState state = RackState.initial();

        // When
        RackState result = StateTransitions.myAction(state);

        // Then
        assertThat(result.myField()).isEqualTo(expectedValue);
    }
}
```

### Test with AssertJ

```java
// Collections
assertThat(state.modules()).hasSize(2);
assertThat(state.order()).containsExactly("saw-1", "out-1");

// Optionals
assertThat(state.selected()).isNotNull();
assertThat(result).isEmpty();

// Values
assertThat(param.value()).isCloseTo(0.5, within(0.001));
```

### KeyStroke Test Helpers

```java
// Character key
KeyStroke key = new KeyStroke('j', false, false);

// With modifier
KeyStroke ctrlS = new KeyStroke('s', true, false);  // ctrl=true, alt=false

// Special key
KeyStroke enter = new KeyStroke(KeyType.Enter);
KeyStroke escape = new KeyStroke(KeyType.Escape);
```

## Code Patterns

### Immutable Records

State objects are immutable records with `withX()` methods:

```java
public record RackState(
    Map<String, Module> modules,
    String selected,
    View view
) {
    public RackState withSelected(String selected) {
        return new RackState(modules, selected, view);
    }

    public RackState withView(View view) {
        return new RackState(modules, selected, view);
    }
}
```

### Switch Expressions

Use exhaustive switch expressions:

```java
// Good - compiler ensures all cases handled
return switch (action) {
    case MOVE_UP -> handleUp(state);
    case MOVE_DOWN -> handleDown(state);
    case CONFIRM -> handleConfirm(state);
    default -> state;  // for actions this view doesn't handle
};

// Use yield for multi-statement cases
case CONFIRM -> {
    history.push(state);
    RackState newState = doThing(state);
    yield newState;
}
```

### Effect Requests

Never do side effects in dispatchers. Return effect requests:

```java
// In dispatcher
case DELETE_MODULE -> {
    history.push(state);
    effectHandler.accept(EffectRequest.killSynth(state.selected()));
    yield StateTransitions.removeModule(state, state.selected());
}

// Audio layer handles the effect
switch (request) {
    case KillSynth(String id) -> rack.killSynth(id);
    case SetParam(String id, String param, double val) -> rack.setParam(id, param, val);
}
```

## Commit Messages

```
<type>: <description>

[body]

Refs: SONNET_TASKS.md#<task-number>  (if applicable)
Co-Authored-By: Claude <noreply@anthropic.com>
```

Types: `feat`, `fix`, `test`, `docs`, `refactor`, `chore`

## File Naming

| Type | Location | Example |
|------|----------|---------|
| State records | `state/` | `RackState.java`, `Module.java` |
| Pure transitions | `state/StateTransitions.java` | (single file) |
| View dispatchers | `core/dispatchers/` | `EditViewDispatcher.java` |
| Keybindings | `tui/input/` | `VimBinding.java` |
| Renderers | `tui/render/` | `RackViewRenderer.java` |
| Tests | `test/.../` | `StateTransitionsTest.java` |

## Pre-commit Hook

The pre-commit hook runs `mvn compile` on Java changes. If compilation fails, the commit is blocked. Fix errors before committing.

To bypass (not recommended):
```bash
git commit --no-verify -m "WIP"
```
