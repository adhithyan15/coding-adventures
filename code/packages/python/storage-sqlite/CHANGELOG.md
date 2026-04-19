# Changelog

## [0.1.0] - 2026-04-19

### Added

- Initial release — phase 1 of the SQLite byte-compatible file backend.
- `storage_sqlite.header.Header` — dataclass for the 100-byte SQLite
  database header. Field-by-field layout per the v3 spec; `from_bytes`
  parses, `to_bytes` serialises, `new_database` constructs a fresh header
  with the conventional defaults (page size 4096, UTF-8, schema format 4).
  Validates magic string, page size (power-of-two, 512..65536), and text
  encoding on parse.
- `storage_sqlite.pager.Pager` — page-at-a-time I/O against a file, with:
  - 1-based page numbers (page 0 is never read or written).
  - Fixed page size of 4096 in v1.
  - Small LRU page cache (default 32 pages).
  - `allocate()` to claim a fresh page number at the file tail.
  - Rollback journal at `<db>-journal`: on first write to a page within a
    transaction, the original page contents are copied to the journal.
    `commit()` fsyncs the journal, applies staged writes to the main file,
    fsyncs the main file, then deletes the journal. `rollback()` discards
    the staged writes and the journal.
  - Context-manager protocol (`with Pager.open(path) as pager:`).
  - Crash-recovery on `open`: if a hot journal is present, its contents are
    replayed back into the main file before the pager becomes usable.
