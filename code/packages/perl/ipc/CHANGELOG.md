# Changelog — ipc (Perl)

## 0.01 — 2026-03-31

### Added
- `Pipe` — circular buffer with broken-pipe and EOF detection
- `MessageQueue` — FIFO of typed messages with capacity limits
- `SharedMemory` — named memory region with attach/detach and bounds-checked read/write
- `Manager` — kernel IPC coordinator
- 95%+ test coverage via Test2::V0
