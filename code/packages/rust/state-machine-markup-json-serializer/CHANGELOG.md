# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-20

### Added

- Added the initial canonical `.states.json` serializer package.
- Added deterministic JSON output for typed `StateMachineDefinition` values.
- Added golden coverage for DFA, NFA, PDA, epsilon, multi-target,
  stack-effect, and JSON string escaping behavior.
- Added canonical JSON output for transition `actions` and non-default
  `consume` flags used by transducer definitions.
