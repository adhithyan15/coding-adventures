# Changelog

## Unreleased

- Replace the actor's private runtime/store ownership with the shared central
  controller authority for coherent reads, exact-revision pairing commits,
  restart recovery, and immediate shared-state publication.
- Prove central visibility and stale-request rejection before credential input,
  ONVIF verification, Vault, or journal activity after another transaction
  advances the controller revision.

## 0.1.0

- Add D23-authorized ONVIF credential provisioning from one-shot owner-only
  secret files.
- Verify exact installed-bridge correspondence through the pending session,
  endpoint-reference identifier, reviewed address, and authenticated inspection.
- Install only transaction-owned opaque references through recoverable runtime
  CAS and revision-bound replacement cleanup.
- Resolve all pending journals before actor startup and keep secret material out
  of messages, runtime state, journals, reports, and logs.
