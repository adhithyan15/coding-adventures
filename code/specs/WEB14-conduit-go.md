# WEB14 — Conduit for Go (via cgo over the reusable `conduit-capi` C ABI)

## Summary

Port the **Conduit** web framework to **Go**, as a cgo wrapper over the
reusable `conduit-capi` C ABI introduced in WEB12. Go is the third consumer of
that ABI (after Swift and C++); the trust boundary (header injection defence,
status clamping, UTF-8 validation, panic isolation) was already audited once in
the Rust crate, so this port is a thin, idiomatic Go layer — no new FFI surface.

## Architecture

```
Go DSL (conduit.Application / Request / Response / Server)
    handlers are func(*Request) Response
    │  //export trampoline + cgo.Handle registry (GC-safe handle ↔ Go func)
    ▼
conduit-capi (Rust staticlib, the C ABI from WEB12)
    ▼
conduit (WEB08 facade) → web-core → embeddable-http-server → tcp-runtime
```

The wrapper is a single Go file `conduit.go` (package `conduit`). It declares
the CGO preamble (C shims + header include), imports `"C"`, and provides the
public API. Tests live in the external test package `conduit_test`.

## Design

### Handler types

| Go type       | C ABI callback   | Purpose                                          |
|---------------|-----------------|--------------------------------------------------|
| `HandlerFunc` | `ConduitHandler` | Routes, not-found, and error handlers            |
| `BeforeFunc`  | `ConduitHandler` | Before-filters: return `*Response` to halt, `nil` to continue |
| `AfterFunc`   | `ConduitAfter`   | Transforming after-hooks: receives current response, returns (potentially mutated) response |

### Closure registry — the `cgo.Handle` pattern

The C ABI stores handlers as `(fn_ptr, void* ctx, ctx_free)` triples. Go
closures cannot be passed directly as C function pointers, and raw Go pointers
cannot be stored in C across GC cycles. The solution:

1. `cgo.NewHandle(fn)` — allocates a GC-safe, int-sized handle in the Go
   runtime's internal handle table. Returns a `cgo.Handle` (wraps `uintptr`).
2. The `uintptr` is cast to `void*` in a C shim and stored as `ctx`. Go's GC
   never sees a raw pointer crossing the boundary.
3. `//export goConduitFree` calls `cgo.Handle(uintptr(ctx)).Delete()`, removing
   the handle when the app/server is freed.
4. The trampolines recover the closure: `cgo.Handle(uintptr(ctx)).Value().(HandlerFunc)`.

This avoids the `go vet` "misuse of unsafe.Pointer" warning that afflicts naive
uintptr-cast patterns (Rule 6 of the cgo pointer rules).

### Trampolines

`//export goConduitHandler/Before/After/Free` are Go functions with C linkage.
Each trampoline:

- recovers the closure from its `cgo.Handle`
- wraps the call in a `defer/recover` so no Go panic can unwind across the
  `extern "C"` boundary
- translates a `haltPanic` into the halted response, any other panic into a
  `conduit_capi_report_error` call (routes through the engine's error handler)

`runHandler/runBefore/runAfter` are helper functions that do the actual call
inside a `defer recover` and use named return values so the deferred recovery
can set `out` even after the return expression has been evaluated.

### Static vs dynamic linking

The C ABI Rust crate builds as both a `staticlib` (`.a`) and a `cdylib` (`.so`/
`.dylib`). On Linux, `ld` prefers the sibling `.so` when given a `-L` path plus
`-l` flag — producing binaries that fail at runtime if the `.so` isn't on
`LD_LIBRARY_PATH`. The `#cgo LDFLAGS` preamble links the `.a` by **full path**,
not with `-l conduit_capi`, so static linking is portable across macOS and
Linux without runtime loader setup.

Native dependency list:
- macOS: `-liconv` (Rust's `encoding_rs` dependency)
- Linux: `-lpthread -ldl -lm -lrt -lutil`

These are extracted in the BUILD file via `cargo rustc --print native-static-libs`.

### Response / Request value types

- `Response{Status, Headers, Body}` is a plain struct. Helpers `HTML`, `JSON`,
  `Text`, `Respond`, `Redirect` build common cases.
- `toC()` converts a Go `Response` into an owned `*C.ConduitResponse`; the
  engine's `ctx_free` chain frees it after the response is sent.
- `responseFromC()` reads a `*C.ConduitResponse` back into a Go `Response` (for
  after-hooks), using `C.GoBytes` to copy the body.
- `Request` is a thin wrapper around `*C.ConduitRequest`; each accessor calls
  the corresponding `conduit_request_*` C function.

### Application / Server RAII

- `Application.Free()` frees an app that was never bound.
- `Application.Bind()` sets `consumed = true` (the C ABI moves the app into the
  server handle on both success and failure) and returns `(*Server, error)`.
- `Server.Close()` guards against double-free with a nil check.

## Threading

Inherited from `conduit-capi` → `embeddable-http-server`:

- `conduit_server_serve()` (foreground) runs the reactor on the **calling
  thread** using a single `TcpRuntime`. No worker threads are spawned.
- `conduit_server_serve_background()` spawns **one OS thread** and starts the
  reactor there.

Go closures are inherently safe to call from multiple goroutines (they share the
heap; the GC handles it). The cgo runtime itself is goroutine-safe for outbound
calls.

## Test coverage

| Test                   | What it exercises                                        |
|------------------------|----------------------------------------------------------|
| `TestHTMLDefaults`     | `HTML()` helper, content-type, status                    |
| `TestHelperStatusAndTypes` | `JSON`, `Text`, `Respond`, explicit status codes     |
| `TestRedirect`         | Redirect helper, response-splitting guard (CR/LF)        |
| `TestSettings`         | `Set`/`GetSetting` round-trip                            |
| `TestChaining`         | All registration methods return `*Application`           |
| `TestBindReturnsPort`  | `Bind` with port 0 allocates a real port                 |
| `TestEndToEnd`         | 8 subtests: root, route param, POST echo, query, before-halt, error handler, not-found, redirect — all over a live `ServeBackground()` server with a 30 s watchdog |

## Files

```
code/packages/go/conduit/
    conduit.go              — cgo wrapper (single source file)
    conduit_test.go         — tests (external package conduit_test)
    go.mod                  — module declaration, no external deps
    BUILD                   — build-tool entry: build conduit-capi, run go test
    BUILD_windows           — skip message (CGO + Rust static lib on Windows needs cl.exe)
    required_capabilities.json
    README.md
    CHANGELOG.md

code/programs/go/conduit-hello/
    main.go                 — demo web app (buildApp() + main)
    smoke_test.go           — end-to-end smoke test using net/http.Client
    go.mod                  — requires conduit via replace directive
    BUILD                   — build + run smoke test
    BUILD_windows
    required_capabilities.json
    README.md
    CHANGELOG.md
```

## Security properties (inherited from conduit-capi)

- **Header injection**: `conduit_response_set_header` validates names/values for
  CR, LF, and NUL bytes; `Redirect` validates the location in Go before
  forwarding.
- **Status clamping**: the C layer clamps status to 100–599; `toC()` adds a
  second Go-side clamp.
- **Panic isolation**: `recover` in every trampoline prevents Go panics from
  unwinding across the C boundary.
- **Handle leaks**: `goConduitFree` always calls `handle.Delete()`, so no handle
  is orphaned when routes/hooks are torn down.
