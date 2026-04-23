# Changelog

All notable changes to this package will be documented in this file.

## [0.2.0] - 2026-04-21

### Added

- `DYNAMIC_REL` / `defdynamic(...)` instruction for declaring dynamic
  predicates in standardized instruction streams
- assembly support that lowers dynamic declarations into
  `logic-engine.Program.dynamic_relations`
- validation and tests for dynamic declarations alongside facts, rules, and
  queries

## [0.1.0] - 2026-04-18

### Added

- initial `logic-instructions` package
- instruction kinds for relation declarations, facts, rules, and queries
- validation for declared relations and ground facts
- lowering into the current `logic-engine` backend
- end-to-end query execution helpers for instruction streams
