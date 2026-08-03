# D18 Chief Daemon WebSocket API

## Status

This document specifies the first runnable, authenticated WebSocket control
surface for the D18 Chief orchestrator.

The Rust implementation package is `chief-of-staff-daemon-api`.

## Purpose

The daemon API binds the transport-independent
`chief-of-staff-orchestrator-core` to `websocket-runtime`. It provides a small
versioned JSON request/response protocol for the operator CLI without moving
process authority, durable state, channel keys, or trust decisions into the
transport layer.

This first slice exposes the host lifecycle operations already implemented by
the runnable core. Pipeline manifests, channel-key distribution, daemon
installation, background scheduling, streaming logs, and the final CLI remain
later adapters.

## Listener Boundary

The production daemon binds the WebSocket server only to an explicitly supplied
address. The application package does not choose a public wildcard address.
Normal local operation uses a loopback address and relies on both OS process
isolation and the authenticated application session described below.

The server accepts only complete WebSocket text messages. Binary application
messages are rejected with WebSocket close status `1003`. Frame and assembled
message sizes are bounded by `WebSocketServerOptions`; the API also rejects JSON
text larger than 64 KiB before parsing it.

## Authentication and Authorization

Every new WebSocket connection starts unauthenticated. Its first successful
operation must be:

```json
{"version":1,"id":"1","method":"authenticate","params":{"credential":"opaque"}}
```

The credential is bounded to 4096 UTF-8 bytes and passed to an injected
`SessionAuthorizer`. The API does not log, persist, return, or otherwise retain
the credential. The authorizer returns an opaque connection-local session value
or a payload-blind denial. A second authentication request on the same
connection is rejected.

Every control operation requires both an authenticated session and an explicit
per-operation authorization decision. This permits production adapters to use
an OS-backed credential, vault challenge, hardware-backed approval, or another
policy without embedding one bearer-secret scheme into the protocol package.
Authentication failure and authorization denial reveal no adapter diagnostics.

The session value is dropped when the WebSocket connection closes. Requests on
different connections never share session state.

## Request Envelope

Each text message is exactly one JSON object:

```json
{"version":1,"id":"request-7","method":"list_hosts","params":{}}
```

- `version` is the integer `1`.
- `id` is 1 to 64 printable ASCII bytes and is echoed in the response.
- `method` is one of the bounded names below.
- `params` is an object.
- Unknown or duplicate fields are rejected.
- Duplicate keys at any JSON object depth are rejected.
- Floating-point values are rejected wherever an integer or string is required.

One request produces one response on the same connection. There are no server
push messages in this slice, so the synchronous reactor handler is sufficient.
Long-running work and streaming operations will use an explicit typed mailbox
in a later slice rather than blocking the reactor.

## Methods

### `authenticate`

Params: `{ "credential": string }`.

Returns `{ "authenticated": true }` after the injected authority creates a
session.

### `register_host`

Params contain `host_name`, `package_path`, a 64-character lowercase hexadecimal
`package_hash`, `restart_policy` (`always`, `on_failure`, or `never`), and
`desired_state` (`running` or `stopped`).

Returns the complete durable host entry.

### `list_hosts`

Params are empty. Returns durable entries in stable host-name order.

### `set_desired_state`

Params contain `host_name` and `desired_state`. Returns the updated durable
entry. The daemon scheduler or an explicit `reconcile_once` request performs
the bounded convergence tick.

### `reconcile_once`

Params are empty. Returns the stable host-name-ordered actions from exactly one
core reconciliation tick. It never sleeps or retries.

### `health_check`

Params contain `host_name`. Returns durable intent and a separate fresh
authoritative supervisor observation. The API never collapses those views into
one potentially misleading status.

### `deregister_host`

Params contain `host_name`. The runnable core still requires stopped durable
intent and absent or exited process authority. Returns `{ "deregistered": true
}`.

## Response Envelope

Success:

```json
{"version":1,"id":"request-7","ok":true,"result":{}}
```

Failure after a valid request ID:

```json
{"version":1,"id":"request-7","ok":false,"error":{"code":"forbidden","message":"operation is not authorized"}}
```

Malformed envelopes that do not establish a valid ID use an empty response ID.
Stable public error codes are:

- `invalid_request`
- `invalid_params`
- `unauthenticated`
- `authentication_failed`
- `already_authenticated`
- `forbidden`
- `not_found`
- `conflict`
- `internal`

Adapter errors, storage roots, package contents, executable arguments,
environment values, credentials, and message payloads never appear in a
response.

Revisions and nanosecond timestamps are encoded as decimal strings so JSON
number implementations cannot silently lose 64-bit precision. Package hashes
and channel identifiers are encoded as canonical strings.

## Capabilities

The protocol and dispatcher introduce no direct filesystem, process,
environment, random, clock, or secret-store access. The WebSocket binding uses
the capabilities already declared by `websocket-runtime`; concrete core and
authentication adapters retain their own manifests.

## Required Tests

The package must cover:

- bounded fallible parsing of malformed and deeply nested untrusted JSON;
- rejection of duplicate and unknown fields;
- request ID, credential, hash, enum, and message-size bounds;
- authentication-first and per-operation authorization;
- no credential retention or diagnostic disclosure;
- stable serialization of durable and authoritative host evidence;
- all seven host lifecycle operations through an injected control plane;
- typed mapping from `OrchestratorCore` into public API errors;
- text-only WebSocket behavior and connection-local session teardown; and
- a real loopback WebSocket client/server exchange.

The implementation forbids unsafe code and targets at least 95 percent line
coverage.
