# Changelog

All notable changes to the `ipc` crate will be documented in this file.

## [0.1.0] - 2026-03-21

### Added
- `Pipe` struct: circular buffer byte stream with configurable capacity (default 4096), reader/writer reference counting, EOF detection, and BrokenPipe error.
- `MessageQueue` struct: FIFO queue of typed messages with configurable max_messages (256) and max_message_size (4096). Supports type-filtered receive.
- `SharedMemoryRegion` struct: named shared memory region with attach/detach tracking, random-access read/write, and bounds checking.
- `IpcManager` struct: kernel-level resource manager for pipes, message queues, and shared memory regions. Supports create, get, close/delete, and list operations.
- `IpcError` enum for structured error handling (BrokenPipe, OutOfBounds, QueueFull, MessageTooLarge, InvalidMessageType).
- Comprehensive test suite covering all IPC mechanisms and edge cases.
