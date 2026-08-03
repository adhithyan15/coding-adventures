# chief-of-staff-service-reconciler

`chief-of-staff-service-reconciler` is the deterministic lifecycle bridge
between D18's durable service registry and an authoritative host-process
supervisor.

It ignores cached PIDs for liveness, validates current supervisor evidence,
checks the live package hash, applies restart policy, detects stale heartbeats,
and CAS-writes the resulting observation. Start and stop operations are exposed
through an injected trait, so the crate performs no process, filesystem,
network, clock, or random access itself.

Each host tick performs at most one supervisor mutation. Transitional state is
claimed with CAS before that mutation, making concurrent operator edits and
orchestrator crash recovery converge on subsequent ticks.

## Validation

```sh
cargo test -p chief-of-staff-service-reconciler -- --nocapture
cargo clippy -p chief-of-staff-service-reconciler --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-service-reconciler --no-deps
```
