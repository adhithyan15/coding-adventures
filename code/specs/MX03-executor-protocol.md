# MX03 — `executor-protocol`: Wire Format and Transport

## Status

Draft — V1 specification.  Read [MX00](MX00-matrix-execution-overview.md),
[MX01](MX01-matrix-ir.md), and [MX02](MX02-compute-ir.md) first.

## Purpose

`executor-protocol` defines the **wire-level contract** between the
runtime and an executor.  It has two parts:

1. **Message types** — the fixed set of requests an executor can answer
   and responses it can produce.
2. **Wire format** — the byte-level encoding of those messages, hand-rolled
   from primitives (varint, length-prefixed bytes, tagged unions).

Plus the `Transport` trait that abstracts how messages are physically
delivered (function call, socket, queue, …).

The unifying principle, restated:  *anything that crosses from runtime to
executor goes as bytes.*  Local executors run the same code path as
remote ones; in-process is just one transport implementation.

This crate has zero external dependencies.

## Why a hand-rolled wire format

The repo's principle is no third-party dependencies.  Existing
serialisation crates (`serde`, `bincode`, `postcard`, `rmp`) are off the
table.  In return:

- The format is **transparent** — every byte's meaning is documented in
  this spec.  A Python or JavaScript or Go client can be written from
  the spec alone, with no port of a Rust crate.
- The format is **versioned at the wire level**, not at the type level.
  Each message starts with a one-byte format version; adding fields is a
  format-version bump that old readers can detect and reject.
- The format is **small** — varints for integers, length-prefixed bytes
  for variable-length data.  No schema descriptors, no field names, no
  reflection.

Trade-offs we accept:

- We hand-roll one encoder/decoder per type.  Tedious; mitigated by a
  small primitives module shared by every type.
- Adding a field to a message requires a coordinated wire-format update.
  This is fine for V1 where the message set is small and stable.

## Wire format primitives

Every value in the protocol is built from six primitives.  All
multi-byte integers are little-endian.

### `u8`, `u16`, `u32`, `u64`

Fixed-width little-endian integers.  Used for short, well-bounded
fields where the cost of varint encoding is not worth its compression.

### Varint (`uv64`)

Unsigned 64-bit varint.  Bytes encode 7 bits each, low-order first; the
top bit of each byte signals "more bytes follow".

```
encode(0)         -> [0x00]
encode(1)         -> [0x01]
encode(127)       -> [0x7F]
encode(128)       -> [0x80, 0x01]
encode(300)       -> [0xAC, 0x02]
encode(2^32 - 1)  -> [0xFF, 0xFF, 0xFF, 0xFF, 0x0F]
```

A reader that sees more than 10 bytes without the high bit clearing
fails the message.  This bounds varint cost.

### Bytes (`bytes`)

Variable-length octet string.  Layout: varint `length`, then `length`
raw bytes.

### String (`str`)

UTF-8 string.  Layout: identical to `bytes`.  Decoders validate UTF-8;
invalid sequences fail the message.

### Tagged union (`enum`)

Layout: varint `tag` followed by the variant's payload.  Tags are
assigned by this spec for each enum and never change once assigned;
new variants get new tags at the end.

### Array (`vec<T>`)

Layout: varint `len` followed by `len` instances of `T`.

## Encoding `matrix-ir` types

Cross-references for MX01 types.  The format applies to anything in
`matrix-ir` and `compute-ir` that needs to travel.

```
DType (u8 tag):
  0x00 = F32
  0x01 = U8
  0x02 = I32
  (V2: 0x03 = F16, 0x04 = I64)

Shape:
  varint dim_count, then dim_count u32 dims

TensorId:           u32
OpId:               u32
ExecutorId:         u32
BufferId:           u64
KernelId:           u64

Tensor:
  TensorId | DType | Shape

Op (tagged union, see MX01 §"Op enum" for the variants):
  varint op_tag | variant payload

Constant:
  TensorId | bytes
```

The tag-to-op mapping for V1 is fixed:

```
0x00 Neg          0x07 Add          0x0E ReduceSum     0x15 MatMul
0x01 Abs          0x08 Sub          0x0F ReduceMax     0x16 Equal
0x02 Sqrt         0x09 Mul          0x10 ReduceMean    0x17 Less
0x03 Exp          0x0A Div          0x11 Reshape       0x18 Greater
0x04 Log          0x0B Max          0x12 Transpose     0x19 Where
0x05 Tanh         0x0C Min          0x13 Broadcast     0x1A Cast
0x06 Recip        0x0D Pow                             0x1B Const
```

Tags `0x1C` and beyond are reserved for V2 ops.

## Message types

The protocol has two enums: `ExecutorRequest` (runtime → executor) and
`ExecutorResponse` (executor → runtime).  Plus an `ExecutorEvent` enum for
unsolicited push messages (heartbeats, lost-buffer notifications).

```rust
pub enum ExecutorRequest {
    /// Executor announces itself to the runtime registry.
    Register {
        protocol_version: u32,        // 1 in V1
        executor_kind:    String,     // "cpu", "metal", "cuda", "vulkan", ...
        profile:          BackendProfile,
    },

    /// Compile a kernel from source.  The executor caches by hash.
    PrepareKernel {
        kernel_id:  KernelId,         // assigned by runtime
        source:     KernelSource,
    },

    /// Allocate a buffer.  The executor returns the BufferId in the response.
    AllocBuffer {
        bytes: u64,
    },

    /// Upload bytes from runtime memory into an allocated buffer.
    UploadBuffer {
        buffer:  BufferId,
        offset:  u64,
        data:    Vec<u8>,
    },

    /// Run a placed graph (or graph slice) end-to-end on this executor.
    /// All inputs must already be resident; all outputs will be left
    /// resident on this executor unless the graph itself contains
    /// download transfers.
    Dispatch {
        job_id:  u64,
        graph:   compute_ir::ComputeGraph,
    },

    /// Read bytes from a buffer back into runtime memory.
    DownloadBuffer {
        buffer: BufferId,
        offset: u64,
        len:    u64,
    },

    /// Release a buffer.  After this, the BufferId is invalid.
    FreeBuffer {
        buffer: BufferId,
    },

    /// Cancel an in-flight job.  Best-effort.
    CancelJob {
        job_id: u64,
    },

    /// Liveness probe.
    Heartbeat,

    /// Graceful shutdown — executor flushes outstanding work and
    /// stops accepting new requests.
    Shutdown,

    /// (v0.2 / protocol v2)  Dispatch a previously-emitted **specialised
    /// kernel** by handle.  The handle was returned by a `Specialiser`
    /// at compile time (see MX05 §"SpecRouter") and is opaque to the
    /// runtime — the executor owns the per-handle table.
    ///
    /// Wire tag: `0x0A`.  Backends that recognise the variant but have
    /// not yet wired execution reply with
    /// `Error { code: NOT_IMPLEMENTED, .. }`.
    DispatchSpecialised {
        job_id:  u64,
        handle:  u64,
        inputs:  Vec<BufferId>,
        outputs: Vec<BufferId>,
    },
}

pub enum ExecutorResponse {
    Registered     { executor_id: ExecutorId },
    KernelReady    { kernel_id:   KernelId },
    BufferAllocated{ buffer:      BufferId },
    BufferUploaded { buffer:      BufferId },
    DispatchDone   { job_id:      u64, timings: Vec<OpTiming> },
    BufferData     { buffer:      BufferId, data: Vec<u8> },
    BufferFreed,
    Cancelled      { job_id:      u64 },
    Alive          { profile:     BackendProfile },
    ShuttingDown,
    Error          { code: ErrorCode, message: String, job_id: Option<u64> },
}

pub enum ExecutorEvent {
    /// Executor lost a buffer (out-of-memory, device reset).  The runtime
    /// must drop its residency tracking for this BufferId.
    BufferLost { buffer: BufferId, reason: String },

    /// Executor's profile changed (e.g. a different process started
    /// using the GPU).  The runtime should re-evaluate.
    ProfileUpdated { profile: BackendProfile },

    /// Executor is going away.
    ShuttingDown,
}

pub struct OpTiming {
    pub op_index: u32,
    pub ns:       u64,
}
```

`KernelSource` is a tagged union over the per-backend source strings:

```rust
pub enum KernelSource {
    Msl       { code: String, entry: String },   // Metal
    CudaC     { code: String, entry: String },   // CUDA
    Glsl      { code: String, entry: String },   // OpenGL / Vulkan-via-glslangValidator
    SpirV     { bytes: Vec<u8>, entry: String }, // Vulkan / WebGPU
    Wgsl      { code: String, entry: String },   // WebGPU
    OpenClC   { code: String, entry: String },   // OpenCL
    Native    { backend: String, blob: Vec<u8> },// catch-all for ASIC IR / proprietary
}
```

`Native` is the escape hatch for backends whose source language doesn't
fit the named variants.  V1 only ever produces `Msl`, `CudaC`, and
(for the CPU executor) emits no kernels at all — its dispatch is direct
Rust function calls per op.

`BackendProfile` is defined in MX04 §"BackendProfile"; for the wire
format it is a struct of `u32`/`u64`/`String` fields.

`ErrorCode` is a `u16` enum; values `0x00–0x7F` reserved for protocol
errors, `0x80–0xFF` reserved for executor-specific.

## Wire layout: top-level frame

Every protocol message — request, response, or event — is wrapped in
the same outer frame:

```
Frame:
  u8           format_version       // 0x01 in V1
  u8           message_kind         // 0=Request, 1=Response, 2=Event
  u64          correlation_id       // request id; responses echo it; events use 0
  uv64         payload_length
  payload_length raw bytes:
    varint     variant_tag
    variant payload
```

Why a frame:

- **Versioning at the byte level.**  A V2 reader that sees a V3 frame
  errors immediately rather than misinterpreting the payload.
- **Multiplexing.**  A single transport carries multiple in-flight
  requests; correlation ids match responses to their requests.
- **Self-delimitation.**  `payload_length` lets a stream-oriented
  transport (TCP, Unix socket) frame messages without another layer.

## The `Transport` trait

```rust
#[async_trait_internal]
pub trait Transport: Send + Sync {
    /// Send a request, await its response.  Correlation ids are managed
    /// inside the Transport.
    async fn request(&self, req: ExecutorRequest) -> Result<ExecutorResponse, TransportError>;

    /// Subscribe to events from this transport's executor.  The runtime
    /// owns the event consumer and dispatches events to listeners.
    fn events(&self) -> EventStream;
}

pub type EventStream = Box<dyn AsyncIterator<Item = ExecutorEvent> + Send + Unpin>;
```

`async_trait_internal` is a hand-rolled in-crate macro (no external
dependency on `async-trait`) that desugars `async fn` in traits to a
boxed `Future`.

`AsyncIterator` is `core::async_iter::AsyncIterator` (when stabilised) or
a hand-rolled equivalent built on `core::future::Future` and `Pin`.  V1
ships the hand-rolled version and switches to the std trait when it
stabilises.

### `LocalTransport`

The only V1-shipped transport.  Implementation:

- Holds a direct reference to an executor's request handler — `fn handle(req: ExecutorRequest) -> ExecutorResponse`.
- `request()` calls `handle()` synchronously and returns a ready future.
- In **debug builds**: serialises the request to bytes, deserialises,
  invokes `handle()`, then serialises and deserialises the response.
  This catches "I accidentally put a non-serializable type in the
  protocol" bugs immediately.
- In **release builds**: skips the round-trip.

Tests in CI run debug builds, so the discipline is exercised on every PR.

### Future transports (designed for, not shipped in V1)

- **`UnixSocketTransport`** — frames over a Unix domain socket.  Uses
  `std::os::unix::net::UnixStream`.  Length-prefixed framing per
  §"Wire layout".
- **`TcpTransport`** — frames over a TCP connection.  Same framing.
- **`ZmqTransport`** — assumes `REQ`/`REP` or `DEALER`/`ROUTER` on the
  underlying socket.  Each protocol frame is one ZMQ message.
- **`NatsTransport`** — pub/sub.  Requests publish to `dispatch.<executor>`
  subjects; responses publish to ephemeral reply subjects.  Enables
  work-stealing setups with many subscribers.

Each future transport is its own crate (`matrix-transport-unix`,
`matrix-transport-tcp`, …) with its own zero-dep stack.  None of them
land in V1; this spec lists them so the protocol design accommodates
them.

## Buffer transport policy

V1 buffer transport is **always through the protocol**:

- A 4-MiB tensor takes a `UploadBuffer` request with 4 MiB of `data`.
  Locally, that bytes vector is moved (no copy in release builds; one
  copy in debug builds for the round-trip test).
- The same path is exercised by network transports unchanged.

V2 may add a zero-copy fast path for `LocalTransport` that hands raw
buffer pointers across without going through `Vec<u8>`.  V1 prefers
uniform code paths so the local executor exercises the same protocol
that remote executors will.

## Kernel cache

The executor caches compiled kernels by content hash.  Cache key:
SipHash (`std::collections::hash_map::DefaultHasher`) of `KernelSource`
bytes.  Hits skip compilation.

When the runtime issues `PrepareKernel` for a kernel the executor
already has cached, the executor still returns `KernelReady` quickly
without re-compiling.  This is transparent to the runtime.

## Error model

Errors propagate from executor to runtime as `ExecutorResponse::Error
{ code, message, job_id }`.  Categories:

- **Protocol errors** (codes `0x00–0x1F`): malformed frame, unknown
  variant, unsupported version.  Always fatal for the connection.
- **Resource errors** (`0x20–0x3F`): out-of-memory, device lost.  May
  be recoverable; the runtime decides.
- **Compilation errors** (`0x40–0x5F`): kernel source rejected.  Does
  not invalidate other state.
- **Runtime errors** (`0x60–0x7F`): kernel produced NaN, dispatch
  exceeded timeout.  Per-job, does not invalidate the executor.  This
  band also holds **`NOT_IMPLEMENTED` (`0x0062`)**, added in protocol
  v2: the request shape is recognised but this backend has not yet
  wired execution.  Distinct from the protocol-band `UNKNOWN_VARIANT`
  (which means the backend doesn't even know the tag).
- **Executor-specific** (`0x80–0xFF`): backend-defined.

Every error carries a UTF-8 `message` for diagnostics.  Messages are
not security-sensitive; they may contain file names and line numbers.

## Test methodology

`executor-protocol` ships:

1. **Wire round-trip** — every variant of every enum is encoded,
   decoded, and asserted equal.  Generated by a macro that walks the
   variants exhaustively; missing a variant is a compile error.
2. **Truncation tests** — every wire form is asserted to fail cleanly
   when the input is truncated by 1 byte at every position.  No panics,
   no UB, only `Err(TruncatedMessage)`.
3. **Forward-compat tests** — frames with `format_version` 2 are
   asserted to fail with `UnsupportedVersion` rather than misinterpret.
4. **Frame fuzzing** — a fixed-seed PRNG generates 100k random byte
   strings and asserts the decoder never panics, only returns `Err`.
5. **Local transport** — round-trip asserts that every supported
   request type produces the expected response when the executor is a
   trivial echo implementation.

Coverage target: **100%** of the public API.

## Out of scope (V1)

- **Compression.**  Buffers are sent raw.  V2 may add `Compressed { codec, bytes }`.
- **Encryption / authentication.**  Local transport doesn't need it.
  V2 transports that cross trust boundaries add it at their layer.
- **Streaming buffers.**  Large buffers are sent as one message.  V2
  may add `BufferChunk` for very large tensors.
- **Bidirectional flow control.**  V1 has no backpressure.  V2 transports
  add it at their layer.

## Open questions

1. Should we have a `Ping` separate from `Heartbeat`?  They overlap.
   V1 has only `Heartbeat` (returns `Alive { profile }`).
2. Should `Error` carry a structured `details: bytes` field as well as
   the human message?  Useful for richer error reporting.  V1 says no;
   V2 may add it.
3. Should `OpTiming` be optional in `DispatchDone`?  Some backends can't
   measure per-op time without overhead.  V1 leaves it always present
   but allows `ns: 0` to mean "unmeasured".

## Protocol version history

### v1 — initial release (executor-protocol 0.1.0)

Everything described above through the V1 lens: 10 request variants,
11 response variants, 3 event variants.  `PROTOCOL_VERSION = 1`.

### v2 — MX05 Phase 4.1 protocol surface (executor-protocol 0.2.0)

Adds the wire-format support for routing pre-emitted **specialised
kernels** (see MX05 §"SpecRouter") through the same `Transport` that
already carries every other request.  `PROTOCOL_VERSION = 2`.

**Forward-compatible with v1 senders.**  Every existing variant still
encodes/decodes byte-identically.  A v1-only executor receiving the new
request would fail with `UNKNOWN_VARIANT` (existing behaviour for any
unknown tag); a v2 executor receiving a v1 request stream decodes it
unchanged.

Added:

- `ExecutorRequest::DispatchSpecialised { job_id, handle, inputs,
  outputs }` — wire tag `0x0A`.

  Wire layout of the payload (after the variant tag):

  ```
  u64           job_id
  u64           handle
  u32           n_inputs
  n_inputs ×    u64 buffer_id
  u32           n_outputs
  n_outputs ×   u64 buffer_id
  ```

  Reply on success: `ExecutorResponse::DispatchDone { job_id, timings }`
  (existing variant, unchanged).  Reply when execution isn't wired yet:
  `ExecutorResponse::Error { code: NOT_IMPLEMENTED, .. }`.

- `ErrorCode::NOT_IMPLEMENTED = 0x0062` — soft refusal in the
  runtime-errors band.  Means "I recognise the request shape, the
  request is well-formed, but I have not yet wired execution for it."
  Distinct from `UNKNOWN_VARIANT` (which is a protocol-band hard error).

  V2 backends ship behaviour:
  - `matrix-cpu` 0.x — replies `NOT_IMPLEMENTED` for `DispatchSpecialised`.
    Phase 4.1 (next) installs a per-handle closure table so this returns
    `DispatchDone`.
  - `matrix-metal` 0.x — same, until Phase 4.2 lands the MSL emitter.

Why a new tag rather than overloading `Dispatch`:

`Dispatch` carries a full `compute_ir::ComputeGraph`; `DispatchSpecialised`
carries a single opaque `u64` handle plus its buffer arguments.  The
two payload shapes are disjoint enough that overloading would force
every reader to peek at sub-fields to disambiguate.  Separate tags keep
the wire format flat and the decode path branchless.

Reserved tags `0x0B`+ for future MX05 phases (e.g. specialised kernel
unload, specialised kernel re-emit).
