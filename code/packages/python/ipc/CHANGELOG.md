# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-21

### Added

- `Pipe` class: unidirectional byte stream using a circular buffer.
  Supports write, read, close_read, close_write, EOF detection, and
  BrokenPipeError on write to a pipe with no readers.
- `MessageQueue` class: FIFO queue of typed messages. Supports send,
  receive (any type or filtered by type), capacity limits, and message
  size validation.
- `SharedMemoryRegion` class: named shared memory segment. Supports
  attach/detach by PID, read/write at arbitrary offsets, and bounds
  checking.
- `IPCManager` class: central coordinator that creates, retrieves,
  and destroys pipes, message queues, and shared memory regions.
- Comprehensive test suite with 90%+ coverage.
