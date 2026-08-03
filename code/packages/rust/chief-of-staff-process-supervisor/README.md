# chief-of-staff-process-supervisor

`chief-of-staff-process-supervisor` is the concrete OS-process authority for
D18 Chief hosts. It re-verifies a registered signed package immediately before
each spawn, owns and reaps the child, bootstraps a fresh UUID-v7 secure channel,
and reports readiness and heartbeat only after authenticated control messages.

The crate implements the dependency-light service reconciler's
`HostSupervisor` interface. It deliberately leaves durable restart policy,
scheduling, backoff, and registry updates to the reconciler and runnable
orchestrator.

## Validation

```sh
cargo test -p chief-of-staff-process-supervisor -- --nocapture
cargo clippy -p chief-of-staff-process-supervisor --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-process-supervisor --no-deps
```
