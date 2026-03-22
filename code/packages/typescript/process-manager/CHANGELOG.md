# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-21

### Added

- `ProcessState` enum: READY, RUNNING, BLOCKED, TERMINATED, ZOMBIE
- `Signal` enum: SIGINT (2), SIGKILL (9), SIGTERM (15), SIGCHLD (17), SIGCONT (18), SIGSTOP (19)
- `ProcessControlBlock` interface with full field set (pid, name, state, registers, pc, sp, memory_base, memory_size, parent_pid, children, pending_signals, signal_handlers, signal_mask, priority, cpu_time, exit_code)
- `createPCB()` factory function with sensible defaults (priority 20, 32 zeroed registers)
- `SignalManager` class: send_signal, deliver_pending, register_handler, mask, unmask, is_fatal
  - SIGKILL and SIGSTOP are uncatchable and unmaskable
  - SIGCHLD default action is ignore (non-fatal)
  - Signal masking defers delivery until unmasked
- `ProcessManager` class: create_process, fork, exec, wait, kill, exit_process
  - fork: clones PCB with new PID, child gets 0 return value, parent gets child PID
  - exec: resets registers/PC/SP, clears signal handlers, preserves PID/parent/children/priority
  - wait: finds and reaps first zombie child, returns PID and exit code
  - exit_process: sets ZOMBIE state, reparents children to init (PID 0), sends SIGCHLD to parent
  - kill: delegates to SignalManager, sends SIGCHLD on termination
- `PriorityScheduler` class: 40-level priority queues with round-robin within each level
  - schedule: picks highest-priority (lowest number) process
  - preempt: returns process to end of its priority queue
  - set_priority: moves process between priority queues
  - get_time_quantum: 200 cycles (priority 0) to 50 cycles (priority 39)
- Comprehensive test suite with 80%+ line coverage
- Knuth-style literate programming throughout all source files
