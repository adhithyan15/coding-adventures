# coding-adventures-storage-sqlite

**SQLite byte-compatible file backend** — a file-backed implementation of the
`sql-backend` `Backend` interface that reads and writes real SQLite
(`sqlite3`-compatible) database files.

## What this package is

The existing `sql-backend` package ships an `InMemoryBackend`. `mini_sqlite`
uses it today when you call `connect(":memory:")`. This package is the
*file-backed* sibling — the thing that will let `connect("app.db")` open a
real on-disk SQLite database file that the `sqlite3` CLI can also read.

It does that by implementing the [SQLite database file format](
https://www.sqlite.org/fileformat2.html) from scratch, layer by layer, in
pure Python.

## Where this fits in the stack

```
mini_sqlite.connect("app.db")
    → Connection(SqliteFileBackend("app.db"))
    → sql-vm (unchanged)
    → Backend interface
    → SqliteFileBackend  (this package)
        ├── pager      (page I/O + LRU cache + rollback journal)    ✓ phase 1
        ├── header     (100-byte database header at offset 0)      ✓ phase 1
        ├── record     (varint + serial types)                     ✓ phase 2
        ├── btree      (leaf + overflow + full recursive splits)   ✓ phases 3 + 4a + 4b
        ├── freelist   (trunk/leaf page reuse)                     ✓ phase 5
        ├── schema     (sqlite_schema round-trip)                  ✓ phase 6
        ├── backend    (Backend adapter / SqliteFileBackend)       ✓ phase 7
        ├── index_tree (index B-tree 0x0A/0x02 + full CRUD)       ✓ phase IX-1
        └── backend    (index interface: create/drop/list/scan)   ✓ phase IX-2
```

Nothing above the `Backend` line changes. The full SQL pipeline (lexer →
parser → planner → optimizer → codegen → VM) runs unmodified against this
backend.

## What works today (phases 1 + 2 + 3 + 4a + 4b + 5 + 6 + 7 + IX-1 + IX-2)

- **`header` module** — the 100-byte database header at the start of page 1.
  Read and write every field, validate magic string and page size on open.
- **`pager` module** — page-at-a-time I/O against the file, a small LRU cache,
  `allocate()` for new pages, and a rollback journal that guarantees durability:
  `commit()` flushes the journal, applies pending writes, flushes the main
  file, then deletes the journal; `rollback()` throws away pending writes and
  the journal.
- **`varint` module** — SQLite's 1..9 byte big-endian variable-length integer
  encoding. `encode`, `decode`, `encode_signed`, `decode_signed`, `size`.
  Always the shortest form — required for byte-compat round-trip.
- **`record` module** — row values ↔ bytes. Picks the smallest serial type
  for every integer (so the integer 7 encodes as a single byte, not eight).
  Round-trips NULL / int / float / str / bytes. Byte-compat verified against
  the real `sqlite3` output for simple records.
- **`btree` module** — table B-tree pages (type `0x0D` leaf + type `0x05`
  interior). `insert`, `find`, `scan`, `delete`, `update` all work on trees
  of **arbitrary depth**. Supports overflow chains for large records. Split
  algorithm:
  - **Root-leaf split**: when the root leaf fills it becomes an interior page
    with two leaf children.
  - **Non-root leaf split**: when a non-root leaf fills, the existing page is
    rewritten with the left half, a right sibling is allocated, and the
    separator key propagates up to the parent.
  - **Interior page split** (recursive): if a parent interior page is also
    full, the median cell is removed and propagated further up the ancestor
    chain, splitting interior pages all the way to the root if necessary.
  - **Root interior split**: if the root interior page fills, two new interior
    children are allocated and the root is rewritten with one separator cell.
    The root page number never changes.
  - Corrupted pages (bad ncells, out-of-range pointers, overflow cycles,
    unknown page types) all raise `CorruptDatabaseError`.
  - **`BTree.free_all(freelist)`** (phase 6): reclaims every page in a B-tree
    (interior, leaf, overflow chains) via a post-order DFS traversal.
- **`freelist` module** — SQLite trunk/leaf freelist (phase 5).
  `Freelist(pager)` manages the linked list of reusable pages rooted in the
  page-1 header (offsets 32–39). `free(pgno)` adds a page as a leaf entry
  in the current trunk, or promotes it to a new trunk when the current one
  is full (capacity: 1 022 leaves per 4 096-byte trunk page). `allocate()`
  pops the last leaf LIFO — matching SQLite's allocation order — and
  zero-fills the page before returning it; when the freelist is empty it
  falls through to `Pager.allocate()`. Pass `freelist=Freelist(pager)` to
  `BTree.create()` / `BTree.open()` to enable automatic overflow-page reuse
  on delete and update.
- **`schema` module** — `sqlite_schema` catalog table (phase 6).
  `initialize_new_database(pager)` writes the 100-byte file header and an
  empty `sqlite_schema` leaf page into a fresh pager and returns a `Schema`.
  `Schema(pager)` attaches to an existing database. `create_table(name, sql)`
  allocates a root page, inserts the five-column schema row, and bumps the
  schema cookie. `drop_table(name)` frees every page in the table's B-tree,
  removes the schema row, and bumps the cookie. `list_tables()` returns table
  names in insertion order. `find_table(name)` returns `(rowid, rootpage, sql)`
  or `None`. `get_schema_cookie()` reads the u32 at page-1 offset 40.

- **`backend` module** — `SqliteFileBackend` (phases 7 + 8).
  Implements the full `sql_backend.Backend` interface against a real `.db`
  file.  `tables()`, `columns()`, `scan()`, `insert()`, `update()`,
  `delete()`, `create_table()`, `drop_table()`, `begin_transaction()`,
  `commit()`, `rollback()`.  Opens existing files or creates new ones.
  Passes all four tiers of `sql_backend.conformance`.

  **Byte-compatible with the real `sqlite3` library** (v0.8.1+): INTEGER
  PRIMARY KEY columns are stored as a NULL slot in the record payload (matching
  the real SQLite convention).  Files produced by this backend are readable by
  the `sqlite3` CLI and Python's stdlib `sqlite3`, and vice-versa.

- **`index_tree` module** — `IndexTree` (phase IX-1 of v2 automatic indexing).
  Index B-tree pages using SQLite's `0x0A` (index leaf) and `0x02` (index
  interior) page types.  Stores `(key_vals, rowid)` pairs sorted by SQLite's
  BINARY collation (NULL < INTEGER/REAL < TEXT < BLOB).  Supports:
  `insert`, `delete`, `lookup`, `range_scan` (with inclusive/exclusive bounds),
  `free_all`, and recursive splits at all tree levels.  The foundation for
  automatic index creation in v2.

## Installation

```bash
uv pip install -e .
```

## Usage (phases 1 + 2 + 3 + 4a + 4b + 5 + 6 + 7 + IX-1 + IX-2)

```python
from storage_sqlite import Header, Pager, record, varint

# --- Pager + Header ---
with Pager.create("app.db") as pager:
    header = Header.new_database(page_size=4096)
    page1 = bytearray(pager.page_size)
    page1[:100] = header.to_bytes()
    pager.write(1, bytes(page1))
    pager.commit()

with Pager.open("app.db") as pager:
    raw = pager.read(1)
    header = Header.from_bytes(raw[:100])
    assert header.magic == b"SQLite format 3\x00"

# --- Varint ---
assert varint.encode(300) == b"\x82\x2c"      # shortest form
value, consumed = varint.decode(b"\x82\x2c")  # → (300, 2)

# --- Record ---
# [NULL, 7, "hi"] encodes to the same bytes the real sqlite3 writes.
raw = record.encode([None, 7, "hi"])          # b"\x04\x00\x01\x11\x07hi"
values, consumed = record.decode(raw)          # → ([None, 7, "hi"], 8)

# --- BTree (single leaf, or multi-level after root split) ---
with Pager.create("data.db") as pager:
    tree = BTree.create(pager)
    # Insert any number of rows — splits happen automatically at every level.
    # Root-leaf split, non-root leaf splits, and interior splits are all
    # transparent: insert/find/scan/delete work identically at any tree depth.
    for i in range(1, 5001):
        tree.insert(i, record.encode([f"row{i}"]))
    print(tree.cell_count())   # 5000
    pager.commit()

with Pager.open("data.db") as pager:
    tree = BTree.open(pager, root_page=1)
    for rowid, payload in tree.scan():   # ascending order across all leaves
        cols, _ = record.decode(payload)
        print(rowid, cols[0])   # 1 row1 / 2 row2 / …

# --- Freelist ---
# Enable overflow-page reuse by injecting a Freelist into BTree.
# Page 1 must already hold the database header (offsets 32–39 are the
# freelist fields).
from storage_sqlite import Freelist

with Pager.create("app.db") as pager:
    page1 = bytearray(pager.page_size)
    page1[:100] = Header.new_database(page_size=4096).to_bytes()
    pager.write(1, bytes(page1))
    fl = Freelist(pager)
    tree = BTree.create(pager, freelist=fl)   # root reuses freelist pages
    tree.insert(1, record.encode([b"X" * 5000]))  # spills to overflow pages
    pager.commit()

with Pager.open("app.db") as pager:
    fl = Freelist(pager)
    tree = BTree.open(pager, root_page=2, freelist=fl)
    tree.delete(1)          # overflow pages returned to freelist
    tree.insert(2, record.encode([b"Y" * 5000]))  # reuses those pages
    print(fl.total_pages)   # 0 — all freed pages were reused
    pager.commit()
```

```python
# --- Schema (phase 6: sqlite_schema catalog) ---
from storage_sqlite import initialize_new_database, Schema, Freelist

with Pager.create("catalog.db") as pager:
    fl = Freelist(pager)
    schema = initialize_new_database(pager)  # sets up page 1 + empty schema tree
    # Schema uses the freelist so dropped tables' pages are reused.
    schema2 = Schema(pager, freelist=fl)

    # CREATE TABLE — allocates a root page, inserts the schema row.
    root = schema2.create_table(
        "users", "CREATE TABLE users (id INTEGER, name TEXT)"
    )
    print(root)                        # e.g. 2
    print(schema2.list_tables())       # ['users']
    print(schema2.get_schema_cookie()) # 1

    # Use the root page directly for DML.
    tree = BTree.open(pager, root, freelist=fl)
    tree.insert(1, record.encode([1, "Alice"]))
    tree.insert(2, record.encode([2, "Bob"]))

    # DROP TABLE — frees every page in the table's B-tree.
    schema2.drop_table("users")
    print(schema2.list_tables())       # []
    print(schema2.get_schema_cookie()) # 2

    pager.commit()
```

```python
# --- Backend (phase 7: full sql_backend.Backend adapter) ---
from sql_backend import ColumnDef
from storage_sqlite import SqliteFileBackend

# Create and populate a database — fully Backend-compatible.
with SqliteFileBackend("app.db") as b:
    b.create_table(
        "users",
        [
            ColumnDef(name="id",    type_name="INTEGER", primary_key=True),
            ColumnDef(name="name",  type_name="TEXT",    not_null=True),
            ColumnDef(name="email", type_name="TEXT",    unique=True),
        ],
        if_not_exists=True,
    )
    b.insert("users", {"id": 1, "name": "Alice", "email": "alice@example.com"})
    b.insert("users", {"id": 2, "name": "Bob",   "email": "bob@example.com"})

    # Explicit transaction for atomic writes.
    h = b.begin_transaction()
    b.insert("users", {"id": 3, "name": "Carol", "email": None})
    b.commit(h)

# Read it back in a new session — data survived on disk.
with SqliteFileBackend("app.db") as b:
    it = b.scan("users")
    while (row := it.next()) is not None:
        print(row)          # {'id': 1, 'name': 'Alice', 'email': 'alice@example.com'}
    it.close()

# Positioned update and delete via _open_cursor().
with SqliteFileBackend("app.db") as b:
    cursor = b._open_cursor("users")
    row = cursor.next()           # first row
    b.update("users", cursor, {"email": "newalice@example.com"})
    cursor.close()

    h = b.begin_transaction()
    b.commit(h)
```

```python
# --- IndexTree (phase IX-1: index B-tree pages 0x0A / 0x02) ---
from storage_sqlite import IndexTree, Pager, Freelist, Header, PAGE_SIZE, initialize_new_database

# Create a database with a header page and build an index tree.
with Pager.create("index.db") as pager:
    schema = initialize_new_database(pager)           # sets up page 1 header
    fl = Freelist(pager)
    tree = IndexTree.create(pager, freelist=fl)        # root on page 3
    root = tree.root_page

    # Insert (indexed_value, rowid) pairs — rowid is the table row pointer.
    tree.insert([42], 1)          # user_id=42, table rowid=1
    tree.insert([42], 2)          # same user_id, different row
    tree.insert([17], 3)          # different user_id
    tree.insert(["alice"], 4)     # text key
    tree.insert([None], 5)        # NULL key (smallest under SQLite ordering)

    # Lookup: all rowids for a given key.
    print(tree.lookup([42]))           # [1, 2]  (non-unique index)
    print(tree.lookup([17]))           # [3]
    print(tree.lookup([99]))           # []

    # Range scan: yield (key_vals, rowid) in ascending order.
    for key_vals, rowid in tree.range_scan([17], [42]):
        print(key_vals, "→ row", rowid)
    # [17] → row 3
    # [42] → row 1
    # [42] → row 2

    # Mixed-type ordering: NULL < INTEGER < TEXT < BLOB
    for key_vals, rowid in tree.range_scan(None, None):
        print(key_vals, rowid)
    # [None]    5   ← NULL first
    # [17]      3
    # [42]      1
    # [42]      2
    # ['alice'] 4   ← TEXT last

    # Delete.
    tree.delete([42], 1)
    print(tree.lookup([42]))           # [2]

    # Splits happen automatically — insert thousands of entries.
    for i in range(100, 5100):
        tree.insert([i], i)
    print(tree.cell_count())           # 5004

    pager.commit()
```

```python
# --- Index interface (phase IX-2: create/drop/list/scan_index) ---
from sql_backend import ColumnDef, IndexDef
from storage_sqlite import SqliteFileBackend

with SqliteFileBackend("app.db") as b:
    b.create_table(
        "users",
        [
            ColumnDef(name="id",  type_name="INTEGER", primary_key=True),
            ColumnDef(name="name", type_name="TEXT",   not_null=True),
            ColumnDef(name="age", type_name="INTEGER"),
        ],
    )
    b.insert("users", {"id": 1, "name": "Alice", "age": 30})
    b.insert("users", {"id": 2, "name": "Bob",   "age": 25})
    b.insert("users", {"id": 3, "name": "Carol", "age": 30})

    # Create an index — backfills all existing rows automatically.
    b.create_index(IndexDef(name="idx_users_age", table="users", columns=["age"]))

    # List all indexes for a table.
    print(b.list_indexes("users"))
    # [IndexDef(name='idx_users_age', table='users', columns=['age'], unique=False, auto=False)]

    # Equality lookup: all rowids with age = 30.
    rowids = list(b.scan_index("idx_users_age", [30], [30]))
    print(rowids)   # [0, 2] (0-based rowids for rows with age=30)

    # Range scan: age between 25 and 30.
    rowids = list(b.scan_index("idx_users_age", [25], [30]))
    print(len(rowids))  # 3

    # Exclusive range: age > 25 (lo bound excluded).
    rowids = list(b.scan_index("idx_users_age", [25], None, lo_inclusive=False))
    print(len(rowids))  # 2 (only the two age=30 rows)

    # Drop the index.
    b.drop_index("idx_users_age")
    print(b.list_indexes("users"))   # []

    # if_exists=True suppresses IndexNotFound for already-dropped indexes.
    b.drop_index("idx_users_age", if_exists=True)   # no error

# The index is stored in sqlite_schema and visible to the real sqlite3 CLI:
#   sqlite3 app.db ".schema"
#   CREATE INDEX idx_users_age ON users (age);
```

## Design notes

- **Page size is pinned to 4096 in v1.** SQLite supports 512–65536; variable
  sizes land in v2.
- **Text encoding is UTF-8.** UTF-16LE/BE are v2.
- **Rollback journal only.** WAL is a separate, larger effort and is
  deliberately deferred.
- **Single-process, single-writer.** No POSIX advisory locks in v1.
- **Pure Python.** No C extensions, no `ctypes`. Speed is not the goal —
  faithfulness to the file format is.

See [`code/specs/storage-sqlite.md`](../../../specs/storage-sqlite.md) for
the full specification, including deferred-to-v2 items and the phased build
order.

## Testing

```bash
./BUILD
```

Runs `ruff` over `src/` and `tests/`, then `pytest` with `--cov-fail-under=95`.

## References

- [SQLite file format 2](https://www.sqlite.org/fileformat2.html)
- Internal: [`sql-backend.md`](../../../specs/sql-backend.md)
- Internal: [`storage-sqlite.md`](../../../specs/storage-sqlite.md)
