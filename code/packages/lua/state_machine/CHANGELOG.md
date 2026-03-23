# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-23

### Added

- **DFA** (Deterministic Finite Automaton): states, transitions, actions, process/process_sequence/accepts, reset, trace, reachable_states, is_complete, validate, to_dot, to_ascii, to_table
- **NFA** (Non-deterministic Finite Automaton): epsilon transitions, epsilon closure, process/accepts, reset, to_dfa (subset construction), to_dot
- **PDA** (Pushdown Automaton): stack-augmented state machine for context-free languages, process/process_sequence/accepts, trace with stack snapshots
- **ModalStateMachine**: collection of named DFA sub-machines with mode switching, switch_mode/process/reset, mode trace
- **Minimize**: Hopcroft's algorithm for DFA minimization with unreachable state removal and partition refinement
- Ported from Go state-machine package with full feature parity
- Literate programming style with inline explanations and Chomsky hierarchy context
- Comprehensive busted test suite targeting 95%+ coverage
