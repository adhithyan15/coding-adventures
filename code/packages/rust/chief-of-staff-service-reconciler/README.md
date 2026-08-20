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
`last_heartbeat_ns` and `last_restart_ns` bare, and those are fine because
nothing compares them *against a clock*. Heartbeat staleness uses the live
supervisor reading, never the stored one. There is one comparison that mixes a
durable reading with a live one -- `inactive_observation` may keep the previous
record's heartbeat beside a live `started_at_ns`, and `HostObservation::new`
checks that the heartbeat does not precede the start. It is benign, because a
previous run's monotonic readings are larger rather than smaller and
`start_instance` clears both fields, but it is the one place the rule is a
convention rather than a type. The property that holds is about comparison, not
storage, and an earlier version of this paragraph claimed the broader thing.

Both halves matter, and shipping only the first was a real bug. A window that
outlived its run wedged a host in a quarantine that re-armed every time it
lifted; fixing that while leaving the *deadline* unstamped moved the same wedge
one layer over, where a sixty-second quarantine written by a month-old daemon
kept the host down for a month. `QuarantineDeadline::Permanent` is a variant
rather than a `u64::MAX` sentinel for the same reason: a sentinel is something
arithmetic can saturate into by accident, and it did.

The honest consequence: the bound holds within a daemon run, and a daemon
restart hands every host a fresh budget. That is a weaker claim than "durable",
and an earlier version of this file made the stronger one. It is an acceptable
trade only for as long as a supervised host cannot force a daemon restart --
which is a property of the *walk*, not of this code, and is why the paragraph
below is part of this rule rather than a separate concern.

**A host that fails is an outcome, not an error.** The reconciler walks every
host per tick, so any per-host failure raised *out* of that walk takes every
other host down with it. `reconcile_all` therefore reports failures:
`ReconcileAction::Failed`, carrying the reason and the status the record holds
after the tick, with the walk continuing. A registry *listing* failure still
ends the tick. That is not because a listing failure is unattributable -- one
undecodable record fails the whole listing, so it is very much attributable --
but because nothing in this crate can currently read past it. Tracked in #12137.

This is load-bearing for the bound above, not a nicety. The daemon's scheduler
treats a reconcile error as terminal: it stops the server and returns. So while
one broken host could error out of the walk, a semi-trusted agent that crashed
its own bootstrap on purpose took down supervision for the whole machine -- and
because the shipped service definitions restart the daemon on failure, it came
back with a fresh `boot_id`, discarding every stored window and lifting every
intensity quarantine. Crash the daemon, get a fresh budget, repeat.

That made the restart bound bypassable by exactly the actor it exists to bound.
The bound's per-run scoping is only defensible because a supervised host cannot
force a daemon restart, and until this changed, it could. The claim and the code
now agree, for that actor: no per-host condition returns an error, no reachable
panic exists on the path, and the outcomes vector is bounded by `MAX_HOSTS`.

Two honest caveats. An undecodable registry record still stops the daemon
(#12137) -- not host-triggerable, but bit rot or a restored backup would do it.
And a `Failed` outcome is currently produced and not consumed: nothing logs it,
counts it, or backs off. A host whose `inspect` fails every tick is now retried
forever in silence, where it used to stop the daemon loudly. That trade is
correct on the security axis and worse on the operability one, and closing it is
#12138.

## Validation

```sh
cargo test -p chief-of-staff-service-reconciler -- --nocapture
cargo clippy -p chief-of-staff-service-reconciler --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-service-reconciler --no-deps
```
