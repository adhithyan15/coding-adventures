# Changelog

All notable changes to the process-manager package will be documented in this file.

## [0.1.0] - 2026-03-21

### Added
- `ProcessControlBlock` with extended fields: parent/child relationships,
  pending signals, signal handlers, signal mask, priority, and CPU time.
- `ProcessState` enum: READY, RUNNING, BLOCKED, TERMINATED, ZOMBIE.
- `Signal` enum with POSIX signal numbers: SIGINT (2), SIGKILL (9),
  SIGTERM (15), SIGCHLD (17), SIGCONT (18), SIGSTOP (19).
- `SignalManager` for signal delivery, masking, handler registration,
  and pending signal processing.
- `ProcessManager` with full process lifecycle:
  - `create_process()` -- allocate a new PCB with a unique PID.
  - `fork()` -- clone a process (copies registers, PC, priority; resets
    children, pending signals, CPU time).
  - `exec()` -- replace process image (reset registers, set PC, clear
    signal handlers; keep PID, parent, children).
  - `wait()` -- reap zombie children and retrieve exit codes.
  - `kill()` -- send signals to processes.
  - `exit_process()` -- terminate a process, set ZOMBIE state, reparent
    children to PID 0, send SIGCHLD to parent.
- `PriorityScheduler` with priority-based scheduling (0=highest, 39=lowest)
  and round-robin within the same priority level.
- Comprehensive test suite with 90%+ coverage.
