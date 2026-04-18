# epoll

## Overview

`epoll` is a thin Rust wrapper over Linux's `epoll` readiness API. It exposes
the raw kernel model directly: interest registration, token-carrying readiness
events, and blocking waits over the ready list.

This package is one of the TCP-first raw backend crates that future transport and
protocol layers build on top of.

## Layer Position

```text
native-event-core
    ↓
epoll
    ↓
Linux epoll syscalls
```

## Concepts

- An epoll instance owns a kernel interest set.
- Each registration binds a file descriptor, a token, and an interest mask.
- Waiting returns only ready descriptors.
- Edge-triggered and one-shot behavior remain explicit and visible.

## Public API

- `Interest`
- `EpollEvent`
- `Epoll::new(cloexec)`
- `Epoll::add(fd, event)`
- `Epoll::modify(fd, event)`
- `Epoll::delete(fd)`
- `Epoll::wait(max_events, timeout)`

## Data Flow

Input:

- raw file descriptors
- interest masks
- wait timeouts

Output:

- readiness events with token and flag accessors

## Test Strategy

- unit tests for interest composition
- Linux-only readiness test using `UnixStream::pair()`
- non-Linux unsupported fallback test

## Future Extensions

- explicit `EPOLLRDHUP` helpers
- reusable caller-owned wait buffers
