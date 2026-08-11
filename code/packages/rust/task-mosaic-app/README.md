# task-mosaic-app

`task-mosaic-app` is the standard Mosaic application adapter for TaskApp. It owns
portable presentation state, delegates domain mutations and projections to the pure
`task-core` engine, and exports the fixed `mosaic-app-capi` ABI consumed by generated
native hosts.

The adapter is deliberately not a second task engine. `task-core` remains the single
trust boundary for task, project, scheduling, label, and note invariants. This crate
only maps TaskApp's MIL slots and semantic events to that typed API, giving every
generated backend the same complete initial prop snapshot, event behavior, and
versioned persistence format.

Build the native runtime library with:

```bash
cargo build --manifest-path code/packages/rust/Cargo.toml -p task-mosaic-app
```

Pass the resulting platform library to `mosaic-compile --runtime-library` when
emitting `code/programs/mosaic/task-app` with the `native-complete` profile.
