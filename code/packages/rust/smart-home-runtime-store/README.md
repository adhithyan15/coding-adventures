# smart-home-runtime-store

`smart-home-runtime-store` persists a versioned `SmartHomeRuntime` snapshot
through the repository-owned `StorageBackend` contract. It restores normalized
topology, state, event and command history, pairing sessions, desired state,
automation definitions, consumed trigger occurrences, and automation audit
after a process restart.

Live discovery workers and event subscriptions are deliberately process-local
and are rebuilt by their owners.

Retained identity migration uses an expected storage revision. The store first
builds and validates a migrated runtime candidate, persists its complete durable
snapshot with compare-and-swap, and replaces the caller's live runtime only
after that write succeeds. A stale revision leaves both the live runtime and
the durable record unchanged. Supplied automation definitions and execution
state must already use destination identities; exact source-ID references are
rejected before either runtime or storage mutation.

Pairing completion follows the same persist-before-swap rule. It executes the
D23 `CompletePairing` authorization and mutation against a cloned runtime,
persists the complete candidate only at the caller's expected revision, and
then swaps live state. The API accepts only an opaque `VaultRef` and returns the
prior bridge reference for explicit post-commit credential cleanup. Coordinating
the separate sealed-Vault write and crash-recoverable old-record deletion
remains the credential host's responsibility.
