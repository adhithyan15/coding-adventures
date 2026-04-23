# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-20

### Added

- Added the initial strict `.states.json` deserializer package.
- Added bounded JSON profile parsing for typed state-machine definitions.
- Added round-trip and validation coverage for DFA, NFA, PDA, epsilon,
  literal event, stack-effect, malformed JSON, and limit-enforcement behavior.
- Added transition `actions` and `consume` field parsing for transducer
  definitions.

## [Unreleased]

### Added

- Added lexer-profile JSON lowering for root profile metadata, token/input/
  register/guard/fixture arrays, inline matcher objects, and transition guards.
