# conduit-capi

A **reusable C ABI** for the [Conduit](../conduit) web framework (a Sinatra-style
facade over the Rust `web-core` engine, WEB08). This crate exposes the whole
framework over `extern "C"` so that any C-capable language can host a Conduit app
without re-implementing dispatch and marshaling against its own runtime.

## Why this exists

The managed-VM ports (Java JNI, Lua, Perl XS) each re-marshal requests and
re-audit the trust boundary against their host VM's C API. The remaining ports —
**Swift (WEB12), C++ (WEB13), Go (WEB14), C# (WEB15), F# (WEB16), Dart (WEB17),
Haskell (WEB18)** — are all C-ABI-capable, so they cross the boundary through
*this one crate* instead of re-wrapping the facade seven times. The trust
boundary is enforced once, here:

- **Header-injection defense**: header names/values with CR/LF/control bytes (and
  `:` in names) are dropped (`header_safe`).
- **Status clamping**: response status is clamped to 100–599.
- **UTF-8 validation**: every inbound `char*` is validated; invalid input is
  rejected rather than propagated.
- **Opaque-handle marshaling**: requests are read through accessor functions, so
  there is no percent-encoding string-smuggling surface.

## The interface

The stable header is [`include/conduit_capi.h`](include/conduit_capi.h). In brief:

| Group | Functions |
| ----- | --------- |
| App | `conduit_app_new`, `conduit_app_set_setting`, `conduit_app_get_setting`, `conduit_app_add_route`, `conduit_app_add_before`, `conduit_app_add_after`, `conduit_app_set_not_found`, `conduit_app_set_error_handler`, `conduit_app_free` |
| Server | `conduit_server_bind`, `conduit_server_serve`, `conduit_server_serve_background`, `conduit_server_stop`, `conduit_server_local_port`, `conduit_server_running`, `conduit_server_free`, `conduit_last_error` |
| Request | `conduit_request_method/path/query_string/content_type/remote_addr/error`, `conduit_request_body`, `conduit_request_param/query/header` |
| Response | `conduit_response_new`, `conduit_response_set_header`, `conduit_response_status`, `conduit_response_body`, `conduit_response_header_count/name/value`, `conduit_response_free`, `conduit_string_free` |

### Dispatch model

A handler is a C function pointer plus an opaque `ctx` (the host boxes its
closure and hands us the pointer) and a `ctx_free` destructor we call when the
owning app/server is freed:

```c
typedef ConduitResponse* (*ConduitHandler)(void* ctx, const ConduitRequest* req);
```

Returning NULL means "no response": *continue* for a before-filter, or — for a
route — route through the error handler using the message the host stashed via
`conduit_capi_report_error`.

### Threading

`embeddable-http-server` runs its reactor inline on the thread that calls
`serve()`, so foreground serving dispatches handlers on the caller's thread —
no lock required. `serve_background` spawns one OS thread. Host closures must be
thread-safe (the facade's `Fn + Send + Sync` contract).

## Build

```sh
cargo test --lib        # the pure-Rust helpers (header_safe, clamping, settings)
cargo build --release   # emits libconduit_capi.{a,dylib,so}
```

The crate is `crate-type = ["staticlib", "cdylib", "lib"]`: a static lib for
compile-time linkers (Swift, C++, Haskell), a dynamic lib for FFI loaders (Go
cgo, .NET P/Invoke, Dart FFI), and `lib` so `cargo test` can exercise the
helpers.
