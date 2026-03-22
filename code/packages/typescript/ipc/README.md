# @coding-adventures/ipc

Inter-Process Communication (IPC) mechanisms for the coding-adventures OS stack.

## What Is IPC?

Processes are isolated by design — separate address spaces, separate file descriptors. IPC is how they collaborate despite that isolation. This package implements three classic mechanisms:

1. **Pipes** — unidirectional byte streams backed by a circular buffer
2. **Message Queues** — FIFO queues of typed, discrete messages
3. **Shared Memory** — a byte region mapped into multiple process address spaces (zero-copy)

## Where It Fits

```
User Programs (shell pipelines, cooperating processes)
    ↓
OS Kernel — Syscall Dispatcher
    ↓
IPC Manager ← THIS PACKAGE
├── Pipe              — circular buffer, read/write ends
├── MessageQueue      — FIFO of typed messages
└── SharedMemoryRegion — named memory segments
    ↓
File System (D15) / Virtual Memory (D13)
```

## Usage

```typescript
import { Pipe, MessageQueue, SharedMemoryRegion, IPCManager } from "@coding-adventures/ipc";

// Pipe: byte stream between two processes
const pipe = new Pipe(4096);
pipe.write(new TextEncoder().encode("hello"));
const data = pipe.read(5); // Uint8Array containing "hello"

// Message Queue: typed, discrete messages
const mq = new MessageQueue();
mq.send(1, new TextEncoder().encode("request"));
const msg = mq.receive(1); // { msgType: 1, data: ..., size: 7 }

// Shared Memory: zero-copy data sharing
const region = new SharedMemoryRegion("pool", 4096, /*ownerPid=*/1);
region.attach(10);
region.write(0, new TextEncoder().encode("shared"));
region.read(0, 6); // "shared"

// IPCManager: kernel-level coordinator
const mgr = new IPCManager();
const handle = mgr.createPipe();
const queue = mgr.createMessageQueue("jobs");
const shm = mgr.createSharedMemory("buffer", 8192, 1);
```

## Running Tests

```bash
npm install
npx vitest run --coverage
```
