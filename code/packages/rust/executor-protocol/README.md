# executor-protocol

Wire format and transport between the matrix-execution runtime and its executors.

This is the implementation of spec **MX03**.  See:

- [`code/specs/MX00-matrix-execution-overview.md`](../../specs/MX00-matrix-execution-overview.md) — architecture
- [`code/specs/MX03-executor-protocol.md`](../../specs/MX03-executor-protocol.md) — this crate's contract

## What it is

The boundary between the runtime and an executor is a **serializable
wire protocol**, never a Rust trait alone.  The runtime always speaks
to executors as bytes-on-a-wire — even when the executor is in the
same process.  The local case is just one transport implementation
where "the wire is a function call."

This crate defines:

- **Messages** — `ExecutorRequest`, `ExecutorResponse`, `ExecutorEvent`
- **Sub-types** — `KernelSource`, `BackendProfile`, `OpTiming`, `ErrorCode`
- **Frame** — `MessageFrame` (versioned envelope wrapping every message)
- **Wire format** — hand-rolled binary encoding per spec MX03
- **Transport trait** — `Transport` (pluggable wire layer, async-first)
- **Local transport** — `LocalTransport` for in-process executors
- **Async runner** — `block_on` (hand-rolled minimal poll loop, ~50 lines)
- **Kernel cache** — `KernelCacheKey` (SipHash-based content key)

V1 ships only `LocalTransport`.  Future transports (TCP, Unix sockets,
ZeroMQ, NATS, WebSocket) are designed for, not shipped.

## Where it sits

```
   matrix-ir   →   compute-ir   →   [executor-protocol]   →   executors
                                                              (cpu / metal / cuda / wgpu / asic)
```

The runtime and executors only see typed messages.  The wire layer is
the only transport-agnostic surface in the layer.

## Worked example

```rust
use compute_ir::BufferId;
use executor_protocol::{
    block_on, ExecutorRequest, ExecutorResponse, LocalTransport, Transport,
};

let executor = LocalTransport::new(|req| match req {
    ExecutorRequest::AllocBuffer { bytes } => ExecutorResponse::BufferAllocated {
        buffer: BufferId(bytes),  // toy: id == size
    },
    _ => ExecutorResponse::ShuttingDown,
});

let resp = block_on(executor.request(ExecutorRequest::AllocBuffer { bytes: 1024 }))
    .expect("transport");
match resp {
    ExecutorResponse::BufferAllocated { buffer } => assert_eq!(buffer.0, 1024),
    _ => unreachable!(),
}
```

## Top-level frame

Every message is wrapped in a versioned envelope:

```text
u8           format_version    (= 1 in V1)
u8           message_kind      (0=Request, 1=Response, 2=Event)
u64          correlation_id    (matches responses to requests; events use 0)
uv64         payload_length
payload_length raw bytes        (the encoded message)
```

The framing serves three purposes: byte-level versioning (so a future
reader rejects unknown versions cleanly), multiplexing (one transport
can carry multiple in-flight requests), and self-delimitation (so
stream-oriented transports like TCP can frame messages without an
extra layer).

## Async-first

The `Transport` trait uses `async fn` so future network transports
(TCP, Unix sockets, ZMQ) can do real I/O without restructuring.
In-process transports return immediately-ready futures.

V1 supplies a hand-rolled `block_on` (~50 lines, no_std-friendly,
single-threaded) for driving the local case.  Real network transports
will need a real reactor; those will live in their own transport
crates.

## Debug-build round-trip

`LocalTransport` round-trips requests and responses through the wire
format on every `request()` call in **debug builds**.  This catches
"I accidentally put a non-serializable type in the protocol" bugs at
PR-test time.  Release builds skip the round-trip.

## Security

The wire decoder accepts untrusted input (a remote executor could be
malicious or compromised).  Hardening — same patterns as `matrix-ir`
and `compute-ir`:

- All `Vec::with_capacity` allocations bounded against remaining buffer
  bytes
- `Reader::need` uses `checked_add` to prevent overflow
- `bytes()` rejects `u64` lengths exceeding `usize::MAX`
- Varint capped at 10 bytes
- UTF-8 validated for string fields
- Truncation-at-every-position and 1024-iteration random-byte fuzz tests
- All 12 message variants exhaustively round-tripped

## Testing

```
cargo test -p executor-protocol
```

Test methodology (per spec MX03 §"Test methodology"):

- **Wire round-trip** — every variant of every enum encoded, decoded,
  and asserted equal
- **Truncation** — every wire form fails cleanly at every byte offset
- **Forward-compat** — frames with future versions are rejected
- **Frame fuzzing** — 1024 deterministic-PRNG random byte strings, no
  panics
- **Local transport** — representative requests round-tripped through
  echo executor
- **Kernel cache** — content-based key is stable and language-distinct

## Zero dependencies

```
$ cargo tree -p executor-protocol
executor-protocol v0.1.0
├── compute-ir v0.1.0
│   └── matrix-ir v0.1.0
└── matrix-ir v0.1.0
```

Only the upstream IR crates as path deps.  No external crates.

## Out of scope (V1)

Reserved for future versions:

- **Compression** of large payloads (`Compressed { codec, bytes }`)
- **Encryption / authentication** (transport-layer concern)
- **Streaming buffers** for very large tensors (`BufferChunk`)
- **Bidirectional flow control** / backpressure
- **Network transports** — `matrix-transport-tcp`, `matrix-transport-zmq`,
  etc., each in its own crate

See spec MX03 §"Out of scope" for the migration plan.
