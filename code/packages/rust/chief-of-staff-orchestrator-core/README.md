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

Channel mutation calls carry a validated Trust Checker request context. The
provided `TrustCheckingChannelWiring` adapter resolves the current tier for the
channel and every originator/receiver through an injected authoritative
resolver, then submits that exact resource set to the Trust Checker. A
domain-separated SHA-256 resource fingerprint binds approval to the operation,
channel UUID, public keys, membership, creation time, key epoch, and lifecycle.
Resolution or approval failure occurs before durable channel storage changes.
Production continues to inject `DenyChannelWiring` until a reviewed provider
and tier resolver are configured by the daemon.

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
