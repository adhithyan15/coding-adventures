# Actor

The Actor model -- immutable messages, append-only channels, and isolated actors for concurrent computation.

This package implements the three primitives that everything in the Chief of Staff system (D18) builds on:

1. **Message** -- the atom of communication. Immutable, typed, serializable to a compact binary wire format.
2. **Channel** -- a one-way, append-only pipe for messages. Persistent to disk and replayable after crashes.
3. **Actor** -- an isolated unit of computation with a mailbox, behavior function, and private state.
4. **ActorSystem** -- the runtime that manages actor lifecycles, message delivery, and channels.

## Where It Fits

```
User Programs / Chief of Staff (D18)
|   create_actor(behavior)     -- spawn a new actor
|   send(actor_ref, message)   -- deliver a message
|   channel.append(message)    -- publish to a channel
|   channel.read(offset)       -- consume from a channel
v
Actor Runtime  <-- THIS PACKAGE
|   |-- Actor         -- isolated computation + mailbox
|   |-- Message       -- immutable typed payload
|   |-- Channel       -- one-way append-only pipe
|   +-- ActorSystem   -- lifecycle, supervision, routing
v
IPC (D16)                     Process Manager (D14)
|   channels build on          |   actors map to processes
|   message queue concepts     |   in production; in-process
v                              |   threads for lightweight use
File System (D15)
|   channel logs persist to disk
|   actor state snapshots
```

## Usage

### Example 1: Echo Actor

```python
from actor import Message, ActorResult, ActorSystem

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

```python
def counter_behavior(state, message):
    """Count messages received. Report count when asked."""
    count = state + 1
    return ActorResult(new_state=count)

system.create_actor("counter", state=0, behavior=counter_behavior)
system.send("counter", Message.text("user", "tick"))
system.send("counter", Message.text("user", "tick"))
system.run_until_done()
```

### Example 3: Two Actors Communicating via Channels

```python
# Create the channel
channel = system.create_channel("ch_001", "greetings")

# Producer appends messages to the channel
msg = Message.text(sender_id="producer", payload="hello")
seq = channel.append(msg)  # seq = 0

# Consumer reads from the channel at its own pace
offset = 0
batch = channel.read(offset=offset, limit=10)
offset += len(batch)
```

### Example 4: Actor That Creates Other Actors

```python
from actor import ActorSpec

def spawner_behavior(state, message):
    """When told to spawn, create a new echo actor."""
    if message.payload_text == "spawn":
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
        )
    return ActorResult(new_state=state)

system.create_actor("spawner", state=0, behavior=spawner_behavior)
```

## Wire Format

Messages serialize to a compact binary format:

```
HEADER (17 bytes):  [magic "ACTM" 4B] [version 1B] [envelope_len 4B] [payload_len 8B]
ENVELOPE:           JSON with id, timestamp, sender_id, content_type, metadata
PAYLOAD:            Raw bytes (no Base64 encoding, zero bloat)
```

## Dependencies

None -- this is a foundational primitive using only the Python standard library.
