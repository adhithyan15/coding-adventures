# WebSocket Protocol Core

## Status

This document specifies the portable, transport-independent RFC 6455 core that
unblocks the D18 Chief daemon and operator CLI.

The first implementation package is Rust `websocket-core`. The byte-level
contract is intentionally suitable for later Python, Go, Ruby, TypeScript, and
Elixir ports required by D18 issue #137.

## Purpose and Layering

The protocol core owns only:

- strict HTTP/1.1 upgrade validation and serialization;
- `Sec-WebSocket-Accept` derivation;
- incremental frame decoding and frame serialization;
- masking and canonical payload lengths;
- fragmentation and UTF-8 message assembly; and
- ping, pong, and close-frame validation.

It does not open sockets, resolve DNS, select TLS, generate randomness, run an
event loop, schedule keepalives, retry connections, or dispatch application
commands. [`websocket-runtime.md`](websocket-runtime.md) composes this core
with TCP and the repository stream/event abstractions.

```text
tcp / stream reactor
        |
websocket-runtime          connection I/O, deadlines, random mask keys
        |
websocket-core             handshake, frames, messages, control semantics
        |
Chief daemon / CLI         authenticated application protocol
```

## Bounded Handshake

The core accepts at most 16 KiB for one HTTP upgrade head. Callers may retain
bytes after the returned consumed offset as the beginning of the WebSocket
frame stream.

A server request is accepted only when all of these hold:

- the head uses CRLF line endings and ends in one empty CRLF line;
- the request is `GET`, HTTP/1.1, and has no HTTP body framing;
- exactly one non-empty `Host` header is present;
- `Upgrade` contains the case-insensitive token `websocket`;
- `Connection` contains the case-insensitive token `Upgrade`;
- exactly one `Sec-WebSocket-Version` is present and equals `13`;
- exactly one `Sec-WebSocket-Key` is present;
- the key is canonical padded Base64 decoding to exactly 16 bytes; and
- extensions and subprotocol negotiation are absent in version 0.1.

The successful response is exactly HTTP/1.1 status 101 with `Upgrade`,
`Connection`, and `Sec-WebSocket-Accept` headers. The accept value is padded
Base64 of SHA-1 over the key text followed by RFC 6455's fixed GUID.

The client request builder receives a caller-generated 16-byte nonce. This
keeps random-number authority in the runtime adapter. It validates the host and
request target against CR/LF injection before serializing a bounded request.
The client response validator requires status 101, the two upgrade tokens, the
exact expected accept value, no body framing, and no unsupported negotiated
extension or subprotocol.

Handshake diagnostics identify fields and failure classes but never reproduce
header values, request targets, or buffered bytes.

## Frames

`EndpointRole` is either `Client` or `Server` and describes the local endpoint.
An inbound decoder enforces the peer's masking rule:

- a server must receive masked frames; and
- a client must receive unmasked frames.

An outbound encoder enforces the inverse. Client callers supply a fresh
unpredictable four-byte mask key for every frame; server callers must not
supply one. The protocol core never invents a constant or counter-based mask.

The frame decoder is incremental. It buffers incomplete headers or payloads,
returns every complete frame in wire order, and retains any incomplete suffix.
It rejects before payload allocation when the declared length exceeds the
configured frame bound.

Every decoded frame must satisfy:

- RSV1, RSV2, and RSV3 are zero because extensions are unsupported;
- the opcode is continuation, text, binary, close, ping, or pong;
- 16-bit and 64-bit extended lengths use their shortest canonical encoding;
- the high bit of a 64-bit length is zero;
- control frames are final and contain at most 125 bytes;
- close payloads are either empty or contain a two-byte code plus UTF-8 reason;
- close payload length one is invalid; and
- close codes are RFC-defined protocol/application codes, not reserved
  pseudo-codes.

Unknown opcodes, non-canonical lengths, mask-direction violations, oversized
declared lengths, and malformed controls are protocol errors.

## Message Assembly

The message assembler receives validated frames and emits:

- complete text messages;
- complete binary messages;
- ping and pong events;
- a validated close event; or
- no event while a fragmented data message remains incomplete.

Only text and binary frames may begin fragmentation. Continuations require an
open fragmented message, and a second data start while fragmented data is open
is rejected. Control frames may appear between fragments without disturbing
the open message.

The assembler checks its configured message bound before extending buffered
data. Text is validated as UTF-8 only when the complete message is available,
so a multi-byte scalar may cross fragment boundaries. A received close frame
ends inbound data delivery; any later data or continuation frame is rejected.

The core exposes a control-reply helper:

- ping maps to a pong with the identical application payload;
- close maps to an echo close frame; and
- other events have no automatic reply.

The runtime decides when to write the reply, when to stop accepting outbound
application data, and when to close the underlying stream.

## Public API Shape

The portable conceptual surface is:

```text
build_client_request(host, target, nonce) -> { bytes, expected_accept }
validate_client_response(bytes, expected_accept) -> consumed | error
accept_server_request(bytes) -> { response_bytes, consumed } | error

FrameDecoder(role, max_frame_bytes).push(bytes) -> Frame[] | error
encode_frame(role, frame, optional_mask_key) -> bytes | error

MessageAssembler(max_message_bytes).push(frame) -> Event? | error
control_reply(event) -> Frame?
```

Concrete language names may follow local conventions while preserving the same
validation and state transitions.

## Errors and Recovery

Errors are typed into incomplete input, handshake validation, frame protocol,
UTF-8, size-limit, fragmentation-state, and closed-session failures.
Incomplete handshake or frame input is not a protocol error and may be retried
with more bytes. After any other decoder or assembler error, a runtime must
send close code 1002 when possible and discard that protocol state.

No error display includes payload bytes, mask keys, handshake nonces, header
values, or close reasons.

## Capability Contract

The core declares no direct filesystem, network, process, environment, clock,
random, or stream capability. SHA-1 and HTTP parsing are pure dependencies.
The runtime package will declare network, clock, random, and stream
capabilities when it composes concrete adapters.

## Required Tests

The Rust package must cover at least 95 percent of lines and include:

- the RFC accept-key example;
- strict valid server and client handshakes with coalesced frame bytes;
- every required handshake field failure and CR/LF injection rejection;
- small, 16-bit, and 64-bit frame lengths;
- canonical-length, RSV, opcode, direction-mask, and size-limit failures;
- masked and unmasked text/binary frames;
- incremental byte-by-byte decoding and multiple frames in one input;
- fragmented text with a UTF-8 scalar split across frames;
- interleaved ping during fragmentation and automatic pong construction;
- invalid fragmentation transitions and oversized assembled messages;
- close-code/reason validation and close echo construction; and
- payload-free stable error diagnostics.

Real TCP and browser-wire interoperability tests belong to
[`websocket-runtime`](websocket-runtime.md), where connection scheduling and
operating-system I/O exist.
