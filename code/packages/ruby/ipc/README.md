# coding_adventures_ipc

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
IPC Manager  <-- THIS PACKAGE
    |-- Pipe (circular buffer)
    |-- MessageQueue (typed FIFO)
    |-- SharedMemoryRegion (named segments)
    |
    v
File System (D15) / Virtual Memory (D13)
```

## Usage

```ruby
require "coding_adventures_ipc"

# Pipes
pipe = CodingAdventures::Ipc::Pipe.new
pipe.write([72, 101, 108, 108, 111])  # "Hello"
pipe.read(5)  # => [72, 101, 108, 108, 111]

# Message Queues
mq = CodingAdventures::Ipc::MessageQueue.new
mq.send(1, [65, 66, 67])
msg = mq.receive(1)  # => Message(msg_type=1, body=[65, 66, 67])

# Shared Memory
shm = CodingAdventures::Ipc::SharedMemoryRegion.new(
  name: "my_region", size: 1024, owner_pid: 1
)
shm.attach(1)
shm.write(0, [1, 2, 3])
shm.read(0, 3)  # => [1, 2, 3]

# IPCManager (kernel-level resource management)
mgr = CodingAdventures::Ipc::IpcManager.new
pipe_id, read_fd, write_fd = mgr.create_pipe
mgr.create_message_queue("request_queue")
mgr.create_shared_memory("buffer_pool", size: 4096, owner_pid: 1)
```

## Development

```bash
bundle install
bundle exec rake test
```
