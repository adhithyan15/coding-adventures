"""
storage-sqlite — a SQLite byte-compatible file backend.

Phase 1 ships the bottom two layers of the file format:

- :mod:`storage_sqlite.header` — the 100-byte database header at the start
  of page 1.
- :mod:`storage_sqlite.pager` — page-at-a-time I/O with an LRU cache and a
  rollback journal.

Everything else (record codec, B-trees, sqlite_schema, the Backend adapter)
lands in subsequent phases.
"""

from storage_sqlite.errors import (
    CorruptDatabaseError,
    JournalError,
    StorageError,
)
from storage_sqlite.header import Header
from storage_sqlite.pager import PAGE_SIZE, Pager

__all__ = [
    "PAGE_SIZE",
    "CorruptDatabaseError",
    "Header",
    "JournalError",
    "Pager",
    "StorageError",
]
