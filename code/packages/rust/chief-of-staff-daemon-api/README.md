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

`wire_host_pipeline` and `unwire_host_pipeline` are independently authorized
session operations. For either mutation, the API constructs Trust Checker
context from the protocol request ID and the authenticated session's stable
requester identity; request JSON cannot supply or override that identity. The
wire request carries the exact registered package identity, a lowercase-hex
UUID-v7 pipeline ID, a lowercase-hex agent ID, canonical named channel
directions and UUIDs, and optional bounded Level 1 model settings. The control
plane then performs exact-resource Trust Checker authorization before durable
pipeline claims or bindings can change.

Successful wire responses return the complete canonical binding and its
revision. Unwire is idempotent and reports whether a binding existed, returning
the removed binding when it did. All nested objects reject duplicate, missing,
or unknown fields, and typed client calls construct the matching schema.

`reload_host` is separately authorized from registration. It can atomically
replace package identity only after stopped durable intent and fresh inactive
supervisor evidence; a running replacement is launched by later reconciliation.

`DaemonApi::reconcile_once` is the local scheduler boundary. It runs convergence
through the same serialized control plane as authenticated requests without
pretending that the daemon process is a remote session.

## Validation

```sh
cargo test -p chief-of-staff-daemon-api -- --nocapture
cargo clippy -p chief-of-staff-daemon-api --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-daemon-api --no-deps
```
