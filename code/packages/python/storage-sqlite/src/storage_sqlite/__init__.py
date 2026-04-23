"""
storage-sqlite — a SQLite byte-compatible file backend.

Phases shipped so far:

- :mod:`storage_sqlite.header` — the 100-byte database header at the start
  of page 1.
- :mod:`storage_sqlite.pager` — page-at-a-time I/O with an LRU cache and a
  rollback journal.
- :mod:`storage_sqlite.varint` — SQLite's 1..9 byte big-endian varints.
- :mod:`storage_sqlite.record` — record codec (serial types + row values).
- :mod:`storage_sqlite.btree` — table B-tree with full recursive splits.
- :mod:`storage_sqlite.freelist` — SQLite trunk/leaf freelist for page reuse.
- :mod:`storage_sqlite.schema` — sqlite_schema catalog table (CREATE / DROP
  TABLE, schema cookie management).
- :mod:`storage_sqlite.backend` — :class:`SqliteFileBackend`, the
  ``sql_backend.Backend`` adapter that wires all layers together (phase 7).
"""

from storage_sqlite import btree, record, varint
from storage_sqlite.backend import SqliteFileBackend
from storage_sqlite.btree import BTree, BTreeError, DuplicateRowidError, PageFullError
from storage_sqlite.errors import (
    CorruptDatabaseError,
    JournalError,
    StorageError,
)
from storage_sqlite.freelist import TRUNK_CAPACITY, Freelist
from storage_sqlite.header import Header
from storage_sqlite.index_tree import (
    PAGE_TYPE_INTERIOR_INDEX,
    PAGE_TYPE_LEAF_INDEX,
    DuplicateIndexKeyError,
    IndexTree,
    IndexTreeError,
)
from storage_sqlite.pager import PAGE_SIZE, Pager
from storage_sqlite.record import Value
from storage_sqlite.schema import Schema, SchemaError, initialize_new_database

__all__ = [
    "PAGE_SIZE",
    "BTree",
    "BTreeError",
    "CorruptDatabaseError",
    "DuplicateIndexKeyError",
    "DuplicateRowidError",
    "Freelist",
    "Header",
    "IndexTree",
    "IndexTreeError",
    "JournalError",
    "PAGE_TYPE_INTERIOR_INDEX",
    "PAGE_TYPE_LEAF_INDEX",
    "PageFullError",
    "Pager",
    "Schema",
    "SchemaError",
    "SqliteFileBackend",
    "StorageError",
    "TRUNK_CAPACITY",
    "Value",
    "btree",
    "initialize_new_database",
    "record",
    "varint",
]
