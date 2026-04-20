"""
storage-sqlite — a SQLite byte-compatible file backend.

Phases shipped so far:

- :mod:`storage_sqlite.header` — the 100-byte database header at the start
  of page 1.
- :mod:`storage_sqlite.pager` — page-at-a-time I/O with an LRU cache and a
  rollback journal.
- :mod:`storage_sqlite.varint` — SQLite's 1..9 byte big-endian varints.
- :mod:`storage_sqlite.record` — record codec (serial types + row values).

Everything else (B-trees, sqlite_schema, the Backend adapter) lands in
subsequent phases.
"""

from storage_sqlite import btree, record, varint
from storage_sqlite.btree import BTree, BTreeError, DuplicateRowidError, PageFullError
from storage_sqlite.errors import (
    CorruptDatabaseError,
    JournalError,
    StorageError,
)
from storage_sqlite.header import Header
from storage_sqlite.pager import PAGE_SIZE, Pager
from storage_sqlite.record import Value

__all__ = [
    "PAGE_SIZE",
    "BTree",
    "BTreeError",
    "CorruptDatabaseError",
    "DuplicateRowidError",
    "Header",
    "JournalError",
    "PageFullError",
    "Pager",
    "StorageError",
    "Value",
    "btree",
    "record",
    "varint",
]
