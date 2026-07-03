# WEB12 — Conduit for Swift (via a reusable `conduit-capi` C ABI)

## Summary

Port the **Conduit** web framework (a Sinatra/Express-style facade over the Rust
`web-core` engine, WEB08) to **Swift**, using Swift's native C interop — no
third-party FFI library.

Unlike the managed-VM ports (Java JNI, Lua, Perl XS), Swift has real C function
pointers and no interpreter lock. That lets us introduce a **reusable, plain
C ABI** for Conduit — a new Rust crate `conduit-capi` that exposes the whole
framework over `extern "C"`. Swift is its first consumer; **WEB13 (C++), WEB14
(Go/cgo), WEB15 (C# P/Invoke), WEB16 (F#), WEB17 (Dart FFI), and WEB18 (Haskell
FFI) will all reuse the same `conduit-capi`** rather than each re-wrapping the
facade. This is the C-ABI analog of the shared lisp-runtime primitives: build
the boundary once, correctly, and let every C-capable language link it.

## Why a C ABI (not per-language wrappers)

The JNI/Lua/Perl ports each re-implement dispatch + marshaling against their
host VM's C API. The seven remaining ports are all **C-ABI-capable** languages.
A single `conduit-capi` gives them:

- one audited trust boundary (header-injection defense, status clamping, UTF-8
  validation, panic catching) instead of seven;
- one dispatch/callback model;
- a stable `conduit_capi.h` header that C, C++, C#, Dart, Haskell, and Go can
  all `#include` / bind directly.

## Architecture

```
Swift DSL (Conduit.Application / Request / Response)
    handlers are Swift closures: (Request) throws -> Response
    │  Swift @convention(c) trampoline + opaque ctx (Unmanaged closure box)
    ▼
conduit-capi (Rust cdylib + staticlib, extern "C")   ← THE reusable C ABI
    conduit_app_* / conduit_server_* / conduit_request_* / conduit_response_*
    ▼
conduit (WEB08 facade) → web-core → embeddable-http-server → tcp-runtime → kqueue/epoll/IOCP
```

### Threading

Established in WEB11: `embeddable-http-server` runs its reactor **inline on the
calling thread** (single `TcpRuntime`, not `ShardedTcpRuntime`). So foreground
`conduit_server_serve` dispatches handlers on the calling thread. Swift closures
are not interpreter-bound and are safe to call from any thread, so **no
per-dispatch lock is needed**. `conduit_server_serve_background` spawns one OS
thread that runs the reactor; Swift handlers invoked from it are fine (the user's
closures must be thread-safe, which the facade's `Fn + Send + Sync` bound already
requires). The stored callback context is wrapped in a `Send + Sync` newtype.

## The `conduit-capi` C ABI

### Handles (opaque pointers)

`ConduitApp*`, `ConduitServer*`, `const ConduitRequest*`, `ConduitResponse*`.
`*App`/`*Server` are `Box::into_raw`'d Rust structs freed by `*_free`; a
`ConduitRequest` is a per-dispatch view; a `ConduitResponse` wraps an owned
`Box<WebResponse>`.

### Callback typedefs

```c
// Returns a NEW response (route / not_found / on_error), or NULL.
// For a before-filter, NULL means "continue"; non-NULL short-circuits.
typedef ConduitResponse* (*ConduitHandler)(void* ctx, const ConduitRequest* req);

// Transforming after-hook: receives & owns `current`, returns a response
// (may be the same pointer mutated, or a fresh one).
typedef ConduitResponse* (*ConduitAfter)(void* ctx, const ConduitRequest* req, ConduitResponse* current);

// Destructor for the ctx, called when the owning app/server is freed.
typedef void (*ConduitCtxFree)(void* ctx);
```

### App / lifecycle

```c
ConduitApp* conduit_app_new(void);
void        conduit_app_set_setting(ConduitApp*, const char* key, const char* value);
char*       conduit_app_get_setting(ConduitApp*, const char* key);   // owned; conduit_string_free
void        conduit_app_add_route(ConduitApp*, const char* method, const char* pattern,
                                  ConduitHandler, void* ctx, ConduitCtxFree);
void        conduit_app_add_before(ConduitApp*, ConduitHandler, void* ctx, ConduitCtxFree);
void        conduit_app_add_after(ConduitApp*, ConduitAfter, void* ctx, ConduitCtxFree);
void        conduit_app_set_not_found(ConduitApp*, ConduitHandler, void* ctx, ConduitCtxFree);
void        conduit_app_set_error_handler(ConduitApp*, ConduitHandler, void* ctx, ConduitCtxFree);
void        conduit_app_free(ConduitApp*);                            // only if never bound
```

### Server

```c
ConduitServer* conduit_server_bind(const char* host, uint16_t port, ConduitApp*); // consumes app; NULL on error
int            conduit_server_serve(ConduitServer*);            // 0 ok; blocks
int            conduit_server_serve_background(ConduitServer*);  // 0 ok
void           conduit_server_stop(ConduitServer*);
uint16_t       conduit_server_local_port(ConduitServer*);
int            conduit_server_running(ConduitServer*);          // 0/1
void           conduit_server_free(ConduitServer*);
const char*    conduit_last_error(void);                        // thread-local; valid until next capi call
```

### Request accessors (valid only during the callback)

```c
const char*    conduit_request_method(const ConduitRequest*);
const char*    conduit_request_path(const ConduitRequest*);
const char*    conduit_request_query_string(const ConduitRequest*);
const char*    conduit_request_content_type(const ConduitRequest*);  // "" if none
const char*    conduit_request_remote_addr(const ConduitRequest*);
const char*    conduit_request_error(const ConduitRequest*);         // for on_error; "" otherwise
const uint8_t* conduit_request_body(const ConduitRequest*, size_t* out_len);
const char*    conduit_request_param(const ConduitRequest*, const char* name);  // NULL if absent
const char*    conduit_request_query(const ConduitRequest*, const char* name);  // NULL if absent
const char*    conduit_request_header(const ConduitRequest*, const char* name); // case-insensitive; NULL if absent
```

### Response builder / reader

```c
ConduitResponse* conduit_response_new(uint16_t status, const uint8_t* body, size_t body_len); // status clamped 100–599
void             conduit_response_set_header(ConduitResponse*, const char* name, const char* value); // CR/LF/CTL/':'-in-name dropped
uint16_t         conduit_response_status(const ConduitResponse*);
const uint8_t*   conduit_response_body(const ConduitResponse*, size_t* out_len);
void             conduit_response_free(ConduitResponse*);   // for responses you build but don't return
void             conduit_string_free(char*);
```

### Dispatch contract

- A route / not_found / on_error handler **must** return non-NULL; if it returns
  NULL the capi substitutes a 500 (and, for routes, routes through the error
  handler with the message set via `conduit_capi_report_error`).
- `before` returns NULL to continue or a response to short-circuit.
- `after` receives ownership of the current response and returns one.
- All header names/values are run through `header_safe` (reject `< 0x20`, `0x7f`,
  and `:` in names) when building the `WebResponse`; status is clamped to 100–599;
  all incoming `char*` are validated as UTF-8 (invalid → replaced/empty).

## Swift layer (`code/packages/swift/conduit`)

```
Package.swift                              # systemLibrary CConduit + target Conduit + tests
Sources/CConduit/
    include/conduit_capi.h                 # copy of the capi header
    include/module.modulemap               # module CConduit { header "conduit_capi.h" export * }
    libconduit_capi.a                       # staged by BUILD (gitignored)
Sources/Conduit/
    Application.swift                      # the DSL: new, get/post/.., before/after, notFound, onError, set, bind
    Request.swift                          # struct over const ConduitRequest*
    Response.swift                         # status/headers/body + html/json/text/respond/halt/redirect
    Server.swift                           # serve / serveBackground / stop / localPort / running
    Halt.swift                             # ConduitHalt error thrown by halt()
Tests/ConduitTests/
    ResponseTests.swift
    RequestTests.swift
    ApplicationTests.swift
    ServerE2ETests.swift                   # launches server, hits it over a socket, alarm-style timeout guard
```

- **Handlers**: `(Request) throws -> Response`. The trampoline runs the closure;
  catches `ConduitHalt` → returns its response; catches any other error →
  `conduit_capi_report_error(msg)` + returns NULL so the capi invokes `on_error`.
- **Closure storage**: each handler box is `Unmanaged.passRetained`'d; the matching
  `ConduitCtxFree` does `.release()`, so closures are freed when the app/server is
  disposed. No leaks.
- **Response helpers** mirror the family: `html/json/text/respond/halt/redirect`;
  `redirect` rejects CR/LF in the location.

## Demo: `code/programs/swift/conduit-hello`

The 8-route demo (root, `/hello/:name`, POST `/echo`, `/search?q=`, `/redirect`,
`/halt`, `/down` via before-filter, `/error` → on_error, custom not_found),
matching the family. Foreground `serve()`.

## Tests (target 30+)

- **Response** unit tests: helper status/headers/body, redirect CR/LF guard.
- **Request** unit tests: a synthetic `ConduitRequest` built via a test-only capi
  entry, asserting method/path/query/param/header/body accessors.
- **Application** unit tests: chainable DSL, settings round-trip, halt/throw
  trampoline behavior (via `conduit handle` of a synthetic request, if exposed) —
  otherwise covered by E2E.
- **Server E2E**: bind on port 0, `serveBackground`, drive with a raw HTTP/1.0
  `Foundation`/POSIX socket client, assert `/`, `/hello/:name`, POST `/echo`,
  query, before-halt(503), throwing-handler→on_error(500), not_found(404),
  redirect(302). A watchdog `DispatchQueue.asyncAfter` (the `alarm()` analog)
  stops the server and fails fast if anything wedges.

## Security

- One audited boundary: header-injection defense (`header_safe`), status clamping
  (100–599), UTF-8 validation on every inbound `char*`, `catch_unwind` around
  every Swift→Rust re-entry so a Swift trap/throw can't unwind into Rust.
- Opaque-handle marshaling (accessor functions) avoids the percent-encoding
  smuggling surface the string-marshaled ports must defend.
- `serve_background`'s stored ctx is `Send + Sync`; the user closures are required
  to be thread-safe (same contract as the facade's `Fn + Send + Sync`).

## Build

- `conduit-capi`: registered in the rust workspace; `crate-type = ["staticlib",
  "cdylib", "lib"]`; emits `include/conduit_capi.h`. BUILD does `cargo build
  --release`.
- Swift package BUILD: build `conduit-capi` (release), copy `libconduit_capi.a`
  into `Sources/CConduit/`, then `swift build` / `swift test`. `BUILD_windows`
  guards on `swift` availability. `required_capabilities`: `["rust","swift",
  "cargo"]`. Demo BUILD declares `# build-tool: deps=swift/conduit`.
