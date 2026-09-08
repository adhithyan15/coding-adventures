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

Protocol 2 adds `mosaic_app_complete_effect(app, id, result, update)`: `id` is
a borrowed JSON integer and `result` a borrowed tagged JSON outcome (`ok`,
`cancelled`, or `failed`). The output follows the same owned-buffer rules as
dispatch. Completion does not consume a UI event sequence. Snapshot and restore
return `MOSAIC_STATUS_PENDING_EFFECTS` (8) with pending IDs in the diagnostic
while awaited work remains. Existing status numbers and v1 symbols are unchanged.
Generated native hosts still opt into v1 until their capability handlers migrate.

Callbacks must remain bound to the originating live handle. Cancel or detach
callbacks before destroying it; a freed raw C pointer must never be passed back
to the ABI, even if another allocation happens to occupy that address.
