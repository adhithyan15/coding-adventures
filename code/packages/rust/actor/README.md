# Actor

Actor model primitives for concurrent computation -- messages, channels, and actors.

This crate implements the three foundational primitives of the Actor model, the
mathematical framework for concurrent computation invented by Carl Hewitt, Peter Bishop,
and Richard Steiger in 1973. Erlang/OTP, Akka, Microsoft Orleans, and Discord's Elixir
backend are all built on this model.

## Primitives

1. **Message** -- the atom of communication. Immutable, typed, binary-serializable.
2. **Channel** -- a one-way, append-only, ordered log of messages. Persistent and replayable.
3. **Actor** -- an isolated unit of computation with a mailbox, behavior function, and private state.
4. **ActorSystem** -- the runtime that manages actor lifecycles, message delivery, and channels.

## Architecture

```text
  ActorSystem
  +-------------------------------------------------------------+
  |                                                               |
  |  actors: HashMap<id, Actor>                                   |
  |  +------------------+  +------------------+                   |
  |  | Actor "echo"     |  | Actor "counter"  |                   |
  |  | mailbox: [m1,m2] |  | mailbox: []      |                   |
  |  | state: ...       |  | state: 42        |                   |
  |  | behavior: fn     |  | behavior: fn     |                   |
  |  +------------------+  +------------------+                   |
  |                                                               |
  |  channels: HashMap<id, Channel>                               |
  |  +----------------------------------------------------+      |
  |  | Channel "email-summaries"                            |      |
  |  | log: [msg0] [msg1] [msg2] [msg3]                    |      |
  |  +----------------------------------------------------+      |
  |                                                               |
  |  dead_letters: [undeliverable messages]                       |
  |  clock: monotonic counter for timestamps                      |
  +-------------------------------------------------------------+
```

## Usage

```rust
use actor::{Message, Channel, Actor, ActorSystem, ActorResult};

// Create an actor system
let mut system = ActorSystem::new();

// Create an echo actor
system.create_actor("echo", Box::new(0_u64), Box::new(|state, msg| {
    let reply = Message::text("echo", &format!("echo: {}", msg.payload_text()));
    ActorResult {
        new_state: state,
        messages_to_send: vec![(msg.sender_id.clone(), reply)],
        actors_to_create: vec![],
        stop: false,
    }
})).unwrap();

// Send a message
let msg = Message::text("user", "hello");
system.send("echo", msg);

// Process the message
system.process_next("echo").unwrap();
```

## Wire Format

Messages serialize to a binary wire format with a 17-byte fixed header:

```text
+------+------+----------------+----------------+
| ACTM | v1   | envelope_len   | payload_len    |
| 4B   | 1B   | 4B big-endian  | 8B big-endian  |
+------+------+----------------+----------------+
| JSON envelope (variable length)                |
+------------------------------------------------+
| Raw payload bytes (variable length)            |
+------------------------------------------------+
```

The envelope contains message metadata (id, timestamp, sender_id, content_type, metadata)
as JSON. The payload is raw bytes -- no Base64 encoding, no bloat.

## No External Dependencies

This crate uses only the Rust standard library. JSON serialization for the envelope
is implemented manually since the envelope has a fixed, known schema.
