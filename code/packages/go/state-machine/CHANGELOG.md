# Changelog

All notable changes to the `state-machine` package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.1] - 2026-04-02

### Fixed

- **`PanicOnUnexpected` on all intentionally-panicking operations** — The ops
  framework swallows panics by default and returns a zero value instead, which
  caused tests that expected panics (e.g., `assertPanics("empty states", ...)`)
  to see no panic and fail. Added `.PanicOnUnexpected()` to the `StartNew`
  chain in every constructor or method that panics as part of its documented
  contract:
  - `NewDFA` — panics on empty states, bad initial/accepting/transition values
  - `DFA.Process` — panics on unknown event or missing transition
  - `DFA.Accepts` — panics on unknown event
  - `NewNFA` — panics on empty states, epsilon in alphabet, bad references
  - `NFA.Process` — panics on unknown event
  - `NFA.Accepts` — panics on unknown event
  - `NewPushdownAutomaton` — panics on empty states, bad references, duplicate transitions
  - `PushdownAutomaton.Process` — panics on missing transition
  - `NewModalStateMachine` — panics on empty modes or bad mode references
  - `ModalStateMachine.SwitchMode` — panics on missing mode transition
  - `ModalStateMachine.Process` — delegates to `DFA.Process` which panics

## [0.1.0] - 2026-03-20

### Added

- **Core types** (`types.go`):
  - `State` and `Event` type aliases for string
  - `Action` callback type for transition side effects
  - `TransitionRecord` struct for execution tracing

- **DFA** (`dfa.go`):
  - `NewDFA` constructor with eager validation (states, alphabet, transitions, initial, accepting, actions)
  - `Process`, `ProcessSequence`, `Accepts` (non-mutating), `Reset`
  - Introspection: `ReachableStates` (BFS), `IsComplete`, `Validate` (warnings)
  - Visualization: `ToDot` (Graphviz), `ToAscii` (ASCII table), `ToTable` (structured data)
  - Getters: `States`, `Alphabet`, `Initial`, `Accepting`, `CurrentState`, `Trace`, `Transitions`
  - Action support with trace recording

- **NFA** (`nfa.go`):
  - `NewNFA` constructor with epsilon transition support
  - `EpsilonClosure` (BFS over epsilon edges)
  - `Process`, `Accepts` (non-mutating), `Reset`
  - `ToDFA` via subset construction algorithm
  - `ToDot` with epsilon character rendering
  - Initial state automatically includes epsilon closure

- **DFA Minimization** (`minimize.go`):
  - `Minimize` function implementing Hopcroft's partition refinement algorithm
  - Removes unreachable states before minimizing
  - Merges equivalent states into single states
  - Produces the unique minimal DFA for any regular language

- **Pushdown Automaton** (`pda.go`):
  - `PDATransition` and `PDATraceEntry` types
  - `NewPushdownAutomaton` constructor with determinism validation
  - `Process`, `ProcessSequence` (with end-of-input epsilon transitions), `Accepts` (non-mutating), `Reset`
  - Stack operations: `Stack`, `StackTop`, `CurrentState`
  - Accepts balanced parentheses, a^n b^n, and other context-free languages

- **Modal State Machine** (`modal.go`):
  - `ModeTransitionRecord` for mode switch tracing
  - `NewModalStateMachine` constructor with mode validation
  - `SwitchMode` (resets target DFA), `Process` (delegates to active DFA), `Reset`
  - Getters: `CurrentMode`, `ActiveMachine`, `ModeTrace`, `Modes`

- **Tests** achieving 99% statement coverage (110 tests):
  - `types_test.go`: TransitionRecord fields, type alias usage, Action callback
  - `dfa_test.go`: Construction validation (7 error cases), process/sequence/accepts, reset, introspection (reachable, complete, validate), visualization (dot, ascii, table), edge cases (single state, no accepting, empty alphabet)
  - `nfa_test.go`: Construction validation (7 error cases), epsilon closure (5 cases), process with epsilon, accepts (contains-ab, ends-with-01, epsilon NFA), reset, ToDFA subset construction (4 NFAs), ToDot, edge cases
  - `minimize_test.go`: Already minimal, removes unreachable, merges equivalent, three equivalent states, single state, no accepting, NFA->DFA->minimize pipeline, incomplete DFA
  - `pda_test.go`: Construction validation (5 error cases), process/sequence/stack contents, accepts (balanced parens 10 cases, a^n b^n 10 cases), non-mutation, reset, stack helpers
  - `modal_test.go`: Construction validation (4 error cases), switch mode (basic, invalid, resets DFA, trace, three-mode), process (data mode, tag mode, invalid, switch-and-process), active machine, reset, modes list, edge cases (single mode, self-transition)

### Notes

- Port of the Python `state-machine` package to Go
- Uses panic for validation (matching the logic-gates convention)
- All maps are defensively copied to prevent aliasing bugs
- Knuth-style literate programming comments throughout
