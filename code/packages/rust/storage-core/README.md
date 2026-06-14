# storage-core

Portable storage backend contract for Chief of Staff stores.

This crate defines the repository-owned record model that higher-level stores
such as ContextStore, ArtifactStore, SkillStore, and MemoryStore should target.
It is deliberately more structured than the existing CAS `BlobStore` trait:

- point reads by `(namespace, key)`
- opaque blob bodies plus JSON metadata
- compare-and-swap revisions
- stable prefix listing
- advisory leases for background work

## What this crate owns

- `StorageRecord`, `StorageRecordSummary`, `StoragePutInput`, `StorageStat`,
  `StoragePage`, `StorageSummaryPage`, `StorageSummaryPageOverview`,
  `StorageRecordInventorySummary`, and `StorageLease`
- `StorageLeaseSummary` and `StorageLeaseInventorySummary` for read-side
  advisory lease telemetry without exposing lease tokens
- `StorageBackend`, the backend trait implemented by local-folder, SQLite, and
  future backends
- `InMemoryStorageBackend`, a pure Rust backend for tests and examples
- repository-owned `StorageError` values
- validation helpers and backend conformance fixtures

## What this crate does not own

- actual persistence backends
- session-specific store logic
- SQLite schema design
- filesystem layout details

## Example

```rust
use coding_adventures_json_value::JsonValue;
use storage_core::{
    Revision, StorageBackend, StorageListOptions, StoragePutInput,
};

struct StubBackend;

impl StorageBackend for StubBackend {
    fn initialize(&self) -> Result<(), storage_core::StorageError> {
        Ok(())
    }

    fn get(
        &self,
        _namespace: &str,
        _key: &str,
    ) -> Result<Option<storage_core::StorageRecord>, storage_core::StorageError> {
        Ok(None)
    }

    fn put(
        &self,
        _input: StoragePutInput,
    ) -> Result<storage_core::StorageRecord, storage_core::StorageError> {
        unimplemented!()
    }

    fn delete(
        &self,
        _namespace: &str,
        _key: &str,
        _if_revision: Option<&Revision>,
    ) -> Result<(), storage_core::StorageError> {
        Ok(())
    }

    fn list(
        &self,
        _namespace: &str,
        _options: StorageListOptions,
    ) -> Result<storage_core::StoragePage, storage_core::StorageError> {
        Ok(storage_core::StoragePage::empty())
    }

    fn stat(
        &self,
        _namespace: &str,
        _key: &str,
    ) -> Result<Option<storage_core::StorageStat>, storage_core::StorageError> {
        Ok(None)
    }

    fn acquire_lease(
        &self,
        _name: &str,
        _ttl_ms: u64,
    ) -> Result<Option<storage_core::StorageLease>, storage_core::StorageError> {
        Ok(None)
    }
}

let input = StoragePutInput::new(
    "context",
    "sessions/demo.json",
    "application/json",
    JsonValue::Object(vec![]),
    br#"{"title":"demo"}"#.to_vec(),
)
.unwrap();

assert_eq!(input.namespace, "context");
```

`StorageBackend` also includes default `get_summary()` and `list_summaries()`
helpers. Backends can override them to serve read models without loading record
bodies, while simple implementations can rely on the default projection from
`stat()` and `list()`.
`StorageSummaryPage::overview()` provides aggregate counts and page-boundary
keys for cheap runtime telemetry over those body-free listings.
`StorageRecordInventorySummary` rolls record summaries into namespace, body,
metadata, content-family, timestamp, and key-boundary facts for store health
views that do not need bodies.
`StorageLease::summary_at()` and `StorageLeaseInventorySummary` provide the same
compact read-side shape for advisory lease status and expiry windows.

## Development

```bash
# Run tests
bash BUILD
```
