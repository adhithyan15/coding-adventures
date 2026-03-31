# Changelog — ipc (Lua)

## 0.1.0 — 2026-03-31

### Added
- `Pipe` — circular buffer byte stream with read/write ends, EOF and broken-pipe detection
- `MessageQueue` — FIFO of typed messages with capacity limits and type-filtered receive
- `SharedMemory` — named byte region with attach/detach and bounds-checked read/write
- `Manager` — kernel IPC coordinator managing all pipe, queue, and shm resources
- Immutable (functional-style) API throughout
- 95%+ test coverage via busted test suite
