# irc-net-epoll — Level 3: Raw epoll / kqueue Syscalls

## Overview

`irc-net-epoll` is the third layer of the Russian Nesting Doll. It replaces the `selectors`
module abstraction with direct calls to the OS's native I/O event notification mechanism:
`epoll` on Linux, `kqueue` on macOS/BSD.

By removing the `selectors` abstraction, you see exactly what syscalls are being made and why
the abstraction exists. You learn what `epoll_create1`, `epoll_ctl`, and `epoll_wait` do, what
edge-triggered mode means and how to use it correctly, and why libraries like `libuv`, `mio`,
and `netpoll` were written.

The `Connection`, `Listener`, `EventLoop`, and `Handler` interfaces are unchanged. The `ircd`
program swaps implementations by changing one import.

---

## Layer Position

```
ircd (program)
    ↓
irc-net-epoll           ← THIS SPEC: direct epoll/kqueue syscalls
    ↓
epoll (Linux) / kqueue (macOS/BSD) — kernel interface
    ↓
Kernel TCP stack
```

---

## Concepts

### Why epoll Exists

Early Unix servers used `select()`. It takes a bitset of file descriptors and returns another
bitset indicating which are ready. Problems:
- Maximum 1024 fds (FD_SETSIZE)
- O(N) scan of the entire bitset on every call
- The bitset must be re-initialized from scratch on every call

`poll()` improved on `select()` by using an array instead of a bitset (no 1024 limit, no
reinitialization) but was still O(N) in the number of watched fds.

`epoll` (Linux 2.5.44, 2002) solved the O(N) problem. The kernel maintains a persistent
interest set. You register fds once with `epoll_ctl`. On each `epoll_wait` call, the kernel
returns **only** the fds that are ready — O(ready) not O(watched). A server with 100,000 idle
connections and 10 active ones returns 10 events, not 100,000.

`kqueue` (BSD/macOS, 2000) has the same design with a different API.

### The Three epoll Syscalls

```c
// Create an epoll instance. Returns an fd that represents the interest set.
// EPOLL_CLOEXEC: close the epoll fd on exec() — good practice.
int epfd = epoll_create1(EPOLL_CLOEXEC);

// Add, modify, or remove an fd from the interest set.
struct epoll_event ev = {
    .events = EPOLLIN,      // interested in readability
    .data.fd = sock_fd,     // user data returned with each event
};
epoll_ctl(epfd, EPOLL_CTL_ADD, sock_fd, &ev);
epoll_ctl(epfd, EPOLL_CTL_MOD, sock_fd, &ev);  // change interest
epoll_ctl(epfd, EPOLL_CTL_DEL, sock_fd, NULL);  // remove

// Wait for events. Returns number of ready fds.
struct epoll_event events[MAX_EVENTS];
int n = epoll_wait(epfd, events, MAX_EVENTS, timeout_ms);
for (int i = 0; i < n; i++) {
    int fd = events[i].data.fd;
    uint32_t flags = events[i].events;
    // flags & EPOLLIN → readable; flags & EPOLLOUT → writable
}
```

### Event Flags

| Flag | Meaning |
|---|---|
| `EPOLLIN` | fd is readable (data available or new connection on listener) |
| `EPOLLOUT` | fd is writable (send buffer has space) |
| `EPOLLERR` | Error condition (always monitored, even if not requested) |
| `EPOLLHUP` | Hang-up (peer closed connection) |
| `EPOLLET` | **Edge-triggered** mode (see below) |
| `EPOLLONESHOT` | Remove after one event; must re-arm with `EPOLL_CTL_MOD` |
| `EPOLLRDHUP` | Peer shut down writing (half-close) |

### Level-Triggered vs Edge-Triggered (Critical)

**Level-triggered (LT)** is the default. The kernel reports a fd as ready on every
`epoll_wait` call as long as the condition holds. If you don't read all available data,
it reports the fd as ready again next iteration. Safe and simple.

**Edge-triggered (ET)** reports a fd exactly once when its state *changes* from not-ready
to ready. If you receive 1000 bytes and only read 500, epoll will NOT report that fd as ready
again until new data arrives. To use ET mode correctly, you must drain the socket completely
after each notification — loop `recv()` until it returns `EAGAIN` or `EWOULDBLOCK`.

```python
# Edge-triggered: MUST drain until EAGAIN
while True:
    try:
        data = sock.recv(4096)
        if not data:
            break  # Connection closed
        framer.feed(data)
    except BlockingIOError:
        break      # EAGAIN: no more data available right now
```

Forgetting to drain with ET mode is one of the most common bugs in epoll-based servers.
The `selectors` module uses LT by default, which is why it does not require this loop.

### The eventfd Trick (Wakeup)

`epoll_wait` blocks the calling thread. To stop the event loop from another thread (e.g. on
shutdown), you need a way to wake it up. The standard solution is an `eventfd` or `pipe`:

```python
import os
# Create a pipe; add the read end to epoll
r_fd, w_fd = os.pipe()
epoll.register(r_fd, select.EPOLLIN)

# To wake up the event loop from another thread:
os.write(w_fd, b"\x00")

# In the event loop, when r_fd is readable: it's a shutdown signal
```

---

## Platform Split

| OS | Mechanism | Python API | Go API | Rust API |
|---|---|---|---|---|
| Linux | `epoll` | `select.epoll()` | `golang.org/x/sys/unix` | `libc::epoll_*` |
| macOS/BSD | `kqueue` | `select.kqueue()` + `select.kevent()` | `golang.org/x/sys/unix` | `libc::kqueue`, `libc::kevent` |
| Windows | IOCP | `select.select()` (fallback) | `internal/poll` | `windows-sys` |

Windows uses a fundamentally different model (completion-based IOCP vs readiness-based epoll).
For this spec, Windows support is limited to the `selectors` abstraction. The raw epoll/kqueue
implementation is Linux/macOS only.

Use build tags / conditional compilation to select the correct implementation:
- Python: `if sys.platform == 'linux': use epoll else: use kqueue`
- Go: `//go:build linux` and `//go:build darwin`
- Rust: `#[cfg(target_os = "linux")]` and `#[cfg(target_os = "macos")]`

---

## Python Implementation

```python
from __future__ import annotations

import os
import select
import socket
import sys

from irc_framing import Framer
from irc_proto import Message, parse, serialize


if sys.platform != "linux":
    raise ImportError("irc-net-epoll requires Linux (epoll)")


class EpollConnection:
    def __init__(self, sock: socket.socket, addr: tuple[str, int], conn_id: ConnId) -> None:
        self._sock = sock
        self._addr = addr
        self._id = conn_id
        self._write_buf = bytearray()
        self.framer = Framer()
        sock.setblocking(False)

    @property
    def id(self) -> ConnId:
        return self._id

    @property
    def peer_addr(self) -> tuple[str, int]:
        return self._addr

    def fileno(self) -> int:
        return self._sock.fileno()

    def recv_all(self) -> bytes:
        """Drain all available data (required for edge-triggered mode)."""
        chunks: list[bytes] = []
        while True:
            try:
                chunk = self._sock.recv(4096)
                if not chunk:
                    return b""      # connection closed
                chunks.append(chunk)
            except BlockingIOError:
                break               # EAGAIN: all data read
        return b"".join(chunks)

    def enqueue(self, data: bytes) -> None:
        self._write_buf.extend(data)

    def flush(self) -> bool:
        """Try to send queued data. Returns True if buffer empty."""
        while self._write_buf:
            try:
                sent = self._sock.send(self._write_buf)
                del self._write_buf[:sent]
            except BlockingIOError:
                return False        # EAGAIN: send buffer full
        return True

    def close(self) -> None:
        try:
            self._sock.close()
        except OSError:
            pass


class EpollEventLoop:
    """Event loop using Linux epoll in edge-triggered mode."""

    _MAXEVENTS = 64

    def __init__(self) -> None:
        self._conns: dict[int, EpollConnection] = {}   # fd → conn
        self._id_to_fd: dict[ConnId, int] = {}
        self._next_id = 1
        self._running = False

    def run(self, listener: EpollListener, handler: Handler) -> None:
        self._running = True

        # Create epoll instance
        ep = select.epoll()

        # Wakeup pipe for graceful shutdown
        r_fd, w_fd = os.pipe()
        os.set_inheritable(r_fd, False)
        os.set_inheritable(w_fd, False)
        self._wakeup_w = w_fd

        # Register listener and wakeup pipe in edge-triggered mode
        # EPOLLIN | EPOLLET: edge-triggered readability
        ep.register(listener.fileno(), select.EPOLLIN | select.EPOLLET)
        ep.register(r_fd, select.EPOLLIN)

        while self._running:
            try:
                events = ep.poll(timeout=1.0, maxevents=self._MAXEVENTS)
            except InterruptedError:
                continue

            for fd, event_mask in events:
                if fd == r_fd:
                    # Shutdown signal
                    self._running = False
                    break

                if fd == listener.fileno():
                    # One or more new connections (must drain due to ET)
                    while True:
                        try:
                            conn = listener.accept_conn(ConnId(self._next_id))
                            self._next_id += 1
                            self._conns[conn.fileno()] = conn
                            self._id_to_fd[conn.id] = conn.fileno()
                            ep.register(
                                conn.fileno(),
                                select.EPOLLIN | select.EPOLLET,
                            )
                            handler.on_connect(conn.id)
                        except BlockingIOError:
                            break   # no more pending connections
                    continue

                conn = self._conns.get(fd)
                if conn is None:
                    continue

                if event_mask & (select.EPOLLERR | select.EPOLLHUP):
                    self._close_conn(ep, conn, handler)
                    continue

                if event_mask & select.EPOLLIN:
                    data = conn.recv_all()
                    if not data:
                        self._close_conn(ep, conn, handler)
                        continue
                    conn.framer.feed(data)
                    for frame in conn.framer.frames():
                        msg = parse(frame.decode("utf-8", errors="replace"))
                        responses = handler.on_message(conn.id, msg)
                        self._dispatch(ep, responses)

                if event_mask & select.EPOLLOUT:
                    drained = conn.flush()
                    if drained:
                        ep.modify(fd, select.EPOLLIN | select.EPOLLET)

        ep.close()
        os.close(r_fd)
        os.close(w_fd)

    def stop(self) -> None:
        self._running = False
        if hasattr(self, "_wakeup_w"):
            os.write(self._wakeup_w, b"\x00")

    def _dispatch(
        self,
        ep: select.epoll,
        responses: list[tuple[ConnId, Message]],
    ) -> None:
        for target_id, msg in responses:
            fd = self._id_to_fd.get(target_id)
            if fd is None:
                continue
            conn = self._conns.get(fd)
            if conn is None:
                continue
            conn.enqueue(serialize(msg))
            # Arm for writability to flush the buffer
            ep.modify(fd, select.EPOLLIN | select.EPOLLOUT | select.EPOLLET)

    def _close_conn(
        self,
        ep: select.epoll,
        conn: EpollConnection,
        handler: Handler,
    ) -> None:
        ep.unregister(conn.fileno())
        responses = handler.on_disconnect(conn.id)
        self._dispatch(ep, responses)
        conn.close()
        del self._conns[conn.fileno()]
        self._id_to_fd.pop(conn.id, None)
```

---

## Go Implementation Sketch

```go
//go:build linux

package ircdepoll

import (
    "golang.org/x/sys/unix"
    "net"
)

func runEpoll(ln net.Listener, handler Handler) error {
    epfd, err := unix.EpollCreate1(unix.EPOLL_CLOEXEC)
    if err != nil {
        return err
    }
    defer unix.Close(epfd)

    lnFd := listenerFd(ln)
    if err := unix.EpollCtl(epfd, unix.EPOLL_CTL_ADD, lnFd, &unix.EpollEvent{
        Events: unix.EPOLLIN | unix.EPOLLET,
        Fd:     int32(lnFd),
    }); err != nil {
        return err
    }

    events := make([]unix.EpollEvent, 64)
    for {
        n, err := unix.EpollWait(epfd, events, 1000 /* ms */)
        if err != nil {
            if err == unix.EINTR {
                continue
            }
            return err
        }
        for i := 0; i < n; i++ {
            fd := int(events[i].Fd)
            ev := events[i].Events
            if fd == lnFd {
                acceptAll(epfd, ln, handler)
            } else {
                handleConn(epfd, fd, ev, handler)
            }
        }
    }
}
```

---

## Rust Implementation Sketch

```rust
use libc::{
    epoll_create1, epoll_ctl, epoll_event, epoll_wait,
    EPOLL_CLOEXEC, EPOLL_CTL_ADD, EPOLL_CTL_DEL, EPOLLET, EPOLLIN,
};

fn run_epoll(listener_fd: i32, handler: &mut dyn Handler) {
    let epfd = unsafe { epoll_create1(EPOLL_CLOEXEC) };
    assert!(epfd >= 0);

    let mut ev = epoll_event {
        events: (EPOLLIN | EPOLLET) as u32,
        u64: listener_fd as u64,
    };
    unsafe { epoll_ctl(epfd, EPOLL_CTL_ADD, listener_fd, &mut ev) };

    let mut events = vec![epoll_event { events: 0, u64: 0 }; 64];
    loop {
        let n = unsafe {
            epoll_wait(epfd, events.as_mut_ptr(), events.len() as i32, 1000)
        };
        for i in 0..n as usize {
            let fd = events[i].u64 as i32;
            if fd == listener_fd {
                accept_all(epfd, listener_fd, handler);
            } else {
                handle_conn(epfd, fd, events[i].events, handler);
            }
        }
    }
}
```

---

## kqueue (macOS/BSD) Sketch

```python
import select

kq = select.kqueue()

# Register listener for readability
kq.control([
    select.kevent(
        listener_fd,
        filter=select.KQ_FILTER_READ,
        flags=select.KQ_EV_ADD | select.KQ_EV_ENABLE,
    )
], 0)

# Wait for events
events = kq.control(None, 64, timeout=1.0)
for event in events:
    fd = event.ident
    if event.filter == select.KQ_FILTER_READ:
        if fd == listener_fd:
            accept_new_connection()
        else:
            read_from_connection(fd)
    elif event.filter == select.KQ_FILTER_WRITE:
        flush_write_buffer(fd)
```

---

## What You Learn at This Layer

| Question | Answer revealed here |
|---|---|
| Why does `selectors` exist? | To hide the platform difference between `epoll` and `kqueue` |
| What is EAGAIN? | The kernel's way of saying "no more data without blocking" |
| Why must ET mode drain? | The kernel fires once per state change; unread data is invisible until new data arrives |
| What is `EPOLLONESHOT`? | An alternative to ET: fire once, manually re-arm with `EPOLL_CTL_MOD` |
| What is `eventfd`? | A kernel object that can be included in epoll's interest set for cross-thread signaling |
| Why does nginx use multiple workers? | `epoll_wait` is not shared across processes without `EPOLLEXCLUSIVE` |

---

## Test Strategy

Same interface contract tests as `irc-net-stdlib` and `irc-net-selectors`. Additional tests:

- **ET drain correctness**: send 512 KB to a connection in a tight loop. Verify all data is
  received. This would fail if the event loop doesn't drain correctly in ET mode.
- **Edge-triggered EPOLLOUT**: fill a connection's send buffer. Verify the event loop buffers
  and retries rather than crashing or losing data.
- **EAGAIN on accept**: simulate rapid burst of connections (> 64 simultaneous). Verify all are
  accepted (the listener accept loop drains until EAGAIN).
- **Linux-only**: test suite is skipped on macOS/Windows unless kqueue variant is present.

---

## Future: Peeling to irc-net-smoltcp

At this level, we are calling OS TCP syscalls directly. The kernel is still managing the TCP
state machine (SYN/SYN-ACK/ACK handshake, retransmission, flow control, congestion control).

The next layer, `irc-net-smoltcp`, removes the OS TCP stack entirely. `smoltcp` implements
TCP/IP in pure Rust userspace. The kernel is bypassed for all TCP logic — only raw Ethernet
frames cross the kernel boundary (via a TUN/TAP device or a raw socket).
