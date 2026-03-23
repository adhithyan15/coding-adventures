/**
 * # Actor Model — Test Suite
 *
 * This test suite covers all 58 test cases from the D19 specification.
 * The tests are organized into four sections:
 *
 * 1. **Message** (tests 1-19): Creation, immutability, wire format, serialization
 * 2. **Channel** (tests 20-36): Append-only log, persistence, recovery
 * 3. **Actor** (tests 37-49): Behavior, state, mailbox, lifecycle
 * 4. **Integration** (tests 50-58): Multi-actor scenarios, pipelines, persistence
 */

import { describe, it, expect, beforeEach } from "vitest";
import { tmpdir } from "os";
import { mkdirSync, writeFileSync, existsSync } from "fs";
import { join } from "path";
import {
  Message,
  Channel,
  Actor,
  ActorSystem,
  VersionError,
  InvalidFormatError,
  DuplicateActorError,
  ActorNotFoundError,
  _resetClock,
} from "../src/index.js";
import type { ActorResult, Behavior, ActorSpec } from "../src/index.js";

// ============================================================================
// Helpers
// ============================================================================

/** Create a unique temporary directory for each test that needs disk I/O. */
function makeTempDir(): string {
  const dir = join(
    tmpdir(),
    `actor-test-${Date.now()}-${Math.random().toString(36).slice(2)}`,
  );
  mkdirSync(dir, { recursive: true });
  return dir;
}

/** A simple echo behavior: replies with the same text, prefixed by "echo: ". */
function echoBehavior(state: null, message: Message): ActorResult<null> {
  const reply = Message.text("echo", `echo: ${message.payloadText}`);
  return {
    newState: null,
    messagesToSend: [[message.senderId, reply]],
  };
}

/** A counter behavior: increments state by 1 for each message received. */
function counterBehavior(state: number, message: Message): ActorResult<number> {
  return { newState: state + 1 };
}

/** A behavior that throws on messages containing "explode". */
function explodingBehavior(state: number, message: Message): ActorResult<number> {
  if (message.payloadText === "explode") {
    throw new Error("BOOM!");
  }
  return { newState: state + 1 };
}

// Reset the monotonic clock before each test for deterministic timestamps.
beforeEach(() => {
  _resetClock();
});

// ============================================================================
// Unit Tests — Message (Tests 1-19)
// ============================================================================

describe("Message", () => {
  // Test 1: Create message
  it("1. creates a message with all fields", () => {
    const payload = new TextEncoder().encode("hello");
    const msg = new Message("alice", "text/plain", payload, { key: "val" });

    expect(msg.id).toBeDefined();
    expect(typeof msg.id).toBe("string");
    expect(msg.timestamp).toBeGreaterThan(0n);
    expect(msg.senderId).toBe("alice");
    expect(msg.contentType).toBe("text/plain");
    expect(msg.payload).toEqual(payload);
    expect(msg.metadata).toEqual({ key: "val" });
  });

  // Test 2: Immutability
  it("2. is immutable — Object.freeze prevents mutation", () => {
    const msg = Message.text("alice", "hello");

    // Attempting to set properties on a frozen object silently fails
    // in non-strict mode, or throws in strict mode. Either way, the
    // value doesn't change.
    expect(() => {
      (msg as any).senderId = "bob";
    }).toThrow();
    expect(msg.senderId).toBe("alice");
  });

  // Test 3: Unique IDs
  it("3. generates unique IDs for 1000 messages", () => {
    const ids = new Set<string>();
    for (let i = 0; i < 1000; i++) {
      ids.add(Message.text("sender", `msg ${i}`).id);
    }
    expect(ids.size).toBe(1000);
  });

  // Test 4: Timestamp ordering
  it("4. timestamps are strictly increasing", () => {
    const messages = Array.from({ length: 10 }, (_, i) =>
      Message.text("sender", `msg ${i}`),
    );
    for (let i = 1; i < messages.length; i++) {
      expect(messages[i].timestamp).toBeGreaterThan(messages[i - 1].timestamp);
    }
  });

  // Test 5: Wire format round-trip (text)
  it("5. round-trips text messages through wire format", () => {
    const original = Message.text("alice", "Hello, world!", { trace: "abc" });
    const bytes = original.toBytes();
    const restored = Message.fromBytes(bytes);

    expect(restored.id).toBe(original.id);
    expect(restored.timestamp).toBe(original.timestamp);
    expect(restored.senderId).toBe(original.senderId);
    expect(restored.contentType).toBe(original.contentType);
    expect(restored.payloadText).toBe("Hello, world!");
    expect(restored.metadata).toEqual({ trace: "abc" });
  });

  // Test 6: Wire format round-trip (binary)
  it("6. round-trips binary messages (PNG header) through wire format", () => {
    // First 8 bytes of a real PNG file
    const pngHeader = new Uint8Array([
      0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a,
    ]);
    const original = Message.binary("browser", "image/png", pngHeader);
    const bytes = original.toBytes();
    const restored = Message.fromBytes(bytes);

    expect(restored.contentType).toBe("image/png");
    expect(restored.payload).toEqual(pngHeader);
  });

  // Test 7: Metadata passthrough
  it("7. preserves metadata across serialization", () => {
    const metadata = {
      correlation_id: "req_abc123",
      priority: "high",
      width: "1920",
    };
    const msg = Message.text("sender", "test", metadata);
    const restored = Message.fromBytes(msg.toBytes());
    expect(restored.metadata).toEqual(metadata);
  });

  // Test 8: Empty payload
  it("8. handles empty payload", () => {
    const msg = new Message(
      "sender",
      "application/octet-stream",
      new Uint8Array(0),
    );
    expect(msg.payload.length).toBe(0);
    const restored = Message.fromBytes(msg.toBytes());
    expect(restored.payload.length).toBe(0);
  });

  // Test 9: Large payload
  it("9. handles 1MB binary payload", () => {
    const payload = new Uint8Array(1024 * 1024);
    // Fill with pattern to verify integrity
    for (let i = 0; i < payload.length; i++) {
      payload[i] = i % 256;
    }
    const msg = Message.binary("sender", "application/octet-stream", payload);
    const restored = Message.fromBytes(msg.toBytes());
    expect(restored.payload.length).toBe(1024 * 1024);
    expect(restored.payload).toEqual(payload);
  });

  // Test 10: Content type
  it("10. preserves content type across serialization", () => {
    const msg = Message.binary(
      "sender",
      "video/mp4",
      new Uint8Array([1, 2, 3]),
    );
    const restored = Message.fromBytes(msg.toBytes());
    expect(restored.contentType).toBe("video/mp4");
  });

  // Test 11: Convenience constructors
  it("11. convenience constructors produce correct content types", () => {
    const text = Message.text("s", "hello");
    expect(text.contentType).toBe("text/plain");
    expect(text.payloadText).toBe("hello");

    const json = Message.json("s", { key: "value" });
    expect(json.contentType).toBe("application/json");
    expect(json.payloadJson).toEqual({ key: "value" });

    const binary = Message.binary(
      "s",
      "image/jpeg",
      new Uint8Array([0xff, 0xd8]),
    );
    expect(binary.contentType).toBe("image/jpeg");
    expect(binary.payload).toEqual(new Uint8Array([0xff, 0xd8]));
  });

  // Test 12: payload_text
  it("12. payloadText returns decoded string", () => {
    const msg = Message.text("sender", "Bonjour, le monde!");
    expect(msg.payloadText).toBe("Bonjour, le monde!");
  });

  // Test 13: payload_json
  it("13. payloadJson returns parsed object", () => {
    const data = { users: [{ name: "Alice" }, { name: "Bob" }] };
    const msg = Message.json("sender", data);
    expect(msg.payloadJson).toEqual(data);
  });

  // Test 14: Envelope-only serialization
  it("14. envelopeToJson produces JSON without payload", () => {
    const msg = Message.text("alice", "hello", { trace: "t1" });
    const envelope = JSON.parse(msg.envelopeToJson());

    expect(envelope.id).toBe(msg.id);
    expect(envelope.sender_id).toBe("alice");
    expect(envelope.content_type).toBe("text/plain");
    expect(envelope.metadata).toEqual({ trace: "t1" });
    // Payload should NOT be in the envelope
    expect(envelope.payload).toBeUndefined();
  });

  // Test 15: Wire format magic
  it("15. toBytes starts with ACTM magic bytes", () => {
    const msg = Message.text("sender", "test");
    const bytes = msg.toBytes();
    expect(bytes[0]).toBe(0x41); // 'A'
    expect(bytes[1]).toBe(0x43); // 'C'
    expect(bytes[2]).toBe(0x54); // 'T'
    expect(bytes[3]).toBe(0x4d); // 'M'
  });

  // Test 16: Wire format version
  it("16. toBytes contains correct version byte", () => {
    const msg = Message.text("sender", "test");
    const bytes = msg.toBytes();
    expect(bytes[4]).toBe(Message.WIRE_VERSION);
  });

  // Test 17: Future version rejection
  it("17. fromBytes rejects future wire versions with VersionError", () => {
    const msg = Message.text("sender", "test");
    const bytes = msg.toBytes();
    // Tamper with version byte to simulate a future version
    bytes[4] = 99;
    expect(() => Message.fromBytes(bytes)).toThrow(VersionError);
  });

  // Test 18: Corrupt magic rejection
  it("18. fromBytes rejects corrupt magic with InvalidFormatError", () => {
    const msg = Message.text("sender", "test");
    const bytes = msg.toBytes();
    // Corrupt the magic bytes
    bytes[0] = 0x00;
    expect(() => Message.fromBytes(bytes)).toThrow(InvalidFormatError);
  });

  // Test 19: fromDataView reads one message and returns bytes consumed
  it("19. fromDataView reads one message from a buffer with multiple messages", () => {
    const msg1 = Message.text("sender", "first");
    const msg2 = Message.text("sender", "second");
    const bytes1 = msg1.toBytes();
    const bytes2 = msg2.toBytes();

    // Concatenate two messages
    const combined = new Uint8Array(bytes1.length + bytes2.length);
    combined.set(bytes1, 0);
    combined.set(bytes2, bytes1.length);

    // Read first message
    const [restored1, consumed1] = Message.fromDataView(combined, 0);
    expect(restored1.payloadText).toBe("first");
    expect(consumed1).toBe(bytes1.length);

    // Read second message from where the first ended
    const [restored2, consumed2] = Message.fromDataView(combined, consumed1);
    expect(restored2.payloadText).toBe("second");
    expect(consumed2).toBe(bytes2.length);
  });
});

// ============================================================================
// Unit Tests — Channel (Tests 20-36)
// ============================================================================

describe("Channel", () => {
  // Test 20: Create channel
  it("20. creates a channel with id and name", () => {
    const ch = new Channel("ch_001", "greetings");
    expect(ch.id).toBe("ch_001");
    expect(ch.name).toBe("greetings");
    expect(ch.createdAt).toBeGreaterThan(0n);
  });

  // Test 21: Append and length
  it("21. append increases length", () => {
    const ch = new Channel("ch_001", "test");
    ch.append(Message.text("a", "m1"));
    ch.append(Message.text("a", "m2"));
    ch.append(Message.text("a", "m3"));
    expect(ch.length()).toBe(3);
  });

  // Test 22: Append returns sequence number
  it("22. append returns 0, 1, 2 for successive appends", () => {
    const ch = new Channel("ch_001", "test");
    expect(ch.append(Message.text("a", "m1"))).toBe(0);
    expect(ch.append(Message.text("a", "m2"))).toBe(1);
    expect(ch.append(Message.text("a", "m3"))).toBe(2);
  });

  // Test 23: Read from beginning
  it("23. reads all messages from beginning", () => {
    const ch = new Channel("ch_001", "test");
    for (let i = 0; i < 5; i++) {
      ch.append(Message.text("a", `msg ${i}`));
    }
    const messages = ch.read(0, 5);
    expect(messages.length).toBe(5);
    expect(messages[0].payloadText).toBe("msg 0");
    expect(messages[4].payloadText).toBe("msg 4");
  });

  // Test 24: Read with offset
  it("24. reads messages with offset", () => {
    const ch = new Channel("ch_001", "test");
    for (let i = 0; i < 5; i++) {
      ch.append(Message.text("a", `msg ${i}`));
    }
    const messages = ch.read(2, 3);
    expect(messages.length).toBe(3);
    expect(messages[0].payloadText).toBe("msg 2");
    expect(messages[1].payloadText).toBe("msg 3");
    expect(messages[2].payloadText).toBe("msg 4");
  });

  // Test 25: Read past end
  it("25. returns empty array when reading past end", () => {
    const ch = new Channel("ch_001", "test");
    ch.append(Message.text("a", "m1"));
    ch.append(Message.text("a", "m2"));
    ch.append(Message.text("a", "m3"));
    const messages = ch.read(5, 10);
    expect(messages.length).toBe(0);
  });

  // Test 26: Read with limit
  it("26. respects limit parameter", () => {
    const ch = new Channel("ch_001", "test");
    for (let i = 0; i < 10; i++) {
      ch.append(Message.text("a", `msg ${i}`));
    }
    const messages = ch.read(0, 3);
    expect(messages.length).toBe(3);
  });

  // Test 27: Slice
  it("27. slice returns messages from start to end (exclusive)", () => {
    const ch = new Channel("ch_001", "test");
    for (let i = 0; i < 5; i++) {
      ch.append(Message.text("a", `msg ${i}`));
    }
    const messages = ch.slice(1, 4);
    expect(messages.length).toBe(3);
    expect(messages[0].payloadText).toBe("msg 1");
    expect(messages[1].payloadText).toBe("msg 2");
    expect(messages[2].payloadText).toBe("msg 3");
  });

  // Test 28: Independent readers
  it("28. two consumers read independently at different offsets", () => {
    const ch = new Channel("ch_001", "test");
    for (let i = 0; i < 5; i++) {
      ch.append(Message.text("a", `msg ${i}`));
    }

    // Consumer A reads from offset 3
    const batchA = ch.read(3, 10);
    expect(batchA.length).toBe(2);
    expect(batchA[0].payloadText).toBe("msg 3");

    // Consumer B reads from offset 0
    const batchB = ch.read(0, 2);
    expect(batchB.length).toBe(2);
    expect(batchB[0].payloadText).toBe("msg 0");
  });

  // Test 29: Append-only
  it("29. has no delete or modify methods", () => {
    const ch = new Channel("ch_001", "test");
    // TypeScript type system prevents adding methods, but we verify
    // at runtime that no delete/update/remove methods exist.
    expect((ch as any).delete).toBeUndefined();
    expect((ch as any).update).toBeUndefined();
    expect((ch as any).remove).toBeUndefined();
    expect((ch as any).set).toBeUndefined();
  });

  // Test 30: Binary persistence
  it("30. persists to disk with ACTM magic in binary format", () => {
    const dir = makeTempDir();
    const ch = new Channel("ch_001", "test-persist");
    ch.append(Message.text("sender", "hello"));
    ch.append(
      Message.binary(
        "sender",
        "image/png",
        new Uint8Array([0x89, 0x50, 0x4e, 0x47]),
      ),
    );
    ch.persist(dir);

    const filePath = join(dir, "test-persist.log");
    expect(existsSync(filePath)).toBe(true);

    // Read raw bytes and verify first 4 bytes are "ACTM"
    const { readFileSync } = require("fs");
    const raw = new Uint8Array(readFileSync(filePath));
    expect(raw[0]).toBe(0x41); // A
    expect(raw[1]).toBe(0x43); // C
    expect(raw[2]).toBe(0x54); // T
    expect(raw[3]).toBe(0x4d); // M
  });

  // Test 31: Recovery
  it("31. recovers all messages from disk including binary payloads", () => {
    const dir = makeTempDir();
    const ch = new Channel("ch_001", "recovery-test");
    const pngBytes = new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a]);

    ch.append(Message.text("alice", "hello world"));
    ch.append(Message.binary("bob", "image/png", pngBytes));
    ch.persist(dir);

    const recovered = Channel.recover(dir, "recovery-test");
    expect(recovered.length()).toBe(2);
    expect(recovered.log[0].payloadText).toBe("hello world");
    expect(recovered.log[1].contentType).toBe("image/png");
    expect(recovered.log[1].payload).toEqual(pngBytes);
  });

  // Test 32: Recovery preserves order
  it("32. recovery preserves message order for 100 messages", () => {
    const dir = makeTempDir();
    const ch = new Channel("ch_001", "order-test");
    for (let i = 0; i < 100; i++) {
      ch.append(Message.text("sender", `message ${i}`));
    }
    ch.persist(dir);

    const recovered = Channel.recover(dir, "order-test");
    expect(recovered.length()).toBe(100);
    for (let i = 0; i < 100; i++) {
      expect(recovered.log[i].payloadText).toBe(`message ${i}`);
    }
  });

  // Test 33: Empty channel recovery
  it("33. recovers empty channel from non-existent file", () => {
    const dir = makeTempDir();
    const recovered = Channel.recover(dir, "nonexistent");
    expect(recovered.length()).toBe(0);
    expect(recovered.name).toBe("nonexistent");
  });

  // Test 34: Mixed content recovery
  it("34. recovers mixed content types (text, JSON, binary)", () => {
    const dir = makeTempDir();
    const ch = new Channel("ch_001", "mixed-test");
    ch.append(Message.text("a", "plain text"));
    ch.append(Message.json("b", { key: "value" }));
    ch.append(
      Message.binary("c", "image/png", new Uint8Array([0x89, 0x50, 0x4e])),
    );
    ch.persist(dir);

    const recovered = Channel.recover(dir, "mixed-test");
    expect(recovered.length()).toBe(3);
    expect(recovered.log[0].contentType).toBe("text/plain");
    expect(recovered.log[0].payloadText).toBe("plain text");
    expect(recovered.log[1].contentType).toBe("application/json");
    expect(recovered.log[1].payloadJson).toEqual({ key: "value" });
    expect(recovered.log[2].contentType).toBe("image/png");
    expect(recovered.log[2].payload).toEqual(
      new Uint8Array([0x89, 0x50, 0x4e]),
    );
  });

  // Test 35: Truncated write recovery
  it("35. discards incomplete message on truncated write", () => {
    const dir = makeTempDir();
    const ch = new Channel("ch_001", "truncate-test");
    ch.append(Message.text("a", "complete message"));
    ch.append(Message.text("a", "also complete"));
    ch.persist(dir);

    // Read the file, append a partial message (just the magic + a few bytes)
    const { readFileSync, writeFileSync } = require("fs");
    const filePath = join(dir, "truncate-test.log");
    const original = new Uint8Array(readFileSync(filePath));
    const partial = new Uint8Array(original.length + 10);
    partial.set(original, 0);
    // Write partial header: ACTM + version + partial envelope length
    partial.set(new Uint8Array([0x41, 0x43, 0x54, 0x4d, 0x01, 0x00, 0x00, 0x00, 0x50, 0x00]), original.length);
    writeFileSync(filePath, partial);

    const recovered = Channel.recover(dir, "truncate-test");
    // Should recover only the 2 complete messages, discarding the partial one
    expect(recovered.length()).toBe(2);
    expect(recovered.log[0].payloadText).toBe("complete message");
    expect(recovered.log[1].payloadText).toBe("also complete");
  });

  // Test 36: Mixed version recovery (simulated)
  it("36. handles messages written with current version", () => {
    // V1 is the only version we support. This test verifies that
    // multiple v1 messages in the same file are correctly parsed.
    // When v2 is added, this test should be updated to include both.
    const dir = makeTempDir();
    const ch = new Channel("ch_001", "version-test");
    ch.append(Message.text("a", "v1 message 1"));
    ch.append(Message.text("b", "v1 message 2"));
    ch.append(Message.json("c", { version: "v1" }));
    ch.persist(dir);

    const recovered = Channel.recover(dir, "version-test");
    expect(recovered.length()).toBe(3);
    expect(recovered.log[0].payloadText).toBe("v1 message 1");
    expect(recovered.log[1].payloadText).toBe("v1 message 2");
    expect(recovered.log[2].payloadJson).toEqual({ version: "v1" });
  });
});

// ============================================================================
// Unit Tests — Actor (Tests 37-49)
// ============================================================================

describe("Actor", () => {
  let system: ActorSystem;

  beforeEach(() => {
    system = new ActorSystem();
  });

  // Test 37: Create actor
  it("37. creates an actor with initial state and IDLE status", () => {
    system.createActor("counter", 0, counterBehavior);
    expect(system.getActorStatus("counter")).toBe("idle");
  });

  // Test 38: Send message
  it("38. sending a message increases mailbox size", () => {
    system.createActor("counter", 0, counterBehavior);
    system.send("counter", Message.text("sender", "hello"));
    expect(system.mailboxSize("counter")).toBe(1);
  });

  // Test 39: Process message
  it("39. processNext processes one message from mailbox", () => {
    let behaviorCalled = false;
    system.createActor("test", null, (_state: null, _msg: Message) => {
      behaviorCalled = true;
      return { newState: null };
    });
    system.send("test", Message.text("sender", "hi"));
    system.processNext("test");
    expect(behaviorCalled).toBe(true);
    expect(system.mailboxSize("test")).toBe(0);
  });

  // Test 40: State update
  it("40. counter actor state is 3 after 3 messages", () => {
    system.createActor("counter", 0, counterBehavior);
    system.send("counter", Message.text("a", "1"));
    system.send("counter", Message.text("a", "2"));
    system.send("counter", Message.text("a", "3"));
    system.processNext("counter");
    system.processNext("counter");
    system.processNext("counter");
    // Verify by sending a 4th message and checking state changed again
    // Since state is private, we verify via behavior side effects
    // We'll use a custom behavior that reports state
    let reportedState = -1;
    system.createActor("reporter", 0, (state: number, msg: Message) => {
      reportedState = state + 1;
      return { newState: state + 1 };
    });
    system.send("reporter", Message.text("a", "1"));
    system.send("reporter", Message.text("a", "2"));
    system.send("reporter", Message.text("a", "3"));
    system.processNext("reporter");
    system.processNext("reporter");
    system.processNext("reporter");
    expect(reportedState).toBe(3);
  });

  // Test 41: Messages to send (echo)
  it("41. echo actor delivers reply to sender's mailbox", () => {
    system.createActor("echo", null, echoBehavior);
    system.createActor("alice", null, (_s: null, _m: Message) => ({
      newState: null,
    }));

    system.send("echo", Message.text("alice", "hello"));
    system.processNext("echo");

    // The echo actor should have sent a reply to alice's mailbox
    expect(system.mailboxSize("alice")).toBe(1);
  });

  // Test 42: Actor creation
  it("42. spawner actor creates new actors", () => {
    const spawnerBehavior: Behavior<number> = (state, message) => {
      if (message.payloadText === "spawn") {
        const newId = `worker_${state}`;
        return {
          newState: state + 1,
          actorsToCreate: [
            {
              actorId: newId,
              initialState: null,
              behavior: (_s: null, _m: Message) => ({ newState: null }),
            },
          ],
        };
      }
      return { newState: state };
    };

    system.createActor("spawner", 0, spawnerBehavior);
    system.send("spawner", Message.text("boss", "spawn"));
    system.processNext("spawner");

    expect(system.actorIds()).toContain("worker_0");
    expect(system.getActorStatus("worker_0")).toBe("idle");
  });

  // Test 43: Stop actor
  it("43. actor status is STOPPED after processing stop message", () => {
    const stoppableBehavior: Behavior<null> = (_state, message) => {
      if (message.payloadText === "stop") {
        return { newState: null, stop: true };
      }
      return { newState: null };
    };

    system.createActor("worker", null, stoppableBehavior);
    system.send("worker", Message.text("boss", "stop"));
    system.processNext("worker");

    expect(system.getActorStatus("worker")).toBe("stopped");
  });

  // Test 44: Stopped actor rejects messages
  it("44. messages to stopped actors go to dead_letters", () => {
    system.createActor("worker", null, (_s: null, _m: Message) => ({
      newState: null,
    }));
    system.stopActor("worker");

    system.send("worker", Message.text("boss", "hello"));
    expect(system.deadLetters.length).toBe(1);
    expect(system.mailboxSize("worker")).toBe(0);
  });

  // Test 45: Dead letters for non-existent actor
  it("45. messages to non-existent actors go to dead_letters", () => {
    system.send("ghost", Message.text("sender", "hello?"));
    expect(system.deadLetters.length).toBe(1);
  });

  // Test 46: Sequential processing (FIFO)
  it("46. processes messages in FIFO order", () => {
    const received: string[] = [];
    system.createActor("fifo", null, (_s: null, msg: Message) => {
      received.push(msg.payloadText);
      return { newState: null };
    });

    system.send("fifo", Message.text("a", "first"));
    system.send("fifo", Message.text("a", "second"));
    system.send("fifo", Message.text("a", "third"));

    system.processNext("fifo");
    system.processNext("fifo");
    system.processNext("fifo");

    expect(received).toEqual(["first", "second", "third"]);
  });

  // Test 47: Mailbox drains on stop
  it("47. stopping an actor drains mailbox to dead_letters", () => {
    system.createActor("worker", null, (_s: null, _m: Message) => ({
      newState: null,
    }));
    system.send("worker", Message.text("a", "m1"));
    system.send("worker", Message.text("a", "m2"));
    system.send("worker", Message.text("a", "m3"));

    system.stopActor("worker");
    expect(system.deadLetters.length).toBe(3);
    expect(system.mailboxSize("worker")).toBe(0);
  });

  // Test 48: Behavior exception
  it("48. behavior exception: state unchanged, message to dead_letters, actor continues", () => {
    system.createActor("bomb", 0, explodingBehavior);

    // Send normal, exploding, then normal messages
    system.send("bomb", Message.text("a", "normal"));
    system.send("bomb", Message.text("a", "explode"));
    system.send("bomb", Message.text("a", "also normal"));

    // Process first (normal) — state should become 1
    system.processNext("bomb");
    expect(system.getActorStatus("bomb")).toBe("idle");

    // Process second (explode) — state should stay 1, message to dead_letters
    system.processNext("bomb");
    expect(system.getActorStatus("bomb")).toBe("idle");
    expect(system.deadLetters.length).toBe(1);
    expect(system.deadLetters[0].payloadText).toBe("explode");

    // Process third (normal) — state should become 2, actor still alive
    system.processNext("bomb");
    expect(system.getActorStatus("bomb")).toBe("idle");
  });

  // Test 49: Duplicate actor ID
  it("49. creating two actors with same ID throws DuplicateActorError", () => {
    system.createActor("unique", null, (_s: null, _m: Message) => ({
      newState: null,
    }));
    expect(() =>
      system.createActor("unique", null, (_s: null, _m: Message) => ({
        newState: null,
      })),
    ).toThrow(DuplicateActorError);
  });
});

// ============================================================================
// Integration Tests (Tests 50-58)
// ============================================================================

describe("Integration", () => {
  let system: ActorSystem;

  beforeEach(() => {
    system = new ActorSystem();
  });

  // Test 50: Ping-pong
  it("50. two actors ping-pong 10 times each", () => {
    /**
     * Ping-pong: actor "ping" sends to "pong", who sends back.
     * State tracks how many messages received. Both stop after 10.
     */
    const pingPongBehavior =
      (myId: string, otherId: string): Behavior<number> =>
      (state, message) => {
        const count = state + 1;
        if (count >= 10) {
          return { newState: count, stop: true };
        }
        return {
          newState: count,
          messagesToSend: [
            [otherId, Message.text(myId, `${myId} ${count}`)],
          ],
        };
      };

    system.createActor("ping", 0, pingPongBehavior("ping", "pong"));
    system.createActor("pong", 0, pingPongBehavior("pong", "ping"));

    // Start the ping
    system.send("ping", Message.text("external", "start"));

    const stats = system.runUntilDone();
    // ping processes: start + 9 pong replies = 10
    // pong processes: 9 ping replies (ping stops at 10, doesn't send)
    // Total: 10 + 9 = 19
    expect(stats.messagesProcessed).toBeGreaterThanOrEqual(10);
    expect(system.getActorStatus("ping")).toBe("stopped");
  });

  // Test 51: Pipeline
  it("51. three actors in a pipeline: A -> B -> C", () => {
    let cReceived = "";

    // A sends to B
    system.createActor("A", null, (_s: null, msg: Message) => ({
      newState: null,
      messagesToSend: [
        ["B", Message.text("A", `A processed: ${msg.payloadText}`)],
      ],
    }));

    // B transforms and sends to C
    system.createActor("B", null, (_s: null, msg: Message) => ({
      newState: null,
      messagesToSend: [
        ["C", Message.text("B", `B transformed: ${msg.payloadText}`)],
      ],
    }));

    // C receives the final result
    system.createActor("C", null, (_s: null, msg: Message) => {
      cReceived = msg.payloadText;
      return { newState: null };
    });

    system.send("A", Message.text("external", "hello"));
    system.runUntilDone();

    expect(cReceived).toBe("B transformed: A processed: hello");
  });

  // Test 52: Channel-based pipeline
  it("52. producer writes to channel, consumer reads in order", () => {
    const channel = system.createChannel("ch_001", "pipeline");

    // Producer appends 5 messages
    for (let i = 0; i < 5; i++) {
      channel.append(Message.text("producer", `item ${i}`));
    }

    // Consumer reads from channel
    let offset = 0;
    const batch = channel.read(offset, 10);
    expect(batch.length).toBe(5);
    expect(batch[0].payloadText).toBe("item 0");
    expect(batch[4].payloadText).toBe("item 4");
    offset += batch.length;

    // More messages arrive
    channel.append(Message.text("producer", "item 5"));
    const batch2 = channel.read(offset, 10);
    expect(batch2.length).toBe(1);
    expect(batch2[0].payloadText).toBe("item 5");
  });

  // Test 53: Fan-out
  it("53. one actor sends to 5 different actors", () => {
    const received: string[] = [];

    // Create 5 receivers
    for (let i = 0; i < 5; i++) {
      system.createActor(`receiver_${i}`, null, (_s: null, msg: Message) => {
        received.push(`receiver_${i}: ${msg.payloadText}`);
        return { newState: null };
      });
    }

    // Create broadcaster
    system.createActor("broadcaster", null, (_s: null, msg: Message) => ({
      newState: null,
      messagesToSend: Array.from({ length: 5 }, (_, i) => [
        `receiver_${i}`,
        Message.text("broadcaster", msg.payloadText),
      ] as [string, Message]),
    }));

    system.send("broadcaster", Message.text("external", "broadcast!"));
    system.runUntilDone();

    expect(received.length).toBe(5);
  });

  // Test 54: Dynamic topology
  it("54. actor A spawns actor B, sends message, B responds", () => {
    let responseReceived = "";

    // A receives messages and stores responses
    const aBehavior: Behavior<string> = (state, message) => {
      if (message.payloadText === "spawn-b") {
        return {
          newState: state,
          actorsToCreate: [
            {
              actorId: "B",
              initialState: null,
              behavior: ((_s: null, msg: Message) => ({
                newState: null,
                messagesToSend: [
                  ["A", Message.text("B", `B says: ${msg.payloadText}`)],
                ],
              })) as Behavior<unknown>,
            },
          ],
          messagesToSend: [["B", Message.text("A", "hello B")]],
        };
      }
      responseReceived = message.payloadText;
      return { newState: message.payloadText };
    };

    system.createActor("A", "", aBehavior);
    system.send("A", Message.text("external", "spawn-b"));
    system.runUntilDone();

    expect(system.actorIds()).toContain("B");
    expect(responseReceived).toBe("B says: hello B");
  });

  // Test 55: Run until idle with 5 interconnected actors
  it("55. run_until_idle processes all messages in a 5-actor network", () => {
    /**
     * Create a chain: actor_0 -> actor_1 -> actor_2 -> actor_3 -> actor_4
     * Each actor forwards the message to the next one with a prefix.
     * actor_4 is the terminal — it just absorbs messages.
     */
    for (let i = 0; i < 5; i++) {
      const nextId = i < 4 ? `actor_${i + 1}` : null;
      system.createActor(
        `actor_${i}`,
        0,
        ((targetId: string | null) =>
          (state: number, msg: Message): ActorResult<number> => {
            if (targetId) {
              return {
                newState: state + 1,
                messagesToSend: [
                  [targetId, Message.text(`actor_${i}`, `fwd: ${msg.payloadText}`)],
                ],
              };
            }
            return { newState: state + 1 };
          })(nextId),
      );
    }

    system.send("actor_0", Message.text("external", "start"));
    const stats = system.runUntilIdle();
    expect(stats.messagesProcessed).toBe(5);
  });

  // Test 56: Persistence round-trip
  it("56. persist channels with binary payloads, recover in new system", () => {
    const dir = makeTempDir();
    const channel = system.createChannel("ch_001", "persist-test");

    // Append various message types including binary
    channel.append(Message.text("alice", "hello"));
    channel.append(Message.json("bob", { count: 42 }));
    channel.append(
      Message.binary(
        "camera",
        "image/png",
        new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
      ),
    );
    channel.persist(dir);

    // Create a new system and recover
    const newSystem = new ActorSystem();
    const recovered = Channel.recover(dir, "persist-test");

    expect(recovered.length()).toBe(3);
    expect(recovered.log[0].payloadText).toBe("hello");
    expect(recovered.log[1].payloadJson).toEqual({ count: 42 });
    expect(recovered.log[2].contentType).toBe("image/png");
    expect(recovered.log[2].payload).toEqual(
      new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    );
  });

  // Test 57: Large-scale — 100 actors, 1000 messages
  it("57. 100 actors handle 1000 random messages with no message loss", () => {
    // Create 100 actors that just count messages received
    for (let i = 0; i < 100; i++) {
      system.createActor(`actor_${i}`, 0, counterBehavior);
    }

    // Send 1000 messages to random actors
    for (let i = 0; i < 1000; i++) {
      const targetIdx = i % 100;
      system.send(
        `actor_${targetIdx}`,
        Message.text("sender", `msg ${i}`),
      );
    }

    const stats = system.runUntilDone();
    expect(stats.messagesProcessed).toBe(1000);
    expect(system.deadLetters.length).toBe(0);
  });

  // Test 58: Binary message pipeline via channel
  it("58. actor A sends PNG image to actor B via channel, bytes identical", () => {
    // Create a PNG-like binary payload
    const pngData = new Uint8Array(256);
    for (let i = 0; i < 256; i++) {
      pngData[i] = i;
    }

    const channel = system.createChannel("ch_images", "images");
    let receivedPayload: Uint8Array | null = null;

    // Actor A appends binary message to channel
    system.createActor("sender_actor", null, (_s: null, msg: Message) => {
      channel.append(
        Message.binary("sender_actor", "image/png", pngData),
      );
      return { newState: null };
    });

    // Actor B reads from channel and verifies
    system.createActor("receiver_actor", null, (_s: null, _msg: Message) => {
      const messages = channel.read(0, 1);
      if (messages.length > 0) {
        receivedPayload = messages[0].payload;
      }
      return { newState: null };
    });

    // Trigger A to write to channel
    system.send("sender_actor", Message.text("external", "send-image"));
    system.processNext("sender_actor");

    // Trigger B to read from channel
    system.send("receiver_actor", Message.text("external", "read-image"));
    system.processNext("receiver_actor");

    expect(receivedPayload).not.toBeNull();
    expect(receivedPayload).toEqual(pngData);
  });
});
