# Changelog

## [0.1.0] - Unreleased

### Added
- `Storage` interface with four capabilities: connection, CRUD, SQL querying, transactions
- `MemoryStorage` implementation — in-memory Map-of-Maps with full Storage support
- Supporting types: `StorageConfig`, `StoreSchema`, `IndexSchema`, `StorageRecord`
- SQL query support via `sql-execution-engine` DataSource bridge
- Transaction support via snapshot-and-restore pattern
- Re-exports `QueryResult` and `SqlValue` from `sql-execution-engine`
- Comprehensive test suite covering CRUD, querying, and transaction rollback
