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

Three properties are worth stating plainly, because each was got wrong once.

**Restart state travels as one value.** `RestartLedger` holds the lifetime
count, the last restart time, and the open window together, and
`HostObservation::new` takes it whole. This is not tidiness. The reconciler
rewrites a host's observation on many paths -- a phase change, a stop, a failed
start -- and every one of them must carry that bookkeeping forward. While the
window was a separate opt-in builder call, the restart path set it and every
other path silently dropped it, so a host that stayed up for a single tick
between crashes reset its own window and was never bounded at all. Passing the
ledger as a unit does not make that mistake less likely; it makes it
unspellable.

**A window belongs to one daemon run.** `start_ns` is read from a monotonic
clock that counts from daemon start, so a value written by one run is not on the
same scale as the next run's readings -- a window opened after an hour of uptime
looks an hour in the *future* to a daemon that has just started. Each window
therefore records the `boot_id` of the run that opened it, and a window from
another run is discarded rather than compared against. The honest consequence:
the bound holds within a daemon run, and a daemon restart hands every host a
fresh budget. Daemon restarts are not something a supervised host gets to
trigger, so that is the right trade -- but it is a weaker claim than "durable",
and the earlier version of this file made the stronger one.

**Exceeding the bound quarantines one host; it does not raise an error.** The
reconciler walks every host per tick, so a per-host failure raised out of the
walk would take every other host down with it, and an agent able to crash itself
on demand could disable supervision for the whole deployment. Note that this is
true of the intensity bound specifically, not of the walk in general: a
supervisor start or stop that *fails* still propagates out of `reconcile_all`.
That is pre-existing and tracked separately.

## Validation

```sh
cargo test -p chief-of-staff-service-reconciler -- --nocapture
cargo clippy -p chief-of-staff-service-reconciler --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-service-reconciler --no-deps
```
