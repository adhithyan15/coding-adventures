/**
 * # IPC Test Suite
 *
 * Comprehensive tests for all three IPC mechanisms (pipes, message queues,
 * shared memory) and the IPC manager that coordinates them.
 *
 * Each test section starts with the simplest cases and builds toward edge
 * cases. The goal is 90%+ coverage of all code paths, including error
 * conditions like BrokenPipeError and out-of-bounds access.
 */
import { describe, it, expect } from "vitest";
import {
  Pipe,
  MessageQueue,
  SharedMemoryRegion,
  IPCManager,
  BrokenPipeError,
  IPCError,
} from "../src/index.js";

// ============================================================================
// Helper: Convert string to Uint8Array and back
// ============================================================================

/** Encode a string as UTF-8 bytes. */
function encode(str: string): Uint8Array {
  return new TextEncoder().encode(str);
}

/** Decode UTF-8 bytes to a string. */
function decode(bytes: Uint8Array): string {
  return new TextDecoder().decode(bytes);
}

// ============================================================================
// Pipe Tests
// ============================================================================

describe("Pipe", () => {
  // --------------------------------------------------------------------------
  // Basic write/read roundtrip
  // --------------------------------------------------------------------------

  describe("write/read roundtrip", () => {
    it("should write and read data back correctly", () => {
      // The most basic test: write "hello", read it back.
      // This verifies the circular buffer's happy path.
      const pipe = new Pipe();
      const data = encode("hello");
      const written = pipe.write(data);
      expect(written).toBe(5);
      expect(pipe.available).toBe(5);

      const result = pipe.read(5);
      expect(decode(result)).toBe("hello");
      expect(pipe.available).toBe(0);
    });

    it("should handle multiple sequential writes and reads", () => {
      const pipe = new Pipe();
      pipe.write(encode("abc"));
      pipe.write(encode("def"));

      // FIFO ordering: "abc" was written first, so reading 6 bytes
      // should return "abcdef" — not "defabc".
      const result = pipe.read(6);
      expect(decode(result)).toBe("abcdef");
    });

    it("should handle partial reads", () => {
      // Write 5 bytes but only read 3. The remaining 2 bytes should
      // still be available for the next read.
      const pipe = new Pipe();
      pipe.write(encode("hello"));

      const first = pipe.read(3);
      expect(decode(first)).toBe("hel");
      expect(pipe.available).toBe(2);

      const second = pipe.read(2);
      expect(decode(second)).toBe("lo");
      expect(pipe.available).toBe(0);
    });

    it("should return empty array when reading 0 or negative bytes", () => {
      const pipe = new Pipe();
      pipe.write(encode("data"));
      expect(pipe.read(0).length).toBe(0);
      expect(pipe.read(-1).length).toBe(0);
    });
  });

  // --------------------------------------------------------------------------
  // Circular buffer wrapping
  // --------------------------------------------------------------------------

  describe("circular wrapping", () => {
    it("should wrap data around the end of the buffer", () => {
      // Use a small buffer to force wrapping. With capacity=8:
      // 1. Write 6 bytes → writePos=6
      // 2. Read 6 bytes → readPos=6
      // 3. Write 5 bytes → writePos=(6+5)%8=3 (wraps around!)
      // 4. Read 5 bytes → should get all 5 bytes correctly
      const pipe = new Pipe(8);

      pipe.write(encode("abcdef")); // writePos=6
      pipe.read(6); // readPos=6, buffer empty

      pipe.write(encode("ghijk")); // wraps: positions 6,7,0,1,2
      const result = pipe.read(5);
      expect(decode(result)).toBe("ghijk");
    });

    it("should handle multiple wrap-arounds", () => {
      const pipe = new Pipe(4);

      // First round
      pipe.write(encode("abcd"));
      expect(decode(pipe.read(4))).toBe("abcd");

      // Second round (wraps)
      pipe.write(encode("efgh"));
      expect(decode(pipe.read(4))).toBe("efgh");

      // Third round (wraps again)
      pipe.write(encode("ijkl"));
      expect(decode(pipe.read(4))).toBe("ijkl");
    });
  });

  // --------------------------------------------------------------------------
  // EOF detection
  // --------------------------------------------------------------------------

  describe("EOF", () => {
    it("should signal EOF when all writers close and buffer is empty", () => {
      const pipe = new Pipe();
      expect(pipe.isEof).toBe(false);

      pipe.closeWrite();
      expect(pipe.isEof).toBe(true); // No writers + empty buffer = EOF
    });

    it("should NOT be EOF if data remains in buffer after writers close", () => {
      const pipe = new Pipe();
      pipe.write(encode("remaining"));
      pipe.closeWrite();

      // Writers are gone, but there is unread data. Not EOF yet.
      expect(pipe.isEof).toBe(false);

      // Drain the buffer.
      pipe.read(9);
      expect(pipe.isEof).toBe(true); // NOW it is EOF
    });

    it("should return empty array when reading from EOF pipe", () => {
      const pipe = new Pipe();
      pipe.closeWrite();
      const result = pipe.read(10);
      expect(result.length).toBe(0);
    });
  });

  // --------------------------------------------------------------------------
  // BrokenPipe
  // --------------------------------------------------------------------------

  describe("BrokenPipe", () => {
    it("should throw BrokenPipeError when writing with no readers", () => {
      const pipe = new Pipe();
      pipe.closeRead();

      expect(() => pipe.write(encode("doomed"))).toThrow(BrokenPipeError);
    });

    it("should report isBroken when readers are 0", () => {
      const pipe = new Pipe();
      expect(pipe.isBroken).toBe(false);
      pipe.closeRead();
      expect(pipe.isBroken).toBe(true);
    });
  });

  // --------------------------------------------------------------------------
  // Capacity and fullness
  // --------------------------------------------------------------------------

  describe("capacity", () => {
    it("should report isFull when buffer is at capacity", () => {
      const pipe = new Pipe(4);
      pipe.write(encode("abcd"));
      expect(pipe.isFull).toBe(true);
      expect(pipe.space).toBe(0);
    });

    it("should write partial data when buffer has limited space", () => {
      const pipe = new Pipe(4);
      pipe.write(encode("ab")); // 2 of 4 used
      const written = pipe.write(encode("cdef")); // only 2 bytes fit
      expect(written).toBe(2);
      expect(pipe.isFull).toBe(true);
      expect(decode(pipe.read(4))).toBe("abcd");
    });

    it("should write 0 bytes when buffer is completely full", () => {
      const pipe = new Pipe(4);
      pipe.write(encode("abcd"));
      const written = pipe.write(encode("e"));
      expect(written).toBe(0);
    });

    it("should reject non-positive capacity", () => {
      expect(() => new Pipe(0)).toThrow(IPCError);
      expect(() => new Pipe(-1)).toThrow(IPCError);
    });

    it("should default to 4096 capacity", () => {
      const pipe = new Pipe();
      expect(pipe.capacity).toBe(4096);
    });
  });

  // --------------------------------------------------------------------------
  // Reader/writer reference counts
  // --------------------------------------------------------------------------

  describe("reference counts", () => {
    it("should start with 1 reader and 1 writer", () => {
      const pipe = new Pipe();
      expect(pipe.readers).toBe(1);
      expect(pipe.writers).toBe(1);
    });

    it("should not go below 0 when closing multiple times", () => {
      const pipe = new Pipe();
      pipe.closeRead();
      pipe.closeRead(); // already 0, should stay 0
      expect(pipe.readers).toBe(0);
    });

    it("should track isEmpty correctly", () => {
      const pipe = new Pipe();
      expect(pipe.isEmpty).toBe(true);
      pipe.write(encode("x"));
      expect(pipe.isEmpty).toBe(false);
      pipe.read(1);
      expect(pipe.isEmpty).toBe(true);
    });
  });
});

// ============================================================================
// MessageQueue Tests
// ============================================================================

describe("MessageQueue", () => {
  // --------------------------------------------------------------------------
  // FIFO ordering
  // --------------------------------------------------------------------------

  describe("FIFO ordering", () => {
    it("should deliver messages in FIFO order", () => {
      const mq = new MessageQueue();

      mq.send(1, encode("first"));
      mq.send(1, encode("second"));
      mq.send(1, encode("third"));

      // receive(0) = any type, FIFO order
      expect(decode(mq.receive(0)!.data)).toBe("first");
      expect(decode(mq.receive(0)!.data)).toBe("second");
      expect(decode(mq.receive(0)!.data)).toBe("third");
    });

    it("should return null when queue is empty", () => {
      const mq = new MessageQueue();
      expect(mq.receive(0)).toBeNull();
    });
  });

  // --------------------------------------------------------------------------
  // Type filtering
  // --------------------------------------------------------------------------

  describe("type filtering", () => {
    it("should filter messages by type", () => {
      const mq = new MessageQueue();

      mq.send(1, encode("type1-first"));
      mq.send(2, encode("type2-only"));
      mq.send(1, encode("type1-second"));

      // Request type 2: should skip type 1 messages and return "type2-only"
      const msg = mq.receive(2);
      expect(msg).not.toBeNull();
      expect(msg!.msgType).toBe(2);
      expect(decode(msg!.data)).toBe("type2-only");

      // Remaining: [type1-first, type1-second]
      expect(mq.messageCount).toBe(2);
    });

    it("should return null when no message matches the requested type", () => {
      const mq = new MessageQueue();
      mq.send(1, encode("only type 1"));
      expect(mq.receive(99)).toBeNull();
      expect(mq.messageCount).toBe(1); // message still in queue
    });

    it("should return oldest matching message for a given type", () => {
      const mq = new MessageQueue();
      mq.send(1, encode("a"));
      mq.send(2, encode("b"));
      mq.send(1, encode("c"));

      // First type-1 receive: should get "a" (oldest type=1)
      expect(decode(mq.receive(1)!.data)).toBe("a");
      // Second type-1 receive: should get "c"
      expect(decode(mq.receive(1)!.data)).toBe("c");
      // Only type-2 remains
      expect(mq.messageCount).toBe(1);
      expect(decode(mq.receive(2)!.data)).toBe("b");
    });
  });

  // --------------------------------------------------------------------------
  // Full queue and oversized messages
  // --------------------------------------------------------------------------

  describe("capacity limits", () => {
    it("should reject sends when queue is full", () => {
      const mq = new MessageQueue(3); // max 3 messages

      expect(mq.send(1, encode("a"))).toBe(true);
      expect(mq.send(1, encode("b"))).toBe(true);
      expect(mq.send(1, encode("c"))).toBe(true);
      expect(mq.isFull).toBe(true);

      // 4th message should be rejected
      expect(mq.send(1, encode("d"))).toBe(false);
    });

    it("should reject oversized messages", () => {
      const mq = new MessageQueue(256, 8); // max 8 bytes per message

      expect(mq.send(1, encode("short"))).toBe(true); // 5 bytes, OK
      expect(mq.send(1, encode("this is way too long"))).toBe(false); // >8 bytes
    });

    it("should accept messages exactly at the size limit", () => {
      const mq = new MessageQueue(256, 5);
      expect(mq.send(1, encode("12345"))).toBe(true); // exactly 5 bytes
    });
  });

  // --------------------------------------------------------------------------
  // Message structure
  // --------------------------------------------------------------------------

  describe("message structure", () => {
    it("should preserve message type and size", () => {
      const mq = new MessageQueue();
      const payload = encode("test payload");
      mq.send(42, payload);

      const msg = mq.receive(0)!;
      expect(msg.msgType).toBe(42);
      expect(msg.size).toBe(payload.length);
      expect(decode(msg.data)).toBe("test payload");
    });

    it("should copy data so sender buffer modifications do not affect queue", () => {
      const mq = new MessageQueue();
      const buffer = new Uint8Array([1, 2, 3]);
      mq.send(1, buffer);

      // Modify the sender's buffer after sending
      buffer[0] = 99;

      // The queued message should be unaffected
      const msg = mq.receive(0)!;
      expect(msg.data[0]).toBe(1);
    });
  });

  // --------------------------------------------------------------------------
  // Status queries
  // --------------------------------------------------------------------------

  describe("status", () => {
    it("should report isEmpty and messageCount correctly", () => {
      const mq = new MessageQueue();
      expect(mq.isEmpty).toBe(true);
      expect(mq.messageCount).toBe(0);

      mq.send(1, encode("a"));
      expect(mq.isEmpty).toBe(false);
      expect(mq.messageCount).toBe(1);

      mq.receive(0);
      expect(mq.isEmpty).toBe(true);
      expect(mq.messageCount).toBe(0);
    });
  });
});

// ============================================================================
// SharedMemoryRegion Tests
// ============================================================================

describe("SharedMemoryRegion", () => {
  // --------------------------------------------------------------------------
  // Attach and detach
  // --------------------------------------------------------------------------

  describe("attach/detach", () => {
    it("should attach a PID successfully", () => {
      const region = new SharedMemoryRegion("test", 1024, 1);
      expect(region.attach(100)).toBe(true);
      expect(region.isAttached(100)).toBe(true);
      expect(region.attachedCount).toBe(1);
    });

    it("should return false when attaching an already-attached PID", () => {
      const region = new SharedMemoryRegion("test", 1024, 1);
      region.attach(100);
      expect(region.attach(100)).toBe(false); // already attached
    });

    it("should detach a PID successfully", () => {
      const region = new SharedMemoryRegion("test", 1024, 1);
      region.attach(100);
      expect(region.detach(100)).toBe(true);
      expect(region.isAttached(100)).toBe(false);
      expect(region.attachedCount).toBe(0);
    });

    it("should return false when detaching a non-attached PID", () => {
      const region = new SharedMemoryRegion("test", 1024, 1);
      expect(region.detach(999)).toBe(false);
    });

    it("should handle multiple PIDs attached simultaneously", () => {
      const region = new SharedMemoryRegion("test", 1024, 1);
      region.attach(10);
      region.attach(20);
      region.attach(30);

      expect(region.attachedCount).toBe(3);
      expect(region.pids).toEqual(new Set([10, 20, 30]));

      region.detach(20);
      expect(region.attachedCount).toBe(2);
      expect(region.isAttached(20)).toBe(false);
    });
  });

  // --------------------------------------------------------------------------
  // Read and write at offsets
  // --------------------------------------------------------------------------

  describe("read/write", () => {
    it("should write and read data at a given offset", () => {
      const region = new SharedMemoryRegion("test", 1024, 1);
      const data = encode("hello");

      const written = region.write(0, data);
      expect(written).toBe(5);

      const result = region.read(0, 5);
      expect(decode(result)).toBe("hello");
    });

    it("should write at non-zero offset", () => {
      const region = new SharedMemoryRegion("test", 1024, 1);
      region.write(100, encode("world"));

      const result = region.read(100, 5);
      expect(decode(result)).toBe("world");
    });

    it("should initialize all bytes to 0", () => {
      const region = new SharedMemoryRegion("test", 16, 1);
      const data = region.read(0, 16);
      expect(data.every((b) => b === 0)).toBe(true);
    });

    it("should handle overlapping writes (last write wins)", () => {
      const region = new SharedMemoryRegion("test", 1024, 1);
      region.write(0, encode("aaaa"));
      region.write(2, encode("bb")); // overwrite positions 2-3

      const result = region.read(0, 4);
      expect(decode(result)).toBe("aabb");
    });
  });

  // --------------------------------------------------------------------------
  // Bounds checking
  // --------------------------------------------------------------------------

  describe("bounds checking", () => {
    it("should throw on read out of bounds", () => {
      const region = new SharedMemoryRegion("test", 16, 1);
      expect(() => region.read(14, 4)).toThrow(IPCError); // 14+4=18 > 16
    });

    it("should throw on write out of bounds", () => {
      const region = new SharedMemoryRegion("test", 16, 1);
      expect(() => region.write(14, encode("abcd"))).toThrow(IPCError);
    });

    it("should throw on negative offset for read", () => {
      const region = new SharedMemoryRegion("test", 16, 1);
      expect(() => region.read(-1, 4)).toThrow(IPCError);
    });

    it("should throw on negative offset for write", () => {
      const region = new SharedMemoryRegion("test", 16, 1);
      expect(() => region.write(-1, encode("x"))).toThrow(IPCError);
    });

    it("should throw on negative count for read", () => {
      const region = new SharedMemoryRegion("test", 16, 1);
      expect(() => region.read(0, -1)).toThrow(IPCError);
    });

    it("should allow read/write at exact boundary", () => {
      const region = new SharedMemoryRegion("test", 4, 1);
      region.write(0, encode("abcd")); // exactly fills the region
      expect(decode(region.read(0, 4))).toBe("abcd");
    });
  });

  // --------------------------------------------------------------------------
  // Constructor validation
  // --------------------------------------------------------------------------

  describe("constructor", () => {
    it("should reject non-positive size", () => {
      expect(() => new SharedMemoryRegion("bad", 0, 1)).toThrow(IPCError);
      expect(() => new SharedMemoryRegion("bad", -1, 1)).toThrow(IPCError);
    });

    it("should store name, size, and ownerPid", () => {
      const region = new SharedMemoryRegion("buffer_pool", 8192, 42);
      expect(region.name).toBe("buffer_pool");
      expect(region.size).toBe(8192);
      expect(region.ownerPid).toBe(42);
    });
  });
});

// ============================================================================
// IPCManager Tests
// ============================================================================

describe("IPCManager", () => {
  // --------------------------------------------------------------------------
  // Pipe management
  // --------------------------------------------------------------------------

  describe("pipe management", () => {
    it("should create a pipe and return a handle with IDs", () => {
      const mgr = new IPCManager();
      const handle = mgr.createPipe();

      expect(handle.pipeId).toBe(0);
      expect(typeof handle.readFd).toBe("number");
      expect(typeof handle.writeFd).toBe("number");
      expect(handle.readFd).not.toBe(handle.writeFd);
    });

    it("should retrieve a pipe by ID", () => {
      const mgr = new IPCManager();
      const handle = mgr.createPipe();
      const pipe = mgr.getPipe(handle.pipeId);
      expect(pipe).toBeDefined();
      expect(pipe).toBeInstanceOf(Pipe);
    });

    it("should return undefined for non-existent pipe", () => {
      const mgr = new IPCManager();
      expect(mgr.getPipe(999)).toBeUndefined();
    });

    it("should close read/write ends via manager", () => {
      const mgr = new IPCManager();
      const handle = mgr.createPipe();
      const pipe = mgr.getPipe(handle.pipeId)!;

      mgr.closePipeRead(handle.pipeId);
      expect(pipe.readers).toBe(0);

      mgr.closePipeWrite(handle.pipeId);
      expect(pipe.writers).toBe(0);
    });

    it("should throw when closing non-existent pipe", () => {
      const mgr = new IPCManager();
      expect(() => mgr.closePipeRead(999)).toThrow(IPCError);
      expect(() => mgr.closePipeWrite(999)).toThrow(IPCError);
    });

    it("should destroy a pipe", () => {
      const mgr = new IPCManager();
      const handle = mgr.createPipe();
      mgr.destroyPipe(handle.pipeId);
      expect(mgr.getPipe(handle.pipeId)).toBeUndefined();
    });

    it("should throw when destroying non-existent pipe", () => {
      const mgr = new IPCManager();
      expect(() => mgr.destroyPipe(999)).toThrow(IPCError);
    });

    it("should create pipe with custom capacity", () => {
      const mgr = new IPCManager();
      const handle = mgr.createPipe(128);
      const pipe = mgr.getPipe(handle.pipeId)!;
      expect(pipe.capacity).toBe(128);
    });
  });

  // --------------------------------------------------------------------------
  // Message queue management
  // --------------------------------------------------------------------------

  describe("message queue management", () => {
    it("should create a message queue", () => {
      const mgr = new IPCManager();
      const mq = mgr.createMessageQueue("jobs");
      expect(mq).toBeInstanceOf(MessageQueue);
    });

    it("should return existing queue for same name (idempotent)", () => {
      const mgr = new IPCManager();
      const mq1 = mgr.createMessageQueue("jobs");
      const mq2 = mgr.createMessageQueue("jobs");
      expect(mq1).toBe(mq2); // same object
    });

    it("should retrieve a queue by name", () => {
      const mgr = new IPCManager();
      mgr.createMessageQueue("alerts");
      expect(mgr.getMessageQueue("alerts")).toBeDefined();
    });

    it("should return undefined for non-existent queue", () => {
      const mgr = new IPCManager();
      expect(mgr.getMessageQueue("nope")).toBeUndefined();
    });

    it("should delete a queue", () => {
      const mgr = new IPCManager();
      mgr.createMessageQueue("temp");
      mgr.deleteMessageQueue("temp");
      expect(mgr.getMessageQueue("temp")).toBeUndefined();
    });

    it("should throw when deleting non-existent queue", () => {
      const mgr = new IPCManager();
      expect(() => mgr.deleteMessageQueue("nope")).toThrow(IPCError);
    });
  });

  // --------------------------------------------------------------------------
  // Shared memory management
  // --------------------------------------------------------------------------

  describe("shared memory management", () => {
    it("should create a shared memory region", () => {
      const mgr = new IPCManager();
      const region = mgr.createSharedMemory("buffer", 4096, 1);
      expect(region).toBeInstanceOf(SharedMemoryRegion);
      expect(region.name).toBe("buffer");
    });

    it("should return existing region for same name (idempotent)", () => {
      const mgr = new IPCManager();
      const r1 = mgr.createSharedMemory("pool", 4096, 1);
      const r2 = mgr.createSharedMemory("pool", 8192, 2);
      expect(r1).toBe(r2); // same object, original size retained
    });

    it("should retrieve a region by name", () => {
      const mgr = new IPCManager();
      mgr.createSharedMemory("data", 1024, 1);
      expect(mgr.getSharedMemory("data")).toBeDefined();
    });

    it("should return undefined for non-existent region", () => {
      const mgr = new IPCManager();
      expect(mgr.getSharedMemory("nope")).toBeUndefined();
    });

    it("should delete a region", () => {
      const mgr = new IPCManager();
      mgr.createSharedMemory("temp", 512, 1);
      mgr.deleteSharedMemory("temp");
      expect(mgr.getSharedMemory("temp")).toBeUndefined();
    });

    it("should throw when deleting non-existent region", () => {
      const mgr = new IPCManager();
      expect(() => mgr.deleteSharedMemory("nope")).toThrow(IPCError);
    });
  });

  // --------------------------------------------------------------------------
  // Listing resources
  // --------------------------------------------------------------------------

  describe("listing resources", () => {
    it("should list all pipes", () => {
      const mgr = new IPCManager();
      mgr.createPipe();
      mgr.createPipe();
      expect(mgr.listPipes()).toEqual([0, 1]);
    });

    it("should list all message queues", () => {
      const mgr = new IPCManager();
      mgr.createMessageQueue("a");
      mgr.createMessageQueue("b");
      expect(mgr.listMessageQueues()).toEqual(["a", "b"]);
    });

    it("should list all shared regions", () => {
      const mgr = new IPCManager();
      mgr.createSharedMemory("x", 100, 1);
      mgr.createSharedMemory("y", 200, 2);
      expect(mgr.listSharedRegions()).toEqual(["x", "y"]);
    });

    it("should return empty lists when no resources exist", () => {
      const mgr = new IPCManager();
      expect(mgr.listPipes()).toEqual([]);
      expect(mgr.listMessageQueues()).toEqual([]);
      expect(mgr.listSharedRegions()).toEqual([]);
    });

    it("should update lists after deletion", () => {
      const mgr = new IPCManager();
      const h = mgr.createPipe();
      mgr.createMessageQueue("q");
      mgr.createSharedMemory("s", 100, 1);

      mgr.destroyPipe(h.pipeId);
      mgr.deleteMessageQueue("q");
      mgr.deleteSharedMemory("s");

      expect(mgr.listPipes()).toEqual([]);
      expect(mgr.listMessageQueues()).toEqual([]);
      expect(mgr.listSharedRegions()).toEqual([]);
    });
  });

  // --------------------------------------------------------------------------
  // Integration: using IPC resources through the manager
  // --------------------------------------------------------------------------

  describe("integration", () => {
    it("should support full pipe lifecycle through manager", () => {
      const mgr = new IPCManager();
      const handle = mgr.createPipe(64);
      const pipe = mgr.getPipe(handle.pipeId)!;

      // Write and read through the pipe
      pipe.write(encode("integration test"));
      expect(decode(pipe.read(16))).toBe("integration test");

      // Close and destroy
      mgr.closePipeWrite(handle.pipeId);
      expect(pipe.isEof).toBe(true);
      mgr.destroyPipe(handle.pipeId);
    });

    it("should support message queue send/receive through manager", () => {
      const mgr = new IPCManager();
      const mq = mgr.createMessageQueue("work");

      mq.send(1, encode("job-1"));
      mq.send(2, encode("job-2"));

      // Retrieve through manager
      const retrieved = mgr.getMessageQueue("work")!;
      expect(decode(retrieved.receive(2)!.data)).toBe("job-2");
      expect(decode(retrieved.receive(1)!.data)).toBe("job-1");
    });

    it("should support shared memory multi-PID access through manager", () => {
      const mgr = new IPCManager();
      const region = mgr.createSharedMemory("shared", 256, 1);

      region.attach(10);
      region.attach(20);

      // PID 10 writes
      region.write(0, encode("shared data"));

      // PID 20 reads (sees the same data — zero-copy!)
      expect(decode(region.read(0, 11))).toBe("shared data");

      // Cleanup
      region.detach(10);
      region.detach(20);
      expect(region.attachedCount).toBe(0);
    });
  });
});
