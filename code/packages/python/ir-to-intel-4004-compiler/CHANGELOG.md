# Changelog - coding-adventures-ir-to-intel-4004-compiler

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-14

### Added

- Renamed the old Intel 4004 backend package to `ir-to-intel-4004-compiler`
- Split hardware feasibility checks into `coding-adventures-intel-4004-ir-validator`
- Kept a facade class that validates IR then emits Intel 4004 assembly
