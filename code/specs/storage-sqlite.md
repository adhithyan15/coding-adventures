# Storage SQLite Specification

## Overview

This document specifies the `storage-sqlite` package: a **SQLite-byte-compatible
file-backed implementation** of the `Backend` interface.

The goal is literal byte-compatibility with the real SQLite on-disk format —
a file written by `storage-sqlite` must be openable by the `sqlite3` CLI, and
a file written by `sqlite3` must be readable by `storage-sqlite`. That
constraint is what makes this package interesting and forces the design.

It plugs into the existing pipeline without changing anything above:

```
mini_sqlite.connect("app.db")        ← today: raises InterfaceError
    │
    ▼
Connection(SqliteFileBackend("app.db"))
    │
    ▼
sql-vm (unchanged) ──► Backend interface ──► SqliteFileBackend
                                                 │
                                                 ├── Pager     (page I/O + cache)
                                                 ├── BTree     (table B-trees)
                                                 ├── Record    (varint + serial types)
                                                 └── Schema    (sqlite_schema table)
```

Everything above the `Backend` line — lexer, parser, adapter, planner,
optimizer, codegen, VM — is untouched. This package slots in behind the
interface and is what makes `connect("myfile.db")` start working.

---

## Where It Fits

```
Depends on: sql-backend               (interface being implemented)
            (internal modules only — no other sql-pipeline deps)

Used by:    mini-sqlite                (selected when database != ":memory:")
            anything else that calls Backend (planner schema queries, etc.)
```

This is deliberately a **leaf-ish** package — it does not know about SQL. It
knows about pages, B-tree cells, varints, records, and `sqlite_schema`. The
SQL pipeline talks to it through the narrow `Backend` interface.

---

## Supported Languages

v1 ships **Python only**, matching the rest of the SQL pipeline. Other
language implementations are deferred until the Python reference is stable
and the conformance tests are known-good against real `.sqlite` files.

---

## Goals

1. **Round-trip byte-compat** for the v1 subset: a file written by this
   package is bit-identical to what `sqlite3` would write for an equivalent
   sequence of statements (where bit-identicality is achievable — free-page
   ordering has legitimate non-determinism, so the test oracle is "sqlite3
   can open and read every row back" rather than literal `diff`).
2. **Implements the `Backend` interface** — no new API invented here. Every
   method on `Backend` works against a real on-disk file.
3. **Single-process, single-writer** durability via a rollback journal.
4. **Pluggable**: `mini_sqlite.connect("foo.db")` just works; the backend
   selection happens at connect time and nothing else changes.
5. **Specs first**: every on-disk layout is documented with a byte diagram
   before the code lands.

## Non-goals (v1)

- **WAL mode** — rollback journal only in v1. WAL is a separate, larger effort.
- **Index B-trees** — `CREATE INDEX` stores the `sqlite_schema` row but the
  planner does not yet consult indexes. Scans remain full table scans.
- **Multiple page sizes** — v1 pins page size at 4096 bytes. SQLite supports
  512–65536; we will add the rest in v2 once one size is solid.
- **Multi-process locking** — no POSIX advisory locks in v1. Concurrent
  writers are undefined behaviour. Single-process safe.
- **AUTOINCREMENT** — the internal `sqlite_sequence` table is not maintained.
  `INTEGER PRIMARY KEY` (without AUTOINCREMENT) works and behaves as a rowid
  alias.
- **Triggers, views, foreign keys, CHECK, PRAGMA, VACUUM** — out of v1 scope.
- **Encrypted databases** (SEE / SQLCipher) — out of scope entirely.

---

## The SQLite file format — layers

The file format is documented at <https://www.sqlite.org/fileformat2.html>.
v1 implements the subset below. Each layer corresponds to a Python module
inside `storage-sqlite`.

### Layer 1 — Pager (`pager.py`)

**Responsibility**: read and write fixed-size pages to/from the file; hold a
small in-memory cache; coordinate writes with the rollback journal.

- Page size: **4096 bytes** (fixed).
- Page numbers: 1-based. Page 1 is the database header page.
- API:
  - `Pager.open(path) -> Pager`
  - `Pager.read(page_no: int) -> bytes`  — returns a 4096-byte view
  - `Pager.write(page_no: int, data: bytes)`  — stages into write-set
  - `Pager.allocate() -> int`  — returns a fresh page number
  - `Pager.commit()` — fsync journal, apply write-set to main file, fsync main, delete journal
  - `Pager.rollback()` — discard write-set, discard journal
- Cache: simple LRU, default 32 pages. Invalidated on rollback.
- **Rollback journal**: hot-journal file at `<db>-journal`. Before any page
  is written for the first time in a transaction, its original bytes are
  copied to the journal. On crash, recovery replays the journal back into
  the main file.

### Layer 2 — Database header (`header.py`)

**Responsibility**: read and write the 100-byte header at the start of page 1.

Fixed byte layout:

```
offset  size  meaning
   0    16    "SQLite format 3\0"  (magic string)
  16     2    page size in bytes   (u16-be, 4096 = 0x1000)
  18     1    file format write version (1 = legacy)
  19     1    file format read version  (1 = legacy)
  20     1    reserved space per page (0)
  21     1    max embedded payload fraction (64)
  22     1    min embedded payload fraction (32)
  23     1    leaf payload fraction (32)
  24     4    file change counter (u32-be) — bumped on every write txn
  28     4    in-header database size in pages (u32-be)
  32     4    page number of first freelist trunk page (u32-be, 0 if none)
  36     4    total number of freelist pages (u32-be)
  40     4    schema cookie (u32-be) — bumped when sqlite_schema changes
  44     4    schema format number (4 — supports everything we need)
  48     4    default cache size (0)
  52     4    largest root b-tree page before incremental vacuum (0, feature off)
  56     4    text encoding (1 = UTF-8)
  60     4    user version (0)
  64     4    incremental vacuum mode (0)
  68     4    application ID (0)
  72    20    reserved zero bytes
  92     4    version-valid-for number (== change counter)
  96     4    SQLite version number (3 046 002 — matches 3.46.2, arbitrary)
```

v1 writes these values exactly; most are fixed constants.

### Layer 3 — Varint and record codec (`record.py`)

**Varint**: 1–9 byte big-endian encoding used throughout the format. Bytes
1–8 have a continuation bit in MSB; byte 9 uses all 8 bits. We need read
and write, both widely tested against golden fixtures.

**Serial types** (varint):

| serial type | meaning          | content size |
|-------------|------------------|--------------|
| 0           | NULL             | 0            |
| 1           | 8-bit int        | 1            |
| 2           | 16-bit int BE    | 2            |
| 3           | 24-bit int BE    | 3            |
| 4           | 32-bit int BE    | 4            |
| 5           | 48-bit int BE    | 6            |
| 6           | 64-bit int BE    | 8            |
| 7           | 64-bit float BE  | 8            |
| 8           | constant 0       | 0            |
| 9           | constant 1       | 0            |
| 10, 11      | internal/reserved| — (unused)   |
| N ≥ 12 even | BLOB of (N−12)/2 | variable     |
| N ≥ 13 odd  | TEXT of (N−13)/2 | variable     |

**Record** layout: `[header-length-varint] [serial-type-varint …] [payload …]`.
Pick the *smallest* serial type that fits each value (e.g. the integer 3
uses serial type 1, not 6) — that's what `sqlite3` does, and matters for
byte-compat on round-trip.

### Layer 4 — Table B-tree pages (`btree.py`)

SQLite uses two B-tree variants: **table** (keyed by rowid) and **index**
(keyed by record). v1 implements **table** only.

Each B-tree page has this layout:

```
offset  size  meaning
   0     1    page type byte:
                 0x0D = leaf table
                 0x05 = interior table
   1     2    first freeblock offset (0 = none)
   3     2    number of cells on the page
   5     2    cell content area start (grows down from end of page)
   7     1    fragmented free bytes
   8     4    (interior pages only) right-most child pointer
  8/12   2*N  cell pointer array (2 bytes per cell, growing up)
  …          free space (shrinking)
  …          cell content (growing down)
```

**Leaf table cell**: `[payload-bytes-varint] [rowid-varint] [record bytes]`.
If the record is too large to fit in the page, the tail spills into an
**overflow chain**: the first 4 bytes of the cell payload become a page
pointer to the first overflow page; overflow pages are linked by a u32 at
their start. v1 supports overflow chains end-to-end.

**Interior table cell**: `[left-child-page-varint] [rowid-varint]`. The
right-most child is stored in the page header, not in a cell.

Operations: `find(rowid)`, `insert(rowid, record)`, `update(rowid, record)`,
`delete(rowid)`, `range_scan()`. The existing `b-tree` package covers the
algorithmic shape; this module re-implements against the SQLite-specific
page layout rather than bending the generic one.

### Layer 5 — `sqlite_schema` (`schema.py`)

The `sqlite_schema` table (on page 2 at fixed rootpage 1 — same page as the
header on page 1, cell area starting at offset 100) is itself a normal table
B-tree whose rows have five columns:

```
type     TEXT   -- 'table' | 'index' | 'view' | 'trigger'
name     TEXT   -- object name
tbl_name TEXT   -- owning table (same as name for 'table' rows)
rootpage INTEGER -- page number of the B-tree root for this object
sql      TEXT   -- CREATE statement as the user typed it (for round-trip)
```

CREATE TABLE:
1. Append a `sqlite_schema` row for the new table.
2. Allocate a fresh leaf page, write empty-leaf bytes, set as the table's
   rootpage.
3. Bump the schema cookie in the header.

DROP TABLE:
1. Free all pages reachable from the rootpage.
2. Delete the `sqlite_schema` row.
3. Bump the schema cookie.

### Layer 6 — Backend adapter (`backend.py`)

Implements the `Backend` interface in terms of the lower layers:

- `tables()` → scan `sqlite_schema`, filter `type = 'table'`, return names.
- `columns(t)` → parse the stored `CREATE TABLE sql` from sqlite_schema and
  return column defs. (Reuses the existing `sql-parser` — the facade is allowed
  to depend up here.)
- `scan(t)` → resolve rootpage, return an iterator over the B-tree's leaf
  cells decoding each record.
- `insert(t, row)` → encode record, pick next rowid, B-tree insert.
- `update(t, cursor, assignments)` / `delete(t, cursor)` — cursor holds
  `(rootpage, rowid)`.
- `create_table(stmt)` / `drop_table(name)` — delegate to `schema.py`.
- `begin_transaction()` → `pager.begin()` returning a handle.
- `commit(handle)` / `rollback(handle)` → pager methods.

### Layer 7 — Freelist (`freelist.py`)

Pages freed by DELETE or DROP are prepended to a linked list of trunk pages
pointed to from offset 32 of the header. Allocations prefer reusing freelist
pages before extending the file. v1 implements the trunk/leaf layout SQLite
uses so a `sqlite3` `VACUUM` on our file wouldn't find surprises.

---

## v1 scope — checkpoint

At end of v1 this sequence works:

```python
# Python side — our code writes the file.
with mini_sqlite.connect("demo.db") as c:
    c.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT)")
    c.executemany("INSERT INTO t VALUES (?, ?)",
                  [(1, "Ada"), (2, "Grace")])

# Shell side — real sqlite3 opens the same file.
$ sqlite3 demo.db "SELECT * FROM t"
1|Ada
2|Grace

# And the reverse direction:
$ sqlite3 fresh.db "CREATE TABLE u(x INTEGER); INSERT INTO u VALUES (42)"
$ python -c "import mini_sqlite; \
             c = mini_sqlite.connect('fresh.db'); \
             print(list(c.execute('SELECT * FROM u')))"
[(42,)]
```

If those two scenarios pass end-to-end, v1 is done.

---

## Testing strategy

Three test tiers:

1. **Unit** — each module in isolation (`test_varint.py`, `test_record.py`,
   `test_header.py`, `test_btree_leaf.py`, …). Golden byte fixtures taken
   from `sqlite3`-produced files.
2. **Round-trip** — write via our backend, read via `sqlite3` CLI (skipped
   in CI if `sqlite3` unavailable, asserted locally and in CI if installed);
   and the reverse direction. This is the **byte-compat oracle**.
3. **Conformance** — run the existing `sql-backend` conformance suite against
   `SqliteFileBackend`, so it passes every test that `InMemoryBackend` passes.

Target coverage: 95%+ (library code — the threshold that libraries in this
repo hold to).

---

## Phased build order

Build leaf-to-root. Each phase commits as its own feature branch / PR.

| Phase | Module | Deliverable |
|-------|--------|-------------|
| 1 | `pager.py` + `header.py` | Open an empty file, read/write page 1, rollback journal |
| 2 | `record.py` (varint + serial types) | Unit-tested round-trip of every scalar SQL value |
| 3 | `btree.py` (leaf + overflow) | Insert/lookup/scan a single B-tree, no splits |
| 4 | `btree.py` (splits + interior pages) | Full B-tree that grows beyond one page |
| 5 | `freelist.py` | Pages freed by DELETE return to a freelist |
| 6 | `schema.py` + DDL round-trip | CREATE/DROP TABLE via sqlite_schema |
| 7 | `backend.py` — wire it all up and pass the conformance suite | `connect("foo.db")` works end-to-end |
| 8 | **The byte-compat oracle** — `sqlite3` CLI round-trip tests pass | v1 ships |

Each phase is a spec-then-tests-then-implementation cycle and should fit in
its own reviewable PR.

---

## Deferred to v2+

These will each get their own spec document when picked up:

- **WAL mode**: `-wal` + `-shm` file, checkpointer, concurrent readers.
- **Index B-trees**: `CREATE INDEX`, index-driven scans in the planner.
- **Autoincrement**: `sqlite_sequence` maintenance and 63-bit monotonic rowids.
- **Variable page sizes**: 512, 1024, …, 65536.
- **File locking**: POSIX advisory locks for multi-process safety.
- **Incremental vacuum / full vacuum**.
- **Foreign-key enforcement, CHECK constraints, triggers, views**.
- **Encoding other than UTF-8**: UTF-16LE, UTF-16BE.

---

## References

- [SQLite file format 2](https://www.sqlite.org/fileformat2.html) — the
  authoritative spec this document tracks.
- [SQLite VFS](https://www.sqlite.org/vfs.html) — relevant when WAL lands.
- [Database File Format](https://www.sqlite.org/fileformat.html) (older,
  superseded by `fileformat2.html` but still useful for context).
- Internal: [`sql-backend.md`](sql-backend.md) — the interface this package
  implements.
- Internal: [`mini-sqlite-python.md`](mini-sqlite-python.md) — the facade
  that selects this backend when `connect(path)` is called with a non-
  `:memory:` path.
