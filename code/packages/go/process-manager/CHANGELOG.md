# Changelog

All notable changes to the process-manager package will be documented in this file.

## [0.2.0] - 2026-04-02

### Changed
- Wrapped all public functions with the Operations system (`StartNew[T]`) for
  uniform observability, tracing, and error propagation across the package.
  Affected files: `process_manager.go`, `priority_scheduler.go`, `signals.go`.

## [0.1.0] - 2026-03-21

### Added
- `ProcessControlBlock` struct with extended fields: parent/child relationships,
  pending signals, signal handlers, priority, and CPU time.
- `ProcessState` constants: Ready, Running, Blocked, Terminated, Zombie.
- Signal constants with POSIX signal numbers: SIGINT (2), SIGKILL (9),
  SIGTERM (15), SIGCHLD (17), SIGCONT (18), SIGSTOP (19).
- `SignalManager` for signal delivery, handler registration, masking,
  and pending signal processing.
- `ProcessManager` with full process lifecycle:
  - `CreateProcess()` -- allocate a new PCB with a unique PID.
  - `Fork()` -- clone a process (copies registers, PC, priority; resets
    children, pending signals, CPU time).
  - `Exec()` -- replace process image (reset registers, set PC, clear
    signal handlers; keep PID, parent, children).
  - `Wait()` -- reap zombie children and retrieve exit codes.
  - `Kill()` -- send signals to processes.
  - `ExitProcess()` -- terminate a process, set Zombie state, reparent
    children to PID 0, send SIGCHLD to parent.
- `PriorityScheduler` with priority-based scheduling (0=highest, 39=lowest)
  and round-robin within the same priority level.
- Comprehensive test suite with 80%+ coverage.
