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
- Treat retrying a *restart* claim as a restart, charging it to the window like
  any other. It previously took a separate path that spent nothing, so a host
  that kept inspecting as absent restarted without limit.
- Require an unfulfilled claim -- no recorded process -- before treating a
  `Starting` record as a first launch. A live process observed mid-bootstrap is
  also recorded as `Starting`, and was taking free starts.
- Refuse a zero `boot_id`, which is where a caller lands when every source of a
  unique value has failed.
- Stamp intensity quarantine deadlines with the daemon run that set them, so a
  deadline from a previous run does not hold a host down for that run's uptime.

