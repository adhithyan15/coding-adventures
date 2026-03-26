# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-03-20

### Added

- Core types: `State`, `Event`, `Action`, `TransitionRecord`
- `DFA` class with full 5-tuple construction and validation
- `DFA.process()` and `DFA.process_sequence()` for event processing
- `DFA.accepts()` for language recognition (non-mutating)
- `DFA.reset()` to return to initial state
- Introspection: `reachable_states()`, `is_complete()`, `validate()`
- Visualization: `to_dot()` (Graphviz), `to_ascii()` (terminal table), `to_table()`
- Full transition tracing via `TransitionRecord`
- Action callbacks on transitions
- Comprehensive test suite with classic examples (turnstile, binary div-by-3, branch predictor)
- `NFA` class with epsilon transitions and non-deterministic processing
- `NFA.epsilon_closure()` for computing reachable states via epsilon
- `NFA.to_dfa()` subset construction algorithm (NFA → equivalent DFA)
- `minimize()` function implementing Hopcroft's DFA minimization algorithm
- `PushdownAutomaton` class with stack-based context-free language recognition
- `PDATransition` and `PDATraceEntry` for declarative PDA definition and tracing
- Classic PDA examples: balanced parentheses, a^n b^n
- `ModalStateMachine` class for context-sensitive mode switching
- `ModeTransitionRecord` for tracking mode switches
- Mode switching with automatic DFA reset on mode entry
- 161 tests across all modules with 99% coverage
