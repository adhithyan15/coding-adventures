# Changelog

All notable changes to the `state-machine` crate will be documented in this file.

## [Unreleased]

### Added

- Added the `definitions` module with `StateMachineDefinition`, state
  definitions, transition definitions, and machine-kind tags.
- Added export helpers for manually constructed DFAs, NFAs, and PDAs so they can
  feed the build-time state-machine compiler pipeline without runtime text
  loading.
- Added import helpers for `DFA`, `NFA`, and `PushdownAutomaton` so validated
  typed definitions can be reconstructed as executable machines.
- Added tests covering DFA, NFA, PDA, epsilon, multi-target, and stack-effect
  definition export behavior.
- Added definition import tests covering language preservation and rejection of
  invalid machine-kind, transition-target, epsilon, empty-event, and
  stack-effect shapes.
- Added an effectful transducer runtime for ordered tokenizer-style state
  machines with portable actions, consume flags, EOF handling, trace records,
  and definition round-tripping.
- Added a minimal HTML tokenizer skeleton test covering text buffering,
  start/end tag emission, and EOF effects.
- Allowed `$any` transitions to accept runtime events outside the declared
  alphabet when the current state has an `Any` fallback, which lets tokenizer
  wrappers process arbitrary Unicode text without declaring every code point.
- Added controlled runtime state hops for effectful transducers via
  `EffectfulStateMachine::set_current_state`, which lets wrapper runtimes model
  return-state flows without embedding host-language callbacks in definitions.

### Changed

- Moved State Machine Markup serialization out of this crate. Format-specific
  writers now belong in sibling serializer libraries.
- Extended `TransitionDefinition` with `actions` and `consume` fields so the
  shared typed definition layer can represent tokenizer/transducer effects.
- Extended the typed definition layer with profile metadata, lexer token/input/
  register/fixture declarations, typed matcher objects, and optional transition
  guards so build-time lexer tooling can lower `.lexer.states.toml` documents
  without embedding host-language code.

## [0.1.0] - 2026-03-20

### Added

- **`types` module**: Core `TransitionRecord` struct for execution tracing, plus `State` and `Event` type aliases.
- **`dfa` module**: Full Deterministic Finite Automaton implementation with:
  - Eager validation at construction time (fail-fast)
  - `process()` and `process_sequence()` for event-driven execution
  - `accepts()` for non-mutating acceptance checking
  - `reachable_states()`, `is_complete()`, `validate()` for introspection
  - `to_dot()`, `to_ascii()`, `to_table()` for visualization
  - `Display` and `Debug` implementations
- **`nfa` module**: Non-deterministic Finite Automaton with:
  - Epsilon transition support via `EPSILON` sentinel
  - `epsilon_closure()` computation (BFS over epsilon edges)
  - `process()` with parallel-universe semantics
  - `accepts()` with early rejection on dead states
  - `to_dfa()` subset construction algorithm
  - `to_dot()` visualization with epsilon labels
- **`minimize` module**: Hopcroft's partition-refinement algorithm for DFA minimization:
  - Unreachable state removal
  - Equivalent state merging
  - Language-preserving guarantees
- **`pda` module**: Deterministic Pushdown Automaton with:
  - Stack operations (push, pop, replace)
  - `PDATransition` and `PDATraceEntry` for full execution tracing
  - Epsilon transitions at end-of-input for acceptance
  - `accepts()` non-mutating simulation
- **`modal` module**: Modal State Machine with:
  - Named DFA sub-machines (modes)
  - Mode transition rules with trigger events
  - Automatic DFA reset on mode switch
  - `ModeTransitionRecord` for mode switch tracing
- **240 tests** across unit and integration test suites covering all modules.
- Literate programming doc comments throughout all source files.

### Notes

- Ported from the Python `state-machine` package with idiomatic Rust patterns.
- Uses `Result<T, String>` for fallible operations instead of Python exceptions.
- Omits action callbacks (Rust closures are complex to store in HashMaps); uses `action_name: None` in traces.
