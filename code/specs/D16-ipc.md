# D16 — Inter-Process Communication (IPC)

## Overview

Processes are isolated by design. Each process has its own virtual address space,
its own file descriptors, its own registers. This isolation is essential for
stability (a buggy program cannot corrupt another program's memory) and security
(a malicious program cannot read another program's secrets).

But isolation creates a problem: **how do processes collaborate?** A web server
might fork worker processes that all need to share a request queue. A shell
pipeline like `ls | grep foo | wc -l` needs three processes to pass data in
sequence. A database might use shared memory for its buffer pool so multiple
query workers can read cached pages without copying.

**Inter-Process Communication (IPC)** is the set of mechanisms the OS provides
for processes to exchange data despite their isolation. This package implements
three classic IPC mechanisms, ordered from simplest to most powerful:

1. **Pipes** — unidirectional byte streams (simple, limited to related
   processes unless named)
2. **Message Queues** — FIFO queues of typed messages (decoupled, any process
   can send/receive)
3. **Shared Memory** — a region of memory mapped into multiple address spaces
   (fastest, but requires explicit synchronization)

**Analogy:** Imagine two people in separate, soundproofed rooms.
- A **pipe** is a pneumatic tube between the rooms — you stuff a message in
  one end, it comes out the other.
- A **message queue** is a shared mailbox in the hallway — anyone can drop off
  or pick up labeled envelopes.
- **Shared memory** is a window between the rooms with a whiteboard visible to
  both — fastest communication, but you need to take turns writing or you will
  get garbled text.

## Where It Fits

```
User Programs
│   pipe(fds)               — create a pipe
│   write(fds[1], data)     — send bytes through pipe
│   read(fds[0], buf)       — receive bytes from pipe
│   msgget(key)             — create/open message queue
│   msgsnd(id, msg)         — send a message
│   msgrcv(id, buf)         — receive a message
│   shmget(key, size)       — create/open shared memory
│   shmat(id)               — attach to address space
▼
OS Kernel — Syscall Dispatcher
▼
IPC Manager ← YOU ARE HERE
│   ├── Pipe              — circular buffer, read/write ends
│   ├── MessageQueue      — FIFO of typed messages
│   └── SharedMemoryRegion — named memory segments
▼
File System (D15)           Virtual Memory (D13)
│   pipes use file          │   shared memory maps
│   descriptors (fds)       │   pages into address spaces
▼                           ▼
Block Device / RAM
```

**Depends on:** File System (D15) — pipes are exposed as file descriptors;
Virtual Memory (D13) — shared memory maps pages into process address spaces

**Used by:** OS Kernel (syscall handlers), Shell (pipe operator `|`), any
cooperating processes

## Key Concepts

### Pipes: The Simplest IPC

A pipe is a unidirectional byte stream connecting two file descriptors: one for
reading, one for writing. Data written to the write end appears at the read end,
in order, exactly once.

```
Process A (writer)                    Process B (reader)
┌──────────────┐                     ┌──────────────┐
│ write(fd_w,  │                     │ read(fd_r,   │
│   "hello")   │                     │   buf, 5)    │
└──────┬───────┘                     └──────▲───────┘
       │                                    │
       ▼                                    │
  ┌────────────────────────────────────────────┐
  │           Pipe (Circular Buffer)            │
  │                                             │
  │  ┌───┬───┬───┬───┬───┬───┬───┬───┬───┐     │
  │  │ h │ e │ l │ l │ o │   │   │   │   │     │
  │  └───┴───┴───┴───┴───┴───┴───┴───┴───┘     │
  │    ▲ read_pos              ▲ write_pos       │
  │                                             │
  │  Capacity: 4096 bytes                       │
  │  Readers: 1    Writers: 1                    │
  └─────────────────────────────────────────────┘
```

#### Pipe Data Structure

```
Pipe
════

  ┌──────────────────┬────────────────────────────────────────────────┐
  │ Field            │ Description                                    │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ buffer           │ Circular byte buffer, fixed size of 4096       │
  │                  │ bytes. Data wraps around: when write_pos       │
  │                  │ reaches the end, it wraps to index 0.          │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ capacity         │ 4096 bytes. Chosen to match one memory page —  │
  │                  │ a common convention in Unix systems.            │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ read_pos         │ Index of the next byte to be read. Advances    │
  │                  │ on each read, wraps around.                     │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ write_pos        │ Index of the next byte to be written. Advances │
  │                  │ on each write, wraps around.                    │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ reader_count     │ Number of open file descriptors for the read   │
  │                  │ end. When this drops to 0, writes will fail     │
  │                  │ with EPIPE ("broken pipe").                      │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ writer_count     │ Number of open file descriptors for the write  │
  │                  │ end. When this drops to 0, reads return EOF     │
  │                  │ (0 bytes) — the reader knows no more data is    │
  │                  │ coming.                                         │
  └──────────────────┴────────────────────────────────────────────────┘
```

#### Pipe Semantics

The behavior of pipe read/write depends on the state of the buffer and the
reference counts:

```
Write Behavior
══════════════

  Buffer has space?    Writers > 0?    Action
  ─────────────────    ────────────    ──────────────────────────
  Yes                  Yes             Write bytes, advance write_pos
  No (buffer full)     Yes             BLOCK — wait until reader drains
  (any)                No              (impossible — we are a writer)

  Reader count = 0?    Action
  ────────────────     ──────────────────────────
  Yes                  Return error EPIPE (broken pipe)
  No                   Write normally


Read Behavior
═════════════

  Buffer has data?     Writers > 0?    Action
  ─────────────────    ────────────    ──────────────────────────
  Yes                  (any)           Read bytes, advance read_pos
  No (buffer empty)    Yes             BLOCK — wait until writer produces
  No (buffer empty)    No              Return 0 (EOF — pipe is done)
```

**Why EOF when all writers close:** This is how shell pipelines terminate.
In `cat file.txt | grep hello`, when `cat` finishes and closes its write end
of the pipe, `grep` sees EOF on its read end and knows there is no more input.

#### Circular Buffer Mechanics

The circular buffer uses modular arithmetic to wrap around:

```
Initial state (empty):
  read_pos = 0, write_pos = 0
  Available to read = 0
  Available to write = 4096

After writing "hello" (5 bytes):
  read_pos = 0, write_pos = 5
  Available to read = 5
  Available to write = 4091

After reading 3 bytes ("hel"):
  read_pos = 3, write_pos = 5
  Available to read = 2
  Available to write = 4094

Wrapping example (buffer size = 8 for illustration):
  ┌───┬───┬───┬───┬───┬───┬───┬───┐
  │   │   │   │ d │ e │   │   │   │
  └───┴───┴───┴───┴───┴───┴───┴───┘
                ▲ read    ▲ write
                pos=3     pos=5

  Write "fghij" (5 bytes, wraps around):
  ┌───┬───┬───┬───┬───┬───┬───┬───┐
  │ i │ j │   │ d │ e │ f │ g │ h │
  └───┴───┴───┴───┴───┴───┴───┴───┘
        ▲ write   ▲ read
        pos=2     pos=3

  bytes_used = (write_pos - read_pos + capacity) % capacity
             = (2 - 3 + 8) % 8 = 7
```

### Named Pipes (FIFOs)

A regular pipe is anonymous — it only exists as a pair of file descriptors and
can only be shared between a parent process and its children (who inherit the
descriptors via `fork()`). A **named pipe** (FIFO) is a pipe that has a name
in the file system, so unrelated processes can use it:

```
Regular pipe:    pipe(fds) → fds[0] and fds[1], no file system entry
Named pipe:      mkfifo("/tmp/my_pipe") → appears in directory listing
                 Process A: open("/tmp/my_pipe", WRITE)
                 Process B: open("/tmp/my_pipe", READ)
```

Named pipes use the file system's inode with `file_type = PIPE`. Opening a
named pipe creates a Pipe object (same circular buffer as above) and returns
file descriptors. The data never touches the disk — it flows through the
in-memory buffer.

### Message Queues: Structured Communication

While pipes transmit raw bytes (the reader must know how to parse them),
message queues transmit discrete, typed **messages**. Each message has a type
tag and a body, and the receiver can filter by type.

```
Message Queue
═════════════

  Process A                              Process B
  msgsnd(qid, type=1, "request")         msgrcv(qid, type=1, buf)
       │                                      ▲
       ▼                                      │
  ┌────────────────────────────────────────────┐
  │           Message Queue (FIFO)              │
  │                                             │
  │  ┌─────────────────┐                        │
  │  │ type=1 "request"│ ← oldest (dequeued     │
  │  ├─────────────────┤    next)                │
  │  │ type=2 "status" │                        │
  │  ├─────────────────┤                        │
  │  │ type=1 "query"  │ ← newest              │
  │  └─────────────────┘                        │
  │                                             │
  │  max_messages: 256                          │
  │  max_message_size: 4096 bytes               │
  └─────────────────────────────────────────────┘
```

#### Message Structure

```
Message
═══════

  ┌──────────────────┬────────────────────────────────────────────────┐
  │ Field            │ Description                                    │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ msg_type         │ Positive integer identifying the message kind. │
  │                  │ Receivers can filter: "give me only type 3     │
  │                  │ messages." Type 0 means "give me any message." │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ body             │ The message payload — up to 4096 bytes of      │
  │                  │ arbitrary data.                                 │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ size             │ Actual size of the body in bytes.               │
  └──────────────────┴────────────────────────────────────────────────┘
```

#### MessageQueue Structure

```
MessageQueue
════════════

  ┌──────────────────┬────────────────────────────────────────────────┐
  │ Field            │ Description                                    │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ id               │ Unique identifier for this queue, returned     │
  │                  │ by sys_msgget.                                  │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ key              │ A well-known integer that unrelated processes   │
  │                  │ use to find this queue. Think of it like a      │
  │                  │ phone number — if two processes agree on the    │
  │                  │ key, they can communicate.                      │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ messages         │ FIFO queue (VecDeque) of Message objects.      │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ max_messages     │ 256. Maximum number of messages the queue can   │
  │                  │ hold. send() blocks when the queue is full.     │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ max_message_size │ 4096 bytes. Messages larger than this are       │
  │                  │ rejected.                                       │
  └──────────────────┴────────────────────────────────────────────────┘
```

#### Send/Receive Semantics

```
msgsnd(queue_id, message)
  1. Validate message.size <= max_message_size.
  2. If queue is full (messages.len() == max_messages): BLOCK.
  3. Push message to the back of the FIFO.

msgrcv(queue_id, msg_type, buffer)
  1. If msg_type == 0: dequeue the first message of any type.
  2. If msg_type > 0: dequeue the first message with matching type
     (skipping non-matching messages, which remain in the queue).
  3. If no matching message exists: BLOCK until one arrives.
  4. Copy message body into buffer, return the size.
```

### Shared Memory: Zero-Copy Communication

Pipes and message queues both **copy** data: the sender writes bytes, the
kernel copies them into a buffer, and the receiver copies them out. For
large data transfers, this double-copy is expensive.

Shared memory eliminates copying entirely. Two processes map the **same
physical pages** into their virtual address spaces. A write by one process is
immediately visible to the other — no system call, no copy, no kernel
involvement (after setup).

```
Process A's Virtual Address Space     Process B's Virtual Address Space
┌──────────────────────────────┐     ┌──────────────────────────────┐
│ ...                          │     │ ...                          │
│ 0x8000 ┌────────────────┐    │     │ 0xC000 ┌────────────────┐   │
│        │ Shared Region  │◄───┼─────┼────────│ Shared Region  │   │
│        │ "Hello from A" │    │     │        │ "Hello from A" │   │
│        └────────────────┘    │     │        └────────────────┘   │
│ ...                          │     │ ...                          │
└──────────────────────────────┘     └──────────────────────────────┘
                │                                    │
                └──────────┬─────────────────────────┘
                           │
                    ┌──────▼──────┐
                    │  Physical   │
                    │  Page Frame │  ← same physical memory
                    │  #42        │
                    └─────────────┘
```

#### SharedMemoryRegion Structure

```
SharedMemoryRegion
══════════════════

  ┌──────────────────┬────────────────────────────────────────────────┐
  │ Field            │ Description                                    │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ id               │ Unique identifier, returned by sys_shmget.     │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ key              │ Well-known integer for finding this segment,   │
  │                  │ like message queue keys.                        │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ size             │ Size in bytes. Rounded up to page boundary.    │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ data             │ The actual shared bytes — a Vec<u8> that       │
  │                  │ represents the physical page(s).                │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ attached_pids    │ Set of process IDs currently attached. Used    │
  │                  │ for cleanup: when the last process detaches,   │
  │                  │ the segment can be destroyed.                   │
  └──────────────────┴────────────────────────────────────────────────┘
```

#### Shared Memory Operations

```
shmget(key, size) → shm_id
  1. If a segment with this key exists, return its id.
  2. Otherwise, create a new segment:
     a. Allocate a Vec<u8> of the requested size (zero-initialized).
     b. Assign a unique id.
     c. Store in the IPCManager's segment table.
  3. Return the id.

shmat(shm_id, process_id) → virtual_address
  1. Look up the segment by id.
  2. Map the segment into the process's address space:
     - In a real OS, this modifies the page table to point to the
       shared physical pages.
     - In our simulation, we record the mapping and provide
       read/write methods that access the shared data Vec.
  3. Add process_id to attached_pids.
  4. Return the virtual address where the segment was mapped.

shmdt(shm_id, process_id)
  1. Remove process_id from attached_pids.
  2. Unmap the segment from the process's address space.
  3. If attached_pids is empty and the segment is marked for
     destruction, free it.

read_shared(shm_id, offset, length) → bytes
  Read directly from the shared data Vec.

write_shared(shm_id, offset, data)
  Write directly into the shared data Vec.

WARNING: Shared memory has NO built-in synchronization. If process A
writes while process B reads, B may see partially-updated data. Real
programs use semaphores or mutexes to coordinate access. We leave
synchronization out of this spec for simplicity, but note the hazard.
```

### Signals (Cross-Reference)

Signals are another form of IPC — asynchronous notifications sent to a process.
They are covered in detail in D14 (Device Driver Framework / Interrupt Handler).
The key signals relevant to IPC:

- **SIGPIPE (13)**: Sent to a process that writes to a pipe with no readers.
- **SIGCHLD (17)**: Sent to a parent when a child process exits.

Signals are "lightweight IPC" — they carry no data payload, just a signal
number. For actual data exchange, use pipes, message queues, or shared memory.

### IPCManager

The `IPCManager` is the kernel component that owns all IPC resources:

```
IPCManager
══════════

  ┌──────────────────┬────────────────────────────────────────────────┐
  │ Field            │ Description                                    │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ pipes            │ Vec<Pipe> — all active pipes in the system.    │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ message_queues   │ HashMap<key, MessageQueue> — all message       │
  │                  │ queues, keyed by their well-known key.          │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ shared_segments  │ HashMap<key, SharedMemoryRegion> — all shared  │
  │                  │ memory segments.                                │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ next_pipe_id     │ Counter for assigning unique pipe IDs.          │
  │ next_queue_id    │ Counter for assigning unique queue IDs.         │
  │ next_shm_id      │ Counter for assigning unique segment IDs.       │
  └──────────────────┴────────────────────────────────────────────────┘

Methods:
  create_pipe() → (read_fd, write_fd)
  create_message_queue(key) → queue_id
  create_shared_memory(key, size) → shm_id
  destroy_pipe(pipe_id)
  destroy_message_queue(queue_id)
  destroy_shared_memory(shm_id)
```

## Algorithms

### Pipe Creation (sys_pipe)

```
sys_pipe(fds_ptr)
═════════════════

1. Create a new Pipe with empty circular buffer (4096 bytes).
2. Set reader_count = 1, writer_count = 1.
3. Create two OpenFile entries in the system-wide OpenFileTable:
   a. One for reading (flags = READ), linked to the pipe.
   b. One for writing (flags = WRITE), linked to the pipe.
4. Allocate two file descriptors in the calling process's
   FileDescriptorTable:
   a. fds[0] = read end
   b. fds[1] = write end
5. Write fds[0] and fds[1] to the memory address fds_ptr.
6. Return 0 on success.

After fork():
  The child inherits copies of the parent's file descriptors.
  Both parent and child now have fds[0] and fds[1].
  reader_count = 2, writer_count = 2.
  Typically, the parent closes the read end and the child closes the
  write end (or vice versa) to establish a one-way channel.
```

### Shell Pipeline: Putting It Together

Here is how `ls | grep foo` works using pipes:

```
Shell process:
  1. sys_pipe(fds)  → fds[0]=read, fds[1]=write
  2. fork() → child1 (will run "ls")
     In child1:
       sys_dup2(fds[1], 1)   — redirect stdout to pipe write end
       sys_close(fds[0])     — close unused read end
       sys_close(fds[1])     — close original write fd (dup2 made a copy)
       exec("ls")
  3. fork() → child2 (will run "grep foo")
     In child2:
       sys_dup2(fds[0], 0)   — redirect stdin to pipe read end
       sys_close(fds[1])     — close unused write end
       sys_close(fds[0])     — close original read fd
       exec("grep foo")
  4. In shell:
       sys_close(fds[0])     — shell does not use the pipe
       sys_close(fds[1])
       wait for child1 and child2

Data flow:
  ls writes to fd 1 (which is the pipe write end)
  │
  └──→ Pipe circular buffer ──→ grep reads from fd 0 (pipe read end)
```

### Blocking Behavior

In our simplified kernel, "blocking" means the process is moved to a WAITING
state and the scheduler runs another process. When the blocking condition is
resolved (e.g., data arrives in the pipe), the waiting process is moved back
to READY.

```
Blocking scenarios:
  ┌────────────────────┬──────────────────────────────────────────────┐
  │ Operation          │ Blocks when...                               │
  ├────────────────────┼──────────────────────────────────────────────┤
  │ Pipe read          │ Buffer is empty AND writer_count > 0         │
  │ Pipe write         │ Buffer is full AND reader_count > 0          │
  │ Message receive    │ No matching message in queue                  │
  │ Message send       │ Queue is at max_messages capacity             │
  └────────────────────┴──────────────────────────────────────────────┘
```

## Syscalls

```
Syscall Table Additions
═══════════════════════

  ┌──────────────┬─────────┬──────────────────────────────────────────┐
  │ Name         │ Number  │ Arguments                                │
  ├──────────────┼─────────┼──────────────────────────────────────────┤
  │ sys_pipe     │ 22      │ (fds_ptr) → 0                            │
  │              │         │ Creates a pipe. Writes read fd and write  │
  │              │         │ fd to fds_ptr[0] and fds_ptr[1].          │
  ├──────────────┼─────────┼──────────────────────────────────────────┤
  │ sys_shmget   │ 29      │ (key, size) → shm_id                     │
  │              │         │ Create or find shared memory segment      │
  │              │         │ identified by key.                         │
  ├──────────────┼─────────┼──────────────────────────────────────────┤
  │ sys_shmat    │ 30      │ (shm_id) → virtual_address                │
  │              │         │ Attach shared memory segment to calling   │
  │              │         │ process's address space.                   │
  ├──────────────┼─────────┼──────────────────────────────────────────┤
  │ sys_shmdt    │ 67      │ (shm_id) → 0                             │
  │              │         │ Detach shared memory from calling process.│
  ├──────────────┼─────────┼──────────────────────────────────────────┤
  │ sys_msgget   │ 68      │ (key) → queue_id                         │
  │              │         │ Create or find message queue identified   │
  │              │         │ by key.                                    │
  ├──────────────┼─────────┼──────────────────────────────────────────┤
  │ sys_msgsnd   │ 69      │ (queue_id, msg_ptr, msg_size, msg_type)  │
  │              │         │ → 0                                       │
  │              │         │ Send a message to the queue.               │
  ├──────────────┼─────────┼──────────────────────────────────────────┤
  │ sys_msgrcv   │ 70      │ (queue_id, buf_ptr, buf_size, msg_type)  │
  │              │         │ → bytes_received                          │
  │              │         │ Receive a message from the queue. If      │
  │              │         │ msg_type=0, receive any; otherwise match. │
  └──────────────┴─────────┴──────────────────────────────────────────┘
```

## Comparing the Three Mechanisms

```
  ┌───────────────────┬────────────┬────────────────┬─────────────────┐
  │                   │ Pipe       │ Message Queue  │ Shared Memory   │
  ├───────────────────┼────────────┼────────────────┼─────────────────┤
  │ Data format       │ Raw bytes  │ Typed messages │ Raw bytes       │
  │ Direction         │ Uni        │ Any-to-any     │ Any-to-any      │
  │ Copies            │ 2 (w→buf,  │ 2 (w→buf,     │ 0 (direct       │
  │                   │  buf→r)    │  buf→r)        │  access)        │
  │ Ordering          │ FIFO       │ FIFO per type  │ None (random)   │
  │ Persistence       │ None (in   │ Until deleted  │ Until deleted   │
  │                   │  memory)   │                │                 │
  │ Synchronization   │ Built-in   │ Built-in       │ None (manual)   │
  │ Max throughput    │ Medium     │ Medium         │ Highest         │
  │ Complexity        │ Low        │ Medium         │ High            │
  │ Related processes │ Required*  │ No             │ No              │
  │ only?             │            │                │                 │
  └───────────────────┴────────────┴────────────────┴─────────────────┘

  * Unless using named pipes (FIFOs), which are accessible by any process
    that knows the path.
```

## Dependencies

```
D16 IPC
│
├── depends on ──→ File System (D15)
│                   └── Pipes use file descriptors (fd 0/1/2/3+)
│                   └── Named pipes have inodes with type PIPE
│
├── depends on ──→ Virtual Memory (D13)
│                   └── Shared memory maps physical pages into
│                       virtual address spaces
│
└── used by ───→ OS Kernel (syscall dispatch)
                  └── Shell (pipe operator |)
                  └── Cooperating processes
```

## Testing Strategy

### Unit Tests

1. **Pipe creation**: Call `create_pipe()`, verify two valid fds are returned,
   one readable and one writable.
2. **Pipe write/read**: Write bytes, read them back, verify order and contents.
3. **Pipe FIFO ordering**: Write "abc" then "def", read 6 bytes, verify "abcdef".
4. **Pipe circular wrap**: Write enough data to wrap the circular buffer,
   read it all back, verify correctness.
5. **Pipe EOF**: Close all write ends, verify read returns 0 bytes.
6. **Pipe broken pipe**: Close all read ends, verify write returns EPIPE error.
7. **Pipe blocking (empty)**: Read from empty pipe with active writers, verify
   process blocks (moves to WAITING state).
8. **Pipe blocking (full)**: Fill pipe buffer completely, verify next write
   blocks.
9. **Message queue create**: Create queue with key, verify same key returns
   same queue_id.
10. **Message send/receive**: Send 3 messages, receive in FIFO order.
11. **Message type filtering**: Send types 1, 2, 1. Receive type 2, verify
    correct message. Receive type 1, verify oldest type-1 message.
12. **Message queue full**: Send max_messages messages, verify next send blocks.
13. **Message too large**: Send message > max_message_size, verify error.
14. **Shared memory create**: shmget, verify segment with correct size.
15. **Shared memory read/write**: Attach, write data at offset, read back,
    verify match.
16. **Shared memory multi-process**: Attach from two processes, write from one,
    read from other, verify data visible.
17. **Shared memory detach**: Detach all processes, verify cleanup.

### Integration Tests

18. **Pipe + fork**: Fork a process, parent writes to pipe, child reads,
    verify data transfer.
19. **Shell pipeline simulation**: Create a 3-stage pipeline with pipes,
    verify data flows through all stages.
20. **IPC manager cleanup**: Create all IPC types, verify proper cleanup
    when processes exit.

### Coverage Target

Target 95%+ line coverage. Every error path (broken pipe, queue full, invalid
key, bad fd) and every edge case (empty read, full write, wrap-around) must
be tested.
