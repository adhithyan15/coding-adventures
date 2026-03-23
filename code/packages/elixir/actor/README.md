# Actor

An implementation of the Actor model in Elixir — three primitives sufficient to build
any concurrent system: **Message**, **Channel**, and **Actor**.

## What Is This?

The Actor model (Hewitt, Bishop, Steiger, 1973) defines computation in terms of
isolated entities that communicate exclusively through messages. No shared memory,
no locks, no mutexes. This package provides the foundational primitives that the
Chief of Staff system (D18) builds on.

## Primitives

```
┌─────────────────────────────────────────────────────────┐
│  Message   — immutable, typed, binary-native wire format│
│  Channel   — one-way, append-only, persistent log       │
│  Actor     — isolated computation with mailbox + state  │
│  ActorSystem — lifecycle, delivery, round-robin runtime │
└─────────────────────────────────────────────────────────┘
```

## Architecture

```
User Programs / Chief of Staff (D18)
│   create_actor(behavior)     — spawn a new actor
│   send_message(actor, msg)   — deliver a message
│   channel.append(message)    — publish to a channel
│   channel.read(offset)       — consume from a channel
▼
Actor Runtime ← THIS PACKAGE
│   ├── Actor         — isolated computation + mailbox
│   ├── Message       — immutable typed payload
│   ├── Channel       — one-way append-only pipe
│   └── ActorSystem   — lifecycle, routing, dead letters
```

## Usage

```elixir
alias CodingAdventures.Actor.{Message, Channel, ActorResult, ActorSpec, ActorSystem}

# --- Messages ---
msg = Message.text("agent", "hello world")
msg = Message.json("agent", %{"key" => "value"})
msg = Message.binary("browser", "image/png", png_bytes)

# Wire format serialization
bytes = Message.to_bytes(msg)
{:ok, decoded} = Message.from_bytes(bytes)

# --- Channels ---
channel = Channel.new("ch_001", "events")
{channel, seq} = Channel.append(channel, msg)
messages = Channel.read(channel, 0, 10)

# Persistence
Channel.persist(channel, "/tmp/channels")
recovered = Channel.recover("/tmp/channels", "events")

# --- Actors ---
counter_behavior = fn state, _msg ->
  %ActorResult{new_state: state + 1}
end

system = ActorSystem.new()
{:ok, system} = ActorSystem.create_actor(system, "counter", 0, counter_behavior)
system = ActorSystem.send_message(system, "counter", Message.text("user", "tick"))
{system, :ok} = ActorSystem.process_next(system, "counter")

# Run until all mailboxes are empty
{system, stats} = ActorSystem.run_until_done(system)
```

## Key Design Decisions

- **No external dependencies** — includes a minimal JSON encoder/decoder
- **Pure functional** — all operations return new structs; originals are unchanged
- **Binary-native wire format** — no Base64 bloat for binary payloads
- **Crash recovery** — channels recover from truncated writes gracefully
- **Dead letters** — undeliverable messages are captured, never silently dropped

## Dependencies

None. This is a foundational primitive.

## Testing

```bash
mix test           # Run all tests
mix test --cover   # Run with coverage report
```

89 tests, 86%+ line coverage.
