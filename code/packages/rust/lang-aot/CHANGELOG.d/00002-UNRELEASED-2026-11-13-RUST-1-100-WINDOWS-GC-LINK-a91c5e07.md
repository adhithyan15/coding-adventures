## Unreleased - 2026-11-13 - Link synchronization in the Windows gc-core test helper

### Fixed

- `tests/common` `gc_link_args` now also links `synchronization` on Windows.
  Rust 1.100's std uses `WaitOnAddress`/`WakeByAddress*` for its futex
  Mutex/Condvar, so clang's link of the gc-core-capi staticlib failed with
  LNK1120 in `array_ref_tracing`.
