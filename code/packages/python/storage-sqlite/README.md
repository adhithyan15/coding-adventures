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
        ├── pager    (page I/O + LRU cache + rollback journal) ✓ phase 1
        ├── header   (100-byte database header at offset 0)    ✓ phase 1
        ├── record   (varint + serial types)      ✓ phase 2
        ├── btree    (leaf + overflow + full recursive splits)  ✓ phases 3 + 4a + 4b
        ├── freelist (page reuse)                 [v1 phase 5]
        └── schema   (sqlite_schema round-trip)   [v1 phase 6]
```

Nothing above the `Backend` line changes. The full SQL pipeline (lexer →
parser → planner → optimizer → codegen → VM) runs unmodified against this
backend.

## What works today (phases 1 + 2 + 3 + 4a + 4b)

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

What's **not yet** in this package (coming in later phases): freelist /
page reuse (phase 5), `sqlite_schema` (phase 6), and the `Backend` adapter
that wires the full pipeline in (phase 7).

## Installation

```bash
uv pip install -e .
```

## Usage (phases 1 + 2 + 3 + 4a + 4b)

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
```

This intentionally looks low-level — the `Backend` adapter that makes
`mini_sqlite.connect("app.db")` work will land in phase 7 once all the
intermediate layers (records, B-trees, schema) are in place.

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
