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
