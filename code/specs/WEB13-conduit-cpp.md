# WEB13 — Conduit for C++ (over the reusable `conduit-capi` C ABI)

## Summary

Port the **Conduit** web framework to **C++**, as a header-only wrapper over the
reusable `conduit-capi` C ABI introduced in WEB12. C++ is the second consumer of
that ABI (after Swift); the trust boundary was already audited once in the Rust
crate, so this port is a thin, idiomatic C++ layer — no new FFI surface.

## Architecture

```
C++ DSL (conduit::Application / Request / Response / Server)
    handlers are std::function<Response(const Request&)>
    │  extern "C" trampoline + heap-boxed closure (ctx) + ctx_free destructor
    ▼
conduit-capi (Rust cdylib + staticlib, the C ABI from WEB12)
    ▼
conduit (WEB08 facade) → web-core → embeddable-http-server → tcp-runtime
```

The wrapper is a single header `include/conduit/conduit.hpp`. It `#include`s
`conduit_capi.h` (via `-I .../conduit-capi/include`) and links
`libconduit_capi.a`.

## Design

- **Handlers**: `Handler = std::function<Response(const Request&)>`;
  `BeforeHandler = std::function<std::optional<Response>(const Request&)>` (nullopt
  = continue); `AfterHandler = std::function<Response(const Request&, Response)>`.
- **Closure boxing**: each handler is `new`'d on the heap; its pointer is the
  opaque `ctx`. A `ctx_free` trampoline `delete`s it when the app/server is freed.
- **Trampolines**: `extern "C" inline` functions (C language linkage so the
  function-pointer types match the ABI typedefs exactly under `-Wpedantic
  -Werror`; `inline` so the single header is include-safe across TUs). They catch
  `Halt` → return its response; catch any other exception → `conduit_capi_report_error`
  + return NULL (so the engine routes through the error handler). **No C++
  exception ever unwinds across the C boundary.**
- **Response/Request** are value types over the C accessors. `Response::toC()`
  builds a native response; `Response::fromC()` reads one back (for after-hooks).
- **RAII**: `Server` and `Application` own their native handles and free them in
  their destructors; `bind` consumes the application.

## Threading

Inherited from `conduit-capi`: foreground `serve()` dispatches on the calling
thread; `serveBackground()` runs the reactor on one OS thread. C++ closures are
thread-safe to call. The E2E test uses `serveBackground` + a `std::thread`
watchdog that stops the server after a deadline.

## Package layout

```
code/packages/cpp/conduit/
    include/conduit/conduit.hpp   # the whole wrapper
    tests/conduit_test.h          # zero-dep assertion harness
    tests/test_response.cpp       # helpers + native round-trip
    tests/test_application.cpp    # DSL / settings / bind
    tests/test_server.cpp         # E2E (POSIX socket client + watchdog)
    tools/run-tests.sh            # build C ABI, query native-static-libs, compile+run
    CMakeLists.txt                # documented end-user path
    BUILD / BUILD_windows / README.md / CHANGELOG.md / required_capabilities.json
code/programs/cpp/conduit-hello/  # 8-route demo + smoke test
```

## Build

- `tools/run-tests.sh` (invoked by `BUILD`): `cargo build` the C ABI, extract the
  platform `native-static-libs` via `cargo rustc -- --print native-static-libs`,
  compile each test with `clang++`/`g++` (`-std=c++17 -Wall -Wextra -Wpedantic
  -Werror -pthread`) linking `libconduit_capi.a` + those libs.
- `BUILD` declares `# build-tool: deps=rust/conduit-capi` so the build graph pulls
  the Rust crate in (the runner gets `cargo`); the C++ package is discovered as
  "unknown" language so no further dep validation applies.
- `BUILD_windows` skips (the E2E uses POSIX sockets). `required_capabilities`:
  `["rust","cargo"]`.

## Tests (target 30+)

16 test functions across the three suites, the E2E alone asserting ~18 behaviors
(routes, params, POST echo + content-type passthrough, query, before-halt 503,
throwing-handler→onError 500, custom 404, redirect 302, after-hook header
stamping). Combined with the inherited 5 `conduit-capi` Rust tests, well over 30
assertions.

## Security

Inherited from `conduit-capi`: header-injection defense, status clamping, UTF-8
validation, panic isolation. C++-specific: `redirect` throws on CR/LF;
trampolines catch every exception so none crosses the C boundary.
