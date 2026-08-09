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

The C ABI and WebAssembly bridge are intentionally separate follow-on crates. They
will encode these same types without exposing Rust layouts across the boundary.
