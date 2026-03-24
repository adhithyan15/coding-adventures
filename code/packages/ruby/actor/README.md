# Actor

An implementation of the Actor model for concurrent computation, providing three
primitives: **Message**, **Channel**, and **Actor**.

The Actor model was invented by Carl Hewitt in 1973. It defines computation in
terms of isolated entities (actors) that communicate exclusively through
immutable messages. No shared memory. No locks. No mutexes. Erlang/OTP, Akka,
and Microsoft Orleans are all built on this model.

## Primitives

```
┌─────────────────────────────────────────────────────────────┐
│  Message   — immutable, typed, binary-native, serializable  │
│  Channel   — one-way, append-only, persistent message log   │
│  Actor     — isolated computation with mailbox + behavior   │
│  ActorSystem — lifecycle, delivery, round-robin processing  │
└─────────────────────────────────────────────────────────────┘
```

## Where It Fits

```
User Programs / Chief of Staff (D18)
│   create_actor(behavior)     — spawn a new actor
│   send(actor_ref, message)   — deliver a message
│   channel.append(message)    — publish to a channel
▼
Actor Runtime ← THIS PACKAGE
│   ├── Actor         — isolated computation + mailbox
│   ├── Message       — immutable typed payload
│   ├── Channel       — one-way append-only pipe
│   └── ActorSystem   — lifecycle, supervision, routing
▼
IPC (D16) / Process Manager (D14)
```

## Usage

```ruby
require "coding_adventures_actor"

# Aliases for convenience
Message     = CodingAdventures::Actor::Message
ActorResult = CodingAdventures::Actor::ActorResult
ActorSystem = CodingAdventures::Actor::ActorSystem

# Create an actor system
system = ActorSystem.new

# Define a counter behavior
counter = ->(state, msg) {
  count = state + 1
  puts "Received message ##{count}: #{msg.payload_text}"
  ActorResult.new(new_state: count)
}

# Create an actor with initial state 0
system.create_actor("counter", 0, counter)

# Send messages
system.send_message("counter", Message.text(sender_id: "user", payload: "hello"))
system.send_message("counter", Message.text(sender_id: "user", payload: "world"))

# Process all messages
system.run_until_done
# => Received message #1: hello
# => Received message #2: world
```

### Echo Actor

```ruby
echo = ->(state, msg) {
  reply = Message.text(sender_id: "echo", payload: "echo: #{msg.payload_text}")
  ActorResult.new(
    new_state: state,
    messages_to_send: [[msg.sender_id, reply]]
  )
}

system.create_actor("echo", nil, echo)
```

### Channels

```ruby
# Create a channel for message persistence
channel = system.create_channel("ch_001", "events")

# Append messages
channel.append(Message.text(sender_id: "producer", payload: "event_1"))
channel.append(Message.json(sender_id: "producer", payload: {"type" => "click"}))

# Read from an offset
messages = channel.read(offset: 0, limit: 10)

# Persist to disk
channel.persist("/tmp/channels")

# Recover from disk
recovered = CodingAdventures::Actor::Channel.recover("/tmp/channels", "events")
```

### Binary Messages

```ruby
# Messages support arbitrary binary payloads
png_bytes = File.binread("photo.png")
msg = Message.binary(
  sender_id: "camera",
  content_type: "image/png",
  payload: png_bytes
)

# Wire format preserves binary data without Base64 bloat
bytes = msg.to_bytes
restored = Message.from_bytes(bytes)
restored.payload == png_bytes  # => true
```

## Dependencies

- **Runtime:** None (stdlib only)
- **Development:** minitest, simplecov, standard, rake
