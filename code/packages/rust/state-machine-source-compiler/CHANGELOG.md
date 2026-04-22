# Changelog

## 0.1.0 - 2026-04-22

- Added the first Rust source generator for typed `StateMachineDefinition`
  values.
- Added validation for supported phase 1 machine kinds before source emission.
- Added deterministic Rust output for DFA, NFA, and PDA definition modules.
- Added snapshot and error-path tests covering generated helpers, escaping,
  semantic validation, and unsupported kinds.
