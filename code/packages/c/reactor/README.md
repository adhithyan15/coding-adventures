# reactor (C)

**CCPP02 Phase 3.** Readiness notification — watch many sockets from one thread
and wake only for the ready ones — on top of the `os-platform` core. The
companion to `net`: blocking one thread per connection does not scale; a reactor
does. There is no ISO C form for it.

Built by `platform-harness`, per-OS **source selection** (`run.sh` picks
`reactor_mac.c` / `reactor_linux.c`; `BUILD_windows` picks `reactor_windows.c`,
linking `ws2_32`). Reuses `os-platform`'s `osp_status`.

## Scope

Each OS uses its scalable readiness mechanism:

| OS      | mechanism | why |
|---------|-----------|-----|
| macOS   | `kqueue`  | register once, wake O(ready) |
| Linux   | `epoll`   | register once, wake O(ready) |
| Windows | `WSAPoll` | readiness-based; adequate here |

`kqueue`/`epoll` register interest once in the kernel and return only the ready
descriptors, versus `poll()`'s O(n) rescan every wait — the scalability win for
many mostly-idle connections. All three present the **identical interface** with
the same semantics (one coalesced event per ready descriptor, `EOF`/error
surfaced as readable).

**Not IOCP.** Windows' IOCP is a *completion* API (post a read, get told when it
finished) — a different model that wouldn't fit this readiness interface; a
completion-style reactor would be its own primitive. `WSAPoll` keeps Windows on
the same readiness contract as the Unix backends.

## API (`reactor/reactor.h`)

```c
#include "reactor/reactor.h"

osp_reactor *r;
osp_reactor_create(&r);
osp_reactor_add(r, fd, OSP_READABLE, my_token);   /* watch fd; carry a token */

osp_event events[64];
int n;
osp_reactor_wait(r, events, 64, 1000, &n);        /* block ≤1s; n = ready count */
for (int i = 0; i < n; i++) {
    if (events[i].events & OSP_READABLE) handle(events[i].token);
}

osp_reactor_del(r, fd);
osp_reactor_destroy(r);
```

| Function | Purpose |
|----------|---------|
| `osp_reactor_create` / `osp_reactor_destroy` | make / free a reactor |
| `osp_reactor_add(r, fd, interest, token)` | watch `fd`; re-adding updates it |
| `osp_reactor_del(r, fd)` | stop watching (absent fd is not an error) |
| `osp_reactor_wait(r, events, max, timeout_ms, &count)` | block for readiness |

- The watched descriptor is the OS-native type via the `osp_fd` typedef (an
  `int` fd on POSIX, a `SOCKET` on Windows).
- `interest` / `events` are `OSP_READABLE | OSP_WRITABLE` bits. A closed/broken
  peer (`POLLHUP`/`POLLERR`) surfaces as readable so the next read sees it.
- `timeout_ms` negative = wait forever; `0` = poll and return immediately.

## Test

`tests/reactor_test.c` makes a connected socket pair (POSIX `socketpair`; a raw
Winsock loopback pair on Windows), registers one end for read-readiness, and
proves: nothing-written → 0 ready; write on the far end → exactly our descriptor
readable with the exact registered token; after `del` → 0 ready again. Plus
NULL-argument validation.

```sh
cd code/packages/c/reactor
sh tools/run.sh        # macOS / Linux (Windows: tools\run.ps1 via BUILD_windows)
```

Locally (macOS): 21 checks / 0 failed under gcc + clang; clean under ASan+UBSan;
0 leaks.

## Layout

```
reactor/
├── include/reactor/reactor.h     # public API (reuses os_platform/status.h)
├── src/reactor_mac.c             # macOS (kqueue) backend
├── src/reactor_linux.c           # Linux (epoll) backend
├── src/reactor_windows.c         # Windows (WSAPoll) backend
├── tests/reactor_test.c          # readiness round-trip on a socket pair
├── tools/run.sh  · run.ps1       # per-OS build drivers
├── BUILD  · BUILD_windows        # per-OS source selection
└── required_capabilities.json    # CI needs gcc, clang, cl
```
