# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-21

### Added

- Process state atoms: `:ready`, `:running`, `:blocked`, `:terminated`, `:zombie`
- Signal atoms: `:sigint` (2), `:sigkill` (9), `:sigterm` (15), `:sigchld` (17), `:sigcont` (18), `:sigstop` (19)
- `PCB` struct with full field set (pid, name, process_state, registers, pc, sp, memory_base, memory_size, parent_pid, children, pending_signals, signal_handlers, signal_mask, priority, cpu_time, exit_code)
- `create_pcb/3` factory function with sensible defaults (priority 20, 32 zeroed registers)
- `SignalManager` module (all functions return new structs — immutable):
  - `send_signal/2`: SIGKILL/SIGSTOP immediate, others enqueued
  - `deliver_pending/1`: delivers unmasked signals, keeps masked ones pending
  - `register_handler/3`: custom handlers (SIGKILL/SIGSTOP uncatchable)
  - `mask_signal/2` / `unmask_signal/2`: block/unblock signal delivery
  - `is_fatal/1`: SIGCHLD and SIGCONT are non-fatal, others are fatal
- `Manager` module (functional state management):
  - `create_process/3`: sequential PID allocation
  - `fork/2`: clone PCB with new PID, register a0 differences
  - `exec/4`: reset registers/PC/SP, clear handlers, preserve identity
  - `wait_for_child/2`: find and reap first zombie child
  - `kill/3`: send signal, SIGCHLD on termination
  - `exit_process/3`: zombie state, reparent to init, SIGCHLD to parent
- `Scheduler` module: 40-level priority queues with round-robin
  - `enqueue/2`, `schedule/1`, `preempt/2`, `set_priority/3`
  - `get_time_quantum/1`: 200 cycles (priority 0) to 50 cycles (priority 39)
- Comprehensive ExUnit test suite with 80%+ coverage
- Knuth-style literate programming throughout
