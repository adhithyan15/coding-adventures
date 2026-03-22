# CodingAdventures.IPC

Inter-Process Communication (IPC) mechanisms for the coding-adventures OS stack, implemented in Elixir.

## What Is IPC?

Processes are isolated by design — separate address spaces, separate file descriptors. IPC is how they collaborate despite that isolation. This package implements three classic mechanisms:

1. **Pipes** — unidirectional byte streams backed by a circular buffer
2. **Message Queues** — FIFO queues of typed, discrete messages
3. **Shared Memory** — a byte region mapped into multiple process address spaces (zero-copy)

## Where It Fits

```
User Programs (shell pipelines, cooperating processes)
    |
OS Kernel — Syscall Dispatcher
    |
IPC Manager <-- THIS PACKAGE
|-- Pipe              — circular buffer, read/write ends
|-- MessageQueue      — FIFO of typed messages
\-- SharedMemoryRegion — named memory segments
    |
File System (D15) / Virtual Memory (D13)
```

## Elixir Design Notes

Elixir data structures are immutable, so all operations return updated structs rather than mutating in place. This mirrors how the OS kernel would handle IPC state in a functional style:

```elixir
pipe = CodingAdventures.IPC.new_pipe(4096)
{:ok, pipe, 5} = CodingAdventures.IPC.pipe_write(pipe, "hello")
{:ok, pipe, data} = CodingAdventures.IPC.pipe_read(pipe, 5)
# data == "hello"

mq = CodingAdventures.IPC.new_message_queue()
{:ok, mq} = CodingAdventures.IPC.mq_send(mq, 1, "request")
{:ok, mq, msg} = CodingAdventures.IPC.mq_receive(mq, 0)

region = CodingAdventures.IPC.new_shared_memory("pool", 4096, 1)
{:ok, region, 5} = CodingAdventures.IPC.shm_write(region, 0, "hello")
{:ok, data} = CodingAdventures.IPC.shm_read(region, 0, 5)
```

## Running Tests

```bash
mix deps.get
mix test
```
