# Conduit (Swift)

A Sinatra/Express-style web framework for Swift, hosting the Rust **web-core**
HTTP engine (WEB08 facade) through the reusable [`conduit-capi`](../../rust/conduit-capi)
C ABI — linked at compile time via Swift's native C interop. No third-party FFI.

This is the WEB12 port in the cross-language Conduit family, and the first
consumer of `conduit-capi` (C++, Go, C#, F#, Dart, and Haskell reuse the same ABI).

## How it fits in the stack

```
your Swift handlers   (Request) throws -> Response
        │  @convention(c) trampoline + boxed closure (ctx)
Conduit  (this package, Swift)
        │  conduit-capi  (C ABI, libconduit_capi.a)
conduit  (WEB08 facade) → web-core → embeddable-http-server → tcp-runtime
```

## Build

This package links a Rust static library, so it needs both a Rust toolchain and
Swift:

```sh
cd code/packages/swift/conduit
( cd ../../rust/conduit-capi && cargo build --release )
cp ../../rust/target/release/libconduit_capi.a Sources/CConduit/
swift test
```

The `BUILD` script does all of this.

## Quick start

```swift
import Conduit

let app = Application()

app.before { req in
    if req.path == "/down" { try halt(503, "maintenance") }
    return nil
}

app.get("/") { _ in .html("<h1>Hello from Conduit!</h1>") }

app.get("/hello/:name") { req in
    .json("{\"hi\":\"\(req.param("name") ?? "")\"}")
}

app.post("/echo") { req in
    .respond(200, req.bodyText, headers: [("content-type", req.contentType)])
}

app.notFound { req in .text("no route: \(req.path)", status: 404) }
app.onError  { _   in .json("{\"error\":\"oops\"}", status: 500) }

let server = try app.bind(host: "127.0.0.1", port: 3000)
server.serve()   // blocks until stopped
```

## Response helpers

Each returns a `Response`:

| Helper | Status | Content-Type |
| ------ | ------ | ------------ |
| `.html(_:status:)`  | 200 | `text/html; charset=utf-8` |
| `.json(_:status:)`  | 200 | `application/json` |
| `.text(_:status:)`  | 200 | `text/plain; charset=utf-8` |
| `.respond(_:_:headers:)` | as given | as given |
| `.redirect(_:status:)` | 302 | sets `Location` (throws on CR/LF) |

`try halt(status, body)` performs a Sinatra-style non-local exit from a handler
or before-filter.

## Request

Handlers receive a `Request`: `method`, `path`, `queryString`, `body` / `bodyText`,
`contentType`, `remoteAddr`, plus `param(_:)` (route params), `query(_:)`
(query string), and `header(_:)` (case-insensitive). Inside `onError`, `error`
carries the failure message.

## Application DSL

`Application()` → `get/post/put/delete/patch(_:_:)` → `before`/`after` →
`notFound`/`onError` → `set`/`getSetting` → `bind(host:port:)`. Every registration
returns `self`, so calls chain. `bind` returns a `Server`.

## Server

`serve()` (foreground, blocks), `serveBackground()` (own OS thread), `stop()`,
`localPort`, `running`. The engine's reactor dispatches handlers inline on the
serving thread.

## Security

The trust boundary is audited once in `conduit-capi`: header names/values with
CR/LF/control bytes are dropped (response-splitting defense), status is clamped to
100–599, and every string crossing the boundary is UTF-8-validated. Swift's
`redirect` additionally rejects CR/LF in the location.

## Tests

`swift test` — 20 tests: response helpers (incl. a native round-trip and the CRLF
guard), the Application DSL / settings / bind, and a full end-to-end server run
(root, route params, POST echo, query, before-halt 503, throwing-handler→onError
500, custom 404, redirect 302, after-hook header stamping) driven by a POSIX
socket HTTP/1.0 client with a watchdog.
