# D19 — Actor

## Overview

The Actor model is a mathematical framework for concurrent computation invented by
Carl Hewitt, Peter Bishop, and Richard Steiger in 1973. It defines computation in terms
of **actors** — independent entities that communicate exclusively through **messages**.
No shared memory. No locks. No mutexes. Just isolated units of computation passing
immutable messages through one-way channels.

This is not an academic exercise. Erlang/OTP — which powers most of the world's telecom
infrastructure and ran WhatsApp's backend for 2 billion users — is built entirely on the
Actor model. Discord's Elixir backend, Akka in Scala/Java, Microsoft Orleans in C# — all
Actor systems. The pattern has been proven at enormous scale because its properties
(isolation, fault tolerance, location transparency) emerge naturally from the model
itself rather than being bolted on later.

**The key insight:** An actor is defined by what it can do, not what it is. An actor can:
1. **Receive** a message
2. **Send** messages to other actors it knows about
3. **Create** new actors
4. **Change its own internal state** in response to a message

An actor **cannot**:
1. Access another actor's internal state
2. Share memory with another actor
3. Communicate except through messages

**Analogy:** Think of a congressional office (the Chief of Staff model from D18). Each
staffer is an actor. The Staff Assistant receives phone calls (messages), writes call
summaries (new messages), and sends them to the Communications Director (another actor).
The Staff Assistant cannot reach into the Communications Director's desk and read their
draft press release. They can only send a message asking for it. The Communications
Director's desk (internal state) is private — only they can see it and change it.

This package implements the three primitives that everything in D18 (Chief of Staff)
builds on:

1. **Message** — the atom of communication. Immutable, typed, serializable.
2. **Channel** — a one-way, append-only pipe for messages. Persistent and replayable.
3. **Actor** — an isolated unit of computation with a mailbox and internal state.

These three primitives are sufficient to build the entire Chief of Staff system — agents,
the orchestrator, the vault, the capability cage, and the encrypted pub/sub layer. But
this package does not implement any of those. It provides only the foundation.

---

## Where It Fits

```
User Programs / Chief of Staff (D18)
│   create_actor(behavior)     — spawn a new actor
│   send(actor_ref, message)   — deliver a message
│   channel.append(message)    — publish to a channel
│   channel.read(offset)       — consume from a channel
▼
Actor Runtime ← YOU ARE HERE
│   ├── Actor         — isolated computation + mailbox
│   ├── Message       — immutable typed payload
│   ├── Channel       — one-way append-only pipe
│   └── ActorSystem   — lifecycle, supervision, routing
▼
IPC (D16)                     Process Manager (D14)
│   channels build on          │   actors map to processes
│   message queue concepts     │   in production; in-process
│   (but add persistence,     │   threads for lightweight use
│   encryption hooks,
│   offset tracking)
▼
File System (D15)
│   channel logs persist to disk
│   actor state snapshots
```

**Depends on:** None at the package level — this is a foundational primitive. It uses
concepts from IPC (D16) and Process Manager (D14) but does not import their code.

**Used by:** Chief of Staff (D18) — agents are actors, channels carry messages between
them; Vault (D18) — the vault is an actor with leased secret delivery; Orchestrator
(D18) — the orchestrator is a supervisory actor; any future concurrent system.

---

## Key Concepts

### Primitive 1: Message

A Message is the atom of communication. Every piece of data that flows between actors —
a user's request, an agent's response, a credential from the vault — is a Message.
Messages are **immutable**: once created, they cannot be modified.

**Analogy:** A Message is a sealed letter. Once sealed, the contents are fixed. The
envelope records who sent it, when, and what kind of letter it is. You can make copies
of the letter, but you cannot change the original.

```
Message
═══════════════════════════════════════════════════════════════
┌──────────────────┬─────────────────────────────────────────┐
│ id               │ Unique identifier (string). Generated    │
│                  │ at creation time. Two messages with the  │
│                  │ same id are the same message.            │
├──────────────────┼─────────────────────────────────────────┤
│ timestamp        │ Monotonic nanosecond counter. Strictly   │
│                  │ increasing within a single actor. Used   │
│                  │ for ordering, not wall-clock time.       │
├──────────────────┼─────────────────────────────────────────┤
│ sender_id        │ The actor that created this message.     │
│                  │ Set automatically — an actor cannot      │
│                  │ forge another actor's sender_id.         │
├──────────────────┼─────────────────────────────────────────┤
│ content_type     │ A string tag describing the payload      │
│                  │ format: "text/plain", "application/json",│
│                  │ "application/octet-stream", etc.         │
│                  │ Actors can filter by content_type.       │
├──────────────────┼─────────────────────────────────────────┤
│ payload          │ The message body. Always raw bytes.       │
│                  │ Content_type tells the receiver how to   │
│                  │ interpret them: UTF-8 text, JSON, PNG,   │
│                  │ MP4, or any other format. The Actor      │
│                  │ system never inspects or transforms the  │
│                  │ payload — it is opaque binary data.      │
├──────────────────┼─────────────────────────────────────────┤
│ metadata         │ Optional key-value pairs for extensibility│
│                  │ (correlation IDs, trace IDs, priority,   │
│                  │ etc.). Not interpreted by the Actor      │
│                  │ system — pass-through for user code.     │
└──────────────────┴─────────────────────────────────────────┘
```

**Key properties:**

1. **Immutability.** A Message has no setter methods. All fields are set at creation
   time. To "modify" a message, create a new one with different values. The original
   is untouched.

2. **Self-describing.** The `content_type` field tells the receiver how to interpret
   the payload without out-of-band knowledge. An actor can inspect content_type before
   deciding whether to process the message or ignore it.

3. **Extensibility.** The `metadata` field is a string-to-string map that carries
   arbitrary context. D18 uses it for encryption nonces, sequence numbers, and
   signatures. This package does not interpret metadata — it passes it through.

4. **Binary-native serialization.** Messages are serialized to a binary wire format
   that separates the **envelope** (metadata — always JSON) from the **payload**
   (always raw bytes). This avoids Base64-encoding binary data like images or videos,
   which would bloat size by 33% and waste CPU.

**Message wire format:**

The serialization format uses a fixed-size header followed by a JSON envelope
followed by raw payload bytes:

```
Message on disk / on wire:

┌─────────────────────────────────────────────┐
│ HEADER (17 bytes, fixed size)               │
│                                              │
│ magic:          4 bytes  "ACTM" (0x4143544D) │
│ version:        1 byte   0x01                │
│ envelope_length: 4 bytes (big-endian u32)    │
│ payload_length:  8 bytes (big-endian u64)    │
└──────────────────────┬──────────────────────┘
┌──────────────────────┴──────────────────────┐
│ ENVELOPE (UTF-8 JSON, variable length)       │
│                                              │
│ {                                            │
│   "id": "msg_01J7X8K9M2N3P4Q5R6S7T8U9V0",  │
│   "timestamp": 1679616000000000000,          │
│   "sender_id": "actor_email_reader",         │
│   "content_type": "image/png",               │
│   "metadata": {                              │
│     "correlation_id": "req_abc123",          │
│     "width": "1920",                         │
│     "height": "1080"                         │
│   }                                          │
│ }                                            │
│                                              │
│ (No payload here — envelope is pure metadata)│
└──────────────────────┬──────────────────────┘
┌──────────────────────┴──────────────────────┐
│ PAYLOAD (raw bytes, variable length)         │
│                                              │
│ 89 50 4E 47 0D 0A 1A 0A ... (PNG bytes)     │
│                                              │
│ No encoding. No Base64. Raw binary.          │
│ Could be 10 bytes or 10 gigabytes.           │
└─────────────────────────────────────────────┘
```

**Why separate envelope from payload?**

1. **No bloat.** A 10MB image is 10MB on disk, not 13.3MB after Base64.
2. **Scannable without loading payloads.** To search for "all messages from
   browser_agent," read headers and envelopes only — skip payload bytes.
   You can index an entire channel of videos without loading a single frame.
3. **Content-type driven.** The envelope's `content_type` tells the receiver
   how to interpret the payload bytes:

```
content_type               payload bytes are...
─────────────              ────────────────────
text/plain                 UTF-8 text
application/json           JSON (UTF-8 text)
image/png                  raw PNG image
image/jpeg                 raw JPEG image
video/mp4                  raw MP4 video
application/octet-stream   opaque binary (receiver knows what to do)
```

**Convenience constructors for common cases:**

For text and JSON messages (the common case), convenience constructors accept
strings and handle UTF-8 encoding automatically:

```python
# Text message — payload stored as UTF-8 bytes internally
msg = Message.text(sender_id="agent", payload="hello world")

# JSON message — payload is JSON-serialized to UTF-8 bytes
msg = Message.json(sender_id="agent", payload={"key": "value"})

# Binary message — payload is raw bytes
msg = Message.binary(
    sender_id="browser",
    content_type="image/png",
    payload=png_bytes,
)
```

---

### Wire Format Versioning and Envelope Encoding

Every schema in this system is versioned. The wire format version is embedded in
every serialized message's header. This enables forward and backward compatibility
as the format evolves.

**Pluggable envelope encoding:**

The envelope (message metadata — everything except the raw payload) is encoded using
a format identified by the version byte in the header. V1 uses JSON because it is
human-readable and debuggable. But the architecture supports swapping the envelope
encoding entirely — to Protocol Buffers, MessagePack, CBOR, FlatBuffers, or any other
format — by bumping the version byte.

```
Version → Envelope Encoding Mapping
════════════════════════════════════

v1:  JSON (UTF-8)        — human-readable, debuggable, V1 default
v2:  (future) Protobuf   — compact, schema-enforced, fast parsing
v3:  (future) MessagePack — compact, schema-less, fast parsing
...

The version byte tells the reader HOW to parse the envelope.
The payload is always raw bytes, unaffected by envelope encoding.
```

**Why this matters:** When both sender and receiver are upgraded to understand Protobuf
(for example), they can switch to v2. The channel log can contain mixed v1 (JSON) and
v2 (Protobuf) messages — the reader dispatches based on each message's version byte.
Old messages remain readable. No migration needed.

**The contract:** The envelope encoding can change. The *fields* in the envelope
(id, timestamp, sender_id, content_type, metadata) are the stable contract. Whether
those fields are serialized as JSON keys, Protobuf field numbers, or MessagePack
maps is an implementation detail hidden behind the version byte.

```
Stable contract (does not change):     Encoding (can change per version):
├── id: string                         ├── v1: {"id": "msg_001", ...}
├── timestamp: u64                     ├── v2: <protobuf bytes>
├── sender_id: string                  └── v3: <msgpack bytes>
├── content_type: string
└── metadata: map<string, string>
```

**Version rules:**

```
Version Compatibility Rules
═══════════════════════════

1. READING: A reader MUST handle all versions ≤ its own version.
   A v3 reader can read v1, v2, and v3 messages.

2. READING: A reader that encounters a version > its own MUST NOT crash.
   It MUST return a clear VersionError with the encountered version
   and the maximum supported version. This tells the user to upgrade.

3. WRITING: A writer ALWAYS writes the latest version it supports.
   A v3 writer writes v3 messages, never v1 or v2.

4. ADDING FIELDS: New optional fields in the envelope are a MINOR change.
   Old readers ignore fields they don't recognize (JSON's natural behavior).
   No version bump needed.

5. CHANGING FIELD TYPES OR REMOVING FIELDS: This is a MAJOR change.
   Requires a version bump (v1 → v2). Old readers use the version byte
   to dispatch to the correct parser.

6. CHANGING THE HEADER LAYOUT: This is an EXTREME change. The first 5
   bytes (magic + version) MUST NEVER change — they are the anchor that
   all future versions use to identify the format. Changes to header
   fields after the version byte require a version bump.
```

**How version dispatch works:**

```
Reading a message:

1. Read 5 bytes: magic (4) + version (1).
2. If magic ≠ "ACTM" → InvalidFormat error.
3. If version > MAX_SUPPORTED_VERSION → VersionError.
4. Dispatch to version-specific parser:
   ├── v1: read 12 more header bytes (envelope_len u32 + payload_len u64)
   ├── v2: (future) might have different header fields
   └── v3: (future) might have a completely different layout

The version byte is the ONLY thing all versions agree on.
Everything after it can change between versions.
```

**Channel log versioning:**

A single channel log file can contain messages of **mixed versions**. If the
system is upgraded from v1 to v2, old messages in the log remain v1. New messages
are written as v2. The reader handles both transparently because each message
self-describes its version in its own header. No migration step needed.

```
Channel file after upgrade from v1 to v2:

[ACTM][v1][...] message 0 (written before upgrade)
[ACTM][v1][...] message 1 (written before upgrade)
[ACTM][v2][...] message 2 (written after upgrade, new format)
[ACTM][v2][...] message 3 (written after upgrade)

Reader handles both v1 and v2 transparently.
No need to rewrite the log.
```

---

### Primitive 2: Channel

A Channel is a one-way, append-only, ordered log of messages. It connects message
producers to message consumers. Messages flow in one direction only. Once appended,
a message cannot be removed, modified, or reordered.

**Analogy:** A Channel is a one-way pneumatic tube in an office building. Documents go
in one end and come out the other. You cannot send documents backwards. The tube keeps
a copy of every document that has ever passed through it (the log), and each office at
the receiving end has a bookmark showing which documents they have already read (the
offset).

**Why one-way?** Bidirectional channels create ambiguity: "who sent this message?" and
"can a receiver inject messages that look like they came from the sender?" One-way
channels eliminate both questions. If you need bidirectional communication, you use
two channels — one in each direction. This is not a limitation. It is the core
security property that D18 builds on.

**Why append-only?** If messages could be deleted or modified, crash recovery becomes
impossible. After a crash, the system asks: "what happened before the crash?" If the
log is mutable, the answer is "we don't know — someone might have changed it." If the
log is append-only, the answer is definitive: "here is exactly what happened, in order,
immutably recorded."

```
Channel
═══════════════════════════════════════════════════════════════
┌───────────────────┬────────────────────────────────────────┐
│ id                │ Unique identifier (string).             │
├───────────────────┼────────────────────────────────────────┤
│ name              │ Human-readable name (e.g.,              │
│                   │ "email-summaries", "vault-requests").   │
│                   │ Used for discovery and debugging.       │
├───────────────────┼────────────────────────────────────────┤
│ log               │ Ordered list of Messages. Append-only.  │
│                   │ Index 0 is the first message ever       │
│                   │ written. Index N is the most recent.    │
├───────────────────┼────────────────────────────────────────┤
│ created_at        │ Timestamp when the channel was created. │
└───────────────────┴────────────────────────────────────────┘
```

**Channel operations:**

```
append(message) → sequence_number
  1. Assign the next sequence number (0-indexed, monotonic).
  2. Add the message to the end of the log.
  3. Return the sequence number.
  4. This is the ONLY write operation. There is no delete,
     no update, no insert-at-position.

read(offset, limit) → list of Messages
  1. Return up to `limit` messages starting from `offset`.
  2. If offset >= log length, return empty list (caller is caught up).
  3. If offset + limit > log length, return remaining messages.
  4. This does NOT consume the messages — they remain in the log.
     Another reader can read the same messages independently.

length() → count
  1. Return the number of messages in the log.

slice(start, end) → list of Messages
  1. Return messages from index `start` to `end` (exclusive).
  2. Equivalent to read(start, end - start).
```

**Offset tracking:**

Each consumer of a channel independently tracks how far it has read. This is NOT
managed by the channel itself — it is the consumer's responsibility. This separation
is deliberate: the channel is a dumb log, consumers are smart readers.

```
Channel log:   [m0] [m1] [m2] [m3] [m4] [m5]
                                     ▲
Consumer A:    offset = 4 ───────────┘
                     ▲
Consumer B:    offset = 1 (behind — maybe processing slowly)

Consumer A has processed messages 0-3 and will read m4 next.
Consumer B has processed message 0 and will read m1 next.
They are independent. A being ahead does not affect B.
```

**Persistence:**

Channels persist to disk as a **binary append log** using the same wire format as
individual messages. Each message is written as its header + envelope + payload,
concatenated end-to-end. This format is:

- **Binary-native** — images, videos, and arbitrary payloads are stored as raw bytes,
  not Base64-encoded text. Zero bloat.
- **Appendable** — just write the next message's bytes at the end of the file.
- **Replayable** — read from the beginning: parse header → read envelope → read/skip
  payload → parse next header → repeat.
- **Scannable without loading payloads** — to index the channel, read each 17-byte
  header, read the envelope JSON, skip `payload_length` bytes, move to the next
  message. You can index a channel full of videos without loading a single frame.

```
Channel file: channels/email-summaries.log

┌──────────────────────────────────────────────┐
│[ACTM][v1][env_len=82][pay_len=45]            │ message 0 header
│{"id":"msg_001","sender_id":"email_reader",...}│ message 0 envelope
│Meeting tomorrow at 3pm...                    │ message 0 payload
├──────────────────────────────────────────────┤
│[ACTM][v1][env_len=78][pay_len=1048576]       │ message 1 header
│{"id":"msg_002","content_type":"image/png",...}│ message 1 envelope
│<1MB of raw PNG bytes>                        │ message 1 payload
├──────────────────────────────────────────────┤
│[ACTM][v1][env_len=85][pay_len=23]            │ message 2 header
│{"id":"msg_003","sender_id":"email_reader",...}│ message 2 envelope
│See attached screenshot                       │ message 2 payload
└──────────────────────────────────────────────┘
```

**Index file (optional, for fast offset-to-byte-position lookups):**

For large channels with many messages, an optional `.idx` file maps sequence
numbers to byte offsets in the log file. This allows O(1) seeking to any message
instead of scanning from the beginning:

```
Channel index: channels/email-summaries.idx

sequence 0 → byte offset 0
sequence 1 → byte offset 144
sequence 2 → byte offset 1048798
...
```

The index is rebuilt from the log on recovery if missing — the log is always the
source of truth. The index is a performance optimization, not a correctness
requirement.

D18 adds encryption on top of this persistence layer. This package stores plaintext —
encryption is a concern of the Chief of Staff system, not the Actor primitive.

---

### Primitive 3: Actor

An Actor is an isolated unit of computation. It has an **address** (so other actors can
send it messages), a **mailbox** (where incoming messages queue up), a **behavior**
(a function that processes one message at a time), and **state** (private data that only
the actor can see or modify).

**Analogy:** An actor is a person sitting alone in a soundproofed room with a mail slot
in the door. Letters (messages) come in through the slot and pile up in a tray (mailbox).
The person reads one letter at a time, thinks about it, possibly writes reply letters
and slides them out through their own mail slot, and possibly rearranges things on their
desk (state). They never leave the room. They never look into anyone else's room. They
only know about other rooms by their mail slot addresses.

```
Actor
═══════════════════════════════════════════════════════════════
┌──────────────────┬─────────────────────────────────────────┐
│ id               │ Unique identifier (string). This is the │
│                  │ actor's "address" — other actors use it  │
│                  │ to send messages.                        │
├──────────────────┼─────────────────────────────────────────┤
│ mailbox          │ FIFO queue of incoming Messages.         │
│                  │ Messages are enqueued by the ActorSystem │
│                  │ when another actor sends to this actor.  │
│                  │ The actor itself dequeues one at a time. │
├──────────────────┼─────────────────────────────────────────┤
│ state            │ Private data owned by this actor. Can    │
│                  │ be any type. Only the actor's behavior   │
│                  │ function can read or write it. No        │
│                  │ external access.                         │
├──────────────────┼─────────────────────────────────────────┤
│ behavior         │ A function: (state, message) → result.  │
│                  │ Processes one message at a time. Returns │
│                  │ an ActorResult containing: new state,    │
│                  │ messages to send, actors to create.      │
├──────────────────┼─────────────────────────────────────────┤
│ status           │ IDLE | PROCESSING | STOPPED              │
│                  │ IDLE: waiting for messages                │
│                  │ PROCESSING: handling a message            │
│                  │ STOPPED: permanently halted               │
└──────────────────┴─────────────────────────────────────────┘
```

**The behavior function:**

The behavior is the heart of an actor. It is a pure function (with one exception:
it can update state) that takes the current state and one message, and returns an
`ActorResult`:

```
ActorResult
═══════════════════════════════════════════════════════════════
┌──────────────────┬─────────────────────────────────────────┐
│ new_state        │ The actor's state after processing this │
│                  │ message. Can be identical to the old     │
│                  │ state (no change) or completely new.     │
├──────────────────┼─────────────────────────────────────────┤
│ messages_to_send │ List of (target_id, message) pairs.     │
│                  │ These messages will be delivered to the  │
│                  │ target actors' mailboxes by the          │
│                  │ ActorSystem. Can be empty (the actor     │
│                  │ absorbed the message without replying).  │
├──────────────────┼─────────────────────────────────────────┤
│ actors_to_create │ List of actor specifications to spawn.  │
│                  │ Each spec includes an id, initial state, │
│                  │ and behavior function. Can be empty.     │
├──────────────────┼─────────────────────────────────────────┤
│ stop             │ Boolean. If true, the actor halts after  │
│                  │ processing this message. Its mailbox is  │
│                  │ drained and no further messages are      │
│                  │ delivered. Default: false.               │
└──────────────────┴─────────────────────────────────────────┘
```

**Processing guarantees:**

1. **Sequential processing.** An actor processes exactly one message at a time. While
   processing message N, messages N+1, N+2, etc. accumulate in the mailbox but are not
   touched. This eliminates all concurrency hazards within a single actor — no races,
   no deadlocks, no need for locks.

2. **At-most-once delivery.** A message in the mailbox is delivered to the behavior
   function exactly once. After processing, it is removed from the mailbox. If the
   actor crashes mid-processing, the message is lost (at-most-once, not exactly-once).
   For stronger guarantees, use channels with offset tracking (where the message
   persists in the log regardless of the actor's state).

3. **No ordering guarantee across actors.** If Actor A sends messages m1 and m2 to
   Actor B, they arrive in order (m1 before m2). But if Actor A sends m1 to Actor B
   and m2 to Actor C, there is no guarantee about the relative timing of B and C
   processing their messages.

---

### The ActorSystem

The ActorSystem is the runtime that manages actor lifecycles, message delivery, and
(optionally) supervision. It is the "world" that actors live in.

**Analogy:** The ActorSystem is the office building. It has a directory (which actors
exist and their addresses), a mail room (message routing), and a building manager
(supervision — restart actors that crash). Actors are tenants. They register with
the building, get an address, and the building delivers their mail. But the building
manager does not read the mail.

```
ActorSystem
═══════════════════════════════════════════════════════════════
┌──────────────────┬─────────────────────────────────────────┐
│ actors           │ Map<actor_id, Actor>. The registry of   │
│                  │ all living actors.                      │
├──────────────────┼─────────────────────────────────────────┤
│ channels         │ Map<channel_id, Channel>. All channels  │
│                  │ in the system.                          │
├──────────────────┼─────────────────────────────────────────┤
│ dead_letters     │ List of Messages that could not be      │
│                  │ delivered (target actor does not exist   │
│                  │ or is stopped). Useful for debugging.   │
├──────────────────┼─────────────────────────────────────────┤
│ clock            │ Monotonic counter. Incremented on each  │
│                  │ message creation. Provides the timestamp│
│                  │ for Message.timestamp.                  │
└──────────────────┴─────────────────────────────────────────┘
```

**ActorSystem operations:**

```
create_actor(id, initial_state, behavior) → actor_id
  1. Verify id is unique (no existing actor with this id).
  2. Create a new Actor with empty mailbox, the given state,
     and the given behavior function.
  3. Register the actor in the actors map.
  4. Set status = IDLE.
  5. Return the actor_id.

send(target_id, message) → Result
  1. Look up target_id in the actors map.
  2. If not found or status == STOPPED:
     a. Add message to dead_letters.
     b. Return error: ActorNotFound.
  3. Enqueue message in the target actor's mailbox.
  4. Return success.

process_next(actor_id) → Result
  1. Look up actor in the actors map.
  2. If mailbox is empty, return Ok(nothing to process).
  3. Dequeue the front message from the mailbox.
  4. Set status = PROCESSING.
  5. Call behavior(state, message) → ActorResult.
  6. Update actor state to ActorResult.new_state.
  7. For each (target, msg) in messages_to_send:
     call send(target, msg).
  8. For each spec in actors_to_create:
     call create_actor(spec.id, spec.state, spec.behavior).
  9. If ActorResult.stop == true:
     set status = STOPPED.
  10. Else set status = IDLE.
  11. Return Ok.

run_until_idle() → stats
  1. Loop:
     a. Find any actor with status == IDLE and non-empty mailbox.
     b. If none found, return (system is idle — no work to do).
     c. Call process_next(actor_id).
  2. Return statistics (messages processed, actors created, etc.).

  NOTE: In V1, this processes actors one at a time in round-robin
  order. True parallelism (multiple actors processing simultaneously)
  is a future enhancement. The sequential model is simpler to test,
  debug, and reason about.

run_until_done() → stats
  1. Call run_until_idle() repeatedly until no messages remain
     in any mailbox and no new messages are being generated.
  2. Return statistics.

create_channel(id, name) → channel_id
  1. Create a new Channel with empty log.
  2. Register in the channels map.
  3. Return the channel_id.

shutdown()
  1. Stop all actors (set status = STOPPED on each).
  2. Drain all mailboxes to dead_letters.
  3. Persist all channels to disk (if persistence is enabled).
```

---

## Algorithms

### Message Delivery (send)

```
send(target_id, message)
════════════════════════

1. Look up target_id in actors map.
   ├── NOT FOUND → dead_letters.append(message). Return error.
   └── FOUND → continue.

2. Check target.status:
   ├── STOPPED → dead_letters.append(message). Return error.
   └── IDLE or PROCESSING → continue.

3. Enqueue message at the back of target.mailbox.
   Mailbox is a FIFO queue — messages are processed in arrival order.

4. Return success.

Time complexity: O(1) — hash map lookup + queue append.
```

### Actor Processing Loop (process_next)

```
process_next(actor_id)
══════════════════════

1. actor = actors[actor_id]
   If actor is None or actor.status == STOPPED:
     return Error(ActorNotFound)

2. If actor.mailbox is empty:
     return Ok(NoWork)

3. message = actor.mailbox.dequeue()    // FIFO: oldest first

4. actor.status = PROCESSING

5. result = actor.behavior(actor.state, message)
   │
   ├── This is the user-defined function.
   │   It can take as long as it needs.
   │   No other message will be processed by THIS actor
   │   until this call returns.
   │
   └── If behavior throws an exception:
       a. Log the error with actor_id and message.id.
       b. Message is LOST (at-most-once semantics).
       c. Actor state is UNCHANGED (we do not apply partial updates).
       d. Actor status returns to IDLE (continues processing next message).
       e. The failed message is added to dead_letters for debugging.

6. actor.state = result.new_state

7. For each (target_id, msg) in result.messages_to_send:
     send(target_id, msg)
     // These are new messages created by the actor.
     // The sender_id on each is automatically set to this actor's id.

8. For each spec in result.actors_to_create:
     create_actor(spec.id, spec.initial_state, spec.behavior)

9. If result.stop:
     actor.status = STOPPED
     // Remaining mailbox messages go to dead_letters
     while actor.mailbox is not empty:
       dead_letters.append(actor.mailbox.dequeue())
   Else:
     actor.status = IDLE

10. Return Ok
```

### Channel Append

```
append(channel, message)
════════════════════════

1. sequence_number = channel.log.length
   // 0-indexed. First message is 0, second is 1, etc.

2. channel.log.append(message)

3. If persistence is enabled:
     a. Serialize message envelope to JSON bytes.
     b. Get payload as raw bytes.
     c. Write 17-byte header:
        - magic: "ACTM" (4 bytes)
        - version: 0x01 (1 byte)
        - envelope_length: len(envelope_json) (4 bytes, big-endian)
        - payload_length: len(payload) (8 bytes, big-endian)
     d. Write envelope JSON bytes.
     e. Write raw payload bytes.
     f. Flush to disk (fsync).
        // fsync ensures the message survives a crash.
        // Without fsync, the OS might buffer the write and lose it.
     g. If index file exists, append: sequence_number → byte_offset.

4. Return sequence_number.

Time complexity: O(1) amortized (list append + file write).
```

### Channel Read

```
read(channel, offset, limit)
═════════════════════════════

1. If offset >= channel.log.length:
     return []    // Caller is caught up. No new messages.

2. end = min(offset + limit, channel.log.length)

3. return channel.log[offset:end]
   // Returns a COPY of the messages, not a reference.
   // The caller cannot modify the channel log through
   // the returned list.

Time complexity: O(limit) — copying the requested slice.
```

### Channel Recovery (from disk)

```
recover(channel_name)
═════════════════════

1. path = channels/{channel_name}.log

2. If file does not exist:
     return new empty Channel

3. Open file for reading.

4. Loop until EOF:
     a. Read 17-byte header.
        - Verify magic = "ACTM".
        - Read version, envelope_length, payload_length.
     b. Read envelope_length bytes → parse JSON → envelope fields.
     c. Read payload_length bytes → raw payload.
     d. Reconstruct Message from envelope + payload.
     e. Append to in-memory log (do NOT write back to file).

5. If header is truncated (partial write before crash):
     Discard the incomplete message. Log a warning.
     The last complete message is the recovery point.

6. Return the reconstructed Channel.

This is how the system recovers after a crash. The channel file
IS the source of truth. The in-memory log is just a cache.
The 17-byte header acts as a frame boundary — if a crash happens
mid-write, the next recovery will see a truncated header or
incomplete payload and cleanly discard only the partial message.
```

---

## Public API

The API is defined here in Python syntax. All implementations across languages must
expose equivalent types and functions.

```python
# ═══════════════════════════════════════════════════════════════
# Message
# ═══════════════════════════════════════════════════════════════

class Message:
    """Immutable message — the atom of actor communication.

    Payload is always bytes. Content_type describes how to interpret them.
    Use convenience constructors for common cases: Message.text(),
    Message.json(), Message.binary().
    """

    WIRE_VERSION = 1  # Bump when wire format changes

    def __init__(
        self,
        sender_id: str,
        content_type: str,
        payload: bytes,
        metadata: dict[str, str] | None = None,
    ) -> None: ...

    # --- Convenience constructors ---

    @classmethod
    def text(
        cls,
        sender_id: str,
        payload: str,
        metadata: dict[str, str] | None = None,
    ) -> "Message": ...
        # content_type = "text/plain", payload = text.encode("utf-8")

    @classmethod
    def json(
        cls,
        sender_id: str,
        payload: dict | list,
        metadata: dict[str, str] | None = None,
    ) -> "Message": ...
        # content_type = "application/json", payload = json.dumps(payload).encode("utf-8")

    @classmethod
    def binary(
        cls,
        sender_id: str,
        content_type: str,
        payload: bytes,
        metadata: dict[str, str] | None = None,
    ) -> "Message": ...
        # For images, videos, arbitrary binary data

    # --- Properties (read-only) ---

    @property
    def id(self) -> str: ...            # Auto-generated unique ID

    @property
    def timestamp(self) -> int: ...     # Monotonic nanosecond counter

    @property
    def sender_id(self) -> str: ...

    @property
    def content_type(self) -> str: ...

    @property
    def payload(self) -> bytes: ...     # Always bytes

    @property
    def payload_text(self) -> str: ...  # payload.decode("utf-8") — convenience

    @property
    def payload_json(self) -> dict | list: ...  # json.loads(payload) — convenience

    @property
    def metadata(self) -> dict[str, str]: ...

    # --- Serialization (binary wire format) ---

    def to_bytes(self) -> bytes: ...
        # Serializes to wire format: 17-byte header + envelope JSON + payload bytes.
        # Header contains: magic ("ACTM"), version, envelope_length, payload_length.

    @classmethod
    def from_bytes(cls, data: bytes) -> "Message": ...
        # Deserializes from wire format. Validates magic and version.
        # Raises VersionError if version > WIRE_VERSION (forward-compat check).

    def envelope_to_json(self) -> str: ...
        # Serializes ONLY the envelope (all fields except payload) to JSON.
        # Useful for logging, indexing, debugging without touching the payload.

    @classmethod
    def from_stream(cls, stream) -> "Message": ...
        # Read one message from a byte stream (file or network socket).
        # Reads header first, then envelope, then payload.
        # Returns None on EOF.


# ═══════════════════════════════════════════════════════════════
# Channel
# ═══════════════════════════════════════════════════════════════

class Channel:
    """One-way, append-only, ordered message log."""

    def __init__(self, channel_id: str, name: str) -> None: ...

    @property
    def id(self) -> str: ...

    @property
    def name(self) -> str: ...

    @property
    def created_at(self) -> int: ...

    def append(self, message: "Message") -> int: ...
        # Returns sequence number (0-indexed)

    def read(self, offset: int = 0, limit: int = 100) -> list["Message"]: ...
        # Returns messages from offset, up to limit

    def length(self) -> int: ...
        # Number of messages in the log

    def slice(self, start: int, end: int) -> list["Message"]: ...
        # Messages from start to end (exclusive)

    def persist(self, directory: str) -> None: ...
        # Write log to disk as NDJSON

    @classmethod
    def recover(cls, directory: str, name: str) -> "Channel": ...
        # Reconstruct channel from disk


# ═══════════════════════════════════════════════════════════════
# Actor
# ═══════════════════════════════════════════════════════════════

class ActorResult:
    """Return value from an actor's behavior function."""

    def __init__(
        self,
        new_state: Any,
        messages_to_send: list[tuple[str, "Message"]] | None = None,
        actors_to_create: list["ActorSpec"] | None = None,
        stop: bool = False,
    ) -> None: ...


class ActorSpec:
    """Specification for creating a new actor."""

    def __init__(
        self,
        actor_id: str,
        initial_state: Any,
        behavior: Callable[[Any, "Message"], "ActorResult"],
    ) -> None: ...


# Behavior type alias:
# Behavior = Callable[[State, Message], ActorResult]
# Where State is any type the actor chooses.


# ═══════════════════════════════════════════════════════════════
# ActorSystem
# ═══════════════════════════════════════════════════════════════

class ActorSystem:
    """Runtime for managing actors, message delivery, and channels."""

    def __init__(self) -> None: ...

    # --- Actor lifecycle ---

    def create_actor(
        self,
        actor_id: str,
        initial_state: Any,
        behavior: Callable[[Any, "Message"], "ActorResult"],
    ) -> str: ...
        # Returns actor_id. Raises if id already exists.

    def stop_actor(self, actor_id: str) -> None: ...
        # Sets status to STOPPED, drains mailbox to dead_letters.

    def get_actor_status(self, actor_id: str) -> str: ...
        # Returns "idle", "processing", or "stopped".

    # --- Messaging ---

    def send(self, target_id: str, message: "Message") -> None: ...
        # Enqueue message in target's mailbox.
        # If target not found/stopped, message goes to dead_letters.

    # --- Processing ---

    def process_next(self, actor_id: str) -> bool: ...
        # Process one message from actor's mailbox.
        # Returns True if a message was processed, False if empty.

    def run_until_idle(self) -> dict: ...
        # Process all actors round-robin until no work remains.
        # Returns stats: {"messages_processed": N, "actors_created": M}

    def run_until_done(self) -> dict: ...
        # Like run_until_idle but keeps going until system is fully quiet.

    # --- Channels ---

    def create_channel(self, channel_id: str, name: str) -> "Channel": ...
        # Create and register a new channel.

    def get_channel(self, channel_id: str) -> "Channel": ...
        # Retrieve a channel by ID.

    # --- Inspection ---

    @property
    def dead_letters(self) -> list["Message"]: ...
        # Messages that could not be delivered.

    def actor_ids(self) -> list[str]: ...
        # List all registered actor IDs.

    def mailbox_size(self, actor_id: str) -> int: ...
        # Number of pending messages for an actor.
```

---

## Examples

### Example 1: Echo Actor

The simplest possible actor. It receives a message and sends it back to the sender.

```python
def echo_behavior(state, message):
    """Echo: send the same message back to whoever sent it."""
    reply = Message.text(
        sender_id="echo",
        payload=f"echo: {message.payload_text}",
    )
    return ActorResult(
        new_state=state,
        messages_to_send=[(message.sender_id, reply)],
    )

system = ActorSystem()
system.create_actor("echo", state=None, behavior=echo_behavior)
```

### Example 2: Counter Actor

An actor that maintains state — it counts how many messages it has received.

```python
def counter_behavior(state, message):
    """Count messages received. Report count when asked."""
    count = state + 1
    if message.payload == "get_count":
        reply = Message(
            sender_id="counter",
            content_type="text/plain",
            payload=str(count),
        )
        return ActorResult(
            new_state=count,
            messages_to_send=[(message.sender_id, reply)],
        )
    return ActorResult(new_state=count)

system.create_actor("counter", state=0, behavior=counter_behavior)
```

### Example 3: Two Actors Communicating via Channels

A producer writes to a channel, a consumer reads from it. This is the pattern that
D18's agent pipelines use.

```python
# Create the channel
channel = system.create_channel("ch_001", "greetings")

# Producer appends messages to the channel
msg = Message(sender_id="producer", content_type="text/plain", payload="hello")
seq = channel.append(msg)  # seq = 0

msg2 = Message(sender_id="producer", content_type="text/plain", payload="world")
seq2 = channel.append(msg2)  # seq = 1

# Consumer reads from the channel at its own pace
offset = 0
batch = channel.read(offset=offset, limit=10)
# batch = [msg, msg2]
# Consumer processes them, advances offset to 2
offset += len(batch)

# More messages arrive later...
msg3 = Message(sender_id="producer", content_type="text/plain", payload="!")
channel.append(msg3)

# Consumer reads again from where it left off
batch2 = channel.read(offset=offset, limit=10)
# batch2 = [msg3]
```

### Example 4: Actor That Creates Other Actors

An actor can spawn new actors dynamically. This is how the orchestrator in D18 starts
agents.

```python
def spawner_behavior(state, message):
    """When told to spawn, create a new echo actor."""
    if message.payload == "spawn":
        new_id = f"echo_{state}"
        return ActorResult(
            new_state=state + 1,
            actors_to_create=[
                ActorSpec(
                    actor_id=new_id,
                    initial_state=None,
                    behavior=echo_behavior,
                ),
            ],
            messages_to_send=[(message.sender_id, Message(
                sender_id="spawner",
                content_type="text/plain",
                payload=f"spawned {new_id}",
            ))],
        )
    return ActorResult(new_state=state)

system.create_actor("spawner", state=0, behavior=spawner_behavior)
```

---

## How This Connects to D18 (Chief of Staff)

This package provides the three primitives. D18 adds security, persistence, and
orchestration on top:

```
┌─────────────────────────────────────────────────────────────┐
│  D18 (Chief of Staff)                                        │
│                                                               │
│  Encrypted Channel = Channel + XChaCha20-Poly1305 encryption│
│  Agent             = Actor + capability manifest + Deno cage │
│  Vault             = Actor + encrypted storage + leases      │
│  Orchestrator      = ActorSystem + trust checker + supervisor│
│  Pipeline          = multiple Actors wired via Channels      │
│                                                               │
│  D18 does NOT reimplement these primitives. It wraps them.   │
└─────────────────────────────────────────────────────────────┘
          │
          │ builds on
          ▼
┌─────────────────────────────────────────────────────────────┐
│  D19 (Actor) ← THIS PACKAGE                                  │
│                                                               │
│  Message   — immutable, serializable, self-describing        │
│  Channel   — one-way, append-only, persistent                │
│  Actor     — isolated computation with mailbox + behavior    │
│  ActorSystem — lifecycle, delivery, round-robin processing   │
│                                                               │
│  No encryption. No capabilities. No vault. No network.       │
│  Just the pure computation model.                            │
└─────────────────────────────────────────────────────────────┘
```

---

## Dependencies

```
D19 Actor
│
├── depends on ──→ (nothing — foundational primitive)
│
├── used by ──→ Chief of Staff (D18)
│                 └── agents, orchestrator, vault are all actors
│                 └── channels carry messages between agents
│
├── conceptually related to ──→ IPC (D16)
│                                 └── channels are like message queues
│                                     but with persistence and one-way
│
└── conceptually related to ──→ Process Manager (D14)
                                  └── actors map to processes in
                                      production deployments
```

---

## Testing Strategy

### Unit Tests — Message

1. **Create message**: Create a Message with all fields, verify all properties return
   correct values.
2. **Immutability**: Verify that Message has no setter methods. Attempting to modify
   fields raises an error or is prevented by the type system.
3. **Unique IDs**: Create 1000 messages, verify all IDs are unique.
4. **Timestamp ordering**: Create messages sequentially, verify timestamps are strictly
   increasing.
5. **Wire format round-trip (text)**: Create text message, to_bytes, from_bytes,
   verify all fields match including payload.
6. **Wire format round-trip (binary)**: Create binary message (PNG header bytes),
   to_bytes, from_bytes, verify payload bytes are identical.
7. **Metadata passthrough**: Create message with metadata, serialize/deserialize,
   verify metadata preserved.
8. **Empty payload**: Create message with empty payload (zero bytes), verify it works.
9. **Large payload**: Create message with 1MB binary payload, verify serialization works.
10. **Content type**: Verify content_type is preserved across serialization.
11. **Convenience constructors**: Message.text(), Message.json(), Message.binary() all
    produce correct content_type and payload encoding.
12. **payload_text**: Text message payload_text returns decoded string.
13. **payload_json**: JSON message payload_json returns parsed dict/list.
14. **Envelope-only serialization**: envelope_to_json() produces JSON without payload.
15. **Wire format magic**: to_bytes() starts with "ACTM" magic bytes.
16. **Wire format version**: to_bytes() contains version byte matching WIRE_VERSION.
17. **Future version rejection**: from_bytes() with version > WIRE_VERSION raises
    VersionError (not a crash, a clear error).
18. **Corrupt magic rejection**: from_bytes() with wrong magic raises InvalidFormat.
19. **Stream reading**: from_stream() reads exactly one message from a byte stream
    and leaves the stream positioned at the start of the next message.

### Unit Tests — Channel

20. **Create channel**: Create a Channel, verify id and name.
21. **Append and length**: Append 3 messages, verify length() returns 3.
22. **Append returns sequence number**: Verify append returns 0, 1, 2 for successive
    appends.
23. **Read from beginning**: Append 5 messages, read(0, 5), verify all 5 returned
    in order.
24. **Read with offset**: Append 5 messages, read(2, 3), verify messages 2, 3, 4
    returned.
25. **Read past end**: Append 3 messages, read(5, 10), verify empty list returned.
26. **Read with limit**: Append 10 messages, read(0, 3), verify only 3 returned.
27. **Slice**: Append 5 messages, slice(1, 4), verify messages 1, 2, 3 returned.
28. **Independent readers**: Two consumers read the same channel at different offsets,
    verify they see correct messages independently.
29. **Append-only**: Verify there is no method to delete or modify messages in the log.
30. **Binary persistence**: Append messages (text + binary), persist to disk, verify
    file starts with "ACTM" magic and contains correct headers.
31. **Recovery**: Persist a channel, recover from disk, verify all messages restored
    including binary payloads.
32. **Recovery preserves order**: Persist 100 messages, recover, verify order matches.
33. **Empty channel recovery**: Recover from non-existent file, verify empty channel.
34. **Mixed content recovery**: Persist text, JSON, and binary (PNG) messages to same
    channel, recover, verify all content types and payloads are correct.
35. **Truncated write recovery**: Simulate crash mid-write (truncate file mid-message),
    recover, verify all complete messages are restored and partial message is discarded.
36. **Mixed version recovery**: Write v1 messages, simulate upgrade to v2 (hypothetical),
    write v2 messages. Recover and verify both versions are correctly parsed.

### Unit Tests — Actor

37. **Create actor**: Create actor with initial state, verify status is IDLE.
38. **Send message**: Send message to actor, verify mailbox_size is 1.
39. **Process message**: Send message, call process_next, verify behavior was called.
40. **State update**: Create counter actor, send 3 messages, verify state is 3.
41. **Messages to send**: Create echo actor, send message, process, verify reply
    was delivered to sender's mailbox.
42. **Actor creation**: Create spawner actor, send "spawn" message, process, verify
    new actor exists in system.
43. **Stop actor**: Send stop message, verify status is STOPPED after processing.
44. **Stopped actor rejects messages**: Stop an actor, send a message, verify it
    goes to dead_letters.
45. **Dead letters**: Send message to non-existent actor, verify dead_letters contains it.
46. **Sequential processing**: Send 3 messages, process_next 3 times, verify they
    were processed in FIFO order.
47. **Mailbox drains on stop**: Actor has 3 pending messages, stop it, verify all 3
    go to dead_letters.
48. **Behavior exception**: Create actor whose behavior throws on certain messages,
    verify: state unchanged, message goes to dead_letters, actor continues processing
    next message.
49. **Duplicate actor ID**: Attempt to create two actors with same ID, verify error.

### Integration Tests

50. **Ping-pong**: Two actors send messages back and forth 10 times. Verify both
    processed 10 messages. Verify system reaches idle state.
51. **Pipeline**: Three actors in a chain: A → B → C. A sends a message, B transforms
    it and forwards to C. Verify C receives the transformed message.
52. **Channel-based pipeline**: Producer writes to channel, consumer reads from channel
    and processes. Verify consumer reads all messages in order.
53. **Fan-out**: One actor sends the same message to 5 different actors. Verify all 5
    receive and process it.
54. **Dynamic topology**: Actor A spawns Actor B, sends B a message, B responds.
    Verify the full round-trip works with a dynamically created actor.
55. **Run until idle**: Create a complex network of 5 actors with interconnected
    messaging. Call run_until_idle(). Verify all messages were processed and system
    is quiet.
56. **Persistence round-trip**: Create actors, send messages through channels (including
    binary payloads), persist channels, create a new ActorSystem, recover channels,
    verify all messages are intact.
57. **Large-scale**: Create 100 actors, send 1000 messages randomly between them,
    run_until_done, verify no messages lost (all delivered or in dead_letters).
58. **Binary message pipeline**: Actor A sends a PNG image to Actor B via a channel.
    Actor B receives and verifies the image bytes are identical to what A sent.

### Coverage Target

Target 95%+ line coverage. Every error path (actor not found, duplicate ID, stopped
actor, behavior exception) and every edge case (empty mailbox, read past end, zero-length
payload) must be tested.

---

## Future Extensions

These are explicitly **not** in V1 but are designed to be addable without changing the
core primitives:

1. **Encryption layer** (D18): Channels gain encryption — each message payload is
   encrypted with the channel master key. The Channel primitive does not change; a
   wrapper `EncryptedChannel` handles encrypt-on-append and decrypt-on-read.

2. **Supervision trees** (D18): The ActorSystem gains a supervision strategy — when an
   actor crashes, its supervisor (another actor) decides what to do: restart it, stop
   it, or escalate. This is the Erlang/OTP supervision model.

3. **Location transparency**: Actors on different machines communicate through network
   channels. The actor code does not change — only the channel transport layer swaps
   from local file to TCP stream.

4. **Capability-aware actors**: Each actor gains an `agent_manifest.json` that declares
   what it is allowed to do. The ActorSystem enforces these capabilities. This is the
   bridge to D18's capability cage.

5. **Backpressure**: When an actor's mailbox exceeds a threshold, the ActorSystem slows
   down senders. This prevents a fast producer from overwhelming a slow consumer.

6. **Snapshotting**: Periodically persist actor state to disk for faster recovery.
   Instead of replaying all messages from the channel log, restore the latest snapshot
   and replay only messages since the snapshot.
