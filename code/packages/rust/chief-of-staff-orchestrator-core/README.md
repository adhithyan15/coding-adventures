# chief-of-staff-orchestrator-core

`chief-of-staff-orchestrator-core` is the transport-independent application
layer for the D18 Chief daemon. It composes durable host intent, deterministic
reconciliation, authoritative process supervision, and authorized channel
topology behind bounded calls suitable for a later WebSocket server and CLI.

The core owns its shared storage handle, and the production process composition
owns shared keyring and zeroizing identity handles. It can therefore move into
the threaded daemon API as a `Send + 'static` control plane without leaked or
self-referential process-lifetime allocations.

The core is keyless and payload-blind. Verified child processes own host-runtime
execution and channel endpoint keys; the parent stores topology and coordinates
lifecycle without reading agent messages or vault secrets.

Package reload retains the stable host name but requires stopped durable intent
and fresh absent or exited supervisor authority. One revision-CAS transaction
then replaces package identity, clears stale observation, and stores whether the
next reconciliation tick should start the replacement or leave it stopped.

## Validation

```sh
cargo test -p chief-of-staff-orchestrator-core -- --nocapture
cargo clippy -p chief-of-staff-orchestrator-core --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-orchestrator-core --no-deps
```
