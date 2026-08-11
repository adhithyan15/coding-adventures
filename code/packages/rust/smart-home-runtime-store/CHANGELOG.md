# Changelog

- Add an exact-revision combined snapshot write for long-lived controller
  owners that must reject external revision drift instead of adopting it.

## Unreleased

- Add D23-authorized, revision-guarded pairing completion that persists a
  candidate runtime before swapping live state and reports the prior opaque
  Vault reference for explicit credential replacement cleanup.
- Add revision-guarded retained identity migration that persists a complete
  migrated candidate before swapping the caller's live runtime.
- Reject automation definitions and execution state that still contain an
  exact source device or entity identity.
- Prove successful restart recovery and storage-conflict rollback with the
  local-folder backend.

## 0.1.0

- Add versioned, compare-and-swap smart-home runtime snapshots.
- Restore registry, state, history, pairing, desired state, and automation
  definitions from durable storage.
- Persist optional versioned automation runtime state for trigger idempotency
  and audit restoration while retaining backward compatibility.
- Prove restart recovery with the local-folder storage backend.
