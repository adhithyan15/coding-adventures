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

## Restart intensity (D18R R2)

A restart policy says *whether* to restart a host; it says nothing about how
often. Without a second bound, a host that fails during startup is restarted as
fast as the tick loop runs, forever.

`ReconcileConfig::with_restart_intensity` bounds the rate: at most N restarts
inside a rolling window, defaulting to five per sixty seconds. A host that
exhausts its budget is quarantined with `restart intensity exceeded` instead of
being restarted, and the quarantine lifts one window later.

Two properties are worth stating plainly, because both are easy to get wrong:

- **The window is durable**, living in the host's registry record beside the
  lifetime restart count. A bound kept in memory would reset whenever the daemon
  did -- and a daemon that itself restarts is exactly the situation the bound
  exists for.
- **Exceeding the bound quarantines one host, it does not raise an error.** The
  reconciler walks every host per tick, so a per-host failure raised out of the
  walk would take every other host down with it. An agent able to crash itself
  on demand could then disable supervision for the whole deployment.

## Validation

```sh
cargo test -p chief-of-staff-service-reconciler -- --nocapture
cargo clippy -p chief-of-staff-service-reconciler --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-service-reconciler --no-deps
```
