# ipc (Lua)

Inter-process communication for the coding-adventures simulated OS.

## What It Does

Implements three classic IPC mechanisms:

1. **Pipe** — Unidirectional byte stream backed by a circular buffer
2. **MessageQueue** — FIFO queue of typed messages with capacity limits
3. **SharedMemory** — Named memory region with byte-level read/write

Plus a **Manager** that acts as the kernel's IPC coordinator.

## Usage

```lua
local IPC = require("coding_adventures.ipc")

-- Pipe
local pipe = IPC.Pipe.new(64)
local _, pipe2, _ = pipe:write("hello")
local _, pipe3, data = pipe2:read(5)  -- data = "hello"

-- Message Queue
local q = IPC.MessageQueue.new()
local _, q2 = q:send(1, "ping")
local _, q3, msg = q2:receive(1)  -- msg.body = "ping"

-- Shared Memory
local shm = IPC.SharedMemory.new("buf", 1024, 100)
local _, shm2, _ = shm:write(0, "shared data")
local _, bytes = shm2:read(0, 11)  -- bytes = "shared data"

-- Manager
local mgr = IPC.Manager.new()
local mgr2, handle = mgr:create_pipe()
```

## Stack Position

```
User Programs (shell, daemons)
    │
    └── IPC Manager  ← this package
          ├── Pipe          (uses file descriptors)
          ├── MessageQueue  (kernel memory)
          └── SharedMemory  (virtual memory pages)
```
