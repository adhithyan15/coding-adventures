# `coding_adventures_vault_pm_storage`

The provider-neutral immutable object-store contract for the local-first
password manager described by VLT-PM02.

The crate keeps product logic independent from local folders, Google Drive,
WebDAV, S3, browser storage, and future providers. Storage sees only opaque
32-byte bucket/object identifiers and bounded ciphertext bytes. Repository code
above this layer remains responsible for verifying hashes, signatures, commit
ancestry, and rollback anchors.

## Included

- `VaultObjectStore`, with initialize, exact get/stat, immutable put, ordered
  pagination, optional deletion, and optional change hints;
- closed, redacted value and error types;
- `BackendCapabilities`, used only to select optimization paths;
- a deterministic thread-safe `InMemoryObjectStore`;
- `FaultInjectingObjectStore`, including ambiguous-after-commit, stale-list,
  duplicate-list, typed-error, and corrupt-read actions;
- `ReplicaSetObjectStore`, VLT-PM00 §11.5's mirror decorator: one primary plus
  zero or more best-effort mirrors, write-time propagation that never blocks
  or fails the primary commit, mirror read fallback, and a per-mirror
  `ReplicaHealth` snapshot (§23 item 14);
- `run_conformance_suite`, reusable by every future adapter; and
- the embedded language-neutral `vault-pm-storage-v1.json` fixture.

## Example

```rust
use coding_adventures_vault_pm_storage::{
    BucketId, InMemoryObjectStore, ObjectBytes, ObjectId, PutImmutableOutcome,
    VaultLocator, VaultObjectStore,
};

let store = InMemoryObjectStore::new();
store.initialize(&VaultLocator::new([1; 32]))?;

let bucket = BucketId::new([2; 32]);
let object = ObjectId::new([3; 32]);
let bytes = ObjectBytes::new(b"opaque encrypted frame".to_vec())?;

assert_eq!(
    store.put_immutable(&bucket, &object, &bytes)?,
    PutImmutableOutcome::Created,
);
assert_eq!(store.get(&bucket, &object)?, Some(bytes));
# Ok::<(), coding_adventures_vault_pm_storage::StoreError>(())
```

## Adapter conformance

An adapter's integration test constructs a clean configured store and delegates
to the common runner:

```rust,ignore
let report = run_conformance_suite(|| MyProviderStore::for_test_account())?;
assert!(report.checks >= 19);
```

The suite checks the baseline contract and compares optional operation behavior
to the adapter's own capability report. Provider-specific sandbox tests remain
necessary for authentication, quotas, and native consistency behavior.

## Mirroring

```rust
use coding_adventures_vault_pm_storage::{InMemoryObjectStore, ReplicaSetObjectStore, VaultLocator, VaultObjectStore};

let replicas = ReplicaSetObjectStore::new(
    InMemoryObjectStore::new(),   // primary
    vec![InMemoryObjectStore::new()], // mirrors
);
replicas.initialize(&VaultLocator::new([1; 32]))?;
// Every put_immutable answers from the primary alone; mirror writes happen
// afterward and never fail the call. Inspect replicas.replica_health() for
// per-mirror attempted/succeeded counts and the most recent error, if any.
# Ok::<(), coding_adventures_vault_pm_storage::StoreError>(())
```

`ReplicaSetObjectStore::single(store)` is the zero-mirror construction used
everywhere a caller does not need replication; it is behaviorally identical to
using the wrapped store directly and passes the same conformance suite.

Deferred: the explicit `sync --wait` ceremony with a configurable
`one`/`all`/quorum durability target, and physical-delete propagation to
mirrors (left to a future replica-aware GC planner, VLT-PM00 §19.4).

## Deliberate exclusions

This package has no filesystem, network, clock, process, environment, entropy,
or key access. The `storage-core` bridge and concrete provider adapters belong
in separate packages. Retry, caching, metrics, commit publication, merge, and
garbage collection also live above or beside this contract.

## Development

```bash
bash BUILD
cargo clippy -p coding_adventures_vault_pm_storage -- -D warnings
```
