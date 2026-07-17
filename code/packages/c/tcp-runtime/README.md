# tcp-runtime (C)

**CCPP02 port campaign.** A reactor-driven TCP server — the reusable seam every
protocol server sits on — built on `net` (sockets) + `reactor` (readiness
multiplexing). The C port of the Rust `tcp-runtime` crate's phase-one core, and
the **first consumer that drives `net` + `reactor` together end-to-end**.

A blocking server spends one thread (and its multi-MB stack) per connection. A
`tcp_runtime` spends **one thread on a reactor**: the kernel wakes it only for
the sockets that are actually ready — the pattern behind nginx, Redis, and Node.

Being OS-agnostic (all per-OS code lives in `net` and `reactor`), it is a single
source file with no `#ifdef`.

## API (`tcp_runtime/tcp_runtime.h`)

```c
#include "tcp_runtime/tcp_runtime.h"

/* Reply with the bytes received; close on "bye". */
static tcp_action echo(uint64_t id, const void *data, size_t len,
                       void *out, size_t cap, void *user) {
    size_t n = len < cap ? len : cap;
    memcpy(out, data, n);
    return (tcp_action){ .write_len = n,
                         .close = (n == 3 && !memcmp(data, "bye", 3)) };
}

tcp_runtime *rt;
tcp_runtime_bind(&rt, "127.0.0.1", 0, echo, NULL);  /* port 0 = ephemeral */
unsigned short port;
tcp_runtime_local_port(rt, &port);

tcp_runtime_serve(rt);       /* accept/read loop until tcp_runtime_stop(rt) */
/* …or step it yourself: int n; tcp_runtime_poll(rt, 100, &n); */

tcp_runtime_destroy(rt);     /* closes the listener + every live connection */
```

| Function | Purpose |
|----------|---------|
| `tcp_runtime_bind(&rt, host, port, handler, user)` | listen + register the reactor |
| `tcp_runtime_local_port(rt, &port)` | the bound port (for port 0) |
| `tcp_runtime_set_max_connections(rt, max)` | cap concurrent connections (0 = unlimited); refuse beyond it |
| `tcp_runtime_connection_count(rt, &n)` | current live connection count |
| `tcp_runtime_poll(rt, timeout_ms, &n)` | one reactor step: accept + service ready sockets |
| `tcp_runtime_serve(rt)` | loop `poll` until stopped (blocks) |
| `tcp_runtime_stop(rt)` | ask `serve` to return |
| `tcp_runtime_destroy(rt)` | close everything, free |
| `tcp_runtime_mailbox(rt)` | the outbound mailbox (post bytes from another thread) |
| `tcp_mailbox_send(mb, id, data, len)` | queue bytes to connection `id` |
| `tcp_mailbox_send_and_close(mb, id, data, len)` | queue bytes, then close |
| `tcp_mailbox_close(mb, id)` | queue a close of connection `id` |

**The handler** mirrors the Rust `TcpHandlerResult`: given the bytes just read,
it fills a reply buffer and returns `{ write_len, close }` — how many bytes to
send and whether to close afterwards. `{ 0, 0 }` keeps the connection idle.

### How it composes net + reactor

```
tcp_runtime_bind:  net listen → osp_socket_fd → reactor_add(listener)
tcp_runtime_poll:  reactor_wait
                     ├─ listener ready → net accept → osp_socket_fd → reactor_add(conn)
                     └─ conn ready     → net recv → handler → net send → (close?)
```

Each accepted connection is a heap node used verbatim as its reactor token, so a
wait result maps back to its connection in O(1). (The token must be a stable
allocation, not a slot in the reallocating connection array.)

### The outbound mailbox

The server touches a socket only on its reactor thread. So how does a **worker
thread** reply to a connection? It hands the runtime a *command* — send these
bytes to connection N, and/or close it — which the reactor thread runs on its
next poll:

```c
tcp_mailbox *mb = tcp_runtime_mailbox(rt);   /* safe to call from any thread */
tcp_mailbox_send(mb, conn_id, "async result\n", 13);
tcp_mailbox_send_and_close(mb, conn_id, "done\n", 5);
tcp_mailbox_close(mb, conn_id);
```

Each command is enqueued under a mutex (os-platform's `thread` primitive) with the
payload **copied**; the queue is the only shared state — the connection table
stays private to the reactor thread. `tcp_runtime_poll` detaches the whole queue
under the lock, then writes each command **without holding the lock** (so a
producer is never blocked on I/O). A command for an unknown or already-closed id
is dropped. There is no cross-thread wakeup yet (a self-pipe/eventfd is a
follow-up), so delivery happens on the next poll — within one poll timeout under
`tcp_runtime_serve` (100 ms), or immediately if you drive `tcp_runtime_poll`.

The send functions are safe to call concurrently with each other and with the
poll, but not with `tcp_runtime_destroy` (which tears down the mailbox) — quiesce
your producer threads before destroying the runtime, as with any shared object.

## Scope

Phase-one core: one listener, many concurrent connections, a stateless handler,
cooperative stop, a concurrent-connection **cap**
(`tcp_runtime_set_max_connections`), and a thread-safe outbound **mailbox**
(`tcp_runtime_mailbox`). Deferred to follow-ups (mirroring the Rust crate's own
phased plan): per-connection state, read-pause/resume **backpressure**
(`defer_read`), a mailbox cross-thread **wakeup**, socket-option policy
(`TCP_NODELAY`/keepalive), and multi-core reactor **sharding**. A reply larger
than the 8 KiB per-read buffer is currently truncated.

## Build & test

`tests/tcp_runtime_test.c` stands up a real server and hits it with real loopback
clients (via `net`), stepping the server with `tcp_runtime_poll` between client
actions — all in one thread. It proves the server **multiplexes** (two
independent connections accepted and echoed on one reactor), honors
echo-and-close, refuses connections beyond the cap, and delivers **mailbox**
commands (send / send-and-close / close, plus the drop of a command for an
unknown id, and destroy draining a still-queued command).

```sh
cd code/packages/c/tcp-runtime
sh tools/run.sh        # macOS / Linux (Windows: tools\run.ps1 via BUILD_windows)
```

Locally (macOS): 101 checks / 0 failed under gcc + clang; clean under ASan+UBSan;
0 leaks.

## Layout

```
tcp-runtime/
├── include/tcp_runtime/tcp_runtime.h   # public API (reuses os_platform/status.h)
├── src/tcp_runtime.c                    # the server — one OS-agnostic file
├── tests/tcp_runtime_test.c             # real loopback multiplexing test
├── tools/run.sh  · run.ps1              # build with net + reactor + os-platform thread
├── BUILD  · BUILD_windows               # per-OS build drivers
└── required_capabilities.json           # CI needs gcc, clang, cl
```

The mailbox mutex is os-platform's `thread` primitive, so the build now also
compiles the os-platform thread backend (`-pthread` on POSIX; the CRT on
Windows) — the same wiring is mirrored in the `resp-server` and `http-server`
consumers, which link `tcp_runtime.c`.
