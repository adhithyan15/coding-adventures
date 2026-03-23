# Changelog

All notable changes to the `coding_adventures_state_machine` Elixir package will
be documented in this file.

## [0.1.0] - 2026-03-20

### Added

- **DFA module** — Deterministic Finite Automaton with `new/5`, `process/2`,
  `process_sequence/2`, `accepts?/2`, `reset/1`, `reachable_states/1`,
  `complete?/1`, `validate/1`, `to_dot/1`, `to_ascii/1`, `to_table/1`.
- **NFA module** — Non-deterministic Finite Automaton with epsilon transitions,
  `epsilon_closure/2`, `process/2`, `accepts?/2`, `reset/1`, `to_dfa/1`
  (subset construction), `to_dot/1`.
- **Minimize module** — Hopcroft's algorithm for DFA minimization via
  partition refinement.
- **PDA module** — Deterministic Pushdown Automaton with stack, `process/2`,
  `process_sequence/2`, `accepts?/2`, `reset/1`. Supports epsilon transitions
  at end of input for acceptance.
- **Modal module** — Modal State Machine with multiple DFA sub-machines,
  `process/2`, `switch_mode/2`, `reset/1`. Mode switches reset the target
  DFA to its initial state.
- **Types module** — `TransitionRecord` struct for execution tracing.
- 178 ExUnit tests covering all modules.
- Full literate programming documentation with `@moduledoc` and `@doc`.
- README with usage examples.

### Notes

- Ported from the Python `state-machine` package in the coding-adventures monorepo.
- All operations return new structs (immutable functional style).
- All fallible operations return `{:ok, result}` or `{:error, reason}` tuples.
- No external dependencies.
