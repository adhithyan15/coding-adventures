# Conduit (C++)

A Sinatra/Express-style web framework for C++, implemented as a **header-only**
wrapper over the reusable [`conduit-capi`](../../rust/conduit-capi) C ABI (which
exposes the Rust **web-core** HTTP engine, WEB08). Your handlers are
`std::function` closures; routing, lifecycle hooks, and HTTP I/O run in Rust.

This is the WEB13 port in the cross-language Conduit family and the second
consumer of `conduit-capi` (after Swift).

## How it fits in the stack

```
your C++ handlers   std::function<Response(const Request&)>
        │  extern "C" trampoline + heap-boxed closure (ctx)
conduit.hpp   (this package, header-only)
        │  conduit-capi   (C ABI, libconduit_capi.a)
conduit  (WEB08 facade) → web-core → embeddable-http-server → tcp-runtime
```

## Build

Header-only, but it links the Rust C ABI static library, so you need a Rust
toolchain and a C++17 compiler:

```sh
sh tools/run-tests.sh        # builds conduit-capi, compiles + runs the tests
```

`tools/run-tests.sh` builds `libconduit_capi.a`, queries the platform's
`native-static-libs`, and compiles each `tests/test_*.cpp` against it. A
`CMakeLists.txt` is provided as the documented end-user path.

## Quick start

```cpp
#include "conduit/conduit.hpp"
using namespace conduit;

int main() {
    Application app;

    app.before([](const Request& req) -> std::optional<Response> {
        if (req.path() == "/down") halt(503, "maintenance");
        return std::nullopt;
    });

    app.get("/", [](const Request&) { return Response::html("<h1>Hello from Conduit!</h1>"); });

    app.get("/hello/:name", [](const Request& req) {
        return Response::json("{\"hi\":\"" + req.param("name").value_or("") + "\"}");
    });

    app.post("/echo", [](const Request& req) {
        return Response::respond(200, req.body(), {{"content-type", req.contentType()}});
    });

    app.notFound([](const Request& req) { return Response::text("no route: " + req.path(), 404); });
    app.onError([](const Request&) { return Response::json("{\"error\":\"oops\"}", 500); });

    Server server = app.bind("127.0.0.1", 3000);
    server.serve();   // blocks until stopped
}
```

## Response helpers

Each returns a `Response`:

| Helper | Status | Content-Type |
| ------ | ------ | ------------ |
| `Response::html(body, status=200)`  | 200 | `text/html; charset=utf-8` |
| `Response::json(body, status=200)`  | 200 | `application/json` |
| `Response::text(body, status=200)`  | 200 | `text/plain; charset=utf-8` |
| `Response::respond(status, body, headers)` | as given | as given |
| `Response::redirect(location, status=302)` | 302 | sets `Location` (throws on CR/LF) |

`halt(status, body)` performs a Sinatra-style non-local exit (throws `Halt`).

## Request

Handlers receive a `const Request&`: `method()`, `path()`, `queryString()`,
`body()`, `contentType()`, `remoteAddr()`, plus `param(name)` (route params),
`query(name)`, and `header(name)` — each returning `std::optional<std::string>`.
Inside `onError`, `error()` carries the failure message.

## Application DSL

`Application` → `get/post/put/del/patch(pattern, handler)` → `before`/`after` →
`notFound`/`onError` → `set`/`getSetting` → `bind(host, port)`. Every registration
returns `*this`, so calls chain. `bind` returns a `Server` and throws on failure.
(`del` is named so because `delete` is a keyword.)

## Server

`serve()` (foreground, blocks), `serveBackground()` (own OS thread), `stop()`,
`localPort()`, `running()`. RAII: the `Server` destructor frees the native server.

## Security

The trust boundary is audited once in `conduit-capi`: header names/values with
CR/LF/control bytes are dropped (response-splitting defense), status is clamped to
100–599, and every string crossing the boundary is UTF-8-validated. `redirect`
additionally throws on CR/LF in the location. The trampolines catch all C++
exceptions so none unwind across the C ABI.

## Tests

`sh tools/run-tests.sh` — 16 tests: response helpers (incl. a native round-trip
and CR/LF guard), the Application DSL / settings / bind, and a full end-to-end
server run (routes, params, POST echo, query, before-halt 503, throwing-handler→
onError 500, custom 404, redirect 302, after-hook header stamping) driven by a
POSIX-socket HTTP/1.0 client with a watchdog thread.
