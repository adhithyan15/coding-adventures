# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-21

### Added

- `ProcessState` module with READY, RUNNING, BLOCKED, TERMINATED, ZOMBIE constants
- `ProcessControlBlock` class with full PCB fields: registers, PC, SP, memory bounds, parent/children, signals, priority, cpu_time
- `Signal` module with POSIX signal constants (SIGINT=2, SIGKILL=9, SIGTERM=15, SIGCHLD=17, SIGCONT=18, SIGSTOP=19)
- `SignalManager` class with send_signal, deliver_pending, register_handler, mask, unmask, fatal? methods
- `ProcessManager` class with create_process, fork, exec, wait, kill, exit_process methods
- `PriorityScheduler` class with 40 priority levels, round-robin within levels, set_priority, time_quantum_for
- Comprehensive test suite covering PCB creation, state transitions, signal delivery/masking, fork/exec/wait lifecycle, and priority scheduling
