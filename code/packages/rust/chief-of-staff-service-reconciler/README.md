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
inside a window, defaulting to five per sixty seconds. The window is a fixed
span that resets once it elapses, not a sliding window over individual restart
timestamps -- a sliding window needs every restart's timestamp, which is
unbounded state to persist per host. A host that
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
ledger as a unit does not make that mistake impossible -- `RestartLedger::NEVER`
is still there for the callers that genuinely mean it -- but it does make it
impossible to drop the window *by omission*, which is how it was dropped.

**Every durable reading this crate *compares* names the run that took it.** A
monotonic clock counts from daemon start, so a reading written by one run is not
on the same scale as the next run's -- a value recorded after an hour of uptime
looks an hour in the *future* to a daemon that has just started. Two durable
values are compared against the clock, and both carry a `boot_id`:
`RestartWindow` and `QuarantineDeadline::Until`. A reading from another run is
treated as elapsed rather than compared against.

Note the precise claim. The host record also persists `started_at_ns`,
`last_heartbeat_ns` and `last_restart_ns` bare, and those are fine *because
nothing compares them across runs* -- they are reported, or compared against
each other inside a single record. Heartbeat staleness uses the live supervisor
reading, never the stored one. The property that holds is about comparison, not
storage, and an earlier version of this paragraph claimed the broader thing.

Both halves matter, and shipping only the first was a real bug. A window that
outlived its run wedged a host in a quarantine that re-armed every time it
lifted; fixing that while leaving the *deadline* unstamped moved the same wedge
one layer over, where a sixty-second quarantine written by a month-old daemon
kept the host down for a month. `QuarantineDeadline::Permanent` is a variant
rather than a `u64::MAX` sentinel for the same reason: a sentinel is something
arithmetic can saturate into by accident, and it did.

The honest consequence: the bound holds within a daemon run, and a daemon
restart hands every host a fresh budget. Daemon restarts are not something a
supervised host gets to trigger, so that is the right trade -- but it is a
weaker claim than "durable", and an earlier version of this file made the
stronger one.

**Exceeding the bound quarantines one host; it does not raise an error.** The
reconciler walks every host per tick, so a per-host failure raised out of the
walk would take every other host down with it, and an agent able to crash itself
on demand could disable supervision for the whole deployment. Note that this is
true of the intensity bound specifically, not of the walk in general. A failed
supervisor `start`, `stop` or `inspect`, a `FutureObservation`, and any
`RegistryError` -- including one raised while validating an observation -- all
still propagate out of `reconcile_all` and abort the tick for every other host.
That is pre-existing and tracked separately in #12122.

## Validation

```sh
cargo test -p chief-of-staff-service-reconciler -- --nocapture
cargo clippy -p chief-of-staff-service-reconciler --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-service-reconciler --no-deps
```
