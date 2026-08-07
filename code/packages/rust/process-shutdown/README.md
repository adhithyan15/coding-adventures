# process-shutdown

`process-shutdown` translates native process termination notifications into a
single cooperative Rust callback:

- Unix `SIGINT` becomes `ShutdownEvent::Interrupt`.
- Unix `SIGTERM` becomes `ShutdownEvent::Terminate`.
- Windows Ctrl+C and Ctrl+Break become `Interrupt`.
- Windows console close, logoff, and shutdown become `Terminate`.

The native callback only records a lock-free atomic event. User code runs at
most once on a named worker thread, outside the restricted signal-handler
context, with at most 10 milliseconds of polling latency. The listener is
process-global and exclusive. Dropping it restores the previous Unix handlers
or removes the Windows console handler; `uninstall` exposes restoration errors.

```rust
use process_shutdown::ShutdownListener;

let listener = ShutdownListener::install(|event| {
    eprintln!("cooperative shutdown requested: {event}");
})?;

// Run the application. Keep `listener` alive for the process lifetime.

listener.uninstall()?;
# Ok::<(), process_shutdown::ShutdownError>(())
```

The package deliberately owns the small amount of platform `unsafe` needed for
`sigaction` and `SetConsoleCtrlHandler`. Consumers, including the Chief daemon
binary, use only the safe API.

## Validation

```sh
cargo test -p process-shutdown -- --nocapture
cargo clippy -p process-shutdown --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p process-shutdown --no-deps
```
