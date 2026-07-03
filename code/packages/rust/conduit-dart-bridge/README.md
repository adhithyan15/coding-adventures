# conduit-dart-bridge

A Rust cdylib that bridges [conduit-capi](../conduit-capi)'s C ABI to Dart's FFI
runtime, solving the fundamental thread-safety problem of calling Dart closures from
Rust background OS threads.

## The Problem

`conduit-capi` dispatches HTTP request handlers from a background OS thread spawned
by `serve_background()`. Dart's `NativeCallable.isolateLocal` is only safe when
called from the Dart isolate's own thread — calling it from an independent OS thread
crashes with *"Cannot invoke native callback outside an isolate"*.

## The Solution

This crate implements a thread-safe **post+block** channel:

1. Dart calls `conduit_dart_init(NativeApi.initializeApiDLData)` once to populate the
   Dart Embedder DL API function pointer table (`Dart_PostCObject_DL`, etc.).
2. Dart creates a `RawReceivePort` and calls `conduit_dart_set_port(port.sendPort.nativePort)`.
3. Dart registers the bridge's handler/before/after/ctx_free function pointers (returned
   by `conduit_dart_*_fn()`) with conduit-capi instead of `NativeCallable` trampolines.
4. When conduit-capi calls a bridge handler from its Rust OS thread:
   - The bridge allocates a slot with a `Condvar` for the response.
   - It posts a `List<int>` message to Dart's event loop via `Dart_PostCObject_DL`
     (safe from any thread).
   - It blocks on the `Condvar`.
5. Dart's event loop fires `_onBridgeMessage`, looks up the Dart closure by integer ID
   (ctx), calls it, and calls `conduit_dart_complete(slotId, responsePtr)`.
6. `conduit_dart_complete` signals the `Condvar`, unblocking the Rust thread.

## ARM64 / Apple Silicon Note

`Dart_PostCObject_DL` is declared in `dart_api_dl.h` as a **global function-pointer
variable**, not a function. On ARM64, calling the DATA address directly causes
`SIGBUS BUS_ADRALN`. The `conduit_dart_bridge.c` shim correctly calls through the
pointer — Rust's `extern "C" { fn }` cannot safely reference this symbol.

## Usage

This crate is not published to crates.io — it is an internal bridge used by
`code/packages/dart/conduit` and `code/programs/dart/conduit-hello`.

## Building

```sh
cargo build -p conduit-dart-bridge --release
```

The Dart SDK headers in `dart/` are vendored from Dart 3.9.4 and compiled by
`build.rs` using `cc` crate.
