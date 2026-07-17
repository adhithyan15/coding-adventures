# http-server (C)

**CCPP02 port campaign.** A tiny HTTP/1.1 server — the second **protocol** server
on the stack. It proves the reactor TCP runtime (`tcp-runtime` → `net` +
`reactor`) can host HTTP: a handler parses the request line + headers, interprets
them with the `http-core` package (version, request-target/path splitting, query
values, header lookup), routes, and writes an HTTP/1.1 response.

```
client ──TCP──▶ http-server
                  └─ tcp-runtime (reactor accept/read loop)
                       └─ HTTP handler ── in-server request framing
                                       └─ http-core (path/query/version/headers)
```

## Routes

| Request | Response |
|---------|----------|
| `GET /` | `200` `hello from http-server` |
| `GET /echo?msg=X` | `200` the `msg` query value (parsed by http-core) |
| `GET /headers` | `200` the request's headers, one per line |
| `GET /`*other* | `404` |
| *non-GET* | `405` |
| *malformed / too large* | `400` |

Every response carries `Connection: close`, so it's one request/response per
connection (HTTP/1.0 style). The lifecycle (`bind`/`poll`/`serve`/`stop`/
`destroy`) mirrors `tcp_runtime`.

## API (`http_server/http_server.h`)

```c
#include "http_server/http_server.h"

http_server *s;
http_server_bind(&s, "127.0.0.1", 0);   /* port 0 = ephemeral */
unsigned short port;
http_server_local_port(s, &port);

http_server_serve(s);        /* accept/read loop until http_server_stop(s) */
/* …or step it: int n; http_server_poll(s, 100, &n); */

http_server_destroy(s);
```

## Where the parsing lives

`http-core` is a **syntax-level** core: it supplies the shapes (`HttpRequestHead`)
and the helpers that interpret a parsed request (version parse, path/query
splitting, header lookup, `Content-*`), but not the byte-level wire framing. So
this server contains a small, defensive request-line/header parser (the role a
standalone `http1` wire crate would fill — a future package), then hands the
result to `http-core`.

## Scope

The request must arrive whole in one read (the phase-one `tcp_runtime` handler is
stateless and cannot reassemble a request split across reads); a request over the
runtime's 8 KiB per-read buffer is rejected with `400`. GET only — no request
body, no chunked, no keep-alive. All documented; each is a follow-up.

## Build & test

`tests/http_server_test.c` speaks HTTP/1.1 to a real server over an actual
loopback TCP connection (client via `net`, single-threaded via
`http_server_poll`), sending raw requests and asserting the status line and body:
`GET /`, `GET /echo?msg=pong`, `GET /headers`, and `404`/`405`/`400`.

```sh
cd code/packages/c/http-server
sh tools/run.sh        # macOS / Linux (Windows: tools\run.ps1 via BUILD_windows)
```

Locally (macOS): 56 checks / 0 failed under gcc + clang; clean under ASan+UBSan;
0 leaks.

## Layout

```
http-server/
├── include/http_server/http_server.h   # public API (reuses os_platform/status.h)
├── src/http_server.c                    # request parser + router + server wrapper
├── tests/http_server_test.c             # real HTTP round-trip over a loopback socket
├── tools/run.sh  · run.ps1              # build with tcp-runtime + net + reactor + http-core
├── BUILD  · BUILD_windows               # per-OS build drivers
└── required_capabilities.json           # CI needs gcc, clang, cl
```
