# Changelog

All notable changes to this package will be documented in this file.

## [0.2.0] - 2026-04-21

### Added

- VM handler for `DYNAMIC_REL` instructions
- runtime tracking for dynamic relation declarations
- assembled engine programs now preserve dynamic predicate metadata
- tests covering dynamic declaration loading, reset behavior, and handler
  validation

## [0.1.0] - 2026-04-18

### Added

- dispatch-table `LogicVM` runtime for LP07 instruction streams
- incremental loading of relation declarations, facts, rules, and queries
- trace entries for step-by-step runtime inspection
- convenience helpers to execute one or all stored queries through the VM
