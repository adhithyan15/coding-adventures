# Changelog

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
