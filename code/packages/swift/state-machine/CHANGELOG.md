# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-04

### Added
- `DFA` — Deterministic Finite Automaton with full 5-tuple definition, eager
  validation, single-event and sequence processing, acceptance checking,
  transition trace recording, and introspection (reachable states, completeness
  check, validation warnings).
- `NFA` — Non-deterministic Finite Automaton with epsilon transitions, epsilon
  closure computation, non-deterministic processing, acceptance checking, and
  subset construction (`toDfa()`) for converting to an equivalent DFA.
- `TransitionRecord` — immutable record of a single DFA transition step for
  execution tracing and debugging.
- `TransitionRule` / `NFATransitionRule` — convenience types for readable
  machine construction.
- `transitionKey(_:_:)` — encodes (state, event) pairs as dictionary keys.
- `stateSetName(_:)` — canonical naming for DFA states produced by subset
  construction.
- `DFAError` / `NFAError` — typed errors for all validation and processing
  failures.
- Comprehensive XCTest suite covering construction, processing, acceptance,
  introspection, reset, subset construction, and classic examples (turnstile,
  binary divisibility-by-3, 2-bit branch predictor).
