# Changelog

## Unreleased

- Replace the actor's private runtime/store ownership with the shared central
  controller authority for coherent reads, exact-revision pairing commits,
  restart recovery, and immediate shared-state publication.
- Prove central visibility and stale-request rejection before credential input,
  ZoneMinder verification, Vault, or journal activity after another transaction
  advances the controller revision.

## 0.1.0

- Add D23-authorized ZoneMinder credential provisioning from one-shot owner-only
  secret files.
- Verify exact installed-bridge correspondence through the pending session,
  HTTPS endpoint identifier, authenticated API 2.0 inspection, and installed
  positive monitor identifiers.
- Persist only the versioned username/password envelope; access and refresh
  tokens remain process-local verification material.
- Install only transaction-owned opaque references through recoverable runtime
  CAS and revision-bound replacement cleanup.
- Resolve all pending journals before actor startup and keep secret material out
  of messages, runtime state, journals, reports, and logs.
