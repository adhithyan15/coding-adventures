# Actor

Actor model implementation — messages, channels, and actors for concurrent computation.

This package implements the three foundational primitives from Carl Hewitt's Actor model (1973), the same model that powers Erlang/OTP, Akka, and Microsoft Orleans:

1. **Message** — immutable, self-describing, binary-serializable unit of communication
2. **Channel** — one-way, append-only, persistent message log
3. **Actor** — isolated unit of computation with a mailbox, private state, and behavior function
4. **ActorSystem** — runtime for actor lifecycle, message delivery, and round-robin processing

```
┌──────────────────────────────────────────────────────┐
│  ActorSystem                                         │
│                                                      │
│  ┌─────────┐   Message   ┌─────────┐                │
│  │ Actor A  │ ─────────→ │ Actor B  │                │
│  │ mailbox  │             │ mailbox  │                │
│  │ state    │             │ state    │                │
│  └─────────┘             └─────────┘                │
│       │                                              │
│       │ append                                       │
│       ▼                                              │
│  ┌──────────────────┐                                │
│  │ Channel (log)    │                                │
│  │ [m0][m1][m2]...  │                                │
│  └──────────────────┘                                │
└──────────────────────────────────────────────────────┘
```

## Usage

### Creating Messages

```typescript
import { Message } from "@coding-adventures/actor";

// Text message
const text = Message.text("alice", "Hello, world!");

// JSON message
const json = Message.json("alice", { action: "greet", target: "bob" });

// Binary message (e.g., PNG image)
const binary = Message.binary("camera", "image/png", pngBytes);

// Access payload
text.payloadText;   // "Hello, world!"
json.payloadJson;   // { action: "greet", target: "bob" }
```

### Using Channels

```typescript
import { Channel, Message } from "@coding-adventures/actor";

const channel = new Channel("ch_001", "events");

// Append messages (returns sequence number)
channel.append(Message.text("producer", "event 1"));  // 0
channel.append(Message.text("producer", "event 2"));  // 1

// Read from any offset
const messages = channel.read(0, 10);  // all messages

// Persist to disk and recover
channel.persist("./data/channels");
const recovered = Channel.recover("./data/channels", "events");
```

### Creating Actors

```typescript
import { ActorSystem, Message } from "@coding-adventures/actor";
import type { ActorResult, Behavior } from "@coding-adventures/actor";

// Define a behavior function
const counterBehavior: Behavior<number> = (state, message) => {
  const newCount = state + 1;
  return {
    newState: newCount,
    messagesToSend: [
      [message.senderId, Message.text("counter", `count: ${newCount}`)],
    ],
  };
};

// Create system and actors
const system = new ActorSystem();
system.createActor("counter", 0, counterBehavior);

// Send messages and process
system.send("counter", Message.text("alice", "increment"));
system.runUntilDone();
```

## Wire Format

Messages serialize to a compact binary format:

```
[ACTM][v1][envelope_len:u32][payload_len:u64][JSON envelope][raw payload]
 4B    1B   4B               8B               variable       variable
```

The envelope (metadata) is JSON; the payload is raw bytes. No Base64 overhead for binary data.

## Dependencies

None at runtime. Dev dependencies: TypeScript, Vitest, coverage tooling.

## Where It Fits

This is D19 in the coding-adventures stack. It provides the foundation for the D18 Chief of Staff system, where agents are actors, channels carry messages between them, and the orchestrator is a supervisory actor.
