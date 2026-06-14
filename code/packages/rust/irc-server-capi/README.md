# irc-server-capi

A reusable **C ABI** for the all-Rust IRC engine
([`irc-net-reactor`](../irc-net-reactor)) — the bridge that lets any C-capable
language embed the high-performance IRC server.

## Why this crate exists

The Python, Ruby, Node, Java, Elixir, and Perl bindings each speak their host
VM's own native protocol (CPython C-API, N-API, JNI, Erlang NIF, Perl XS) through
a dedicated bridge crate. **Swift has no such bridge in this repo** — but Swift,
like C, C++, Go (cgo), C# (P/Invoke), Dart (FFI), and Zig, speaks the plain **C
ABI** fluently.

So rather than write a Swift-specific bridge, this crate exposes the engine
through a flat `extern "C"` surface that *any* C-FFI language can import. The
Swift package [`code/packages/swift/IrcServerNative`](../../swift/IrcServerNative)
is the first consumer; the same `.a`/`.dylib` + header works for the others too.

This mirrors `conduit-capi`, which does the same for the Conduit web framework.

## The ABI (control surface only — no callbacks)

Because **all** IRC and TCP logic lives in Rust, the binding is a pure lifecycle
controller:

| function                      | meaning                                          |
|-------------------------------|--------------------------------------------------|
| `irc_server_new`              | bind a server, return an opaque handle (NULL on error) |
| `irc_server_serve`            | run the loop on the **calling** thread (blocks)  |
| `irc_server_serve_background` | run the loop on a background Rust thread          |
| `irc_server_stop`             | signal stop + join the background thread          |
| `irc_server_running`          | is the loop running?                             |
| `irc_server_local_host`       | bound IP as a heap C string (caller frees)       |
| `irc_server_local_port`       | bound TCP port (the OS port when bound to 0)     |
| `irc_server_string_free`      | free a string returned by this library           |
| `irc_server_free`             | stop, join, and free the handle                  |

The C header is [`include/irc_server_capi.h`](include/irc_server_capi.h).

## Trust boundary

Every pointer crossing the boundary is untrusted:

- **Strings** are validated as UTF-8 (`CStr::to_str`); NULL / non-UTF-8 inputs
  fall back to safe defaults — raw bytes never reach the engine.
- **Numbers** are clamped (`max_connections >= 1`); `port` is a `u16`.
- **Every function** wraps its body in `catch_unwind` — a Rust panic must never
  unwind across the C ABI (undefined behaviour).
- **`serve`/`serve_background`** run an **owned clone** of the engine, so the
  background thread never dereferences the handle and it can't dangle.
- **Ownership contract:** `irc_server_local_host` strings → `irc_server_string_free`;
  the handle → `irc_server_free` exactly once.

## Build

```
cargo test --lib       # drives the C ABI directly from Rust (broadcast + lifecycle)
cargo build --release  # emits libirc_server_capi.a (staticlib) + .dylib (cdylib)
```

The crate is a member of the Rust workspace and builds three artifact kinds:
`staticlib` (compile-time link for SwiftPM / C), `cdylib` (dynamic link for cgo /
P/Invoke / Dart FFI), and `lib` (so `cargo test --lib` can exercise the ABI).

## Layer position

```
IrcServerNative (Swift)  +  any other C-FFI language
        ↓ irc_server_capi.h  (this crate — flat C ABI)
irc-net-reactor   (Rust IRC engine, in-process broadcast)
        ↓
tcp-runtime → transport-platform → kqueue / epoll / IOCP
```
