# Changelog

## 0.1.0

- Add versioned, compare-and-swap smart-home runtime snapshots.
- Restore registry, state, history, pairing, desired state, and automation
  definitions from durable storage.
- Persist optional versioned automation runtime state for trigger idempotency
  and audit restoration while retaining backward compatibility.
- Prove restart recovery with the local-folder storage backend.
