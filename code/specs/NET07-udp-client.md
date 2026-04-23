# NET07 — UDP Client

## Overview

UDP (User Datagram Protocol) is the simplest transport protocol in the normal
internet stack. Unlike TCP, it does not create a connection, retransmit lost
packets, preserve ordering across multiple packets, or expose a byte stream. It
sends **datagrams**: one bounded message at a time.

This package specifies a small, OS-socket-backed UDP client abstraction for
application protocols such as DNS:

- bind a UDP socket to a local address and port
- send one datagram to a remote address
- receive one datagram plus the sender's address
- configure read/write timeouts
- keep payload bytes opaque

It deliberately does **not** parse DNS, RTP, QUIC, game packets, or any other
application protocol. UDP should be as transport-layer-only as TCP client
`NET01` is byte-stream-only.

**Analogy:** UDP is a postcard:

```
┌──────────────────────────────────────────────┐
│ To:   8.8.8.8:53                             │
│ From: 192.0.2.10:49152                       │
│                                              │
│ Payload: [opaque bytes]                      │
└──────────────────────────────────────────────┘

You drop it in the mail. It may arrive, arrive once, arrive later, or not
arrive at all. If the application needs retry behavior, the application owns
that policy.
```

## Where It Fits

```
future dns-client
     │
     ├── dns-message (NET06)
     │     build query bytes / parse response bytes
     │
     └── udp-client (NET07) ← THIS PACKAGE
           send and receive opaque datagrams
```

`NET07` is the real OS-socket counterpart to the simulated UDP layer described
inside `D17-network-stack.md`.

**Depends on:** nothing (std/socket APIs only)
**Depended on by:** future DNS client, future datagram protocols

---

## Concepts

### 1. Datagram, Not Stream

TCP gives callers a stream:

```
write("abc")
write("def")
read() -> maybe "abcdef", maybe "abc", maybe "ab"
```

UDP preserves message boundaries:

```
send_datagram("abc")
send_datagram("def")
recv_datagram() -> "abc"
recv_datagram() -> "def"
```

That property is exactly why classic DNS fits UDP well: a query is one datagram,
and a response is one datagram.

### 2. No Delivery Guarantees

UDP does not promise:

- delivery
- retry
- ordering
- deduplication
- congestion control
- flow control

The package should expose timeout errors cleanly, but it should not retry
automatically. A DNS client may decide to retry the same query ID, try another
resolver, or fall back to TCP. A game client may simply send the next state
update. The transport layer should not guess.

### 3. Connected vs Unconnected UDP

Operating systems allow two common UDP modes:

1. **Unconnected socket**
   - each send specifies a destination
   - each receive reports the sender
   - useful for servers and multi-peer clients

2. **Connected UDP socket**
   - records one default remote peer
   - send can omit destination
   - receive filters packets from other peers

The first version should support both, but the unconnected API is the most
important because DNS clients may switch resolvers and DNS servers receive from
many clients.

### 4. Payload Size

A UDP datagram has a finite size. IPv4 UDP length is 16 bits, so the absolute
wire-format maximum is 65,535 bytes including IP and UDP headers. In practice,
applications should send much smaller packets to avoid IP fragmentation.

This package should not impose DNS-specific limits. It should provide a
configurable receive-buffer size and return a clear truncation error if the
incoming datagram is larger than the caller's buffer.

### 5. Address Handling

The UDP layer should work with resolved socket addresses, not hostnames. DNS is
one of the protocols that resolves hostnames, so making UDP do DNS resolution
would create an awkward dependency loop.

Callers pass concrete socket addresses:

```
8.8.8.8:53
[2001:4860:4860::8888]:53
127.0.0.1:5353
```

If a higher-level caller starts with a hostname, that caller must resolve it
before constructing a UDP destination.

---

## Public API

The examples use Rust syntax as the reference implementation shape. Language
ports should expose the same concepts with idiomatic local names.

### Core Types

```rust
use std::net::SocketAddr;
use std::time::Duration;

pub struct UdpOptions {
    /// Optional local bind address. Defaults to an ephemeral port on all
    /// IPv4 interfaces for `UdpClient::bind`. The `send_and_receive`
    /// convenience helper may infer IPv4 vs IPv6 from its destination when no
    /// bind address is supplied.
    pub bind_addr: Option<SocketAddr>,

    /// Maximum number of bytes to allocate for one receive.
    /// Default: 65_535.
    pub max_datagram_size: usize,

    /// Read timeout applied to receive operations.
    pub read_timeout: Option<Duration>,

    /// Write timeout applied to send operations when the platform supports it.
    pub write_timeout: Option<Duration>,
}

pub struct UdpDatagram {
    pub source: SocketAddr,
    pub destination: SocketAddr,
    pub payload: Vec<u8>,
}

pub struct UdpClient {
    // language-specific socket handle
}
```

### Client API

```rust
impl UdpClient {
    /// Open a UDP socket with the configured options.
    pub fn bind(options: UdpOptions) -> Result<Self, UdpError>;

    /// Record a default peer for connected-UDP mode.
    ///
    /// This should use the OS `connect()` operation for UDP where available.
    pub fn connect(&mut self, remote: SocketAddr) -> Result<(), UdpError>;

    /// Send one datagram to an explicit destination.
    pub fn send_to(&self, payload: &[u8], destination: SocketAddr)
        -> Result<usize, UdpError>;

    /// Send one datagram to the previously connected peer.
    pub fn send(&self, payload: &[u8]) -> Result<usize, UdpError>;

    /// Receive one datagram and return its payload plus address metadata.
    pub fn recv_from(&self) -> Result<UdpDatagram, UdpError>;

    /// Return the local socket address assigned by the OS.
    pub fn local_addr(&self) -> Result<SocketAddr, UdpError>;
}
```

### Convenience Function

```rust
/// Send one datagram and wait for one response from the same peer.
///
/// This is useful for simple request/response protocols such as DNS, but it
/// still treats payloads as opaque bytes.
pub fn send_and_receive(
    destination: SocketAddr,
    payload: &[u8],
    options: UdpOptions,
) -> Result<UdpDatagram, UdpError>;
```

### Error Types

```rust
#[derive(Debug)]
pub enum UdpError {
    /// The socket could not bind to the requested local address.
    BindFailed(String),

    /// The socket could not record the requested peer.
    ConnectFailed(String),

    /// Sending failed.
    SendFailed(String),

    /// Receiving failed.
    ReceiveFailed(String),

    /// A read or write operation timed out.
    Timeout,

    /// Caller attempted `send()` without first calling `connect()`.
    NotConnected,

    /// Payload or receive buffer size is invalid for this package.
    InvalidDatagramSize { size: usize, max: usize },

    /// The OS reported that a datagram did not fit in the receive buffer.
    TruncatedDatagram,
}
```

Implementations should map platform-specific errors into these semantic
variants while preserving enough detail in string fields to debug locally.

---

## Behavioral Rules

### 1. Payloads Are Opaque

`udp-client` never inspects payload bytes. The following must be equally valid:

- DNS message bytes
- game state updates
- test fixture bytes
- compressed binary payloads
- empty datagrams, if the host platform allows them

### 2. No Automatic Retries

`send_and_receive()` sends once and receives once. If receive times out, it
returns `UdpError::Timeout`. Higher layers decide whether to retry.

### 3. No Hostname Resolution

The public API accepts `SocketAddr`, not `host + port`. This keeps the package
transport-only and avoids DNS recursion.

### 4. Timeout Behavior

Timeouts should apply to blocking operations:

- `read_timeout` controls `recv_from()`
- `write_timeout` controls `send_to()`/`send()` where supported

If a platform cannot set a write timeout for UDP, document that and preserve the
field for API consistency.

### 5. Datagram Size Guard

The implementation must reject caller-provided payloads larger than
`max_datagram_size` before passing them to the OS. This prevents accidental huge
allocations and gives deterministic errors in tests.

---

## Testing Strategy

### Unit Tests

1. **Default options:** verify `UdpOptions::default()` chooses a safe buffer
   size and no unnecessary timeouts.
2. **Invalid max size:** reject zero-sized receive buffers.
3. **Oversized send:** reject payloads larger than the configured maximum before
   sending.
4. **send without connect:** `send()` returns `NotConnected`.

### Local Socket Tests

These tests use loopback only; they do not need internet access.

5. **bind localhost:** open a UDP socket on `127.0.0.1:0` and verify the OS
   assigns a port.
6. **send_to / recv_from:** client sends bytes to a local UDP server; server
   sees exact payload and client address.
7. **echo round trip:** local server echoes one datagram; client receives exact
   response.
8. **connected UDP:** client connects to server and uses `send()`.
9. **source metadata:** received datagram includes the sender address.
10. **timeout:** receive on an idle socket with a short timeout returns
    `Timeout`.

### Edge Cases

11. **empty datagram:** if supported by the platform, verify it round-trips; if
    not supported, map the OS error cleanly.
12. **IPv6 loopback:** run when `::1` is available.
13. **large but valid datagram:** verify a payload near the configured limit.
14. **parallel sockets:** bind multiple ephemeral sockets without port clashes.

### Future Integration Tests

When `dns-client` exists, add an ignored/live-network smoke test that sends a
NET06 DNS query over NET07 UDP to a configured resolver. That test should stay
outside the core `udp-client` package because UDP itself is protocol-agnostic.

---

## Scope

### In Scope

- OS-backed UDP sockets
- bind to local address or ephemeral port
- send one datagram
- receive one datagram
- connected and unconnected UDP modes
- read/write timeout configuration
- loopback-only tests
- IPv4 and IPv6 socket addresses where available

### Out of Scope

- DNS parsing or query construction
- hostname resolution
- retries
- connection pooling
- multicast / broadcast
- raw IP packet construction
- UDP checksum implementation
- simulated D17 stack integration
- QUIC

### Future Work

- multicast options for mDNS or service discovery
- broadcast support for LAN discovery protocols
- async/nonblocking integration with event-loop packages
- UDP server abstraction
- TCP fallback support for DNS clients that receive truncated UDP responses

---

## Implementation Languages

- **Rust** (primary first implementation)
- Future: all supported package languages following the repo scaffold pattern

The first implementation should be intentionally small and transport-only. If a
test needs to parse a DNS message to prove UDP works, that test belongs in a
future integration package, not here.
