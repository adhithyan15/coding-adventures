# WebSocket Runtime

## Status

This document specifies the first concrete TCP runtime for the portable
[`websocket-core`](websocket-core.md) contract. The first implementation is the
Rust `websocket-runtime` package required by D18 issues #137, #138, and #140.

## Layering

```text
Chief daemon / operator CLI
            |
websocket-runtime        TCP progression, entropy, connection lifecycle
            |
websocket-core           bounded RFC 6455 state machine
            |
tcp-runtime / tcp-client repository TCP adapters
```

The runtime must reuse `tcp-runtime` for servers and `tcp-client` for blocking
clients. It must not create a third socket abstraction or duplicate RFC 6455
parsing.

## Server Adapter

The server owns one `websocket-core` session per accepted TCP connection. Its
connection phase is either:

- a bounded HTTP upgrade buffer; or
- an open frame decoder and message assembler.

Arbitrary TCP read boundaries are valid. A read may contain part of an upgrade,
the complete upgrade, or the upgrade followed by one or more frames. The server
retains no more than the core's 16 KiB handshake bound and forwards coalesced
frame bytes immediately after writing the HTTP 101 response.

Applications receive complete `MessageEvent` values with mutable
connection-local application state. Their handler may return zero or more
validated outbound frames plus close-after-flush intent. The adapter encodes
all server frames unmasked.

The adapter automatically:

- replies to ping with pong carrying the identical payload;
- echoes a valid close and closes TCP after the reply flushes;
- maps invalid UTF-8 to close code 1007;
- maps frame or message size overflow to close code 1009; and
- maps other post-upgrade protocol failures to close code 1002.

An invalid HTTP upgrade receives a bounded HTTP 400 response and closes after
flush. Error responses and public errors never include peer-controlled bytes.

## Client Adapter

The blocking client:

1. resolves and connects through `tcp-client` with caller-supplied timeouts;
2. obtains a fresh 16-byte nonce from the repository OS CSPRNG;
3. writes the bounded client upgrade request;
4. reads complete HTTP lines without consuming coalesced frame bytes;
5. validates the HTTP 101 response through `websocket-core`; and
6. owns a client-role decoder and message assembler for the open session.

Every client frame obtains a fresh unpredictable four-byte mask key from the
OS CSPRNG. The adapter never reuses the handshake nonce, a counter, or a fixed
test key as a production mask.

`receive` preserves all events decoded from a coalesced TCP read. It replies to
ping and close controls before returning the event to the caller. Clean TCP EOF
before a WebSocket close exchange is an abnormal-close error.

Version 0.1 supports `ws` over TCP. TLS-backed `wss`, proxy negotiation,
extensions, subprotocols, redirects, reconnects, keepalive deadlines, and
asynchronous clients are later work.

## Public API Shape

The initial Rust surface provides:

```text
WebSocketRuntime::bind(platform, address, options, init, handler, on_close)
WebSocketRuntime::serve / local_addr / stop_handle / mailbox
host-OS bind_kqueue / bind_epoll / bind_windows constructors

WebSocketClient::connect(host, port, target, options)
WebSocketClient::send_frame / send_text / send_binary / send_ping
WebSocketClient::receive / close
```

The server handler result contains outbound `Frame` values and
close-after-flush intent. The mailbox is TCP-flavored in version 0.1; a later
protocol mailbox may add off-reactor frame encoding with explicit entropy and
session-state coordination.

## Bounds and Backpressure

Runtime options must expose positive bounds for:

- maximum frame payload;
- maximum assembled message payload;
- client read chunk size; and
- all inherited TCP runtime buffers, connection caps, and queued-write caps.

Invalid zero bounds fail before binding or connecting. The server delegates
queued-write overflow and connection admission to `tcp-runtime`.

## Errors

`WebSocketRuntimeError` distinguishes:

- invalid options;
- TCP client failures;
- TCP server/platform failures;
- OS entropy failures;
- protocol failures; and
- abnormal EOF or use after the closing handshake.

Display strings name only the failure class. They do not include frame
payloads, mask keys, handshake nonces, header values, request targets, or close
reasons.

## Capability Contract

The runtime is authorized to resolve arbitrary caller-supplied hosts, connect
to arbitrary TCP endpoints, and listen on arbitrary caller-supplied local
addresses. It inherits OS entropy access from `coding_adventures_csprng` and
socket behavior from `tcp-client` and `tcp-runtime`. It performs no filesystem,
process, environment, stdin, stdout, or FFI access of its own.

## Required Tests

The Rust package must include:

- fragmented server upgrades and upgrade-plus-frame coalescing;
- invalid/oversized upgrade rejection;
- text, binary, ping/pong, and close progression;
- protocol-error close-code mapping;
- multiple events preserved from one TCP read;
- deterministic injected entropy tests proving a fresh nonce and mask per use;
- real loopback TCP client/server text, binary, ping, and close exchange;
- a raw RFC 6455 browser-wire client against the server; and
- Linux, macOS, and Windows build coverage through repository CI.
