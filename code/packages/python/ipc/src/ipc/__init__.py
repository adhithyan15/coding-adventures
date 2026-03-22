"""IPC (Inter-Process Communication) -- pipes, message queues, and shared memory.

Processes are isolated by design: each has its own address space, file
descriptors, and registers. But processes need to collaborate. A shell
pipeline like ``ls | grep foo | wc -l`` needs three processes passing data
in sequence. A database might use shared memory so query workers can read
cached pages without copying.

This package implements three classic IPC mechanisms, ordered from simplest
to most powerful:

1. **Pipe** -- a unidirectional byte stream (like a garden hose: water
   goes in one end, comes out the other).
2. **MessageQueue** -- a FIFO of typed messages (like a shared mailbox
   in the hallway: anyone can drop off or pick up labeled envelopes).
3. **SharedMemoryRegion** -- a region of memory visible to multiple
   processes (like a whiteboard between two rooms: fastest, but you
   need to take turns writing).

The **IPCManager** coordinates creation, lookup, and destruction of all
three types.

Modules:
    pipe           - Pipe: circular-buffer byte stream
    message_queue  - MessageQueue: typed message FIFO
    shared_memory  - SharedMemoryRegion: named memory segment
    ipc_manager    - IPCManager: central IPC coordinator

Quick start:
    >>> from ipc import Pipe, MessageQueue, SharedMemoryRegion, IPCManager
    >>> pipe = Pipe(capacity=64)
    >>> pipe.write(b"hello")
    5
    >>> pipe.read(5)
    b'hello'
"""

from ipc.ipc_manager import IPCManager
from ipc.message_queue import MessageQueue
from ipc.pipe import Pipe
from ipc.shared_memory import SharedMemoryRegion

__all__ = [
    "IPCManager",
    "MessageQueue",
    "Pipe",
    "SharedMemoryRegion",
]
