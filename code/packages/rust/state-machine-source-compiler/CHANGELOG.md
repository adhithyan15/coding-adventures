# Changelog

## 0.1.0 - 2026-04-22

- Added the first Rust source generator for typed `StateMachineDefinition`
  values.
- Added validation for supported phase 1 machine kinds before source emission.
- Added deterministic Rust output for DFA, NFA, and PDA definition modules.
- Added snapshot and error-path tests covering generated helpers, escaping,
  semantic validation, and unsupported kinds.
- Added end-to-end generated-code tests that compile temporary Rust wrapper
  crates and exercise generated DFA, NFA, and PDA constructors when the local
  Rust toolchain can execute freshly built binaries.
- Added Rust source generation for effectful transducer definitions, including
  transition actions, consume flags, and an `EffectfulStateMachine` constructor.
- Added lexer-profile Rust source generation for version/profile/runtime
  metadata, token/input/register/guard declarations, fixtures, and matcher-only
  transducer transitions.
