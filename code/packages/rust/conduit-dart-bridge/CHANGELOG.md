# conduit-dart-bridge Changelog

## 0.1.0 — 2026-06-14

### Added
- Initial release: Dart FFI bridge for conduit-capi (WEB17).
- `conduit_dart_init(data)` — initialises the Dart DL API (wraps `Dart_InitializeApiDL`).
- `conduit_dart_set_port(port_id)` — registers the Dart `RawReceivePort` native port.
- `conduit_dart_handler_fn() / before_fn / after_fn / ctx_free_fn` — return C function pointers
  that Dart passes to `conduit_app_add_route` / `conduit_app_add_before` / etc. instead of
  `NativeCallable.isolateLocal` trampolines (which crash from Rust background threads).
- `conduit_dart_complete(slot_id, response_ptr)` — called by Dart after processing a bridge
  request to unblock the Rust thread that posted the message.
- Thread-safe request dispatch via `Dart_PostCObject_DL` + `Condvar`: bridge handlers
  post a message to Dart's event loop and block until Dart delivers the response.
- `conduit_dart_bridge.c` C shim: correctly calls through the `Dart_PostCObject_DL`
  function-pointer variable (not directly to the data address, which crashes on ARM64).
- `DL_INITIALIZED` atomic guard prevents calling `Dart_PostCObject_DL` before
  `Dart_InitializeApiDL` succeeds.
- Dart SDK headers vendored from Dart 3.9.4 in `dart/`.
