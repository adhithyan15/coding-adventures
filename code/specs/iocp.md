# iocp

## Overview

`iocp` is a thin Rust wrapper over Windows I/O Completion Ports. It does not
pretend to be a readiness API. It exposes completion-port creation, handle
association, queue waiting, and user-space completion posting directly.

This package is the Windows TCP-first raw backend crate in the initial native
event stack.

## Layer Position

```text
native-event-core
    ↓
iocp
    ↓
Windows I/O Completion Port APIs
```

## Concepts

- A completion port receives notifications for completed overlapped operations.
- Handles are explicitly associated with a completion port.
- Completion packets carry a completion key and an overlapped pointer.
- `PostQueuedCompletionStatus` is the user-space wakeup path.

## Public API

- `CompletionPacket`
- `CompletionPort::new()`
- `CompletionPort::associate_handle(handle, key)`
- `CompletionPort::post(bytes, key, overlapped)`
- `CompletionPort::get(timeout)`

## Data Flow

Input:

- Windows handles
- completion keys
- posted completion packets

Output:

- completion packets returned from the kernel or from user space

## Test Strategy

- Windows-only queue-post round-trip test
- non-Windows unsupported fallback test

## Future Extensions

- batch dequeue wrappers
- explicit overlapped-operation helper types
