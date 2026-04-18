# kqueue

## Overview

`kqueue` is a thin Rust wrapper over BSD/macOS `kqueue` and `kevent`. It
supports the TCP-first readiness use case by exposing read and write filters,
token-carrying user data, and blocking waits.

Because the user is on macOS, this crate must be locally testable and is part of
the immediate implementation scope.

## Layer Position

```text
native-event-core
    ↓
kqueue
    ↓
BSD / macOS kqueue syscalls
```

## Concepts

- A kqueue object owns a kernel event queue.
- Registrations are expressed as `kevent` change records.
- Read and write readiness are separate filters.
- The `udata` field carries an application token back out of `kevent`.

## Public API

- `Filter`
- `EventFlags`
- `KqueueChange`
- `KqueueEvent`
- `Kqueue::new()`
- `Kqueue::apply(change)`
- `Kqueue::apply_all(changes)`
- `Kqueue::wait(max_events, timeout)`

## Data Flow

Input:

- raw file descriptors
- change records with filters, flags, and tokens
- wait timeouts

Output:

- ready events with filter, flags, token, and kernel-provided data

## Test Strategy

- macOS/BSD readiness test using `UnixStream::pair()`
- non-BSD unsupported fallback test

## Future Extensions

- timer, signal, vnode, and user-event helpers
- registration helpers for level-triggered versus edge-clearing styles
