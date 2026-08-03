# chief-of-staff-daemon-api

`chief-of-staff-daemon-api` is the authenticated WebSocket control surface for
the D18 Chief daemon. It translates bounded versioned JSON requests into the
host-lifecycle operations of `chief-of-staff-orchestrator-core` while keeping
authentication policy injected and connection-local.

The package accepts text messages only, parses untrusted JSON through the
fallible depth-capped parser, rejects duplicate or unknown fields, and encodes
64-bit revisions and timestamps as strings. It does not retain credentials or
expose adapter diagnostics.

`DaemonClient` provides the matching blocking client for operator adapters. It
constructs typed host-lifecycle requests, checks response versions and IDs, and
retains stable remote error codes without exposing remote details through its
diagnostic display.

## Validation

```sh
cargo test -p chief-of-staff-daemon-api -- --nocapture
cargo clippy -p chief-of-staff-daemon-api --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-daemon-api --no-deps
```
