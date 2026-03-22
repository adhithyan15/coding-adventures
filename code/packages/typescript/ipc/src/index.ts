/**
 * # Inter-Process Communication (IPC)
 *
 * ## What Is IPC?
 *
 * Processes in an operating system are isolated by design. Each process has its
 * own virtual address space, its own file descriptors, its own registers. This
 * isolation is essential: a buggy program cannot corrupt another program's
 * memory, and a malicious program cannot read another program's secrets.
 *
 * But isolation creates a problem: **how do processes collaborate?**
 *
 * - A web server might fork worker processes that share a request queue.
 * - A shell pipeline like `ls | grep foo | wc -l` needs three processes to
 *   pass data in sequence.
 * - A database might use shared memory so multiple query workers can read
 *   cached pages without copying.
 *
 * **Inter-Process Communication (IPC)** is the set of mechanisms the OS
 * provides for processes to exchange data despite their isolation.
 *
 * This module implements three classic IPC mechanisms:
 *
 * 1. **Pipes** — unidirectional byte streams (simplest)
 * 2. **Message Queues** — FIFO queues of typed messages (structured)
 * 3. **Shared Memory** — a region of memory mapped into multiple address
 *    spaces (fastest, zero-copy)
 *
 * ## Analogy
 *
 * Imagine two people in separate, soundproofed rooms:
 * - A **pipe** is a pneumatic tube between the rooms — you stuff a message in
 *   one end, it comes out the other.
 * - A **message queue** is a shared mailbox in the hallway — anyone can drop
 *   off or pick up labeled envelopes.
 * - **Shared memory** is a window between the rooms with a whiteboard visible
 *   to both — fastest communication, but you need to take turns writing.
 */

// ============================================================================
// Error Types
// ============================================================================

/**
 * ## BrokenPipeError
 *
 * This error occurs when a process tries to write to a pipe whose read end
 * has been closed. In Unix systems, this generates the SIGPIPE signal (signal
 * 13) and the write() syscall returns EPIPE.
 *
 * **Real-world example:** If you run `head -1 /usr/share/dict/words | cat`,
 * `cat` writes to a pipe. When `head` reads one line and exits (closing the
 * read end), `cat`'s next write gets SIGPIPE.
 */
export class BrokenPipeError extends Error {
  constructor(message = "Broken pipe: no readers") {
    super(message);
    this.name = "BrokenPipeError";
  }
}

/**
 * ## IPCError
 *
 * General error for IPC operations — invalid IDs, names not found, etc.
 */
export class IPCError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "IPCError";
  }
}

// ============================================================================
// Pipe — Unidirectional Byte Stream
// ============================================================================

/**
 * ## Pipe: The Simplest IPC Mechanism
 *
 * A pipe is a unidirectional byte stream backed by a **circular buffer**.
 * Data written to the write end appears at the read end, in order, exactly
 * once. Think of it as a conveyor belt: items placed on one end come out the
 * other in the same order, and once consumed they are gone.
 *
 * ### The Circular Buffer
 *
 * The buffer is a fixed-size array that wraps around. We track two positions:
 *
 * ```
 * ┌───┬───┬───┬───┬───┬───┬───┬───┐
 * │ h │ e │ l │ l │ o │   │   │   │   capacity = 8
 * └───┴───┴───┴───┴───┴───┴───┴───┘
 *   ▲ readPos=0          ▲ writePos=5
 *
 * Available to read:  (writePos - readPos + capacity) % capacity = 5
 * Available to write: capacity - available = 3
 * ```
 *
 * When writePos reaches the end, it wraps to index 0 using modular
 * arithmetic: `(writePos + n) % capacity`. This avoids ever needing to
 * shift data — a key performance optimization.
 *
 * ### Reader/Writer Counts
 *
 * A pipe tracks how many readers and writers are connected:
 * - **All writers closed + buffer empty** → read returns empty (EOF).
 *   This is how shell pipelines terminate: when `cat` finishes, `grep`
 *   sees EOF.
 * - **All readers closed** → write throws BrokenPipeError.
 *   No point writing data nobody will ever read.
 *
 * ### Default Capacity
 *
 * 4096 bytes — one memory page. This is the traditional Unix pipe buffer
 * size. Modern Linux uses 65536 bytes, but 4096 is simpler to reason about
 * and sufficient for our educational purposes.
 */

/** Default pipe buffer size: one memory page (4096 bytes). */
const DEFAULT_PIPE_CAPACITY = 4096;

export class Pipe {
  /**
   * The circular buffer storing pipe data. This is a Uint8Array because
   * pipes transport raw bytes — the pipe has no idea what the bytes mean.
   * It could be text, binary data, serialized objects, or anything else.
   */
  private buffer: Uint8Array;

  /** Maximum number of bytes the buffer can hold. */
  readonly capacity: number;

  /**
   * Index of the next byte to be read. Advances on each read and wraps
   * around when it reaches the end of the buffer. Think of it as a
   * "consumption cursor."
   */
  private readPos: number = 0;

  /**
   * Index of the next byte to be written. Advances on each write and wraps
   * around. Think of it as a "production cursor."
   */
  private writePos: number = 0;

  /**
   * Number of bytes currently stored in the buffer. We track this
   * explicitly rather than computing it from readPos/writePos because the
   * formula `(writePos - readPos + capacity) % capacity` is ambiguous
   * when readPos === writePos (could mean empty OR full). An explicit
   * count removes the ambiguity.
   */
  private count: number = 0;

  /**
   * Number of open "read end" file descriptors. When this drops to 0 and
   * a write is attempted, the write throws BrokenPipeError — there is no
   * one to read the data.
   */
  private readerCount: number = 1;

  /**
   * Number of open "write end" file descriptors. When this drops to 0 and
   * the buffer is empty, a read returns an empty array — this is EOF.
   */
  private writerCount: number = 1;

  constructor(capacity: number = DEFAULT_PIPE_CAPACITY) {
    if (capacity <= 0) {
      throw new IPCError("Pipe capacity must be positive");
    }
    this.capacity = capacity;
    this.buffer = new Uint8Array(capacity);
  }

  // --------------------------------------------------------------------------
  // Writing to the pipe
  // --------------------------------------------------------------------------

  /**
   * ## write(data)
   *
   * Write bytes into the pipe's circular buffer.
   *
   * ### Behavior Table
   *
   * | Readers alive? | Buffer has space? | Result                        |
   * |----------------|-------------------|-------------------------------|
   * | No             | (any)             | Throw BrokenPipeError         |
   * | Yes            | Yes               | Write bytes, return count     |
   * | Yes            | No (full)         | Write 0 bytes (would block)   |
   * | Yes            | Partial           | Write what fits, return count |
   *
   * In a real OS, "write 0 bytes" would block the process (move it to
   * WAITING state) until a reader drains some data. In our simulation,
   * we return the number of bytes actually written and let the caller
   * retry.
   *
   * @param data - The bytes to write into the pipe.
   * @returns The number of bytes actually written (may be less than
   *          data.length if the buffer is partially full).
   * @throws BrokenPipeError if all read ends have been closed.
   */
  write(data: Uint8Array): number {
    // If nobody is listening, writing is pointless. In Unix, this triggers
    // SIGPIPE and write() returns -1 with errno=EPIPE.
    if (this.readerCount <= 0) {
      throw new BrokenPipeError();
    }

    // Calculate how many bytes we can actually write. The pipe buffer is
    // finite — we write as many bytes as will fit and return the count.
    const spaceAvailable = this.capacity - this.count;
    const bytesToWrite = Math.min(data.length, spaceAvailable);

    // Copy bytes into the circular buffer, wrapping around if needed.
    // We handle the wrap by splitting the write into at most two segments:
    //
    //   Segment 1: from writePos to end of buffer (or fewer if data is short)
    //   Segment 2: from index 0 onward (only if we wrapped)
    //
    // Example with capacity=8, writePos=6, writing 4 bytes:
    //   Segment 1: indices 6,7 (2 bytes)
    //   Segment 2: indices 0,1 (2 bytes)
    for (let i = 0; i < bytesToWrite; i++) {
      this.buffer[(this.writePos + i) % this.capacity] = data[i];
    }
    this.writePos = (this.writePos + bytesToWrite) % this.capacity;
    this.count += bytesToWrite;

    return bytesToWrite;
  }

  // --------------------------------------------------------------------------
  // Reading from the pipe
  // --------------------------------------------------------------------------

  /**
   * ## read(maxBytes)
   *
   * Read up to `maxBytes` bytes from the pipe's circular buffer.
   *
   * ### Behavior Table
   *
   * | Buffer has data? | Writers alive? | Result                       |
   * |------------------|----------------|------------------------------|
   * | Yes              | (any)          | Read bytes, return them      |
   * | No               | Yes            | Return empty (would block)   |
   * | No               | No             | Return empty (EOF)           |
   *
   * In a real OS, "buffer empty + writers alive" would block the process.
   * We return an empty array in both blocking and EOF cases, and let the
   * caller distinguish by checking `isEof`.
   *
   * @param maxBytes - Maximum number of bytes to read.
   * @returns A Uint8Array containing the bytes read (may be shorter than
   *          maxBytes if fewer bytes are available).
   */
  read(maxBytes: number): Uint8Array {
    if (maxBytes <= 0) {
      return new Uint8Array(0);
    }

    const bytesToRead = Math.min(maxBytes, this.count);
    if (bytesToRead === 0) {
      return new Uint8Array(0);
    }

    // Extract bytes from the circular buffer, handling wrap-around just
    // like we do in write().
    const result = new Uint8Array(bytesToRead);
    for (let i = 0; i < bytesToRead; i++) {
      result[i] = this.buffer[(this.readPos + i) % this.capacity];
    }
    this.readPos = (this.readPos + bytesToRead) % this.capacity;
    this.count -= bytesToRead;

    return result;
  }

  // --------------------------------------------------------------------------
  // Closing pipe ends
  // --------------------------------------------------------------------------

  /**
   * Close the read end of the pipe. Decrements the reader count. If no
   * readers remain, subsequent writes will throw BrokenPipeError.
   *
   * In Unix, this happens when a process calls close() on the read fd,
   * or when the process exits (all its fds are closed automatically).
   */
  closeRead(): void {
    if (this.readerCount > 0) {
      this.readerCount--;
    }
  }

  /**
   * Close the write end of the pipe. Decrements the writer count. If no
   * writers remain and the buffer is empty, subsequent reads return EOF.
   *
   * This is how shell pipelines signal completion: when `ls` exits, its
   * write end closes, and `grep` eventually sees EOF on its read end.
   */
  closeWrite(): void {
    if (this.writerCount > 0) {
      this.writerCount--;
    }
  }

  // --------------------------------------------------------------------------
  // Status queries
  // --------------------------------------------------------------------------

  /** Is the buffer empty? If writers are also closed, this means EOF. */
  get isEmpty(): boolean {
    return this.count === 0;
  }

  /** Is the buffer completely full? Writes would block (or write 0). */
  get isFull(): boolean {
    return this.count === this.capacity;
  }

  /** Number of bytes available to read right now. */
  get available(): number {
    return this.count;
  }

  /** Number of bytes that can be written before the buffer is full. */
  get space(): number {
    return this.capacity - this.count;
  }

  /**
   * ## EOF Detection
   *
   * A pipe is at EOF (End Of File) when:
   * 1. All writers have closed their end (writerCount === 0), AND
   * 2. The buffer is empty (no unread data remains).
   *
   * If writers are closed but data remains in the buffer, the pipe is NOT
   * at EOF — the reader should still consume the remaining data first.
   *
   * This two-condition check is exactly what Unix does:
   * - `read()` returns 0 only when BOTH conditions hold.
   * - If data remains, `read()` returns that data even if all writers
   *   have closed.
   */
  get isEof(): boolean {
    return this.writerCount <= 0 && this.count === 0;
  }

  /** Check if writing would cause a BrokenPipeError (no readers). */
  get isBroken(): boolean {
    return this.readerCount <= 0;
  }

  /** Current reader reference count. */
  get readers(): number {
    return this.readerCount;
  }

  /** Current writer reference count. */
  get writers(): number {
    return this.writerCount;
  }
}

// ============================================================================
// Message Queue — Structured Communication
// ============================================================================

/**
 * ## Message
 *
 * A message in a message queue has three parts:
 *
 * | Field    | Purpose                                              |
 * |----------|------------------------------------------------------|
 * | msgType  | Positive integer tag — receivers can filter by type   |
 * | data     | The payload — up to 4096 bytes of arbitrary data     |
 * | size     | Actual size of the data in bytes                     |
 *
 * **Why typed messages?** Unlike pipes (raw byte streams), message queues
 * preserve message boundaries and let receivers be selective. A print
 * spooler might use type 1 for "print job" and type 2 for "cancel job."
 * The cancel handler can call `receive(type=2)` to skip print jobs and
 * only see cancellation requests.
 */
export interface Message {
  /** Positive integer identifying the message kind. */
  readonly msgType: number;
  /** The message payload as raw bytes. */
  readonly data: Uint8Array;
  /** Actual size of the data in bytes. */
  readonly size: number;
}

/**
 * ## MessageQueue: FIFO of Typed Messages
 *
 * A message queue decouples senders and receivers. Unlike pipes:
 * - Any process can send to the queue (not just the creator).
 * - Messages have boundaries — you always get a complete message.
 * - Receivers can filter by message type.
 *
 * ```
 * Process A (sender)                     Process B (receiver)
 * send(qid, type=1, "request")          receive(qid, type=1) → "request"
 *      │                                       ▲
 *      ▼                                       │
 * ┌─────────────────────────────────────────────────┐
 * │             Message Queue (FIFO)                 │
 * │  ┌─────────────────┐                             │
 * │  │ type=1 "request"│ ← oldest (dequeued next)    │
 * │  ├─────────────────┤                             │
 * │  │ type=2 "status" │ ← skipped by type=1 recv   │
 * │  ├─────────────────┤                             │
 * │  │ type=1 "query"  │ ← next type=1 message      │
 * │  └─────────────────┘                             │
 * │  max_messages: 256    max_message_size: 4096     │
 * └──────────────────────────────────────────────────┘
 * ```
 *
 * ### Capacity Limits
 *
 * - **max_messages (256):** Prevents a fast sender from consuming all
 *   kernel memory. In a real OS, the sender would block when the queue
 *   is full. In our simulation, `send()` returns false.
 *
 * - **max_message_size (4096):** One memory page. Messages larger than
 *   this should use shared memory instead. The limit prevents a single
 *   message from dominating the queue.
 */

const DEFAULT_MAX_MESSAGES = 256;
const DEFAULT_MAX_MESSAGE_SIZE = 4096;

export class MessageQueue {
  /** FIFO queue of messages, oldest at index 0. */
  private messages: Message[] = [];

  /** Maximum number of messages the queue can hold. */
  readonly maxMessages: number;

  /** Maximum size (in bytes) of a single message's data. */
  readonly maxMessageSize: number;

  constructor(
    maxMessages: number = DEFAULT_MAX_MESSAGES,
    maxMessageSize: number = DEFAULT_MAX_MESSAGE_SIZE
  ) {
    this.maxMessages = maxMessages;
    this.maxMessageSize = maxMessageSize;
  }

  // --------------------------------------------------------------------------
  // Sending messages
  // --------------------------------------------------------------------------

  /**
   * ## send(msgType, data)
   *
   * Send a message to the back of the queue.
   *
   * ### Validation Steps
   *
   * 1. Check message size: `data.length <= maxMessageSize`?
   *    - If too large, return false. The caller should use shared memory
   *      for large data transfers.
   *
   * 2. Check queue capacity: `messages.length < maxMessages`?
   *    - If full, return false. In a real OS this would block; here the
   *      caller should retry later.
   *
   * 3. Push the message onto the FIFO.
   *
   * @param msgType - Positive integer message type tag.
   * @param data - The message payload (copied, so the caller can reuse
   *               their buffer).
   * @returns true if the message was enqueued, false if rejected.
   */
  send(msgType: number, data: Uint8Array): boolean {
    // Reject oversized messages. This is a hard limit — unlike the "queue
    // full" case (which resolves itself when a receiver drains a message),
    // an oversized message can never be sent. The caller must redesign
    // their protocol (e.g., split the data, or use shared memory).
    if (data.length > this.maxMessageSize) {
      return false;
    }

    // Reject if the queue is at capacity. In a real kernel, the sending
    // process would be moved to WAITING state until a receive() frees a
    // slot. We return false for simplicity.
    if (this.messages.length >= this.maxMessages) {
      return false;
    }

    // Copy the data so the sender can safely reuse their buffer without
    // corrupting the queued message. This is the "two-copy" cost of
    // message queues: one copy here (sender → kernel buffer), one copy
    // when the receiver calls receive() (kernel buffer → receiver).
    const messageCopy: Message = {
      msgType,
      data: new Uint8Array(data),
      size: data.length,
    };
    this.messages.push(messageCopy);
    return true;
  }

  // --------------------------------------------------------------------------
  // Receiving messages
  // --------------------------------------------------------------------------

  /**
   * ## receive(msgType)
   *
   * Receive (dequeue) a message from the queue.
   *
   * ### Type Filtering
   *
   * The `msgType` parameter controls which messages are eligible:
   *
   * | msgType value | Behavior                                      |
   * |---------------|-----------------------------------------------|
   * | 0             | Return the oldest message of ANY type          |
   * | > 0           | Return the oldest message matching this type   |
   *
   * When filtering by type, non-matching messages are **skipped but not
   * removed**. They remain in the queue for other receivers.
   *
   * **Example:** Queue contains [type=1, type=2, type=1]. Calling
   * `receive(2)` returns the type=2 message. The queue now contains
   * [type=1, type=1].
   *
   * @param msgType - 0 for any type, or a positive integer to filter.
   * @returns The message if one was found, or null if no match exists.
   */
  receive(msgType: number = 0): Message | null {
    if (this.messages.length === 0) {
      return null;
    }

    if (msgType === 0) {
      // Type 0 = "give me anything." Dequeue the oldest message.
      return this.messages.shift()!;
    }

    // Search for the first message matching the requested type.
    // We scan from oldest to newest to maintain FIFO ordering within
    // each type. Non-matching messages are left in place.
    const index = this.messages.findIndex((m) => m.msgType === msgType);
    if (index === -1) {
      return null;
    }

    // Remove the matching message from its position. The splice()
    // operation is O(n), but with a max of 256 messages this is fine.
    // A real kernel might use a more sophisticated data structure
    // (e.g., separate queues per type).
    return this.messages.splice(index, 1)[0];
  }

  // --------------------------------------------------------------------------
  // Status queries
  // --------------------------------------------------------------------------

  /** Is the queue empty? No messages to receive. */
  get isEmpty(): boolean {
    return this.messages.length === 0;
  }

  /** Is the queue full? No room to send. */
  get isFull(): boolean {
    return this.messages.length >= this.maxMessages;
  }

  /** Number of messages currently in the queue. */
  get messageCount(): number {
    return this.messages.length;
  }
}

// ============================================================================
// Shared Memory — Zero-Copy Communication
// ============================================================================

/**
 * ## SharedMemoryRegion: The Fastest IPC
 *
 * Pipes and message queues both **copy** data: the sender writes bytes, the
 * kernel copies them into a buffer, and the receiver copies them out. For
 * large data transfers (e.g., a database buffer pool), this double-copy is
 * expensive.
 *
 * Shared memory eliminates copying entirely. Two processes map the **same
 * physical pages** into their virtual address spaces. A write by one process
 * is immediately visible to the other — no system call, no copy.
 *
 * ```
 * Process A's address space          Process B's address space
 * ┌──────────────────────┐           ┌──────────────────────┐
 * │ 0x8000 ┌──────────┐  │           │ 0xC000 ┌──────────┐  │
 * │        │ Shared   │◄─┼───────────┼────────│ Shared   │  │
 * │        │ Region   │  │           │        │ Region   │  │
 * │        └──────────┘  │           │        └──────────┘  │
 * └──────────────────────┘           └──────────────────────┘
 *          │                                      │
 *          └──────────┬───────────────────────────┘
 *                     │
 *              ┌──────▼──────┐
 *              │  Physical   │  ← same physical memory
 *              │  Page Frame │
 *              └─────────────┘
 * ```
 *
 * ### The Catch: No Built-In Synchronization
 *
 * Shared memory has NO built-in synchronization. If process A writes while
 * process B reads, B may see partially-updated data ("torn reads"). Real
 * programs use semaphores, mutexes, or atomic operations to coordinate.
 * We omit synchronization here for simplicity but note the hazard.
 *
 * ### Attached PIDs
 *
 * The region tracks which process IDs are currently attached. When the last
 * process detaches, the region can be cleaned up. This reference counting
 * is similar to how pipes track reader/writer counts.
 */
export class SharedMemoryRegion {
  /** Human-readable name for this shared memory segment. */
  readonly name: string;

  /** Size of the shared region in bytes. */
  readonly size: number;

  /**
   * The actual shared data. In a real OS, this would be physical page
   * frames mapped into multiple virtual address spaces. In our simulation,
   * it is a Uint8Array that all "attached" processes access directly.
   */
  private data: Uint8Array;

  /** The process ID of the creator / owner of this segment. */
  readonly ownerPid: number;

  /**
   * Set of process IDs currently attached to this region. A process must
   * attach before it can read or write, and should detach when done.
   */
  private attachedPids: Set<number> = new Set();

  constructor(name: string, size: number, ownerPid: number) {
    if (size <= 0) {
      throw new IPCError("Shared memory size must be positive");
    }
    this.name = name;
    this.size = size;
    this.data = new Uint8Array(size);
    this.ownerPid = ownerPid;
  }

  // --------------------------------------------------------------------------
  // Attaching and detaching processes
  // --------------------------------------------------------------------------

  /**
   * Attach a process to this shared memory region.
   *
   * In a real OS, this modifies the process's page table to map the shared
   * physical pages into its virtual address space. Here, we just record the
   * PID so we can track who is attached for cleanup purposes.
   *
   * @param pid - The process ID to attach.
   * @returns true if newly attached, false if already attached.
   */
  attach(pid: number): boolean {
    if (this.attachedPids.has(pid)) {
      return false;
    }
    this.attachedPids.add(pid);
    return true;
  }

  /**
   * Detach a process from this shared memory region.
   *
   * In a real OS, this unmaps the shared pages from the process's virtual
   * address space. The process can no longer access the shared data
   * (attempting to would cause a segfault).
   *
   * @param pid - The process ID to detach.
   * @returns true if successfully detached, false if was not attached.
   */
  detach(pid: number): boolean {
    return this.attachedPids.delete(pid);
  }

  // --------------------------------------------------------------------------
  // Reading and writing shared data
  // --------------------------------------------------------------------------

  /**
   * ## read(offset, count)
   *
   * Read `count` bytes starting at `offset` in the shared region.
   *
   * Bounds checking is performed: if `offset + count > size`, an error is
   * thrown. This is analogous to a segfault in a real OS — accessing memory
   * outside the mapped region is illegal.
   *
   * @param offset - Starting byte position within the region.
   * @param count - Number of bytes to read.
   * @returns A copy of the requested bytes.
   * @throws IPCError if the read would go out of bounds.
   */
  read(offset: number, count: number): Uint8Array {
    if (offset < 0 || count < 0 || offset + count > this.size) {
      throw new IPCError(
        `Shared memory read out of bounds: offset=${offset}, count=${count}, size=${this.size}`
      );
    }
    // Return a copy, not a view. In a real OS the process reads from its
    // own virtual address space (which maps to the shared physical pages).
    // We copy here to prevent direct reference to the internal buffer
    // outside of the read/write API.
    return new Uint8Array(this.data.slice(offset, offset + count));
  }

  /**
   * ## write(offset, data)
   *
   * Write bytes starting at `offset` in the shared region.
   *
   * **WARNING:** There is no built-in synchronization. If two processes
   * write concurrently, the result is a race condition. Real programs
   * must use semaphores or mutexes to coordinate writes.
   *
   * @param offset - Starting byte position within the region.
   * @param data - The bytes to write.
   * @returns Number of bytes written.
   * @throws IPCError if the write would go out of bounds.
   */
  write(offset: number, data: Uint8Array): number {
    if (offset < 0 || offset + data.length > this.size) {
      throw new IPCError(
        `Shared memory write out of bounds: offset=${offset}, length=${data.length}, size=${this.size}`
      );
    }
    this.data.set(data, offset);
    return data.length;
  }

  // --------------------------------------------------------------------------
  // Status queries
  // --------------------------------------------------------------------------

  /** Set of currently attached process IDs (as a read-only copy). */
  get pids(): Set<number> {
    return new Set(this.attachedPids);
  }

  /** Number of processes currently attached. */
  get attachedCount(): number {
    return this.attachedPids.size;
  }

  /** Is a specific PID attached to this region? */
  isAttached(pid: number): boolean {
    return this.attachedPids.has(pid);
  }
}

// ============================================================================
// IPC Manager — The Kernel's IPC Coordinator
// ============================================================================

/**
 * ## Pipe Handle
 *
 * When the IPC manager creates a pipe, it returns a handle containing:
 * - `pipeId`: unique identifier for this pipe
 * - `readFd`: the file descriptor for reading
 * - `writeFd`: the file descriptor for writing
 *
 * In a real OS, these file descriptors would be entries in the process's
 * file descriptor table. Here, they are just unique integers.
 */
export interface PipeHandle {
  pipeId: number;
  readFd: number;
  writeFd: number;
}

/**
 * ## IPCManager: The Kernel's IPC Bookkeeper
 *
 * The IPCManager is the kernel component that owns all IPC resources. It
 * provides the system call interface for creating, accessing, and destroying
 * pipes, message queues, and shared memory regions.
 *
 * ```
 * IPCManager
 * ├── pipes: Map<pipeId, Pipe>
 * ├── messageQueues: Map<name, MessageQueue>
 * └── sharedRegions: Map<name, SharedMemoryRegion>
 * ```
 *
 * ### File Descriptor Numbering
 *
 * The manager assigns file descriptors starting from 100 (to avoid
 * conflicting with stdin=0, stdout=1, stderr=2 and other low fds).
 * Each pipe gets two consecutive fds: one for reading, one for writing.
 */
export class IPCManager {
  /** All active pipes, keyed by pipe ID. */
  private pipes: Map<number, Pipe> = new Map();

  /** File descriptor → pipe ID mapping (for both read and write fds). */
  private fdToPipe: Map<number, number> = new Map();

  /** Track which fds are read ends vs write ends. */
  private fdIsReadEnd: Map<number, boolean> = new Map();

  /** All message queues, keyed by name (the "well-known key"). */
  private messageQueues: Map<string, MessageQueue> = new Map();

  /** All shared memory regions, keyed by name. */
  private sharedRegions: Map<string, SharedMemoryRegion> = new Map();

  /** Counter for assigning unique pipe IDs. */
  private nextPipeId: number = 0;

  /** Counter for assigning unique file descriptors. */
  private nextFd: number = 100;

  // --------------------------------------------------------------------------
  // Pipe management
  // --------------------------------------------------------------------------

  /**
   * Create a new pipe and return a handle with its pipe ID and two
   * file descriptors (one for reading, one for writing).
   *
   * @param capacity - Buffer size in bytes (default 4096).
   * @returns A PipeHandle with pipeId, readFd, and writeFd.
   */
  createPipe(capacity: number = DEFAULT_PIPE_CAPACITY): PipeHandle {
    const pipe = new Pipe(capacity);
    const pipeId = this.nextPipeId++;
    const readFd = this.nextFd++;
    const writeFd = this.nextFd++;

    this.pipes.set(pipeId, pipe);
    this.fdToPipe.set(readFd, pipeId);
    this.fdToPipe.set(writeFd, pipeId);
    this.fdIsReadEnd.set(readFd, true);
    this.fdIsReadEnd.set(writeFd, false);

    return { pipeId, readFd, writeFd };
  }

  /**
   * Get a pipe by its ID.
   *
   * @param pipeId - The pipe ID returned by createPipe().
   * @returns The Pipe object, or undefined if not found.
   */
  getPipe(pipeId: number): Pipe | undefined {
    return this.pipes.get(pipeId);
  }

  /**
   * Close the read end of a pipe (by pipe ID). Decrements the reader
   * count on the underlying Pipe.
   */
  closePipeRead(pipeId: number): void {
    const pipe = this.pipes.get(pipeId);
    if (!pipe) {
      throw new IPCError(`Pipe ${pipeId} not found`);
    }
    pipe.closeRead();
  }

  /**
   * Close the write end of a pipe (by pipe ID). Decrements the writer
   * count on the underlying Pipe.
   */
  closePipeWrite(pipeId: number): void {
    const pipe = this.pipes.get(pipeId);
    if (!pipe) {
      throw new IPCError(`Pipe ${pipeId} not found`);
    }
    pipe.closeWrite();
  }

  /**
   * Fully remove a pipe from the manager. After this, the pipe ID and
   * its associated file descriptors are invalid.
   */
  destroyPipe(pipeId: number): void {
    const pipe = this.pipes.get(pipeId);
    if (!pipe) {
      throw new IPCError(`Pipe ${pipeId} not found`);
    }

    // Clean up fd mappings for this pipe.
    for (const [fd, id] of this.fdToPipe.entries()) {
      if (id === pipeId) {
        this.fdToPipe.delete(fd);
        this.fdIsReadEnd.delete(fd);
      }
    }

    this.pipes.delete(pipeId);
  }

  // --------------------------------------------------------------------------
  // Message queue management
  // --------------------------------------------------------------------------

  /**
   * Create a new message queue with the given name (key). If a queue
   * with this name already exists, return it (idempotent, like msgget).
   *
   * @param name - The well-known key/name for this queue.
   * @param maxMessages - Max number of messages (default 256).
   * @param maxMessageSize - Max size per message (default 4096).
   * @returns The MessageQueue (newly created or existing).
   */
  createMessageQueue(
    name: string,
    maxMessages: number = DEFAULT_MAX_MESSAGES,
    maxMessageSize: number = DEFAULT_MAX_MESSAGE_SIZE
  ): MessageQueue {
    const existing = this.messageQueues.get(name);
    if (existing) {
      return existing;
    }
    const queue = new MessageQueue(maxMessages, maxMessageSize);
    this.messageQueues.set(name, queue);
    return queue;
  }

  /**
   * Get a message queue by name.
   *
   * @param name - The queue's well-known key/name.
   * @returns The MessageQueue, or undefined if not found.
   */
  getMessageQueue(name: string): MessageQueue | undefined {
    return this.messageQueues.get(name);
  }

  /**
   * Delete a message queue. Any unread messages are lost.
   *
   * @param name - The queue's name.
   * @throws IPCError if the queue does not exist.
   */
  deleteMessageQueue(name: string): void {
    if (!this.messageQueues.has(name)) {
      throw new IPCError(`Message queue '${name}' not found`);
    }
    this.messageQueues.delete(name);
  }

  // --------------------------------------------------------------------------
  // Shared memory management
  // --------------------------------------------------------------------------

  /**
   * Create a new shared memory region. If a region with this name already
   * exists, return it (idempotent, like shmget).
   *
   * @param name - The well-known name/key for this region.
   * @param size - Size in bytes.
   * @param ownerPid - PID of the creating process.
   * @returns The SharedMemoryRegion (newly created or existing).
   */
  createSharedMemory(
    name: string,
    size: number,
    ownerPid: number
  ): SharedMemoryRegion {
    const existing = this.sharedRegions.get(name);
    if (existing) {
      return existing;
    }
    const region = new SharedMemoryRegion(name, size, ownerPid);
    this.sharedRegions.set(name, region);
    return region;
  }

  /**
   * Get a shared memory region by name.
   *
   * @param name - The region's name/key.
   * @returns The SharedMemoryRegion, or undefined if not found.
   */
  getSharedMemory(name: string): SharedMemoryRegion | undefined {
    return this.sharedRegions.get(name);
  }

  /**
   * Delete a shared memory region. Any attached processes lose access.
   *
   * @param name - The region's name.
   * @throws IPCError if the region does not exist.
   */
  deleteSharedMemory(name: string): void {
    if (!this.sharedRegions.has(name)) {
      throw new IPCError(`Shared memory '${name}' not found`);
    }
    this.sharedRegions.delete(name);
  }

  // --------------------------------------------------------------------------
  // Listing resources
  // --------------------------------------------------------------------------

  /** List all active pipe IDs. */
  listPipes(): number[] {
    return Array.from(this.pipes.keys());
  }

  /** List all message queue names. */
  listMessageQueues(): string[] {
    return Array.from(this.messageQueues.keys());
  }

  /** List all shared memory region names. */
  listSharedRegions(): string[] {
    return Array.from(this.sharedRegions.keys());
  }
}
