# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-06-14

### Added

- `irc-server-capi` — a reusable, flat **C ABI** over the all-Rust IRC engine
  (`irc-net-reactor`), so any C-capable language can embed the high-performance
  IRC server. The Swift package `code/packages/swift/IrcServerNative` is the
  first consumer; the same artifacts work for C, C++, Go/cgo, C# P/Invoke, and
  Dart FFI.
- `extern "C"` surface (control surface only — no callbacks): `irc_server_new`
  (NULL on bind failure), `irc_server_serve` (foreground, blocks),
  `irc_server_serve_background`, `irc_server_stop`, `irc_server_running`,
  `irc_server_local_host` (heap C string), `irc_server_local_port`,
  `irc_server_string_free`, `irc_server_free`.
- C header `include/irc_server_capi.h` (opaque `IrcServer` handle, ownership and
  threading contracts documented inline).
- `crate-type = ["staticlib", "cdylib", "lib"]` so the crate emits a `.a` for
  SwiftPM/C compile-time linking, a `.dylib`/`.so` for dynamic FFI, and a Rust
  `lib` so `cargo test --lib` can drive the ABI directly.
- `cargo test --lib` suite exercising the ABI from Rust: NULL-handle safety on
  every entry point, NULL/non-UTF-8 string inputs falling back to defaults, and
  the full real-socket broadcast scenario (two clients register, JOIN `#test`,
  one PRIVMSGs, the other receives it — proving the in-process mailbox fan-out).

### Safety / robustness

- All untrusted C strings are validated as UTF-8 (`CStr::to_str`); NULL or
  non-UTF-8 inputs fall back to safe defaults rather than forwarding raw bytes
  into the engine. `max_connections` is clamped to at least 1.
- Every `extern "C"` function wraps its body in `catch_unwind` so a Rust panic
  can never unwind across the C ABI (undefined behaviour).
- `serve`/`serve_background` run the blocking loop on an **owned clone** of the
  engine, so the background thread never dereferences the handle. `running` is set
  before the background thread is spawned so a racing `stop()` is not lost.
- **Cross-thread shutdown is data-race-free.** Every entry point takes only a
  shared `&*srv` reference (never `&mut`), so a foreground `serve` and a
  cross-thread `stop`/`running`/`local_*` never form aliasing `&mut`s. The only
  post-construction mutable field (the background join handle) is guarded by a
  `Mutex`; `running` is atomic; `server`/`local_host`/`port` are read-only after
  `new`. `irc_server_free` takes ownership and must happen-after every other call
  on the handle has returned (the standard C ownership contract, documented in the
  header) — the Swift wrapper upholds this automatically via ARC.
