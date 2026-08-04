# resp-server (C)

**CCPP02 port campaign.** A tiny Redis-style server — the first **protocol**
server on the stack. It proves the reactor TCP runtime (`tcp-runtime` → `net` +
`reactor`) can host a real wire protocol by speaking **RESP** (the line protocol
Redis uses), parsed and encoded by the `resp-protocol` package.

```
client ──TCP──▶ resp-server
                  └─ tcp-runtime (reactor accept/read loop)
                       └─ RESP handler ── resp-protocol decode/encode
                            └─ shared in-memory keyspace
```

## Commands

| Command | Reply |
|---------|-------|
| `PING` | `+PONG` (`PING <msg>` echoes `<msg>` as a bulk string) |
| `ECHO <msg>` | `$<msg>` |
| `SET <k> <v>` | `+OK` (stores `v` under `k`) |
| `GET <k>` | `$<v>` or `$-1` (the value, or the null bulk on a miss) |
| *anything else* | `-ERR unknown command` |

Every connection shares one keyspace: the handler's `user` pointer is the store,
so `SET` on one connection is visible to `GET` on another.

## API (`resp_server/resp_server.h`)

```c
#include "resp_server/resp_server.h"

resp_server *s;
resp_server_bind(&s, "127.0.0.1", 0);   /* port 0 = ephemeral */
unsigned short port;
resp_server_local_port(s, &port);

resp_server_serve(s);        /* accept/read loop until resp_server_stop(s) */
/* …or step it: int n; resp_server_poll(s, 100, &n); */

resp_server_destroy(s);      /* closes everything + frees the keyspace */
```

A thin wrapper over `tcp_runtime`: `bind` installs a RESP handler; the lifecycle
(`bind`/`poll`/`serve`/`stop`/`destroy`) mirrors it.

## Scope

**One command per read chunk.** The phase-one `tcp_runtime` handler is stateless,
so it cannot yet reassemble a RESP frame split across TCP reads, nor handle
several pipelined commands in one chunk — both need `tcp_runtime`'s
stateful-handler follow-up. A value larger than the runtime's 8 KiB per-read
buffer is truncated. Command set is deliberately small (PING/ECHO/SET/GET);
more commands are a follow-up. Single-threaded on the reactor, so the keyspace
needs no locking.

## Build & test

`tests/resp_server_test.c` stands up a real server and speaks RESP to it over an
actual loopback TCP connection (via `net`), single-threaded by stepping the
server with `resp_server_poll`. It sends literal RESP request frames and asserts
the exact reply bytes — `PING → +PONG`, `ECHO`, `SET`/`GET` (including overwrite
and a null-bulk miss), and an error for an unknown command.

```sh
cd code/packages/c/resp-server
sh tools/run.sh        # macOS / Linux (Windows: tools\run.ps1 via BUILD_windows)
```

Locally (macOS): 54 checks / 0 failed under gcc + clang; clean under ASan+UBSan;
0 leaks.

## Layout

```
resp-server/
├── include/resp_server/resp_server.h   # public API (reuses os_platform/status.h)
├── src/resp_server.c                    # keyspace + RESP handler + server wrapper
├── tests/resp_server_test.c             # real RESP round-trip over a loopback socket
├── tools/run.sh  · run.ps1              # build with tcp-runtime + net + reactor + resp-protocol
├── BUILD  · BUILD_windows               # per-OS build drivers
└── required_capabilities.json           # CI needs gcc, clang, cl
```
