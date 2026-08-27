# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-08-27

### Added

- The complete seven-operation Manchester Baby functional simulator, including
  both SUB encodings and pre-increment instruction fetch.
- Typed errors for invalid origins, stepping after STP, and exhausting the
  caller-provided execution bound.
- Owned architectural state and per-step trace records.
- Instruction, boundary, wraparound, self-modification, and loop tests.
