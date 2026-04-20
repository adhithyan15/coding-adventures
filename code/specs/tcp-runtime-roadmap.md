# TCP Runtime Roadmap

## Overview

The repository now has the first real pieces of a native evented transport
stack:

- raw native backends: `epoll`, `kqueue`, `iocp`
- a generic substrate: `native-event-core`
- a proof crate: `tcp-reactor`

That is a strong start, but it is not yet the whole thing we need for a
language-agnostic, safe, high-performance TCP server runtime.

This document answers three questions:

1. What should the operating system's TCP/IP stack continue to own?
2. What must the repository's Rust runtime still build on top of that?
3. How should Ruby, Python, Perl, and other C-FFI languages consume it without
   paying per-event interpreter overhead?

The core idea is:

- we are **not** reimplementing the kernel TCP/IP stack
- we **are** building a reusable user-space TCP server runtime on top of the OS
  stack
- that runtime should present one stable C ABI so any language with a C FFI can
  use it

---

## Current State

Today the stack looks like this:

```text
language binding / protocol runtime / UI runtime
    ↓
tcp-reactor
    ↓
native-event-core
    ↓
epoll / kqueue / iocp
```

What exists today:

- registration and polling against real native backends
- a generic token-based event abstraction
- a small nonblocking TCP echo-style server runtime
- connection caps and pending-write caps for basic resource defense

What is still true:

- `tcp-reactor` is still a single-threaded proof layer
- it is transport-specific rather than the final reusable stream substrate
- there is no stable C ABI yet
- there are no language bindings yet
- there is no benchmarked production tuning story yet

So the honest summary is:

> We have built a real reactor-based TCP foundation, not a dummy event loop, but
> we have not yet completed the full cross-language TCP runtime.

---

## Responsibility Boundary

The most important design choice is deciding what belongs to the kernel and what
belongs to our runtime.

### The OS TCP/IP stack owns

The repository should rely on the operating system for:

- IP routing
- ARP / ND and link-layer neighbor discovery
- SYN / ACK / FIN / RST packet handling
- retransmission
- congestion control
- RTT estimation
- packet segmentation and reassembly
- receive ordering and duplicate suppression
- checksum handling
- keepalive probe mechanics when socket keepalive is enabled
- kernel listen backlog mechanics

If we try to rebuild these in user space, we are no longer building a portable
TCP server runtime. We are building a kernel bypass or user-space network stack,
which is a different project entirely.

### Our runtime owns

The repository runtime should own:

- socket creation and configuration policy
- nonblocking accept, read, write, and close orchestration
- event-loop integration
- connection state machines
- buffering and backpressure
- timeouts and deadlines
- fairness and scheduling policy
- resource accounting and caps
- telemetry, metrics, and tracing hooks
- graceful shutdown policy
- C ABI stability
- language binding integration

This is the missing user-space layer we still need to build.

---

## The Platform Seam

There is one more boundary that matters beyond "kernel versus runtime":

- the seam between the repository's transport runtime and the provider that
  offers eventing, timers, wakeups, listeners, sockets, and packet delivery

Today that provider is the host operating system through:

- `epoll`, `kqueue`, or IOCP for eventing
- kernel TCP sockets for networking
- kernel timers and wakeups

In the future, that provider may instead be:

- a library OS
- a unikernel networking layer
- a kernel-bypass transport substrate

If we want today's user-space TCP runtime to survive that future intact, then
our transport crates must not be welded directly to Unix or Win32 details above
the raw backend boundary.

### The seam should expose capabilities, not operating-system trivia

The runtime-facing contract should describe capabilities like:

- create or adopt a listener
- accept a connection
- read bytes
- write bytes
- close or half-close
- register interest in read, write, wakeup, and timer events
- create timers and wakeups
- surface transport errors and readiness or completion events

The runtime-facing contract should not require upper layers to understand:

- raw file descriptor numbers
- `SOCKET` versus fd distinctions
- `OVERLAPPED` structures
- `kevent` filter flags
- `epoll_event` bit layout
- host-kernel-specific timer objects

That means the long-term goal is not simply:

- `tcp-runtime` over `std::net`

It is:

- `tcp-runtime` over a repository-owned platform contract

### Recommended layering for that seam

The stack should evolve toward:

```text
language binding / protocol runtime
    ↓
tcp-runtime-c
    ↓
tcp-runtime
    ↓
stream-reactor
    ↓
transport-platform
    ↓
native-event-core + native socket providers
    or
library-OS / unikernel provider
```

Where:

- `transport-platform` defines the runtime-facing contract
- OS-backed adapters implement that contract on top of `native-event-core` and
  native sockets
- a future library-OS adapter can implement the same contract without changing
  `stream-reactor` or `tcp-runtime`

### What belongs below the seam

Below the seam:

- socket or transport-handle representation
- event delivery mechanics
- timer primitives
- wakeup primitives
- listener creation and transport-handle provisioning
- platform-specific zero-copy or vectored-I/O hooks

### What belongs above the seam

Above the seam:

- connection state machines
- buffering policy
- backpressure policy
- fairness policy
- idle and write deadlines
- graceful shutdown and drain policy
- metrics and tracing
- C ABI and language bindings

If we keep this split disciplined, then the future unikernel experiment becomes:

- "write a new platform provider"

instead of:

- "rewrite the TCP runtime from scratch"

---

## Missing Pieces

The current stack is missing work in six major areas.

### 1. A reusable byte-stream substrate

Right now `tcp-reactor` proves the event core with a concrete TCP echo-style
loop. The next reusable layer should be:

- `transport-platform`
- `stream-reactor`

Responsibilities:

- `transport-platform`
  - defines the provider contract for listeners, streams, timers, wakeups, and
    event delivery
  - hides OS-specific handle types from upper layers
  - creates the seam for a future library-OS or unikernel backend
- normalized readable / writable / closed / error events
- per-connection read buffer management
- per-connection write queue management
- half-close tracking
- deadline bookkeeping
- generic stream callbacks or completion delivery

Why it matters:

- WebSockets, TLS, plain TCP services, and protocol framers all want "managed
  byte streams"
- this keeps `tcp-reactor` from becoming a grab bag of every future transport
  concern
- this preserves a clean portability boundary for a future library-OS provider

### 2. A real TCP server runtime

Above `stream-reactor`, the repository should grow a real `tcp-runtime` crate.

Responsibilities:

- listener lifecycle and listener groups
- bind / listen configuration
- accept loop strategy
- socket option policy
- connection admission control
- connection registry
- graceful connection draining
- listener pause / resume
- shutdown orchestration

Missing features relative to that goal:

- configurable backlog policy
- socket options such as `SO_REUSEADDR`, `SO_REUSEPORT` where appropriate,
  `TCP_NODELAY`, keepalive policy, buffer sizing, and linger policy
- per-listener and per-connection statistics
- explicit error taxonomy rather than ad hoc close behavior
- drain mode for controlled shutdowns

### 3. Timers, wakeups, and cancellation

Transport runtimes are not only about fd readiness.

Missing pieces:

- a wakeup mechanism so another thread can immediately nudge the loop
- timer integration for idle timeouts, write deadlines, handshake deadlines, and
  drain deadlines
- cancellation handles for pending operations

Recommended direction:

- Linux: `eventfd` plus `timerfd`
- BSD/macOS: `EVFILT_USER` plus `EVFILT_TIMER`
- Windows: event object or completion-post wakeups plus waitable timers or a
  timer queue translated into the event substrate

Without this, the runtime can serve traffic, but it cannot yet behave like a
well-controlled transport engine.

### 4. Multi-core scaling

The current `tcp-reactor` proves correctness and architecture, but a serious
runtime needs a scaling story.

Missing pieces:

- one-reactor-per-core or one-reactor-per-shard architecture
- listener sharding policy
- connection affinity policy
- cross-thread wakeups
- lock-minimal metrics and connection handoff paths

The key decision:

- keep each connection owned by exactly one reactor thread
- communicate across reactors with explicit wakeups and queues, not shared
  mutable connection state

### 5. Hardening, observability, and benchmarking

A reusable runtime needs more than passing tests.

Missing pieces:

- metrics for accepts, closes, bytes read, bytes written, queue depth, and
  timeout counts
- tracing hooks for connection lifecycle events
- slow-client and write-pressure observability
- benchmark harnesses for concurrent connections, idle connections, echo load,
  and write-heavy load
- regression tests for resource exhaustion and fairness under load

This is how we stop calling it "high-performance" aspirationally and start
measuring it.

The shared benchmarking design now lives in
[`benchmarking-tools.md`](benchmarking-tools.md). TCP runtime benchmarks should
use that repo-wide harness instead of ad hoc scripts so results can separate
connect latency, frame latency, throughput, correctness, and statistical
confidence.

### 6. A stable FFI and binding model

This is the piece that makes the whole effort useful to Ruby, Python, Perl, and
other ecosystems.

Missing pieces:

- a dedicated `tcp-runtime-c` crate exposing a small stable C ABI
- opaque handles for runtimes, listeners, and connections
- explicit ownership and lifetime rules
- copy-in / copy-out or borrowed-buffer APIs with a documented safety contract
- callback and completion delivery design
- binding crates or native extensions per language

The language bindings should not link directly against `epoll`, `kqueue`, or
`OVERLAPPED` details. They should bind to the repository-owned transport ABI.

---

## Detailed Gap Inventory

| Area | Current state | Missing work |
|---|---|---|
| Accept path | single listener, single-threaded accept loop | listener groups, admission policy, backlog tuning, pause/resume, drain mode |
| Read path | nonblocking read and handler callback | read quotas, buffer reuse, framed delivery helpers, deadline checks |
| Write path | pending-write cap and nonblocking flush | vectored I/O, flush fairness, corking/batching policy, high/low watermarks |
| Connection lifecycle | open, read, write, close | half-close states, idle timeout, protocol handoff, structured close reasons |
| Scheduling | one event loop thread | sharding, cross-thread wakeups, reactor ownership rules |
| Socket policy | minimal | keepalive, `TCP_NODELAY`, reuse policy, linger policy, DSCP/TOS hooks if needed |
| Error model | mostly implicit close behavior | typed transport errors and stable ABI error codes |
| Timers | effectively absent | idle timers, operation deadlines, scheduled wakeups |
| Observability | tests only | counters, histograms, trace hooks, benchmark suite |
| FFI | absent | C ABI, language bindings, memory ownership rules, callback discipline |
| Platform seam | upper layers still conceptually OS-backed | provider trait or contract for sockets, listeners, timers, wakeups, and event delivery |

---

## Language-Bridge Model

The Rust runtime should not be embedded as "a tiny callback helper". It should
be the hot path.

Recommended layering:

```text
Ruby / Python / Perl / other host language
    ↓
native extension or FFI wrapper
    ↓
tcp-runtime-c
    ↓
tcp-runtime
    ↓
stream-reactor
    ↓
transport-platform
    ↓
native-event-core + native socket providers
    or
library-OS / unikernel provider
```

### C ABI design rules

- keep the ABI handle-based, not struct-layout-based
- return explicit status codes and out-parameters
- avoid host-language object pointers crossing the ABI as the primary data path
- support polling or batched completion retrieval, not one callback per packet
- separate control-plane calls from data-plane calls

Examples of control-plane calls:

- create runtime
- create listener
- configure limits
- start or stop serving
- poll or retrieve batched events

Examples of data-plane calls:

- submit bytes to write
- drain accepted connections or receive buffers
- close a connection

This keeps the fastest path in Rust and minimizes interpreter crossings.

---

## GIL, GVL, and Native Extensions

### Short answer

A Rust native extension helps a lot, but it does **not** automatically sidestep
the GIL or GVL completely.

The hot path can live in Rust outside the interpreter lock. But the moment the
runtime touches Python or Ruby objects, or calls back into host-language code,
the interpreter lock rules apply again.

### Standard CPython

On standard CPython builds:

- code that touches Python objects or the Python/C API requires the GIL
- long-running native work can detach thread state and run without holding the
  GIL
- blocking I/O commonly releases the GIL

That means a Rust extension can absolutely run the event loop, socket I/O,
buffer management, and worker threads outside the GIL, as long as it does not
touch Python objects on that path.

But:

- Python callbacks
- Python object creation
- manipulating Python-owned buffers through the Python/C API

all pull you back into GIL-governed territory.

There is one important modern wrinkle:

- CPython 3.13 introduced an optional free-threaded build
- extensions must explicitly support it, or importing them can re-enable the
  GIL at runtime

So the safe design target is still:

- assume standard GIL-constrained CPython first
- structure the ABI so the data plane stays in Rust either way

### CRuby / MRI

On CRuby:

- Ruby execution is governed by the GVL
- C extensions can release it with `rb_thread_call_without_gvl`
- touching Ruby objects or most Ruby C APIs requires reacquiring the GVL

That means a Rust-backed native extension can run network I/O, polling, timers,
and buffer work outside the GVL, but Ruby-level callbacks and object work still
need the lock.

Ractors help with parallel Ruby execution across ractors, but that does not
remove the need for careful native-extension boundaries.

### Practical consequence

For Ruby, Python, and similar runtimes:

- do **not** design a callback-per-read or callback-per-write API
- do **not** surface every native event directly to the interpreter
- do keep accept/read/write/state/timer logic in Rust
- do return batched completions, handles, and byte buffers through a narrow API

If we follow that rule, the extension avoids most interpreter-lock overhead on
the hot path without pretending the lock does not exist.

---

## Recommended Crate Roadmap

### Phase 1: finish the substrate

Add or expand:

- `transport-platform`
- `eventfd`
- `timerfd`
- `signalfd`
- richer `iocp` completion modeling
- `stream-reactor`

Target outcome:

- one generic byte-stream engine above a repository-owned provider seam rather
  than directly above host-OS details

### Phase 2: build the real transport runtime

Add:

- `tcp-runtime`

Target outcome:

- listener management
- connection deadlines
- backpressure policy
- structured lifecycle events
- drain and shutdown control

### Phase 3: stabilize the ABI

Add:

- `tcp-runtime-c`

Target outcome:

- stable C ABI
- opaque runtime and connection handles
- batched event delivery
- documented memory ownership rules

### Phase 4: ship host-language bindings

Add:

- Python extension
- Ruby extension
- Perl bridge
- later Node / Lua / Erlang bridges as needed

Target outcome:

- one repository-owned transport engine shared across languages

### Phase 5: layer protocols on top

Add later:

- TLS stream adapter
- WebSocket runtime
- protocol framers
- HTTP or RPC runtimes

These belong above the TCP runtime, not inside it.

---

## What "Done" Looks Like

The transport foundation is not "done" when it can echo bytes.

It is done when:

- many concurrent connections can be served reliably
- idle, slow, and abusive peers are bounded by policy
- timers and shutdowns are first-class
- multiple reactor threads can scale across cores
- the runtime exposes stable metrics and lifecycle events
- the runtime depends on a repository-owned platform contract rather than being
  permanently coupled to a particular host-kernel socket API
- a C ABI exists
- Ruby, Python, Perl, and other languages can use it without the interpreter
  sitting in the middle of every socket event

That is the target this roadmap is aiming at.

---

## References

- [CPython thread state and GIL docs](https://docs.python.org/3/c-api/init.html)
- [CPython free-threaded extension support](https://docs.python.org/3/howto/free-threading-extensions.html)
- [Ruby extension threading docs](https://docs.ruby-lang.org/en/master/extension_rdoc.html)
- [Ruby C API for `rb_thread_call_without_gvl`](https://docs.ruby-lang.org/capi/en/master/d6/dfb/include_2ruby_2thread_8h.html)
