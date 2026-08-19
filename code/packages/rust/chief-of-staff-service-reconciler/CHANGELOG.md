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
- Charge every retry of an unfulfilled start claim, launch or restart, to both
  the window and the lifetime restart counter. Only a genuine first launch, from
  `Stopped` or from no record at all, is free. Claim retries previously spent
  nothing, and `Starting` with no process id is a fixed point, so a host that
  kept inspecting as absent restarted without limit.
- Report a per-host reconcile failure as a `Failed` outcome carrying its reason,
  instead of raising it out of the walk. A per-host error used to abort the tick
  for every other host and, through the daemon's scheduler, stop the daemon --
  which a supervised host could trigger on demand, and which reset every
  restart budget when the daemon came back.
- Refuse a zero `boot_id`, which is where a caller lands when every source of a
  unique value has failed.
- Stamp intensity quarantine deadlines with the daemon run that set them, so a
  deadline from a previous run does not hold a host down for that run's uptime.
- Report the stored status on a failed outcome. A failed start has already
  written its claim and its crash record, so the pre-tick status contradicted
  the register for exactly the hosts under investigation.
- Drop a carried-forward heartbeat when the live instance reports a later start.
  The pair is rejected at construction, and nothing durable changed on the
  failing tick, so the host was wedged for good.
