# Changelog

## 0.1.0 — 2026-03-21

### Added
- `Pipe` class: circular buffer-backed unidirectional byte stream with reader/writer reference counting, EOF detection, and BrokenPipeError.
- `MessageQueue` class: FIFO queue of typed messages with capacity limits, oversized message rejection, and type-filtered receive.
- `SharedMemoryRegion` class: named byte region with PID-based attach/detach, offset-based read/write, and bounds checking.
- `IPCManager` class: kernel-level coordinator for creating, retrieving, closing, and destroying pipes, message queues, and shared memory regions.
- `BrokenPipeError` and `IPCError` custom error types.
- Comprehensive test suite covering all three IPC mechanisms and the manager (90%+ coverage target).
