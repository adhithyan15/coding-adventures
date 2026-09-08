# mosaic-app-runtime

`mosaic-app-runtime` is the application boundary shared by every Mosaic backend.
An app implements `MosaicApp` once in Rust. Generated native and web hosts send
semantic events to `MosaicRuntime` and receive revisioned props, effects, and
accessibility announcements.

The runtime, rather than application code, owns protocol metadata. It rejects:

- an incompatible protocol version;
- dispatch, restore, or snapshot calls before startup;
- a second startup;
- duplicate, stale, or skipped event sequence numbers; and
- sequence or revision overflow.

Rejected events and application errors do not advance the sequence or revision.
Together with `MosaicApp`'s transactional-error contract, that lets a host retry a
failed event without desynchronizing the bridge.

```rust
use mosaic_app_runtime::{
    AppUpdate, Event, MosaicApp, MosaicRuntime, Platform, Snapshot, StartContext,
};

# #[derive(Debug)]
# struct AppError;
# impl std::fmt::Display for AppError {
#     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
#         f.write_str("app error")
#     }
# }
# impl std::error::Error for AppError {}
# struct Counter;
impl MosaicApp for Counter {
    type Error = AppError;

    fn start(&mut self, _context: StartContext) -> Result<AppUpdate, Self::Error> {
        Ok(AppUpdate::new(serde_json::json!({ "count": 0 })))
    }

    fn dispatch(&mut self, _event: Event) -> Result<AppUpdate, Self::Error> {
        Ok(AppUpdate::new(serde_json::json!({ "count": 1 })))
    }

    fn snapshot(&self) -> Result<Option<Snapshot>, Self::Error> {
        Ok(None)
    }

    fn restore(&mut self, _snapshot: Snapshot) -> Result<AppUpdate, Self::Error> {
        Ok(AppUpdate::new(serde_json::json!({ "count": 0 })))
    }
}

let mut runtime = MosaicRuntime::new(Counter);
let first = runtime.start(StartContext::new("en-US", Platform::Linux))?;
assert_eq!(first.revision, 1);
# Ok::<(), mosaic_app_runtime::RuntimeError<AppError>>(())
```

The C ABI and WebAssembly bridge encode these types without exposing Rust layouts.

## Awaited capabilities (protocol 2)

Startup defaults to protocol 1 for existing generated hosts. Opt into
`EFFECT_PROTOCOL_VERSION` (2) in `StartContext` to emit effects with explicit
`delivery: "notify"` or `"await"`. Protocol 1 update serialization retains the
original effect shape without that field; emitting Await to a v1 host is an error.

An app implements `complete_effect(id, result)` for awaited work. Results are
exactly one of `{"ok": <payload>}`, `{"cancelled": {}}`, or
`{"failed": {"message": "..."}}`. Accepted completion retires the pending ID
and advances the render revision without consuming a UI event sequence. An
application error leaves the result pending and must not change app state.
Retry the corrected completion, not the external operation. The default trait
method explicitly reports unsupported completion. Unknown, duplicate and Notify
IDs never reach the application callback.

Every v2 effect ID must be a positive JavaScript-safe integer, unique in its
batch and greater than all IDs from previous updates on that instance. The app
owns allocation and must not reset it during same-instance restore. Invalid
outbound effects poison the instance because arbitrary application mutations
cannot be rolled back. Recreate the instance from its last settled checkpoint.

`pending_effects()` reports outstanding Await IDs. While any remain, standalone
snapshot and restore return typed `RuntimeError::PendingEffects` before calling
the app. Notify does not block checkpoints. Chained Await requests remain pending;
there is no implicit timeout retirement or I/O replay. Capture Save bytes before
requesting a write, and apply Open bytes transactionally inside accepted completion.
See UI47 section 8 for the complete persistence contract.
