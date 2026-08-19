# Changelog

## 0.1.0

- Add deterministic desired-state reconciliation over the CAS-backed service registry.
- Add authoritative supervisor observations with package identity and heartbeat validation.
- Add first-launch, restart-policy, quarantine, and crash-recovery state transitions.
- Claim one bounded start or stop mutation per host before invoking the supervisor.
- Add an explicit empty-registry report constructor for adapters and deterministic tests.
- Bound how often one host may be restarted (D18R R2). A host that exhausts its
  budget inside the window is quarantined with `restart intensity exceeded` rather
  than restarted again; the quarantine lifts one window later.
- Add `ReconcileConfig::with_restart_intensity`, defaulting to five restarts per
  sixty seconds. A zero window or count is refused rather than silently meaning
  "never restart".
- Require a `boot_id` in `ReconcileConfig::new`, identifying the daemon run. A
  restart window recorded by a previous run is discarded rather than measured
  against this run's monotonic clock.
- Consult the bound when retrying a start claimed on an earlier tick, so a host
  left durably in `Starting` or `Restarting` cannot start unbounded.
- Stamp intensity quarantine deadlines with the daemon run that set them, so a
  deadline from a previous run does not hold a host down for that run's uptime.
- Charge a restart for retrying a restart claim that no window from this run
  vouches for, instead of granting it free. Retrying a first-launch claim is
  still free, since a host that has never run has not restarted.
