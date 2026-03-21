# Changelog

All notable changes to the `coding_adventures_state_machine` gem will be documented in this file.

## [0.1.0] - 2026-03-20

### Added

- **DFA** (Deterministic Finite Automaton): 5-tuple construction with eager validation, process/process_sequence/accepts/reset, reachable_states/complete?/validate introspection, to_dot/to_ascii/to_table visualization, optional actions with trace recording.
- **NFA** (Non-deterministic Finite Automaton): epsilon transitions via EPSILON constant, epsilon_closure computation with worklist algorithm, non-deterministic processing with parallel state tracking, to_dfa subset construction algorithm, to_dot visualization.
- **Minimize**: Hopcroft's partition refinement algorithm for DFA minimization. Removes unreachable states, merges equivalent states, produces canonical minimal DFA.
- **PDA** (Pushdown Automaton): deterministic PDA with stack, PDATransition/PDATraceEntry structs, process/process_sequence/accepts/reset, epsilon transitions at end-of-input, stack inspection.
- **Modal State Machine**: collection of named DFA sub-machines with mode transitions, ModeTransitionRecord tracking, mode switching with target DFA reset, full reset of all sub-machines.
- **TransitionRecord**: Struct for execution trace entries (source, event, target, action_name).
- Comprehensive test suite with 150+ assertions covering all modules.
- Literate programming comments throughout with truth tables, diagrams, and analogies.

### Notes

- Ported from the Python `state-machine` package in the same repository.
- Follows the same algorithms and API design as the Python version.
- Uses idiomatic Ruby: Set for sets, Struct for value types, frozen strings, snake_case methods.
