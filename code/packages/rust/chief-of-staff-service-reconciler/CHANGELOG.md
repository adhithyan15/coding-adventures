# Changelog

## 0.1.0

- Add deterministic desired-state reconciliation over the CAS-backed service registry.
- Add authoritative supervisor observations with package identity and heartbeat validation.
- Add first-launch, restart-policy, quarantine, and crash-recovery state transitions.
- Claim one bounded start or stop mutation per host before invoking the supervisor.
