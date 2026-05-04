# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Added

- Extended serializer/deserializer round-trip tests to reconstruct executable
  DFA, NFA, and PDA machines from parsed `StateMachineDefinition` values.
- Added phase 1 transducer validation for `$any`, `$end`, transition actions,
  and `consume = false` EOF transitions.
- Added lexer-profile TOML lowering for `profile = "lexer/v1"` root fields,
  `[[tokens]]`, `[[inputs]]`, `[[registers]]`, `[[guards]]`, `[[fixtures]]`,
  inline matcher tables, and multiline string arrays.
- Added lexer-profile validation for duplicate token/register/input/guard
  identifiers, matcher references, done-state rules, and portable action/token
  references.
- Recognized the temporary-buffer conditional state-switch action in
  lexer-profile validation.

## [0.1.0] - 2026-04-20

### Added

- Initial package scaffolding for strict State Machine Markup deserialization.
- Added `.states.toml` parsing for the serializer's phase 1 TOML-compatible
  subset.
- Added semantic validation for DFA, NFA, and PDA definitions before they cross
  into the typed `StateMachineDefinition` layer.
- Added round-trip tests and hostile-input rejection coverage for malformed
  source, duplicate states, unknown references, duplicate DFA transitions,
  epsilon DFA transitions, and invalid PDA stack symbols.
