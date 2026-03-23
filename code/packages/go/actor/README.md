# Actor

A pure Go implementation of the Actor model — the mathematical framework for
concurrent computation invented by Carl Hewitt, Peter Bishop, and Richard
Steiger in 1973.

This package provides three foundational primitives that everything in the
Chief of Staff system (D18) builds on:

1. **Message** — the atom of communication. Immutable, typed, serializable.
2. **Channel** — a one-way, append-only pipe for messages. Persistent and replayable.
3. **Actor** — an isolated unit of computation with a mailbox and internal state.
4. **ActorSystem** — the runtime that manages actor lifecycles, message delivery, and channels.

## Architecture

```
User Programs / Chief of Staff (D18)
|   create_actor(behavior)     -- spawn a new actor
|   send(actor_ref, message)   -- deliver a message
|   channel.append(message)    -- publish to a channel
|   channel.read(offset)       -- consume from a channel
v
Actor Runtime  <-- THIS PACKAGE
|   +-- Actor         -- isolated computation + mailbox
|   +-- Message       -- immutable typed payload
|   +-- Channel       -- one-way append-only pipe
|   +-- ActorSystem   -- lifecycle, routing, dead letters
v
No dependencies -- this is a foundational primitive
```

## Primitives

### Message

An immutable, self-describing unit of communication. Every piece of data that
flows between actors is a Message. Fields are set at creation time and cannot
be modified.

- **ID**: Auto-generated unique identifier
- **Timestamp**: Monotonic nanosecond counter (strictly increasing)
- **SenderID**: The actor that created this message
- **ContentType**: MIME type describing the payload format
- **Payload**: Raw bytes (never interpreted by the system)
- **Metadata**: Optional key-value pairs for extensibility

### Channel

A one-way, append-only, ordered log of messages. Connects producers to
consumers. Messages flow in one direction only. Once appended, a message
cannot be removed, modified, or reordered. Channels persist to disk as
binary append logs using the Message wire format.

### Actor

An isolated unit of computation with an address (ID), a mailbox (FIFO queue),
a behavior function, and private state. Processes one message at a time.
Cannot access another actor's state.

### ActorSystem

The runtime that manages actor lifecycles, message delivery, and channels.
Processes actors in round-robin order. Undeliverable messages go to dead letters.

## Usage

### Echo Actor

```go
package main

import "github.com/adhithyan15/coding-adventures/code/packages/go/actor"

func echoBehavior(state interface{}, msg *actor.Message) (*actor.ActorResult, error) {
    reply := actor.NewTextMessage("echo", "echo: "+msg.PayloadText(), nil)
    return &actor.ActorResult{
        NewState: state,
        MessagesToSend: []actor.OutgoingMessage{
            {TargetID: msg.SenderID(), Msg: reply},
        },
    }, nil
}

func main() {
    sys := actor.NewActorSystem()
    sys.CreateActor("echo", nil, echoBehavior)
    sys.Send("echo", actor.NewTextMessage("user", "hello", nil))
    sys.RunUntilIdle()
}
```

### Counter Actor

```go
func counterBehavior(state interface{}, msg *actor.Message) (*actor.ActorResult, error) {
    count := state.(int) + 1
    return &actor.ActorResult{NewState: count}, nil
}

sys := actor.NewActorSystem()
sys.CreateActor("counter", 0, counterBehavior)
sys.Send("counter", actor.NewTextMessage("user", "tick", nil))
sys.RunUntilIdle()
```

### Channel Pipeline

```go
sys := actor.NewActorSystem()
ch := sys.CreateChannel("ch_001", "events")

// Producer writes
ch.Append(actor.NewTextMessage("producer", "event-1", nil))
ch.Append(actor.NewTextMessage("producer", "event-2", nil))

// Consumer reads
msgs := ch.Read(0, 10)
for _, msg := range msgs {
    fmt.Println(msg.PayloadText())
}
```

### Persistence and Recovery

```go
ch := actor.NewChannel("ch_001", "events")
ch.Append(actor.NewTextMessage("sender", "hello", nil))
ch.Persist("/path/to/channels")

// Later, after crash/restart:
recovered, _ := actor.Recover("/path/to/channels", "events")
msgs := recovered.Read(0, 100)
```

## Wire Format

Messages serialize to a binary format with a 17-byte fixed header:

```
+-------------------------------------------+
| HEADER (17 bytes)                         |
|   magic:          4 bytes  "ACTM"         |
|   version:        1 byte   0x01           |
|   envelope_length: 4 bytes (big-endian)   |
|   payload_length:  8 bytes (big-endian)   |
+-------------------------------------------+
| ENVELOPE (JSON, variable length)          |
+-------------------------------------------+
| PAYLOAD (raw bytes, variable length)      |
+-------------------------------------------+
```

This format avoids Base64 bloat for binary data — a 10MB image is 10MB on disk.

## Dependencies

None. This is a foundational primitive with zero external dependencies.

## Testing

```bash
go test ./... -v -cover
```

Target: 95%+ line coverage across all files.
