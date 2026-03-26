# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-21

### Added

- `Pipe` struct: unidirectional byte stream using a circular buffer.
  Supports Write, Read, CloseRead, CloseWrite, EOF detection, and
  ErrBrokenPipe on write to a pipe with no readers.
- `MessageQueue` struct: FIFO queue of typed messages. Supports Send,
  Receive (any type or filtered by type), capacity limits, and message
  size validation.
- `SharedMemoryRegion` struct: named shared memory segment. Supports
  Attach/Detach by PID, ReadAt/WriteAt at arbitrary offsets, and bounds
  checking.
- `IPCManager` struct: central coordinator that creates, retrieves,
  and destroys pipes, message queues, and shared memory regions.
- Comprehensive test suite with 80%+ coverage.
