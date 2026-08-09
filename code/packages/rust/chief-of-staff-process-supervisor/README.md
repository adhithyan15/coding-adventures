# chief-of-staff-process-supervisor

`chief-of-staff-process-supervisor` is the concrete OS-process authority for
D18 Chief hosts. It re-verifies a registered signed package immediately before
each spawn, owns and reaps the child, bootstraps a fresh UUID-v7 secure channel,
delivers the exact relevant public package trust over that authenticated channel,
obtains launch bindings from an injected manifest-blind pipeline authority, and
reports readiness and heartbeat only after the child receives both inputs and
independently verifies the package with that trust.
The single configured host executable receives a final reserved
`--package-runtime deno|skill` pair derived from that verified package snapshot,
giving the production host a fail-closed runtime-dispatch seam without ambient
environment or registry input.

The same authenticated session now carries the host data plane. Child-side
helpers serialize bounded channel receive/publish/acknowledge and provider-neutral
completion exchanges. The supervisor retains one authenticated request per host
until an injected service adapter answers through `respond_data_plane`, preserving
exact correlation across real cross-platform process pipes without adding an
unauthenticated side channel.

Its keyring and X3DH identity are shared through owned `Arc` handles, and its
session source is `Send`, so the complete supervisor can move with the daemon's
threaded control plane without copying secret key material.

The crate implements the dependency-light service reconciler's
`HostSupervisor` interface. It deliberately leaves durable restart policy,
scheduling, backoff, and registry updates to the reconciler and runnable
orchestrator. The production storage-backed provider revalidates exact host
registration, immutable pipeline channel claims, active topology, and directional
membership before every spawn. A fail-closed provider remains available to
compositions that intentionally have no durable pipeline wiring.
Channel endpoint and LLM service implementations remain injected by
the concrete host/daemon composition rather than entering this process-authority
crate.

## Validation

```sh
cargo test -p chief-of-staff-process-supervisor -- --nocapture
cargo clippy -p chief-of-staff-process-supervisor --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-process-supervisor --no-deps
```
