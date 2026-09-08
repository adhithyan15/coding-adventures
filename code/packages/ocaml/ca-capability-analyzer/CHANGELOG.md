# Changelog

All notable changes to this package are recorded here.

## 0.1.0 — 2026-09-07

### Added

- Compiler-libs parsing for implementation and interface files.
- Deterministic recursive package discovery with symlink rejection.
- Capability rules for filesystem, network, process, environment, FFI, time,
  standard input, and standard output access.
- Module-alias, supported-open, and lexical-shadowing resolution.
- Hard bans for `Obj`, unsafe Marshal operations, and closure serialization.
- Dual exception-and-capability opt-in for `external` and Dynlink boundaries.
- Strict schema-v1 manifest parsing with the closed Spec 13 taxonomy.
- Stable `CAP001` and `CAP002` findings, CLI exit codes, library API, shared
  behavior fixtures, package integration tests, and measured coverage gates.
