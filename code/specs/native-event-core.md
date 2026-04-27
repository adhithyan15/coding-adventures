# native-event-core

## Overview

`native-event-core` is the generic event substrate above raw native backends such
as `epoll`, `kqueue`, and `iocp`.

It deliberately stops at the event boundary. It does not know about TCP request
parsing, WebSocket framing, or UI widget trees. It knows about:

- tokens
- interests
- source kinds
- normalized events
- polling backends

## Layer Position

```text
stream-reactor / tcp-reactor / websocket-runtime / ui-event-core
    ↓
native-event-core
    ↓
epoll / kqueue / iocp
```

## Concepts

- `Token` identifies a registered source.
- `Interest` describes readable and writable interest in backend-neutral terms.
- `SourceKind` distinguishes ordinary I/O from wakeups or timers in the event model.
- `NativeEvent` is the normalized output of polling.
- `EventBackend` is the trait implemented by platform backends.
- backend implementations keep direct token-to-source-kind metadata so event
  translation remains constant-time under load.

## Public API

- `Token`
- `Interest`
- `SourceKind`
- `NativeEvent`
- `PollTimeout`
- `EventBackend`
- `NativeEventLoop<B>`
- `linux::LinuxBackend`
- `bsd::KqueueBackend`
- `windows::IocpBackend`

## Data Flow

Input:

- registrations against raw backend handles
- poll timeouts

Output:

- backend-neutral ready or completion events

## Test Strategy

- fake-backend unit tests for generic abstractions
- macOS/BSD integration test using `KqueueBackend`
- unsupported-platform tests for gated backends

## Future Extensions

- eventfd/timerfd-backed Linux wakeup and timer helpers
- richer completion-event support for Windows
- source registration helpers for higher-level protocol crates
