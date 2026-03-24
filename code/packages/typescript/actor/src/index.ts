/**
 * # Actor Model — Messages, Channels, and Actors
 *
 * ## What Is the Actor Model?
 *
 * The Actor model is a mathematical framework for concurrent computation invented
 * by Carl Hewitt, Peter Bishop, and Richard Steiger in 1973. It defines computation
 * in terms of **actors** — independent entities that communicate exclusively through
 * **messages**. No shared memory. No locks. No mutexes. Just isolated units of
 * computation passing immutable messages through one-way channels.
 *
 * Erlang/OTP — which powers telecom infrastructure and WhatsApp's backend — is built
 * entirely on the Actor model. Discord's Elixir backend, Akka in Scala/Java,
 * Microsoft Orleans in C# — all Actor systems.
 *
 * ## The Three Primitives
 *
 * This package implements three primitives:
 *
 * 1. **Message** — the atom of communication. Immutable, typed, serializable.
 *    Every piece of data that flows between actors is a Message.
 *
 * 2. **Channel** — a one-way, append-only pipe for messages. Persistent and
 *    replayable. Like a Kafka topic or a Unix FIFO, but simpler.
 *
 * 3. **Actor** — an isolated unit of computation with a mailbox and internal
 *    state. Processes one message at a time, can send messages, create new
 *    actors, and update its own state.
 *
 * ```
 * ┌──────────────────────────────────────────────────────┐
 * │  ActorSystem (the "world")                           │
 * │                                                      │
 * │  ┌─────────┐   Message   ┌─────────┐                │
 * │  │ Actor A  │ ─────────→ │ Actor B  │                │
 * │  │ mailbox  │             │ mailbox  │                │
 * │  │ state    │             │ state    │                │
 * │  └─────────┘             └─────────┘                │
 * │       │                                              │
 * │       │ append                                       │
 * │       ▼                                              │
 * │  ┌──────────────────┐                                │
 * │  │ Channel (log)    │                                │
 * │  │ [m0][m1][m2]...  │                                │
 * │  └──────────────────┘                                │
 * └──────────────────────────────────────────────────────┘
 * ```
 */

import {
  writeFileSync,
  readFileSync,
  existsSync,
  mkdirSync,
  appendFileSync,
} from "fs";
import { join } from "path";

// ============================================================================
// Constants
// ============================================================================

/**
 * ## Wire Format Magic Bytes
 *
 * Every serialized message starts with the 4-byte magic "ACTM" (0x41 0x43
 * 0x54 0x4D). This serves two purposes:
 *
 * 1. **Identification**: If you open a random file, these 4 bytes immediately
 *    tell you "this is an Actor message file."
 * 2. **Alignment**: During crash recovery, we scan for "ACTM" to find message
 *    boundaries if the file is truncated.
 *
 * The letters stand for "ACTor Message."
 */
const WIRE_MAGIC = new Uint8Array([0x41, 0x43, 0x54, 0x4d]); // "ACTM"

/**
 * ## Wire Format Version
 *
 * The version byte follows the magic and tells the reader how to parse the
 * rest of the message. V1 uses JSON for the envelope (human-readable,
 * debuggable). Future versions might use Protobuf or MessagePack.
 *
 * ```
 * Version → Encoding
 * v1:  JSON (UTF-8) — current
 * v2:  (future) Protobuf
 * v3:  (future) MessagePack
 * ```
 */
const WIRE_VERSION = 1;

/**
 * ## Header Size
 *
 * The fixed-size header is 17 bytes:
 *
 * ```
 * Offset  Size  Field
 * ──────  ────  ─────
 * 0       4     magic ("ACTM")
 * 4       1     version (0x01)
 * 5       4     envelope_length (big-endian uint32)
 * 9       8     payload_length (big-endian uint64)
 * ──────  ────
 * Total:  17 bytes
 * ```
 */
const HEADER_SIZE = 17;

// ============================================================================
// Error Types
// ============================================================================

/**
 * ## VersionError
 *
 * Thrown when we encounter a message with a wire format version higher than
 * what this code supports. This is NOT a corruption — it means the message
 * was written by a newer version of the software.
 *
 * The error message tells the user exactly what version was encountered and
 * what the maximum supported version is, so they know to upgrade.
 */
export class VersionError extends Error {
  constructor(
    public readonly encountered: number,
    public readonly maxSupported: number,
  ) {
    super(
      `Wire format version ${encountered} is not supported. ` +
        `Maximum supported version: ${maxSupported}. Please upgrade.`,
    );
    this.name = "VersionError";
  }
}

/**
 * ## InvalidFormatError
 *
 * Thrown when the magic bytes don't match "ACTM", meaning the data is not
 * a valid Actor message at all. Could be a corrupt file, wrong file type,
 * or truncated data.
 */
export class InvalidFormatError extends Error {
  constructor(message = "Invalid format: expected ACTM magic bytes") {
    super(message);
    this.name = "InvalidFormatError";
  }
}

/**
 * ## ActorNotFoundError
 *
 * Thrown when trying to interact with an actor that doesn't exist in the
 * system or has been stopped.
 */
export class ActorNotFoundError extends Error {
  constructor(actorId: string) {
    super(`Actor not found: ${actorId}`);
    this.name = "ActorNotFoundError";
  }
}

/**
 * ## DuplicateActorError
 *
 * Thrown when trying to create an actor with an ID that already exists.
 * Actor IDs must be unique within an ActorSystem.
 */
export class DuplicateActorError extends Error {
  constructor(actorId: string) {
    super(`Actor already exists: ${actorId}`);
    this.name = "DuplicateActorError";
  }
}

// ============================================================================
// Monotonic Clock
// ============================================================================

/**
 * ## Monotonic Clock
 *
 * Messages need timestamps that are strictly increasing. We cannot use
 * `Date.now()` because:
 *
 * 1. It has millisecond resolution — two messages created in the same
 *    millisecond would have the same timestamp.
 * 2. Wall-clock time can go backwards (NTP adjustments, leap seconds).
 *
 * Instead, we use a simple monotonic counter. Each call to `nextTimestamp()`
 * returns a value strictly greater than the previous one. This gives us
 * a total ordering of messages within a single system.
 *
 * We use `bigint` because the spec calls for nanosecond-precision counters,
 * and JavaScript's `number` type loses precision above 2^53.
 */
let globalClock = 0n;

function nextTimestamp(): bigint {
  globalClock += 1n;
  return globalClock;
}

/**
 * Reset the global clock. Used only in testing to ensure deterministic
 * behavior between test runs.
 */
export function _resetClock(): void {
  globalClock = 0n;
}

// ============================================================================
// Text Encoding Utilities
// ============================================================================

/**
 * ## TextEncoder / TextDecoder
 *
 * These are Web APIs available in all modern browsers and Node.js 11+.
 * We use them instead of Node.js `Buffer` to keep this package compatible
 * with browser environments. `TextEncoder` converts strings to UTF-8
 * `Uint8Array`; `TextDecoder` converts back.
 */
const encoder = new TextEncoder();
const decoder = new TextDecoder();

// ============================================================================
// Message
// ============================================================================

/**
 * ## Message — The Atom of Communication
 *
 * A Message is a sealed letter. Once created, its contents are fixed. The
 * envelope records who sent it, when, and what kind of data it carries.
 * You can make copies, but you cannot change the original.
 *
 * ### Immutability
 *
 * All properties are `readonly` in TypeScript (compile-time enforcement),
 * and the constructor calls `Object.freeze()` (runtime enforcement). This
 * means:
 *
 * ```typescript
 * const msg = Message.text("alice", "hello");
 * msg.senderId = "bob";  // TypeScript error at compile time
 * (msg as any).senderId = "bob";  // Silently fails at runtime (frozen)
 * ```
 *
 * ### Payload Is Always Bytes
 *
 * The payload is always a `Uint8Array`. The `contentType` field tells the
 * receiver how to interpret those bytes:
 *
 * ```
 * contentType               payload bytes are...
 * ─────────────              ────────────────────
 * text/plain                 UTF-8 text
 * application/json           JSON (UTF-8 text)
 * image/png                  raw PNG image
 * application/octet-stream   opaque binary
 * ```
 *
 * Convenience accessors `payloadText` and `payloadJson` handle decoding
 * for common cases.
 *
 * ### Wire Format
 *
 * Messages serialize to a binary format with a 17-byte header:
 *
 * ```
 * [ACTM][v1][envelope_len:u32][payload_len:u64][JSON envelope][raw payload]
 *  4B    1B   4B               8B               variable       variable
 * ```
 *
 * The envelope contains all metadata (id, timestamp, senderId, contentType,
 * metadata). The payload is raw bytes — no Base64, no encoding overhead.
 */
export class Message {
  /**
   * The current wire format version. Bump this when the serialization
   * format changes in a backwards-incompatible way.
   */
  static readonly WIRE_VERSION = WIRE_VERSION;

  /** Unique identifier. Generated automatically at creation time. */
  readonly id: string;

  /** Monotonic nanosecond counter. Strictly increasing within a system. */
  readonly timestamp: bigint;

  /** The actor that created this message. */
  readonly senderId: string;

  /** MIME-like string describing the payload format. */
  readonly contentType: string;

  /** The message body — always raw bytes. */
  readonly payload: Uint8Array;

  /** Optional key-value pairs for extensibility (trace IDs, priority, etc.). */
  readonly metadata: Readonly<Record<string, string>>;

  /**
   * Create a new Message.
   *
   * @param senderId - The actor that created this message
   * @param contentType - MIME type describing the payload format
   * @param payload - Raw bytes of the message body
   * @param metadata - Optional key-value pairs
   * @param id - Optional explicit ID (used during deserialization)
   * @param timestamp - Optional explicit timestamp (used during deserialization)
   */
  constructor(
    senderId: string,
    contentType: string,
    payload: Uint8Array,
    metadata: Record<string, string> = {},
    id?: string,
    timestamp?: bigint,
  ) {
    this.id = id ?? crypto.randomUUID();
    this.timestamp = timestamp ?? nextTimestamp();
    this.senderId = senderId;
    this.contentType = contentType;
    this.payload = payload;
    this.metadata = Object.freeze({ ...metadata });

    // Runtime immutability enforcement. Even if someone bypasses TypeScript's
    // readonly with `as any`, Object.freeze prevents actual mutation.
    Object.freeze(this);
  }

  // ────────────────────────────────────────────────────────────────────────
  // Convenience Constructors
  // ────────────────────────────────────────────────────────────────────────

  /**
   * Create a text message.
   *
   * The string is encoded to UTF-8 bytes and stored as the payload.
   * The content type is set to "text/plain".
   *
   * @example
   * ```typescript
   * const msg = Message.text("alice", "Hello, world!");
   * msg.contentType;  // "text/plain"
   * msg.payloadText;  // "Hello, world!"
   * ```
   */
  static text(
    senderId: string,
    payload: string,
    metadata?: Record<string, string>,
  ): Message {
    return new Message(
      senderId,
      "text/plain",
      encoder.encode(payload),
      metadata,
    );
  }

  /**
   * Create a JSON message.
   *
   * The value is serialized to a JSON string, then encoded to UTF-8 bytes.
   * The content type is set to "application/json".
   *
   * @example
   * ```typescript
   * const msg = Message.json("alice", { greeting: "hello" });
   * msg.contentType;    // "application/json"
   * msg.payloadJson;    // { greeting: "hello" }
   * ```
   */
  static json(
    senderId: string,
    payload: unknown,
    metadata?: Record<string, string>,
  ): Message {
    return new Message(
      senderId,
      "application/json",
      encoder.encode(JSON.stringify(payload)),
      metadata,
    );
  }

  /**
   * Create a binary message.
   *
   * The payload is raw bytes. The caller specifies the content type
   * (e.g., "image/png", "video/mp4", "application/octet-stream").
   *
   * @example
   * ```typescript
   * const pngBytes = new Uint8Array([0x89, 0x50, 0x4E, 0x47]);
   * const msg = Message.binary("browser", "image/png", pngBytes);
   * msg.contentType;  // "image/png"
   * msg.payload;      // Uint8Array [0x89, 0x50, 0x4E, 0x47]
   * ```
   */
  static binary(
    senderId: string,
    contentType: string,
    payload: Uint8Array,
    metadata?: Record<string, string>,
  ): Message {
    return new Message(senderId, contentType, payload, metadata);
  }

  // ────────────────────────────────────────────────────────────────────────
  // Convenience Accessors
  // ────────────────────────────────────────────────────────────────────────

  /**
   * Decode the payload as a UTF-8 string.
   *
   * This is a convenience accessor for text messages. If the payload is not
   * valid UTF-8, the decoder will replace invalid bytes with the Unicode
   * replacement character (U+FFFD).
   */
  get payloadText(): string {
    return decoder.decode(this.payload);
  }

  /**
   * Parse the payload as JSON.
   *
   * This is a convenience accessor for JSON messages. Throws a SyntaxError
   * if the payload is not valid JSON.
   */
  get payloadJson(): unknown {
    return JSON.parse(decoder.decode(this.payload));
  }

  // ────────────────────────────────────────────────────────────────────────
  // Serialization — Binary Wire Format
  // ────────────────────────────────────────────────────────────────────────

  /**
   * Serialize the envelope (everything except payload) to a JSON string.
   *
   * Useful for logging, indexing, and debugging — you can see all the
   * metadata about a message without touching the (potentially huge)
   * payload.
   *
   * The timestamp is serialized as a string because JSON cannot represent
   * BigInt natively. We prefix with "n" to distinguish from regular numbers
   * during deserialization.
   */
  envelopeToJson(): string {
    return JSON.stringify({
      id: this.id,
      timestamp: this.timestamp.toString(),
      sender_id: this.senderId,
      content_type: this.contentType,
      metadata: this.metadata,
    });
  }

  /**
   * ## toBytes — Serialize to Wire Format
   *
   * Produces the binary wire format:
   *
   * ```
   * ┌────────────────────────────────────────────┐
   * │ HEADER (17 bytes)                          │
   * │  magic:          "ACTM" (4 bytes)          │
   * │  version:        0x01   (1 byte)           │
   * │  envelope_length: u32   (4 bytes, big-end) │
   * │  payload_length:  u64   (8 bytes, big-end) │
   * ├────────────────────────────────────────────┤
   * │ ENVELOPE (UTF-8 JSON, variable length)     │
   * ├────────────────────────────────────────────┤
   * │ PAYLOAD (raw bytes, variable length)       │
   * └────────────────────────────────────────────┘
   * ```
   *
   * We use `DataView` for writing multi-byte integers because it gives
   * explicit control over byte order (big-endian). `Uint8Array` alone
   * would use the platform's native byte order, which varies between
   * architectures.
   */
  toBytes(): Uint8Array {
    const envelopeBytes = encoder.encode(this.envelopeToJson());
    const envelopeLength = envelopeBytes.length;
    const payloadLength = this.payload.length;

    // Total size = 17 header + envelope + payload
    const totalSize = HEADER_SIZE + envelopeLength + payloadLength;
    const buffer = new ArrayBuffer(totalSize);
    const view = new DataView(buffer);
    const bytes = new Uint8Array(buffer);

    // Write magic bytes: "ACTM"
    bytes.set(WIRE_MAGIC, 0);

    // Write version byte
    view.setUint8(4, WIRE_VERSION);

    // Write envelope length as big-endian uint32
    view.setUint32(5, envelopeLength, false);

    // Write payload length as big-endian uint64 (BigInt)
    view.setBigUint64(9, BigInt(payloadLength), false);

    // Write envelope JSON bytes
    bytes.set(envelopeBytes, HEADER_SIZE);

    // Write raw payload bytes
    bytes.set(this.payload, HEADER_SIZE + envelopeLength);

    return bytes;
  }

  /**
   * ## fromBytes — Deserialize from Wire Format
   *
   * Reads a complete message from a `Uint8Array`. Validates the magic bytes
   * and version number. Throws `InvalidFormatError` if the magic is wrong,
   * `VersionError` if the version is unsupported.
   *
   * @param data - The raw bytes to deserialize
   * @returns A new Message instance with all fields restored
   */
  static fromBytes(data: Uint8Array): Message {
    if (data.length < HEADER_SIZE) {
      throw new InvalidFormatError(
        `Data too short: expected at least ${HEADER_SIZE} bytes, got ${data.length}`,
      );
    }

    const view = new DataView(
      data.buffer,
      data.byteOffset,
      data.byteLength,
    );

    // Validate magic bytes
    for (let i = 0; i < 4; i++) {
      if (data[i] !== WIRE_MAGIC[i]) {
        throw new InvalidFormatError(
          `Invalid magic bytes at offset ${i}: expected 0x${WIRE_MAGIC[i].toString(16)}, got 0x${data[i].toString(16)}`,
        );
      }
    }

    // Validate version
    const version = view.getUint8(4);
    if (version > WIRE_VERSION) {
      throw new VersionError(version, WIRE_VERSION);
    }

    // Read envelope and payload lengths
    const envelopeLength = view.getUint32(5, false);
    const payloadLength = Number(view.getBigUint64(9, false));

    // Extract envelope JSON
    const envelopeStart = HEADER_SIZE;
    const envelopeEnd = envelopeStart + envelopeLength;
    const envelopeBytes = data.slice(envelopeStart, envelopeEnd);
    const envelopeJson = decoder.decode(envelopeBytes);
    const envelope = JSON.parse(envelopeJson);

    // Extract payload
    const payloadStart = envelopeEnd;
    const payloadEnd = payloadStart + payloadLength;
    const payload = data.slice(payloadStart, payloadEnd);

    return new Message(
      envelope.sender_id,
      envelope.content_type,
      payload,
      envelope.metadata ?? {},
      envelope.id,
      BigInt(envelope.timestamp),
    );
  }

  /**
   * ## fromDataView — Deserialize from a DataView at a Given Offset
   *
   * Like `fromBytes`, but reads from a specific offset within a larger
   * buffer. Returns both the deserialized message and the number of bytes
   * consumed, so the caller can advance to the next message.
   *
   * This is used during channel recovery to read multiple messages from
   * a single file buffer.
   *
   * @param data - The raw bytes containing one or more messages
   * @param offset - The byte offset to start reading from
   * @returns [message, bytesConsumed] tuple
   */
  static fromDataView(
    data: Uint8Array,
    offset: number,
  ): [Message, number] {
    const remaining = data.length - offset;
    if (remaining < HEADER_SIZE) {
      throw new InvalidFormatError(
        `Not enough bytes for header at offset ${offset}: need ${HEADER_SIZE}, have ${remaining}`,
      );
    }

    const view = new DataView(
      data.buffer,
      data.byteOffset + offset,
      remaining,
    );

    // Validate magic bytes
    for (let i = 0; i < 4; i++) {
      if (data[offset + i] !== WIRE_MAGIC[i]) {
        throw new InvalidFormatError(
          `Invalid magic bytes at offset ${offset + i}`,
        );
      }
    }

    // Validate version
    const version = view.getUint8(4);
    if (version > WIRE_VERSION) {
      throw new VersionError(version, WIRE_VERSION);
    }

    const envelopeLength = view.getUint32(5, false);
    const payloadLength = Number(view.getBigUint64(9, false));

    const totalSize = HEADER_SIZE + envelopeLength + payloadLength;

    if (remaining < totalSize) {
      throw new InvalidFormatError(
        `Truncated message at offset ${offset}: need ${totalSize} bytes, have ${remaining}`,
      );
    }

    const messageBytes = data.slice(offset, offset + totalSize);
    const message = Message.fromBytes(messageBytes);
    return [message, totalSize];
  }
}

// ============================================================================
// Channel
// ============================================================================

/**
 * ## Channel — One-Way, Append-Only Message Log
 *
 * A Channel is a pneumatic tube in an office building. Documents go in one
 * end and come out the other. You cannot send documents backwards. The tube
 * keeps a copy of every document that has ever passed through it (the log).
 *
 * ### Why One-Way?
 *
 * Bidirectional channels create ambiguity: "who sent this?" One-way channels
 * eliminate that question. For bidirectional communication, use two channels.
 *
 * ### Why Append-Only?
 *
 * If messages could be deleted or modified, crash recovery becomes impossible.
 * With append-only, the log IS the truth — every message ever sent is recorded
 * in order, immutably.
 *
 * ### Persistence
 *
 * Channels persist to disk as binary append logs. Each message is written in
 * the wire format (17-byte header + JSON envelope + raw payload), concatenated
 * end-to-end. This is:
 *
 * - **Binary-native**: images are raw bytes, not Base64. Zero bloat.
 * - **Appendable**: just write the next message at the end of the file.
 * - **Replayable**: parse header → read envelope → read payload → repeat.
 * - **Crash-safe**: if a write is interrupted, recovery discards the partial
 *   message and keeps everything before it.
 *
 * ### Offset Tracking
 *
 * Each consumer independently tracks how far it has read. The channel does
 * NOT manage offsets — consumers are smart readers.
 *
 * ```
 * Channel log:   [m0] [m1] [m2] [m3] [m4]
 *                                ▲
 * Consumer A:    offset = 3 ─────┘
 *                 ▲
 * Consumer B:    offset = 0 (hasn't read anything yet)
 * ```
 */
export class Channel {
  /** Unique identifier for this channel. */
  readonly id: string;

  /** Human-readable name (e.g., "email-summaries", "vault-requests"). */
  readonly name: string;

  /** Timestamp when this channel was created. */
  readonly createdAt: bigint;

  /**
   * The ordered log of messages. This is the in-memory representation.
   * For persistence, call `persist(directory)`.
   *
   * This is exposed as readonly — external code can read but not modify it.
   */
  private _log: Message[] = [];

  constructor(id: string, name: string) {
    this.id = id;
    this.name = name;
    this.createdAt = nextTimestamp();
  }

  /**
   * Get a readonly copy of the log.
   */
  get log(): readonly Message[] {
    return this._log;
  }

  /**
   * Append a message to the end of the log.
   *
   * Returns the sequence number (0-indexed, monotonically increasing).
   * This is the ONLY write operation. There is no delete, no update,
   * no insert-at-position.
   *
   * @param message - The message to append
   * @returns The sequence number assigned to this message
   */
  append(message: Message): number {
    const sequenceNumber = this._log.length;
    this._log.push(message);
    return sequenceNumber;
  }

  /**
   * Read messages from the log starting at `offset`, returning up to
   * `limit` messages.
   *
   * This does NOT consume messages — they remain in the log. Another
   * reader can read the same messages independently.
   *
   * @param offset - The index to start reading from (default 0)
   * @param limit - Maximum number of messages to return (default 100)
   * @returns Array of messages (may be empty if offset >= length)
   */
  read(offset = 0, limit = 100): Message[] {
    if (offset >= this._log.length) {
      return [];
    }
    const end = Math.min(offset + limit, this._log.length);
    return this._log.slice(offset, end);
  }

  /**
   * Return the number of messages in the log.
   */
  length(): number {
    return this._log.length;
  }

  /**
   * Return messages from index `start` to `end` (exclusive).
   *
   * Equivalent to `read(start, end - start)`.
   */
  slice(start: number, end: number): Message[] {
    return this._log.slice(start, end);
  }

  /**
   * ## persist — Write the Channel Log to Disk
   *
   * Writes every message in the log to a binary file using the wire format.
   * The file is named `{channel_name}.log` and placed in the given directory.
   *
   * Each message is written as: header (17 bytes) + envelope + payload,
   * concatenated end-to-end. This format supports:
   *
   * - Efficient append (just write at end of file)
   * - Crash recovery (scan for ACTM magic to find message boundaries)
   * - Scanning without loading payloads (read header → skip payload_length bytes)
   *
   * @param directory - The directory to write the log file to
   */
  persist(directory: string): void {
    if (!existsSync(directory)) {
      mkdirSync(directory, { recursive: true });
    }

    const filePath = join(directory, `${this.name}.log`);

    // Write all messages by concatenating their wire-format bytes
    const chunks: Uint8Array[] = [];
    for (const message of this._log) {
      chunks.push(message.toBytes());
    }

    // Calculate total size and merge into single buffer
    const totalSize = chunks.reduce((sum, chunk) => sum + chunk.length, 0);
    const buffer = new Uint8Array(totalSize);
    let offset = 0;
    for (const chunk of chunks) {
      buffer.set(chunk, offset);
      offset += chunk.length;
    }

    writeFileSync(filePath, buffer);
  }

  /**
   * ## recover — Reconstruct a Channel from Disk
   *
   * Reads a binary log file and reconstructs the channel's in-memory log.
   * If the file doesn't exist, returns an empty channel.
   *
   * During recovery, if a message is partially written (truncated mid-write
   * due to a crash), the partial message is discarded and all complete
   * messages before it are kept. This is the crash recovery guarantee.
   *
   * @param directory - The directory containing the log file
   * @param name - The channel name (used as filename: {name}.log)
   * @returns A Channel with all recovered messages
   */
  static recover(directory: string, name: string): Channel {
    const filePath = join(directory, `${name}.log`);
    const channel = new Channel(`recovered_${name}`, name);

    if (!existsSync(filePath)) {
      return channel;
    }

    const data = new Uint8Array(readFileSync(filePath));
    let offset = 0;

    while (offset < data.length) {
      try {
        // Check if we have enough bytes for even a header
        if (data.length - offset < HEADER_SIZE) {
          // Truncated header — discard and stop
          break;
        }

        const [message, bytesConsumed] = Message.fromDataView(data, offset);
        channel._log.push(message);
        offset += bytesConsumed;
      } catch (e) {
        // Truncated or corrupt message — stop recovery here.
        // All complete messages before this point are preserved.
        break;
      }
    }

    return channel;
  }
}

// ============================================================================
// Actor Types
// ============================================================================

/**
 * ## ActorStatus
 *
 * An actor is always in one of three states:
 *
 * - **IDLE**: Waiting for messages. Can receive and will process when asked.
 * - **PROCESSING**: Currently handling a message. Other messages queue up.
 * - **STOPPED**: Permanently halted. Cannot receive messages. Any messages
 *   sent to a stopped actor go to dead_letters.
 *
 * ```
 * State transitions:
 *
 * IDLE ──(process_next)──→ PROCESSING ──(done)──→ IDLE
 *   │                        │
 *   └──(stop)──→ STOPPED ←──(stop)──┘
 *
 * STOPPED is a terminal state. Once stopped, an actor cannot be restarted.
 * ```
 */
export type ActorStatus = "idle" | "processing" | "stopped";

/**
 * ## Behavior — The Heart of an Actor
 *
 * A behavior function takes the current state and one message, and returns
 * an `ActorResult` describing what should happen next. This is the only
 * user-defined code in the actor system.
 *
 * The behavior is a pure function (with one exception: it can produce a
 * new state). It must NOT:
 * - Access global mutable state
 * - Directly send messages (return them in ActorResult instead)
 * - Directly create actors (return them in ActorResult instead)
 *
 * @typeParam S - The type of the actor's state
 */
export type Behavior<S> = (state: S, message: Message) => ActorResult<S>;

/**
 * ## ActorResult — What a Behavior Returns
 *
 * After processing a message, the behavior returns an ActorResult that
 * describes:
 *
 * 1. The new state (can be the same as the old state)
 * 2. Messages to send to other actors (can be empty)
 * 3. New actors to create (can be empty)
 * 4. Whether to stop (default: false)
 *
 * This is a declarative return value — the actor does not directly send
 * messages or create actors. It tells the ActorSystem what to do, and the
 * system does it. This separation makes testing easy: you can call the
 * behavior function directly and inspect the result without running a
 * full ActorSystem.
 */
export interface ActorResult<S> {
  /** The actor's state after processing this message. */
  newState: S;

  /**
   * Messages to send. Each entry is a [targetActorId, message] tuple.
   * The ActorSystem will deliver these after processing completes.
   */
  messagesToSend?: Array<[string, Message]>;

  /**
   * Actors to create. Each entry is a specification for a new actor.
   * The ActorSystem will create these after processing completes.
   */
  actorsToCreate?: ActorSpec<unknown>[];

  /**
   * If true, the actor stops after processing this message. Its mailbox
   * is drained to dead_letters and no further messages are delivered.
   */
  stop?: boolean;
}

/**
 * ## ActorSpec — Blueprint for Creating an Actor
 *
 * When a behavior function wants to create a new actor, it returns an
 * ActorSpec describing the new actor's ID, initial state, and behavior.
 * The ActorSystem uses this spec to create the actor.
 */
export interface ActorSpec<S> {
  /** Unique identifier for the new actor. */
  actorId: string;

  /** The initial state for the new actor. */
  initialState: S;

  /** The behavior function for the new actor. */
  behavior: Behavior<S>;
}

// ============================================================================
// Actor
// ============================================================================

/**
 * ## Actor — An Isolated Unit of Computation
 *
 * An actor is a person sitting alone in a soundproofed room with a mail
 * slot in the door. Letters (messages) come in through the slot and pile
 * up in a tray (mailbox). The person reads one letter at a time, thinks
 * about it, possibly writes reply letters, and possibly rearranges things
 * on their desk (state). They never leave the room. They never look into
 * anyone else's room.
 *
 * ### Key Properties
 *
 * 1. **Isolation**: An actor's state is private. No other actor can read
 *    or modify it. Communication is only through messages.
 *
 * 2. **Sequential processing**: An actor processes one message at a time.
 *    While processing message N, messages N+1, N+2, etc. queue up but
 *    are not touched. No races, no deadlocks, no locks.
 *
 * 3. **At-most-once delivery**: A message in the mailbox is delivered to
 *    the behavior exactly once. If the actor crashes mid-processing, the
 *    message is lost.
 */
export class Actor<S = unknown> {
  /** Unique identifier — the actor's "address." */
  readonly id: string;

  /** FIFO queue of incoming messages. */
  mailbox: Message[] = [];

  /** Private data owned by this actor. */
  state: S;

  /** The function that processes messages. */
  behavior: Behavior<S>;

  /** Current lifecycle status. */
  status: ActorStatus = "idle";

  constructor(id: string, initialState: S, behavior: Behavior<S>) {
    this.id = id;
    this.state = initialState;
    this.behavior = behavior;
  }
}

// ============================================================================
// ActorSystem
// ============================================================================

/**
 * ## ActorSystem — The Runtime
 *
 * The ActorSystem is the office building. It has a directory (which actors
 * exist and their addresses), a mail room (message routing), and a building
 * manager (lifecycle management). Actors are tenants — they register with
 * the building, get an address, and the building delivers their mail.
 *
 * ### Operations
 *
 * ```
 * create_actor(id, state, behavior)  → register a new actor
 * send(target_id, message)           → deliver a message to an actor's mailbox
 * process_next(actor_id)             → process one message from an actor's mailbox
 * run_until_idle()                   → process all actors round-robin until quiet
 * run_until_done()                   → like run_until_idle but keeps going
 * create_channel(id, name)           → create a new message channel
 * shutdown()                         → stop all actors, drain mailboxes
 * ```
 *
 * ### Message Delivery
 *
 * When you call `send(targetId, message)`:
 *
 * 1. Look up targetId in the actors map
 * 2. If not found or STOPPED → message goes to dead_letters
 * 3. Otherwise → enqueue in target's mailbox
 *
 * ### Processing Loop
 *
 * `run_until_idle()` processes actors in round-robin order:
 *
 * 1. Find any IDLE actor with a non-empty mailbox
 * 2. Call process_next on that actor
 * 3. Repeat until no actor has pending messages
 *
 * This is sequential (V1). True parallelism is a future enhancement.
 */
export class ActorSystem {
  /** Registry of all living actors. */
  private actors: Map<string, Actor> = new Map();

  /** Registry of all channels. */
  private channels: Map<string, Channel> = new Map();

  /** Messages that could not be delivered. */
  private _deadLetters: Message[] = [];

  /** Monotonic counter for system-level ordering. */
  private _clock = 0n;

  // ────────────────────────────────────────────────────────────────────────
  // Actor Lifecycle
  // ────────────────────────────────────────────────────────────────────────

  /**
   * Create and register a new actor.
   *
   * @param actorId - Unique identifier for the actor
   * @param initialState - The actor's initial state
   * @param behavior - The function that processes messages
   * @returns The actor ID
   * @throws DuplicateActorError if an actor with this ID already exists
   */
  createActor<S>(
    actorId: string,
    initialState: S,
    behavior: Behavior<S>,
  ): string {
    if (this.actors.has(actorId)) {
      throw new DuplicateActorError(actorId);
    }
    const actor = new Actor(actorId, initialState, behavior as Behavior<unknown>);
    this.actors.set(actorId, actor);
    return actorId;
  }

  /**
   * Stop an actor. Sets status to STOPPED and drains its mailbox to
   * dead_letters. A stopped actor cannot be restarted.
   *
   * @param actorId - The actor to stop
   */
  stopActor(actorId: string): void {
    const actor = this.actors.get(actorId);
    if (!actor) return;
    actor.status = "stopped";
    // Drain mailbox to dead_letters
    while (actor.mailbox.length > 0) {
      this._deadLetters.push(actor.mailbox.shift()!);
    }
  }

  /**
   * Get the status of an actor.
   *
   * @param actorId - The actor to query
   * @returns "idle", "processing", or "stopped"
   * @throws ActorNotFoundError if the actor doesn't exist
   */
  getActorStatus(actorId: string): ActorStatus {
    const actor = this.actors.get(actorId);
    if (!actor) {
      throw new ActorNotFoundError(actorId);
    }
    return actor.status;
  }

  // ────────────────────────────────────────────────────────────────────────
  // Messaging
  // ────────────────────────────────────────────────────────────────────────

  /**
   * Send a message to an actor's mailbox.
   *
   * If the target actor doesn't exist or is stopped, the message goes to
   * dead_letters instead. This never throws — undeliverable messages are
   * silently collected for debugging.
   *
   * @param targetId - The actor to send to
   * @param message - The message to deliver
   */
  send(targetId: string, message: Message): void {
    const actor = this.actors.get(targetId);
    if (!actor || actor.status === "stopped") {
      this._deadLetters.push(message);
      return;
    }
    actor.mailbox.push(message);
  }

  // ────────────────────────────────────────────────────────────────────────
  // Processing
  // ────────────────────────────────────────────────────────────────────────

  /**
   * Process one message from an actor's mailbox.
   *
   * 1. Dequeue the front message
   * 2. Set status to PROCESSING
   * 3. Call behavior(state, message)
   * 4. Apply the result: update state, send messages, create actors
   * 5. If stop requested, set STOPPED and drain mailbox
   * 6. Otherwise, set IDLE
   *
   * If the behavior throws an exception:
   * - State is NOT updated (rollback to pre-call state)
   * - The failed message goes to dead_letters
   * - The actor returns to IDLE and continues with the next message
   *
   * @param actorId - The actor to process
   * @returns true if a message was processed, false if mailbox was empty
   */
  processNext(actorId: string): boolean {
    const actor = this.actors.get(actorId);
    if (!actor || actor.status === "stopped") {
      return false;
    }

    if (actor.mailbox.length === 0) {
      return false;
    }

    const message = actor.mailbox.shift()!;
    actor.status = "processing";

    try {
      const result = actor.behavior(actor.state, message);

      // Update state
      actor.state = result.newState;

      // Create actors FIRST — so newly spawned actors can receive
      // messages sent in the same result. This ordering matches the
      // practical expectation: "spawn B and send it a message" should
      // work in a single behavior return.
      if (result.actorsToCreate) {
        for (const spec of result.actorsToCreate) {
          try {
            this.createActor(spec.actorId, spec.initialState, spec.behavior as Behavior<unknown>);
          } catch {
            // If actor already exists, silently skip
          }
        }
      }

      // Send messages (after actors are created)
      if (result.messagesToSend) {
        for (const [targetId, msg] of result.messagesToSend) {
          this.send(targetId, msg);
        }
      }

      // Stop if requested
      if (result.stop) {
        actor.status = "stopped";
        while (actor.mailbox.length > 0) {
          this._deadLetters.push(actor.mailbox.shift()!);
        }
      } else {
        actor.status = "idle";
      }
    } catch {
      // Behavior threw an exception:
      // - State unchanged (we didn't assign result.newState)
      // - Message goes to dead_letters
      // - Actor returns to IDLE
      this._deadLetters.push(message);
      actor.status = "idle";
    }

    return true;
  }

  /**
   * Process all actors round-robin until no actor has pending messages.
   *
   * This finds any IDLE actor with a non-empty mailbox, processes one
   * message, and repeats. The round-robin ensures fairness — no single
   * actor monopolizes processing.
   *
   * @returns Statistics about what happened
   */
  runUntilIdle(): { messagesProcessed: number; actorsCreated: number } {
    let messagesProcessed = 0;
    const initialActorCount = this.actors.size;

    let madeProgress = true;
    while (madeProgress) {
      madeProgress = false;
      for (const [actorId, actor] of this.actors) {
        if (actor.status === "idle" && actor.mailbox.length > 0) {
          if (this.processNext(actorId)) {
            messagesProcessed++;
            madeProgress = true;
          }
        }
      }
    }

    return {
      messagesProcessed,
      actorsCreated: this.actors.size - initialActorCount,
    };
  }

  /**
   * Like `runUntilIdle` but keeps going until the system is completely
   * quiet — no messages remain in any mailbox and no new messages are
   * being generated.
   *
   * This is essentially `runUntilIdle()` in a loop, since processing
   * messages can generate new messages. We add a safety limit to prevent
   * infinite loops from ping-pong actors.
   */
  runUntilDone(
    maxIterations = 10000,
  ): { messagesProcessed: number; actorsCreated: number } {
    let totalProcessed = 0;
    const initialActorCount = this.actors.size;
    let iterations = 0;

    while (iterations < maxIterations) {
      const stats = this.runUntilIdle();
      totalProcessed += stats.messagesProcessed;
      if (stats.messagesProcessed === 0) {
        break;
      }
      iterations++;
    }

    return {
      messagesProcessed: totalProcessed,
      actorsCreated: this.actors.size - initialActorCount,
    };
  }

  // ────────────────────────────────────────────────────────────────────────
  // Channels
  // ────────────────────────────────────────────────────────────────────────

  /**
   * Create and register a new channel.
   *
   * @param channelId - Unique identifier for the channel
   * @param name - Human-readable name
   * @returns The created Channel
   */
  createChannel(channelId: string, name: string): Channel {
    const channel = new Channel(channelId, name);
    this.channels.set(channelId, channel);
    return channel;
  }

  /**
   * Retrieve a channel by ID.
   *
   * @param channelId - The channel to look up
   * @returns The Channel, or undefined if not found
   */
  getChannel(channelId: string): Channel | undefined {
    return this.channels.get(channelId);
  }

  // ────────────────────────────────────────────────────────────────────────
  // Inspection
  // ────────────────────────────────────────────────────────────────────────

  /**
   * Messages that could not be delivered (target not found or stopped).
   * Useful for debugging and monitoring.
   */
  get deadLetters(): Message[] {
    return this._deadLetters;
  }

  /**
   * List all registered actor IDs (including stopped actors).
   */
  actorIds(): string[] {
    return Array.from(this.actors.keys());
  }

  /**
   * Get the number of pending messages in an actor's mailbox.
   *
   * @param actorId - The actor to query
   * @returns Number of pending messages
   * @throws ActorNotFoundError if the actor doesn't exist
   */
  mailboxSize(actorId: string): number {
    const actor = this.actors.get(actorId);
    if (!actor) {
      throw new ActorNotFoundError(actorId);
    }
    return actor.mailbox.length;
  }

  /**
   * Shut down the entire system.
   *
   * 1. Stop all actors (set status to STOPPED)
   * 2. Drain all mailboxes to dead_letters
   */
  shutdown(): void {
    for (const [actorId] of this.actors) {
      this.stopActor(actorId);
    }
  }
}
