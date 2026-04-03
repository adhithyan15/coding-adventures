# Changelog

All notable changes to this package will be documented in this file.

## [0.1.1] - 2026-04-02

### Changed

- Wrapped all public functions (`NewIPCManager`, `CreatePipe`, `GetPipe`,
  `ClosePipeRead`, `ClosePipeWrite`, `DestroyPipe`, `CreateMessageQueue`,
  `GetMessageQueue`, `DeleteMessageQueue`, `CreateSharedMemory`,
  `GetSharedMemory`, `DeleteSharedMemory`, `ListPipes`, `ListMessageQueues`,
  `ListSharedRegions`, `NewMessageQueue`, `Send`, `Receive`, `MessageCount`,
  `IsEmpty`, `IsFull`, `NewPipe`, `Write`, `Read`, `CloseRead`, `CloseWrite`,
  `Available`, `Space`, `IsEOF`, `Capacity`, `NewSharedMemoryRegion`,
  `Attach`, `Detach`, `ReadAt`, `WriteAt`, `Name`, `Size`, `OwnerPID`,
  `AttachedCount`, `IsAttached`) with the Operations system via `StartNew[T]`.
  Public API signatures are unchanged.

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
