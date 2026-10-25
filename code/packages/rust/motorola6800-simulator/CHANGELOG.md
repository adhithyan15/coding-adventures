# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-10-24

### Added

- Complete execution for all 72 base-MC6800 operations and 198 documented
  opcode/addressing-mode encodings.
- Complete 64 KiB snapshots, checked program transport and state restoration,
  exact instruction traces, typed atomic faults, and transactional bounded
  execution.
- Big-endian data paths, condition-code behavior, descending stack, subroutine
  calls, software interrupts, interrupt return, and deterministic `WAI` halt.
