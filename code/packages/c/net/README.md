# net (C)

**CCPP02 Phase 3.** Real TCP sockets over IPv4, on top of the `os-platform`
bucket-B core. ISO C has no networking at all, so this is pure OS territory — and
the two socket families diverge enough (int fds vs the `SOCKET` handle, `close`
vs `closesocket`, no init vs `WSAStartup`) that a thin portable layer earns its
keep.

Built by the `platform-harness` (warnings-as-errors, no `-pedantic-errors`), with
per-OS **source selection** via `BUILD` (POSIX BSD sockets) and `BUILD_windows`
(Winsock2, linking `ws2_32`). It reuses the shared `osp_status` error vocabulary
from `os-platform`.

## API (`net/tcp.h`)

```c
#include "net/tcp.h"

osp_net_init();                                   /* WSAStartup on Windows */

osp_socket *lis;
osp_tcp_listen(&lis, "127.0.0.1", 0, 1);          /* port 0 = ephemeral */
unsigned short port;
osp_tcp_local_port(lis, &port);                   /* read the chosen port */

osp_socket *cli;
osp_tcp_connect(&cli, "127.0.0.1", port);
osp_socket *conn;
osp_tcp_accept(lis, &conn);

size_t n;
osp_socket_send(cli, "hello", 5, &n);             /* sends ALL bytes */
char buf[64];
osp_socket_recv(conn, buf, sizeof buf, &n);       /* n == 0 => peer closed */

osp_socket_close(cli); osp_socket_close(conn); osp_socket_close(lis);
osp_net_shutdown();
```

| Operation | POSIX (macOS/Linux) | Windows (Winsock2) |
|-----------|---------------------|--------------------|
| init / shutdown | (none) | `WSAStartup` / `WSACleanup` |
| listen | `socket`+`bind`+`listen` | same |
| accept / connect | `accept` / `connect` | same |
| transfer | `send` / `recv` | same (`int` lengths) |
| close | `close` | `closesocket` |

- Addresses are **numeric IPv4** dotted-quads (`inet_pton`); no DNS.
- `osp_socket_send` is send-**all** (loops over partial sends, retries `EINTR`);
  `osp_socket_recv` is a single `recv` reporting the byte count (`0` = orderly
  peer shutdown).
- Blocking I/O only. Non-blocking readiness (epoll/kqueue/iocp) is the next
  primitive, `reactor`.
- SIGPIPE on a dead peer is suppressed portably (Linux `MSG_NOSIGNAL`, macOS
  `SO_NOSIGPIPE` — selected by feature-macro presence, not by OS name).

## Test

`tests/net_test.c` is a **single-threaded loopback echo round-trip**: it listens
on an ephemeral `127.0.0.1` port, connects, accepts, sends bytes one way, echoes
them back, and confirms a post-close `recv` reports 0 — then validates NULL /
malformed-address arguments. It is single-threaded because on loopback a blocking
`connect` completes the handshake before returning and a few bytes never fill the
socket buffer.

```sh
cd code/packages/c/net
sh tools/run.sh        # macOS / Linux (Windows: tools\run.ps1 via BUILD_windows)
```

Locally (macOS): 27 checks / 0 failed under gcc + clang; clean under ASan+UBSan;
0 leaks.

## Layout

```
net/
├── include/net/tcp.h             # public API (reuses os_platform/status.h)
├── src/net_posix.c               # macOS + Linux (BSD sockets) backend
├── src/net_windows.c             # Windows (Winsock2) backend
├── tests/net_test.c              # loopback echo round-trip
├── tools/run.sh  · run.ps1       # per-OS build drivers
├── BUILD  · BUILD_windows        # per-OS source selection
└── required_capabilities.json    # CI needs gcc, clang, cl
```
