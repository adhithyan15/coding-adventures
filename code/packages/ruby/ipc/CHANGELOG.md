# Changelog

All notable changes to the `coding_adventures_ipc` gem will be documented in this file.

## [0.1.0] - 2026-03-21

### Added
- `Pipe` class: circular buffer byte stream with configurable capacity (default 4096), reader/writer reference counting, EOF detection, and BrokenPipeError.
- `MessageQueue` class: FIFO queue of typed messages with configurable max_messages (256) and max_message_size (4096). Supports type-filtered receive.
- `SharedMemoryRegion` class: named shared memory region with attach/detach tracking, random-access read/write, and bounds checking.
- `IpcManager` class: kernel-level resource manager for pipes, message queues, and shared memory regions. Supports create, get, close/delete, and list operations.
- Comprehensive test suite covering all IPC mechanisms and edge cases.
