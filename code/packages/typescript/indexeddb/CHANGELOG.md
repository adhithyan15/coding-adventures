# Changelog

## [0.2.0] - 2026-03-28

### Added

- **`StoreSchema.renamedFrom`** (`src/types.ts`) — optional field that triggers
  an automatic cursor-based migration in `IndexedDBStorage.onupgradeneeded`.
  When a store declares `renamedFrom: "old-name"`:
  1. The new store is created as normal (Phase 1, synchronous).
  2. If the old store still exists in the DB, a cursor opens on it, every
     record is `put` into the new store, and once the cursor is exhausted
     the old store is deleted — all within the versionchange transaction.
  Migration is idempotent: after the first successful upgrade the old store
  no longer exists, so the guard `objectStoreNames.contains(renamedFrom)`
  returns false and no migration runs on subsequent opens.

## [0.1.0] - Unreleased

### Added

- `KVStorage` interface — the contract for all storage backends
- `IndexedDBStorage` — Promise wrapper around the raw browser IndexedDB API
  with literate-programming documentation of every IndexedDB concept
- `MemoryStorage` — in-memory Map-of-Maps for tests and fallback
- `StoreSchema` and `IndexSchema` types for declarative schema definition
- Shared test suite (`storage.shared.ts`) that validates any KVStorage implementation
- 10 unit tests against MemoryStorage covering put, get, getAll, delete, overwrite,
  cross-store isolation, and nested object preservation
