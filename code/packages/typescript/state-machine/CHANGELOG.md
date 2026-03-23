# Changelog

## 0.1.0 — 2026-03-20

### Added

- **types.ts**: Core type aliases (`State`, `Event`, `Action`), `TransitionRecord` interface, and `transitionKey()` helper for encoding tuple keys as strings.
- **dfa.ts**: Full `DFA` class — construction with eager validation, `process()`, `processSequence()`, `accepts()` (non-mutating), `reset()`, `reachableStates()` (BFS), `isComplete()`, `validate()`, `toDot()`, `toAscii()`, `toTable()`. Actions with trace recording.
- **nfa.ts**: Full `NFA` class — `epsilonClosure()` (BFS), `process()`, `processSequence()`, `accepts()` (non-mutating), `reset()`, `toDfa()` (subset construction), `toDot()`. `EPSILON` sentinel and `stateSetName()` helper.
- **minimize.ts**: `minimize()` function implementing Hopcroft's partition-refinement algorithm. Removes unreachable states, merges equivalent states.
- **pda.ts**: `PushdownAutomaton` class with deterministic constraint, `PDATransition` and `PDATraceEntry` interfaces. Stack operations, epsilon transitions at end-of-input, `accepts()` (non-mutating).
- **modal.ts**: `ModalStateMachine` class with `ModeTransitionRecord`. Mode switching resets target DFA. Full mode trace.
- **index.ts**: Barrel exports for all public types and classes.
- Comprehensive test suite: 150+ tests across 5 test files covering DFA, NFA, minimize, PDA, and modal machines.

### Notes

- This is a direct port of the Python `state-machine` package to TypeScript.
- Uses `Map<string, ...>` with null-byte-separated keys (`transitionKey()`) instead of Python's tuple-keyed dicts.
- All `accepts()` methods are non-mutating (simulate on copies).
- Literate programming JSDoc comments throughout.
