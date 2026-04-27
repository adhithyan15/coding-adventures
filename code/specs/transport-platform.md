# transport-platform

## Overview

`transport-platform` is the seam between the repository's higher-level
transport runtimes and the provider that actually offers networking, eventing,
timers, and wakeups.

It exists so that crates like `stream-reactor`, `tcp-runtime`, Redis servers,
IRC servers, and future protocol stacks can depend on one repository-owned
contract instead of being coupled directly to:

- Unix file descriptors
- Win32 socket handles
- `epoll_event`
- `kevent`
- IOCP completion records
- host-kernel timer and wakeup quirks

The first implementation target is host-OS-backed networking over native TCP
sockets and native eventing APIs. The long-term design target is broader: the
same `transport-platform` contract should be implementable by:

- Linux / BSD / macOS / Windows socket stacks
- a future library OS networking layer
- a future unikernel eventing and transport substrate

That is why this crate matters so much. It is the portability boundary that
lets the user-space transport runtime survive backend changes.

---

## Why This Exists

The repository already has:

- `epoll`, `kqueue`, and `iocp`
- `native-event-core`
- a proof TCP loop in `tcp-reactor`

That stack proves the native eventing side, but it still leaves upper layers
too close to operating-system details. If we build `stream-reactor` or
`tcp-runtime` directly on raw sockets and native backend APIs, then:

- Redis will get a working server, but the runtime seam will be wrong
- IRC will get a faster loop, but the stable network interface will drift
- future language bindings will inherit OS-specific assumptions
- a later library-OS or unikernel effort will force a rewrite

`transport-platform` fixes that by owning the platform contract explicitly.

---

## Consumers

This crate should be designed against at least three concrete consumers.

### 1. `stream-reactor`

Needs:

- accepted streams
- readable and writable events
- close and error events
- timers for deadlines
- wakeups for cross-thread nudges and shutdown

### 2. `tcp-runtime`

Needs:

- listener creation and lifecycle
- accept loops
- socket option configuration
- half-close and full-close operations
- backpressure-friendly write operations

### 3. Application servers such as Redis and IRC

Redis-like needs:

- many mostly idle connections
- low-latency request/response
- strong write-pressure handling
- predictable shutdown

IRC-like needs:

- long-lived connections
- line-oriented framed traffic above the transport layer
- fanout writes to many connections
- graceful disconnect handling

These consumers are intentionally different. If one seam serves all three
well, it is probably the right seam.

---

## Layer Position

```text
Redis / IRC / WebSocket / future protocols
    ↓
tcp-runtime
    ↓
stream-reactor
    ↓
transport-platform
    ↓
native socket + event providers
    or
library-OS / unikernel provider
```

`transport-platform` is below connection state machines and buffering policy,
but above raw poller details.

One implementation note matters for the current codebase:

- the public seam is the stable goal
- the current providers talk directly to raw backends like `kqueue`, `epoll`,
  and `WSAPoll` while lower layers continue maturing

That does not change the intended layering above the seam. It only keeps the
first implementation practical while the lower native crates continue maturing.

---

## Implementation Status

The current crate ships three host-OS providers:

- macOS / BSD: `KqueueTransportPlatform`
- Linux: `EpollTransportPlatform`
- Windows: `WindowsTransportPlatform`

The Linux provider is the phase-one design target for high-performance host
servers:

- `epoll` for readiness delivery
- `timerfd` for deadline events
- `eventfd` for explicit wakeups

The BSD/macOS provider uses:

- `kqueue` read and write filters for sockets
- `EVFILT_TIMER` for timers
- `EVFILT_USER` for wakeups

The Windows provider is intentionally a seam-preserving phase one rather than
the final Windows story:

- nonblocking Winsock sockets
- `WSAPoll` for readiness
- loopback socket pairs for wakeups
- user-space timer bookkeeping

The future Windows end-state should still be an IOCP-backed provider. The point
of the current implementation is to let the seam, tests, and higher layers land
now instead of blocking until the richer completion backend is ready.

---

## Design Goals

`transport-platform` should:

- expose capabilities, not operating-system trivia
- support TCP listeners and TCP streams first
- include timers and wakeups as first-class primitives
- support both readiness-style and completion-style backends
- preserve enough detail for performance-sensitive runtimes
- avoid forcing upper layers to traffic in raw fds or OS structs
- make ownership and lifecycle explicit
- be implementable atop host OS sockets today
- be implementable atop a future library OS later

## Non-Goals

`transport-platform` should not:

- parse protocols
- know about RESP, IRC, HTTP, WebSocket, or TLS framing
- own connection buffering policy
- expose a C ABI directly
- pretend every backend has identical semantics
- model every possible network transport on day one

Initial scope is:

- TCP listeners
- TCP streams
- timers
- wakeups
- event delivery

Future transports such as UDP, Unix domain sockets, or kernel-bypass rings can
be added later if the core seam holds up.

---

## Responsibility Boundary

### What belongs below `transport-platform`

Below this seam:

- native socket creation or adoption
- accept mechanics
- read and write syscalls or completion submission
- event registration and event retrieval
- timer primitives
- wakeup primitives
- platform-specific socket configuration plumbing
- platform-specific vectored-I/O or zero-copy hooks

### What belongs above `transport-platform`

Above this seam:

- connection registries
- read and write buffering
- framing
- deadlines and idle policy
- fairness policy
- write watermarks
- graceful drain behavior
- metrics aggregation
- application protocol logic

This split is deliberate: a future library OS should only need to replace what
is below the seam.

---

## Core Concepts

### Opaque resource identifiers

Upper layers should identify resources with repository-owned opaque IDs, not raw
OS handles:

- `ListenerId`
- `StreamId`
- `TimerId`
- `WakeupId`

The provider may internally map those to:

- fds
- `SOCKET` handles
- IOCP state records
- library-OS connection objects

But the upper layers should not depend on that representation.

### Provider-owned event delivery

The platform provider owns the mechanics of waiting for:

- accept readiness or accept completion
- stream readability or receive completion
- stream writability or send completion
- close or hangup conditions
- timer expiry
- explicit wakeups

Upper layers should receive normalized events.

### Explicit interest management

Upper layers need control over which resources want:

- read events
- write events
- timer events

This is critical for backpressure and fairness. A stream with nothing queued for
write should not keep asking for writable notifications.

### Capability reporting

Not every backend will have identical features. The provider should report
capabilities such as:

- half-close support
- vectored-write support
- zero-copy send support
- one-shot eventing support
- native timer support
- native wakeup support

Upper layers can then optimize when available without baking those assumptions
into the contract itself.

---

## Public API Shape

The exact Rust syntax can evolve during implementation, but the contract should
look roughly like this.

### Core IDs and capabilities

```rust
pub struct ListenerId(u64);
pub struct StreamId(u64);
pub struct TimerId(u64);
pub struct WakeupId(u64);

pub struct PlatformCapabilities {
    pub supports_half_close: bool,
    pub supports_vectored_write: bool,
    pub supports_zero_copy_send: bool,
    pub supports_native_timers: bool,
    pub supports_native_wakeups: bool,
}
```

### Interest model

```rust
pub struct StreamInterest {
    pub readable: bool,
    pub writable: bool,
}
```

### Endpoint and configuration

```rust
pub enum BindAddress {
    Ip(std::net::SocketAddr),
}

pub struct ListenerOptions {
    pub backlog: u32,
    pub reuse_address: bool,
    pub reuse_port: bool,
    pub nodelay_default: bool,
    pub keepalive_default: Option<std::time::Duration>,
}

pub struct StreamOptions {
    pub nodelay: Option<bool>,
    pub keepalive: Option<std::time::Duration>,
    pub recv_buffer_size: Option<usize>,
    pub send_buffer_size: Option<usize>,
}
```

### Event model

```rust
pub enum PlatformEvent {
    ListenerAcceptReady { listener: ListenerId },
    StreamReadable { stream: StreamId },
    StreamWritable { stream: StreamId },
    StreamClosed { stream: StreamId, kind: CloseKind },
    TimerExpired { timer: TimerId },
    Wakeup { wakeup: WakeupId },
    Error { resource: ResourceId, error: PlatformError },
}

pub enum ResourceId {
    Listener(ListenerId),
    Stream(StreamId),
    Timer(TimerId),
    Wakeup(WakeupId),
}

pub enum CloseKind {
    ReadClosed,
    WriteClosed,
    FullyClosed,
    Reset,
}
```

### Provider trait

```rust
pub trait TransportPlatform {
    fn capabilities(&self) -> PlatformCapabilities;

    fn bind_listener(
        &mut self,
        address: BindAddress,
        options: ListenerOptions,
    ) -> Result<ListenerId, PlatformError>;

    fn local_addr(&self, listener: ListenerId) -> Result<std::net::SocketAddr, PlatformError>;

    fn set_listener_interest(
        &mut self,
        listener: ListenerId,
        readable: bool,
    ) -> Result<(), PlatformError>;

    fn accept(&mut self, listener: ListenerId) -> Result<Option<AcceptedStream>, PlatformError>;

    fn configure_stream(
        &mut self,
        stream: StreamId,
        options: StreamOptions,
    ) -> Result<(), PlatformError>;

    fn set_stream_interest(
        &mut self,
        stream: StreamId,
        interest: StreamInterest,
    ) -> Result<(), PlatformError>;

    fn read(&mut self, stream: StreamId, buffer: &mut [u8]) -> Result<ReadOutcome, PlatformError>;

    fn write(&mut self, stream: StreamId, buffer: &[u8]) -> Result<WriteOutcome, PlatformError>;

    fn write_vectored(
        &mut self,
        stream: StreamId,
        buffers: &[std::io::IoSlice<'_>],
    ) -> Result<WriteOutcome, PlatformError>;

    fn shutdown_read(&mut self, stream: StreamId) -> Result<(), PlatformError>;

    fn shutdown_write(&mut self, stream: StreamId) -> Result<(), PlatformError>;

    fn close_stream(&mut self, stream: StreamId) -> Result<(), PlatformError>;

    fn close_listener(&mut self, listener: ListenerId) -> Result<(), PlatformError>;

    fn create_timer(&mut self) -> Result<TimerId, PlatformError>;

    fn arm_timer(
        &mut self,
        timer: TimerId,
        deadline: std::time::Instant,
    ) -> Result<(), PlatformError>;

    fn disarm_timer(&mut self, timer: TimerId) -> Result<(), PlatformError>;

    fn create_wakeup(&mut self) -> Result<WakeupId, PlatformError>;

    fn wake(&mut self, wakeup: WakeupId) -> Result<(), PlatformError>;

    fn poll(
        &mut self,
        timeout: Option<std::time::Duration>,
        output: &mut Vec<PlatformEvent>,
    ) -> Result<(), PlatformError>;
}
```

### Operation outcomes

```rust
pub struct AcceptedStream {
    pub stream: StreamId,
    pub peer_addr: std::net::SocketAddr,
}

pub enum ReadOutcome {
    Read(usize),
    WouldBlock,
    Closed,
}

pub enum WriteOutcome {
    Wrote(usize),
    WouldBlock,
    Closed,
}
```

This shape is intentionally explicit:

- `poll()` delivers events
- `accept()`, `read()`, and `write()` perform operations
- operations can report `WouldBlock` without treating that as an error
- timers and wakeups are part of the same seam

---

## Why This Contract Works For Both Readiness And Completion Backends

Linux `epoll` and BSD `kqueue` are readiness-oriented. Windows IOCP is
completion-oriented. A future library OS may be either.

This contract absorbs the difference by keeping the upper layer focused on:

- "I was told this resource can make progress"
- "I attempted the operation"
- "Here is the result"

For readiness backends:

- `poll()` emits readable or writable events
- `read()` and `write()` call nonblocking operations directly

For completion backends:

- the provider may internally post operations ahead of time
- `poll()` emits normalized progress events when those operations complete
- `read()` and `write()` can become façade operations over provider-managed
  completion state

The important constraint is that upper layers still see one contract.

---

## Socket Configuration Surface

The first version should expose only the configuration needed by real server
workloads.

Must-have listener controls:

- backlog
- `SO_REUSEADDR`
- `SO_REUSEPORT` where available
- local address lookup

Must-have stream controls:

- `TCP_NODELAY`
- TCP keepalive
- send and receive buffer sizing

Must-have lifecycle controls:

- full close
- half-close read
- half-close write

Nice-to-have later:

- linger policy
- DSCP / traffic class
- TCP quick-ack or platform-specific tuning

---

## Event Semantics

The event model must be stable and boring.

### Rules

- a readiness event means "an operation may now make progress"
- a close event means "some part of the stream lifecycle has ended"
- an error event means "the resource encountered a transport or provider error"
- timer and wakeup events are first-class and do not need special side channels

### Upper-layer expectations

- spurious readiness is allowed
- duplicate readiness is allowed
- `WouldBlock` after a readiness event is allowed
- events must never reference unknown resources
- provider implementations must never emit events for closed and fully removed
  resources

These rules let the seam stay implementable on real kernels without pretending
they are perfectly deterministic.

---

## Resource Ownership

`transport-platform` must make lifecycle explicit.

### Creation

- the provider creates listeners, timers, and wakeups
- accepted streams are created by the provider and surfaced via `accept()`

### Closure

- upper layers explicitly close streams and listeners
- provider-owned resources must be detached from future polling once closed
- closing twice should be safe and idempotent where practical

### No leaked OS handles above the seam

Upper layers should never assume:

- they can duplicate a raw fd
- they can call `fcntl`, `setsockopt`, or `shutdown` themselves
- they can manipulate event registration directly

That is part of what keeps the seam portable.

---

## Error Model

The provider needs a typed error surface.

```rust
pub enum PlatformError {
    Unsupported(&'static str),
    InvalidResource,
    ResourceClosed,
    AddressInUse,
    AddressNotAvailable,
    PermissionDenied,
    ConnectionRefused,
    ConnectionReset,
    BrokenPipe,
    TimedOut,
    Interrupted,
    ProviderFault(String),
}
```

Two principles matter:

- common transport conditions like reset, broken pipe, and timeout should be
  normalized
- `WouldBlock` should stay in operation outcomes, not get escalated to a hard
  error

That keeps upper-layer state machines simpler.

---

## Redis And IRC Fit

### Why Redis fits this seam

Redis wants:

- one or more listeners
- many client streams
- fast small reads and writes
- backpressure-friendly writable interest
- eventual deadlines and shutdown control

`transport-platform` gives Redis exactly the pieces it needs while leaving RESP
framing and command dispatch above the seam.

### Why IRC fits this seam

IRC wants:

- long-lived streams
- readable events for line framing
- fanout writes to many peers
- clean disconnect semantics
- stop and wakeup support

This seam lets the IRC stack keep its stable application-facing interfaces while
swapping the underlying transport implementation from threads to a single
reactor or later a sharded runtime.

---

## Implementation Plan

### Phase 1: Rust crate and OS-backed adapter

Build a new crate:

- `code/packages/rust/transport-platform`

First backend target:

- macOS `kqueue`, because it is locally testable right now

Likely implementation split:

- `transport-platform`
  - public IDs, events, options, errors, and the `TransportPlatform` trait
- `transport-platform-kqueue`
  - or an internal `kqueue` adapter module in the same crate for the first pass
- later Linux and Windows adapters

### Phase 2: integrate timers and wakeups

Add:

- `EVFILT_TIMER` and `EVFILT_USER` on macOS/BSD
- `timerfd` and `eventfd` on Linux
- timer plus wakeup primitives on Windows

### Phase 3: refactor the existing proof stack upward

Refactor:

- `tcp-reactor` into `stream-reactor` + `tcp-runtime`

New layering:

- `tcp-runtime` over `stream-reactor`
- `stream-reactor` over `transport-platform`
- `transport-platform` over native providers

### Phase 4: prepare the C ABI

Once the seam is stable:

- add `tcp-runtime-c`
- bind Ruby, Python, and Perl above that

---

## Test Strategy

The crate should be tested at three levels.

### 1. Trait-level fake provider tests

Verify:

- ID lifecycle
- event invariants
- timer and wakeup semantics
- close semantics
- operation outcome rules

### 2. Platform integration tests

On macOS/BSD first:

- bind listener and accept multiple concurrent clients
- read and write bytes through the provider
- writable-interest transitions
- timer expiry delivery
- wakeup delivery from another thread
- close and half-close behavior

### 3. Consumer-oriented smoke tests

Build thin tests showing:

- a Redis-like request/response loop can sit above the seam
- an IRC-like long-lived connection loop can sit above the seam

These tests should not re-test protocol logic. They should prove the seam is
good enough for the protocols we care about.

---

## What Success Looks Like

`transport-platform` is successful when:

- `stream-reactor` no longer depends on OS socket trivia
- `tcp-runtime` can be built entirely above this seam
- Redis and IRC can both consume that runtime without transport-specific hacks
- timers and wakeups are part of the normal runtime contract
- the same upper layers can later target a library-OS backend with minimal
  change

That is the contract we want to build next.
