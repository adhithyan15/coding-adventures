# ipc

Inter-Process Communication (IPC) mechanisms for the coding-adventures OS stack.

## What It Does

Implements three classic IPC mechanisms that operating systems use to let isolated processes exchange data:

1. **Pipes** -- unidirectional byte streams backed by a circular buffer (4096 bytes). Support EOF detection (all writers closed) and broken pipe errors (all readers closed).

2. **Message Queues** -- FIFO queues of typed messages. Each message carries a type tag and a body (up to 4096 bytes). Receivers can filter by type.

3. **Shared Memory** -- named regions of memory that multiple processes can read and write directly. Zero-copy communication (no kernel buffer in between).

Plus an **IPCManager** that acts as the kernel component owning and tracking all IPC resources.

## Where It Fits

```
User Programs (pipe, msgget, shmget syscalls)
    |
    v
OS Kernel -- Syscall Dispatcher
    |
    v
IPC Manager  <-- THIS CRATE
    |-- Pipe (circular buffer)
    |-- MessageQueue (typed FIFO)
    |-- SharedMemoryRegion (named segments)
    |
    v
File System (D15) / Virtual Memory (D13)
```

## Usage

```rust
use ipc::{Pipe, MessageQueue, SharedMemoryRegion, IpcManager};

// Pipes
let mut pipe = Pipe::new(4096);
pipe.write(&[72, 101, 108, 108, 111]);  // "Hello"
let data = pipe.read(5);                 // => [72, 101, 108, 108, 111]

// Message Queues
let mut mq = MessageQueue::new(256, 4096);
mq.send(1, &[65, 66, 67]);
let msg = mq.receive(0);  // any type

// Shared Memory
let mut shm = SharedMemoryRegion::new("my_region".to_string(), 1024, 1);
shm.attach(1);
shm.write(0, &[1, 2, 3]).unwrap();
let data = shm.read(0, 3).unwrap();

// IPCManager
let mut mgr = IpcManager::new();
let (pipe_id, read_fd, write_fd) = mgr.create_pipe(4096);
mgr.create_message_queue("request_queue".to_string(), 256, 4096);
mgr.create_shared_memory("buffer_pool".to_string(), 4096, 1);
```
