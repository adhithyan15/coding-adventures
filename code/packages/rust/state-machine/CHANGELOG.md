# Changelog

All notable changes to the `state-machine` crate will be documented in this file.

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
