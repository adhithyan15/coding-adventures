# Changelog — process-manager (Perl)

## 0.01 — 2026-03-31

### Added
- `PCB` — Process Control Block with state, registers, signals, priority, cpu_time
- `Manager` — spawn, fork, exec, wait_child, exit_process, kill, schedule, block/unblock
- Priority-based round-robin scheduler
- SIGCHLD sent to parent on child exit
- 95%+ test coverage via Test2::V0
