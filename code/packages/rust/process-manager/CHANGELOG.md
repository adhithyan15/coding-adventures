# Changelog

All notable changes to this crate will be documented in this file.

## [0.1.0] - 2026-03-21

### Added

- `ProcessState` enum with Ready, Running, Blocked, Terminated, Zombie variants
- `Signal` enum with SigInt(2), SigKill(9), SigTerm(15), SigChld(17), SigCont(18), SigStop(19)
- `ProcessControlBlock` struct with full PCB fields: registers, PC, SP, memory bounds, parent/children, signals, priority, cpu_time
- `SignalManager` struct with send_signal, deliver_pending, register_handler, mask, unmask, is_fatal methods
- `ProcessManager` struct with create_process, fork, exec, wait, kill, exit_process methods
- `PriorityScheduler` struct with 40 priority levels, round-robin within levels, set_priority, time_quantum_for
- Comprehensive test suite covering all components
