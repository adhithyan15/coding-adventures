# Changelog — process-manager (Perl)

## [Unreleased]

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for the sibling libraries) instead of an `echo ... skipping` recipe. The package is a pure in-memory simulation (no OS `fork`/`kill`), so it needs no platform-specific code.

### Fixed

- Two `SIGCHLD`/`SIGTERM queued` assertions passed their message into `grep`'s list, producing `Argument ... isn't numeric` warnings; the count is now taken with `scalar(grep ...)`.

## 0.01 — 2026-03-31

### Added
- `PCB` — Process Control Block with state, registers, signals, priority, cpu_time
- `Manager` — spawn, fork, exec, wait_child, exit_process, kill, schedule, block/unblock
- Priority-based round-robin scheduler
- SIGCHLD sent to parent on child exit
- 95%+ test coverage via Test2::V0
