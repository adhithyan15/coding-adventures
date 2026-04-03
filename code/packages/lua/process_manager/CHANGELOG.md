# Changelog — process_manager (Lua)

## 0.1.0 — 2026-03-31

### Added
- `PCB` — Process Control Block with state, registers, signals, and priority
- `Manager` — process table with spawn, fork, exec, wait, kill, block/unblock
- Priority-based round-robin scheduler (lower priority number = higher priority)
- Signal delivery: SIGKILL/SIGSTOP uncatchable, SIGCONT resumes blocked processes
- SIGCHLD automatically sent to parent when child exits
- Immutable (functional-style) API throughout
- 95%+ test coverage via busted test suite
