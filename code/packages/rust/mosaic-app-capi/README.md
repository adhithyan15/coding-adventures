# mosaic-app-capi

`mosaic-app-capi` exposes the backend-neutral `mosaic-app-runtime` contract through
a small, stable C ABI. Swift, Kotlin/JNI, Dart FFI, C#, and Qt/C++ bindings can all
use the same symbols and JSON envelopes.

The final application library invokes one macro with its Rust app type and factory:

```rust,ignore
mosaic_app_capi::export_mosaic_app!(TrestleApp, TrestleApp::default());
```

The Mosaic artifact builder will generate that wrapper. Application authors should
not write platform adapters or duplicate the state machine in host languages.

## Ownership

- `MosaicBytes` borrows caller-owned bytes for the duration of a call.
- Every non-null `MosaicBuffer` is allocated by Rust and must be released exactly
  once with `mosaic_buffer_free` before its output slot is reused.
- `MosaicHandle` is opaque and must be released exactly once with
  `mosaic_app_destroy`.
- On success, output buffers contain a JSON `Update` or `Snapshot` (`null` when the
  app does not support snapshots).
- On failure, the same output buffer contains a bounded UTF-8 diagnostic.

The ABI catches Rust panics. A panic while operating on a live app poisons that
handle, and later calls return `MOSAIC_STATUS_POISONED`; unknown state is never
allowed to continue silently.

See [`include/mosaic_app.h`](include/mosaic_app.h) for the C contract.
