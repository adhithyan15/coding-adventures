# IPC (Inter-Process Communication)

Simulates three classic IPC mechanisms found in Unix-like operating systems:

1. **Pipes** -- unidirectional byte streams using a circular buffer
2. **Message Queues** -- FIFO queues of typed messages
3. **Shared Memory** -- named memory regions accessible by multiple processes

## Where It Fits

This package sits in the OS kernel layer. Pipes expose byte streams via file
descriptors. Message queues provide structured, typed communication between
unrelated processes. Shared memory offers zero-copy data sharing at the cost
of requiring manual synchronization.

```
User Programs
    |   pipe(), write(), read()
    |   msgget(), msgsnd(), msgrcv()
    |   shmget(), shmat(), shmdt()
    v
OS Kernel -- Syscall Dispatcher
    v
IPC Manager  <-- THIS PACKAGE
    |-- Pipe (circular buffer)
    |-- MessageQueue (FIFO of typed messages)
    +-- SharedMemoryRegion (named memory segments)
```

## Usage

```python
from ipc import Pipe, MessageQueue, SharedMemoryRegion, IPCManager

# Pipe: unidirectional byte stream
pipe = Pipe(capacity=4096)
pipe.write(b"hello")
data = pipe.read(5)  # b"hello"

# Message queue: typed messages
mq = MessageQueue(max_messages=256)
mq.send(msg_type=1, data=b"request")
msg = mq.receive(msg_type=1)  # (1, b"request")

# Shared memory: direct read/write
shm = SharedMemoryRegion("buffer", size=4096, owner_pid=1)
shm.attach(pid=1)
shm.write(offset=0, data=b"shared data")
result = shm.read(offset=0, count=11)  # b"shared data"

# IPCManager: coordinates all IPC resources
mgr = IPCManager()
pipe_id, read_fd, write_fd = mgr.create_pipe()
mq = mgr.create_message_queue("work_queue")
region = mgr.create_shared_memory("cache", size=8192, owner_pid=1)
```

## Testing

```bash
uv venv && uv pip install -e ".[dev]"
.venv/bin/python -m pytest tests/ -v
```
