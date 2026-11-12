# Changelog — ipc (Perl)

## [Unreleased]

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries) instead of the previous skip or bare-`perl` recipe.

## 0.01 — 2026-03-31

### Added
- `Pipe` — circular buffer with broken-pipe and EOF detection
- `MessageQueue` — FIFO of typed messages with capacity limits
- `SharedMemory` — named memory region with attach/detach and bounds-checked read/write
- `Manager` — kernel IPC coordinator
- 95%+ test coverage via Test2::V0
