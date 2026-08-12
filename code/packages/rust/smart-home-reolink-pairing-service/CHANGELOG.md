# Changelog

## Unreleased

- Replace the actor's private runtime/store ownership with the shared central
  controller authority for coherent reads, exact-revision pairing commits,
  restart recovery, and immediate shared-state publication.
- Prove central visibility and stale-request rejection before credential input,
  Reolink verification, Vault, or journal activity after another transaction
  advances the controller revision.

## 0.2.0

- Extend exact recoverable credential provisioning to installed Reolink NVRs.
- Require authenticated NVR product type, exact NVR model and serial, exact
  per-channel `typeInfo`, and supported executable `abilityChn.snap`
  correspondence before any pairing transaction write.
- Continue using operation-scoped query tokens with explicit logout after every
  authenticated success or failure.

## 0.1.0

- Add D23-authorized Reolink credential provisioning from one-shot owner-only
  secret files.
- Verify exact installed-bridge correspondence through the pending session,
  host-owned canonical-name and reviewed-address pinning, authenticated CGI
  inspection, stable serial, exact physical channels, and an awake online
  `RLC-*` JPEG snapshot channel.
- Keep CGI query tokens process-local, require explicit logout, and leave NVR
  credentials gated on per-channel model identity.
- Persist only the snapshot host's versioned username/password envelope.
- Install only transaction-owned opaque references through recoverable runtime
  CAS and revision-bound replacement cleanup.
- Resolve all pending journals before actor startup and keep secret material out
  of messages, runtime state, journals, reports, and logs.
