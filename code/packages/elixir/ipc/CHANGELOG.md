# Changelog

## 0.1.0 — 2026-03-21

### Added
- `Pipe` struct with circular buffer, `pipe_write/2`, `pipe_read/2`, `close_read/1`, `close_write/1`, EOF and broken pipe detection.
- `MessageQueue` struct with `mq_send/3`, `mq_receive/2`, type-filtered receive, capacity limits.
- `SharedMemoryRegion` struct with `shm_attach/2`, `shm_detach/2`, `shm_read/3`, `shm_write/3`, bounds checking.
- `Manager` struct (IPCManager) for creating, retrieving, closing, and destroying pipes, message queues, and shared memory regions.
- `PipeHandle` and `Message` structs.
- Comprehensive ExUnit test suite covering all three IPC mechanisms and the manager.
