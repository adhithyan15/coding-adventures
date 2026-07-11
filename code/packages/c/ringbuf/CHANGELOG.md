# Changelog

All notable changes to `ringbuf` are documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C17 fixed-capacity ring buffer of ints (CCPP01 PR5):
  `ringbuf_init`, `push`, `pop`, `peek`, `count`, `capacity`, `is_empty`,
  `is_full`, `clear`. Caller-owned backing array, O(1) FIFO, no allocation.
- Tests (via the shared `iso-harness`) covering empty/full states, index
  wraparound, peek-vs-pop, and clear — compiled and run under GCC, Clang, and
  MSVC with strict ISO-conformance flags.
