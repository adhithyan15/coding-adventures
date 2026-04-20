# storage-sqlite v2 — Automatic Index Building

## Overview

This document specifies v2 of the `storage-sqlite` / `mini-sqlite` stack: a
system that **watches incoming queries and automatically builds B-tree indexes
without the user ever writing `CREATE INDEX`**.

The key idea is simple:

> The database is a silent observer. Every time a `SELECT` runs a full table
> scan because of a `WHERE` clause, it takes a note. After enough notes pile
> up on the same column, it quietly builds a B-tree index and starts using it.
> The user sees faster queries. They do nothing.

This is the foundation for a longer-term vision where the note-taking becomes
an online-learning model — a small neural network that can generalise across
query shapes and predict which indexes will pay off before they are needed.
v2 does not build the neural network. It builds the **plumbing the neural
network will eventually plug into**, and ships a dead-simple heuristic in
its place. Swapping in the smarter model later touches only one component.

---

## What changes relative to v1

v1 is complete and byte-compatible with real SQLite. v2 adds on top of it:

| Component | v1 state | v2 addition |
|---|---|---|
| `storage_sqlite.btree` | table B-trees only | + index B-tree page types (0x0A / 0x02) |
| `storage_sqlite.index_tree` | absent | new module — `IndexTree` CRUD |
| `storage_sqlite.backend` | table scan only | + `create_index`, `drop_index`, `scan_index`, `list_indexes` |
| `sql_backend` | no index interface | + `IndexDef`, index methods on `Backend` |
| `sql_vm` | executes, returns result | + emits a `QueryEvent` after each SELECT |
| `mini_sqlite.advisor` | absent | new module — `IndexAdvisor` (observer + actuator) |
| `mini_sqlite.connection` | passes backend to vm | + wires `IndexAdvisor` into every execute |
| `sql_planner` | always full scan | + index selection for equality / range predicates |

Everything else (pager, record, freelist, schema, varint, header, lexer,
parser, optimizer, codegen) is **untouched**.

---

## Architecture

```
┌─────────────────────────────────────────────────────┐
│  Application                                         │
│  conn.execute("SELECT * FROM orders WHERE user_id=?")│
└────────────────────────┬────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────┐
│  mini_sqlite.Connection                              │
│                                                      │
│  1. run SQL through pipeline → QueryResult           │
│  2. emit QueryEvent to IndexAdvisor                  │
│  3. if IndexAdvisor says "build index X" → do it     │
└───────┬─────────────────────────┬───────────────────┘
        │ SQL pipeline            │ index DDL
        ▼                         ▼
┌───────────────┐       ┌─────────────────────────────┐
│  sql-vm       │       │  IndexAdvisor               │
│               │       │                              │
│  executes     │       │  • accumulates QueryEvents   │
│  plan using   │       │  • scores (table, column)    │
│  available    │       │    pairs by hit count        │
│  indexes      │       │  • fires create_index when   │
└───────┬───────┘       │    score ≥ threshold         │
        │               │  • fires drop_index when     │
        ▼               │    column goes cold          │
┌───────────────────────┴─────────────────────────────┐
│  sql_backend.Backend (SqliteFileBackend)             │
│                                                      │
│  create_index / drop_index / scan_index /            │
│  list_indexes — new in v2                            │
│                                                      │
│  storage_sqlite.IndexTree — index B-tree pages       │
└─────────────────────────────────────────────────────┘
```

---

## Component specifications

### Component 1 — Index B-tree pages (`storage_sqlite.index_tree`)

SQLite uses two B-tree page subtypes for index pages:

| page type byte | meaning |
|---|---|
| `0x0A` | index leaf page |
| `0x02` | index interior page |

The page layout is identical to table B-tree pages (same header format, same
cell pointer array, same cell-content area) with one difference: cells do not
carry a separate rowid field. Instead, the **sort key IS the indexed column
value** and the **rowid is appended at the end of each index record**.

#### Index cell format (leaf, 0x0A)

```
[payload-size varint] [index-record bytes]
```

The index record is a normal SQLite record (same serial-type header as table
records) whose columns are:

```
[indexed_column_value, ..., rowid]
```

For a single-column index on `orders.user_id`:

```
record = [user_id_value, rowid]
         ^               ^
         sort key         pointer back to table row
```

For a composite index on `(last_name, first_name)`:

```
record = [last_name_value, first_name_value, rowid]
```

The sort order is lexicographic over the record values — same as `sqlite3`.

#### Index cell format (interior, 0x02)

```
[left-child-page u32] [index-record bytes (no rowid suffix)]
```

Interior cells carry a **separator key** (the indexed value only, without the
rowid) plus a left-child pointer. The rightmost child lives in the page header
(`rightmost_child` field at offset 8). This matches the table interior layout.

#### `IndexTree` API

```python
class IndexTree:
    @classmethod
    def create(cls, pager: Pager, freelist: Freelist | None = None) -> IndexTree:
        """Allocate a fresh root page and return an attached IndexTree."""

    @classmethod
    def open(cls, pager: Pager, rootpage: int,
             freelist: Freelist | None = None) -> IndexTree:
        """Open an existing index B-tree by root page number."""

    def insert(self, key: list[SqlValue], rowid: int) -> None:
        """Insert (key, rowid) into the index. Splits as needed."""

    def delete(self, key: list[SqlValue], rowid: int) -> None:
        """Remove the entry matching (key, rowid). No-op if absent."""

    def lookup(self, key: list[SqlValue]) -> list[int]:
        """Return all rowids whose index key equals *key*."""

    def range_scan(
        self,
        lo: list[SqlValue] | None,
        hi: list[SqlValue] | None,
        *,
        lo_inclusive: bool = True,
        hi_inclusive: bool = True,
    ) -> Iterator[tuple[list[SqlValue], int]]:
        """Yield (key, rowid) pairs in ascending key order within [lo, hi]."""

    def free_all(self, freelist: Freelist) -> None:
        """Reclaim all pages in the index tree (used by drop_index)."""
```

**Comparison order** — the same as `sqlite3`'s default collation:

1. NULL < integer/float < text < blob
2. Integers and floats compare numerically (cross-type)
3. Texts compare by UTF-8 byte value (case-sensitive, like `BINARY` collation)
4. Blobs compare by byte value

This matches the serial-type ordering real sqlite3 uses for index entries.

---

### Component 2 — Backend interface extension (`sql_backend`)

Two new types and four new methods are added to the `Backend` ABC.

#### `IndexDef` dataclass

```python
@dataclass
class IndexDef:
    name: str           # index name (e.g. "auto_orders_user_id")
    table: str          # table the index covers
    columns: list[str]  # column names in sort order (left to right)
    unique: bool        # True → UNIQUE index (v2 ships non-unique only)
    auto: bool          # True → created by the advisor, not the user
```

#### New `Backend` methods

```python
def create_index(self, index: IndexDef) -> None:
    """Create a new index B-tree and backfill it from the existing table rows.

    Raises IndexAlreadyExists if an index with the same name already exists.
    Raises TableNotFound if the table does not exist.
    Raises ColumnNotFound if any column in index.columns is unknown.
    """

def drop_index(self, name: str, *, if_exists: bool = False) -> None:
    """Drop an index by name.

    Raises IndexNotFound if the index does not exist and if_exists=False.
    """

def list_indexes(self, table: str | None = None) -> list[IndexDef]:
    """Return all indexes, optionally filtered to a single table."""

def scan_index(
    self,
    index_name: str,
    lo: list[SqlValue] | None,
    hi: list[SqlValue] | None,
    *,
    lo_inclusive: bool = True,
    hi_inclusive: bool = True,
) -> Iterator[int]:
    """Yield rowids from the named index within the given key range."""
```

#### New exceptions

```python
class IndexAlreadyExists(BackendError):
    index: str

class IndexNotFound(BackendError):
    index: str
```

#### `sqlite_schema` integration

Indexes are stored in `sqlite_schema` the same way real SQLite stores them:

```
type     = 'index'
name     = <index name>
tbl_name = <table name>
rootpage = <root page of the index B-tree>
sql      = 'CREATE INDEX <name> ON <table> (<col>, ...)'
           or NULL for auto-created indexes (matching sqlite3 convention
           for internal indexes on UNIQUE / PRIMARY KEY constraints)
```

This means the index is visible to the real `sqlite3` CLI and can survive a
`sqlite3` `VACUUM` or `.schema` inspection.

---

### Component 3 — Query event system (`sql_vm`)

After every successful `SELECT` execution, `sql_vm` emits a `QueryEvent`.

```python
@dataclass
class QueryEvent:
    table: str                    # primary table being scanned
    filtered_columns: list[str]   # columns that appeared in WHERE predicates
    rows_scanned: int             # rows examined (full-scan count)
    rows_returned: int            # rows in result set
    used_index: str | None        # index name used, or None (full scan)
    duration_us: int              # wall-clock microseconds
```

The event is passed to a registered `EventListener` callback. The VM itself
does not decide what to do with it — that is the advisor's job.

```python
# sql_vm public API addition
def set_event_listener(listener: Callable[[QueryEvent], None] | None) -> None:
    """Register a callback to receive QueryEvents after each SELECT.

    Pass None to remove the listener. The listener is called synchronously
    before execute() returns, so it must be fast (no blocking I/O).
    """
```

`QueryEvent` is only emitted for SELECT statements that perform a scan
(i.e. statements the planner resolved to a `Scan` or `IndexScan` node).
INSERT / UPDATE / DELETE do not emit events in v2.

---

### Component 4 — Index advisor (`mini_sqlite.advisor`)

The `IndexAdvisor` receives `QueryEvent` objects, maintains scores for
`(table, column)` pairs, and decides when to build or drop indexes.

#### Design principle: heuristic is always the floor

The neural network (when one is eventually added) is an **optional
accelerator** that makes the heuristic smarter — it is not a replacement for
it. When the neural network is absent or disabled, the heuristic runs alone
and the system works exactly as well as it ever did.

Think of it as two layers:

```
┌─────────────────────────────────────────────────┐
│  Layer 2 (optional): Learned scorer             │  ← accelerator
│  A small online-learning model that adjusts     │    can be disabled
│  the column scores based on workload patterns   │
└───────────────────┬─────────────────────────────┘
                    │ score adjustment (additive)
                    ▼
┌─────────────────────────────────────────────────┐
│  Layer 1 (always on): Hit-count heuristic       │  ← the floor
│  Every full-table-scan WHERE increments a       │    always works
│  counter. Counter ≥ threshold → create index.  │
└─────────────────────────────────────────────────┘
```

The learned layer does NOT gate the heuristic layer. If you disable the neural
network at runtime (or it raises an exception), the advisor falls back to the
plain hit-count heuristic without interrupting query execution or dropping any
existing indexes.

```python
class ScoringModel(Protocol):
    """Optional learned model that adjusts column scores.

    The advisor calls score() on every query event. If no model is registered,
    the advisor uses pure hit-count scoring. The model can boost or dampen
    the raw hit count but cannot suppress index creation entirely — the
    heuristic floor remains in effect.
    """

    def score(
        self,
        table: str,
        column: str,
        raw_hit_count: int,
        event: QueryEvent,
    ) -> float:
        """Return an adjusted score in [0.0, ∞).

        Return raw_hit_count to defer entirely to the heuristic.
        Return a higher value to accelerate index creation.
        Return a lower (but non-negative) value to slow it down.
        """


class IndexAdvisor:
    """Observes query events and auto-creates/drops B-tree indexes.

    Works correctly with no model registered (pure heuristic). A learned
    model can be plugged in to accelerate or tune decisions without replacing
    the underlying hit-count logic.
    """

    def __init__(
        self,
        backend: Backend,
        *,
        create_threshold: int = 10,   # create index after N effective hits
        cold_window: int = 100,       # queries before a unused index is dropped
        max_auto_indexes: int = 20,   # cap on total auto indexes per database
        model: ScoringModel | None = None,  # optional learned accelerator
    ) -> None: ...

    def on_query_event(self, event: QueryEvent) -> None:
        """Process one query event. May trigger create_index or drop_index."""

    def set_model(self, model: ScoringModel | None) -> None:
        """Attach or detach the learned model at runtime.

        Detaching (passing None) falls back to pure heuristic scoring
        immediately. Existing indexes are not affected.
        """

    # ── Internal scoring pipeline ─────────────────────────────────────────

    def _raw_score(self, table: str, column: str) -> int:
        """Return the raw hit count for this (table, column) pair."""

    def _effective_score(self, table: str, column: str, event: QueryEvent) -> float:
        """Return the final score used to make decisions.

        = model.score(raw_hit_count, event) if model is set
        = raw_hit_count                     otherwise
        """

    def _should_create(self, table: str, column: str, event: QueryEvent) -> bool:
        """Return True if an index should be created.

        True when effective_score >= create_threshold AND no existing index
        covers this column. The model cannot suppress this — if the raw
        hit count alone crosses the threshold, an index is created regardless
        of what the model returns.
        """
        raw = self._raw_score(table, column)
        if raw >= self.create_threshold:
            return True                     # heuristic floor: always fires
        effective = self._effective_score(table, column, event)
        return effective >= self.create_threshold

    def _should_drop(self, index_name: str) -> bool:
        """Return True if an auto-created index should be dropped.

        True when the index has not appeared in used_index for the last
        cold_window query events.
        """
```

#### Hit counting

Every `QueryEvent` with `used_index=None` (a full scan) increments the hit
count for each `(event.table, col)` pair in `event.filtered_columns`. When
a `QueryEvent` with `used_index != None` arrives, the hit count for the
covered column is NOT incremented (the index is already working).

#### Auto-index naming

Auto-created index names follow a deterministic pattern so they can be
identified and cleaned up:

```
auto_{table}_{column}
# examples:
auto_orders_user_id
auto_users_email
```

If a column name would produce a collision (e.g. the user already has an index
named `auto_orders_user_id`), a numeric suffix is appended:

```
auto_orders_user_id_2
```

#### Write-ahead of index creation

Index creation is a DDL operation. The advisor wraps it in a transaction
using the normal `begin_transaction` / `commit` cycle so it is durable and
atomic. If the index creation fails (e.g. unique constraint violation on a
backfill), the advisor logs the failure, resets the hit count to 0, and waits
for the threshold to be crossed again before retrying.

---

### Component 5 — Planner index selection (`sql_planner`)

When the planner builds a `Scan` node for a table, it checks whether any
available index covers the predicate columns. If so, it substitutes an
`IndexScan` node.

#### Index-eligible predicates (v2 subset)

| Predicate form | Eligible? | Notes |
|---|---|---|
| `col = literal` | ✅ | Point lookup — best case |
| `col = ?` | ✅ | Parameterised equality |
| `col > literal`, `col >= literal` | ✅ | Range scan lower bound |
| `col < literal`, `col <= literal` | ✅ | Range scan upper bound |
| `col BETWEEN a AND b` | ✅ | Desugared to two range bounds |
| `col IN (v1, v2, …)` | ✅ | Multiple point lookups (union of results) |
| `col IS NULL` | ❌ | v2 deferred — NULL key handling is subtle |
| `col LIKE 'prefix%'` | ❌ | v2 deferred — prefix scan needs collation |
| Multi-column compound predicates | ❌ | v2 deferred — composite indexes later |

#### Index selection algorithm

```
for each Scan(table, predicate) in the logical plan:
    candidate_indexes = list_indexes(table)
    for each index in candidate_indexes:
        if index covers a column in predicate:
            replace Scan with IndexScan(index_name, range_from_predicate)
            break  # first match wins in v2
```

In v2 there is no cost model — any index match triggers substitution. A
future phase can add selectivity estimates to prefer the best index when
multiple candidates exist.

#### `IndexScan` logical plan node

```python
@dataclass
class IndexScan:
    table: str
    index_name: str
    column: str
    lo: SqlValue | None       # lower bound (None = unbounded)
    hi: SqlValue | None       # upper bound (None = unbounded)
    lo_inclusive: bool
    hi_inclusive: bool
    predicate: Expr | None    # residual predicate (post-index filter)
```

The `IndexScan` is compiled to a new `INDEX_SCAN` opcode in `sql_codegen`
and executed in `sql_vm` via `backend.scan_index`.

---

## End-to-end example

```python
import mini_sqlite

conn = mini_sqlite.connect("shop.db")

conn.execute("CREATE TABLE orders (id INTEGER PRIMARY KEY, user_id INTEGER, total REAL)")
for i in range(10_000):
    conn.execute("INSERT INTO orders VALUES (?, ?, ?)", (i, i % 500, float(i)))
conn.commit()

# First 9 queries on user_id — full table scans, hit count climbs.
for _ in range(9):
    conn.execute("SELECT * FROM orders WHERE user_id = ?", (42,)).fetchall()

# 10th query crosses the threshold — advisor silently creates
# auto_orders_user_id index and re-runs using it.
rows = conn.execute("SELECT * FROM orders WHERE user_id = ?", (42,)).fetchall()

# All subsequent queries use the index — O(log n) instead of O(n).
rows = conn.execute("SELECT * FROM orders WHERE user_id = ?", (99,)).fetchall()

# The index is visible to sqlite3 too.
import sqlite3
with sqlite3.connect("shop.db") as db:
    schema = db.execute("SELECT name FROM sqlite_schema WHERE type='index'").fetchall()
    print(schema)  # [('auto_orders_user_id',)]
```

---

## Phased build order

| Phase | Deliverable | Package(s) touched |
|---|---|---|
| IX-1 | Index B-tree page types — `IndexTree` with insert/lookup/range_scan/delete | `storage-sqlite` |
| IX-2 | Backend interface extension — `IndexDef`, `create_index`, `drop_index`, `list_indexes`, `scan_index` in `sql_backend` + `SqliteFileBackend` | `sql-backend`, `storage-sqlite` |
| IX-3 | `CREATE INDEX` / `DROP INDEX` DDL wired end-to-end through the pipeline | `sql-planner`, `sql-codegen`, `sql-vm`, `mini-sqlite` |
| IX-4 | Query event system — `QueryEvent`, `set_event_listener` in `sql_vm` | `sql-vm` |
| IX-5 | Index advisor — `IndexAdvisor`, wired into `mini_sqlite.Connection` | `mini-sqlite` |
| IX-6 | Planner index selection — `IndexScan` node, index substitution | `sql-planner`, `sql-codegen`, `sql-vm` |

Each phase is a separate feature branch and PR, in order. Do not start IX-2
until IX-1's tests are green.

---

## Testing strategy

### IX-1: `IndexTree` unit tests
- Insert N keys, scan in order → keys come back sorted
- Point lookup: present and absent keys
- Range scan: open/closed bounds, NULL boundary behaviour
- Delete: remove a key, verify it is gone, verify adjacent keys intact
- Splits: insert enough keys to force root split, interior splits
- Overflow: index a long TEXT value that requires overflow pages
- Free all: after `free_all`, all pages are on the freelist

### IX-2: Backend index tests
- `create_index` backfills existing rows
- `scan_index` equality and range
- `drop_index` removes the B-tree and the `sqlite_schema` row
- `list_indexes` filters by table correctly
- `IndexAlreadyExists` and `IndexNotFound` raised correctly
- Index row visible to real `sqlite3` (oracle test)
- `sqlite3`-created index readable by `scan_index` (reverse oracle)

### IX-3: DDL pipeline tests
- `CREATE INDEX idx ON t (col)` wired end-to-end
- `DROP INDEX idx` wired end-to-end
- `CREATE INDEX IF NOT EXISTS` is idempotent
- `DROP INDEX IF EXISTS` is idempotent
- Errors translate correctly (table not found, column not found, duplicate)

### IX-4: Query event tests
- Event listener receives events after SELECT with WHERE
- `rows_scanned` matches actual scan count
- `used_index` is None for full scans
- INSERT/UPDATE/DELETE do not emit events
- Removing listener stops events

### IX-5: Advisor unit tests
- Hit count below threshold → no index created
- Hit count at threshold → index created, advisor resets counter
- Same column queried after index exists → `used_index` set → count not re-incremented
- `max_auto_indexes` cap respected
- Index name collision → numeric suffix appended
- Advisor survives failed backfill gracefully

### IX-6: Planner index selection tests
- `SELECT … WHERE col = ?` uses index after it exists
- `SELECT … WHERE col > ?` uses index for range scan
- Full scan used when no index covers the predicate column
- Two indexes available, first match used (no cost model yet)
- `QueryEvent.used_index` set correctly when index is used

---

## Non-goals (v2)

- **Composite / multi-column indexes** — deferred to v3
- **UNIQUE indexes** — the `IndexDef.unique` field is reserved but not enforced in v2
- **`ANALYZE` / statistics-based cost model** — deferred to v3
- **Neural / learned advisor** — the `ScoringModel` protocol and `set_model()` hook are present in v2 so the interface is stable, but the first actual neural implementation lands in v3. The heuristic works on its own and is never gated behind the model.
- **User-visible `CREATE INDEX`** in mini-sqlite's public API — IX-3 wires the DDL but the primary UX in v2 is the advisor; explicit `CREATE INDEX` is a bonus
- **Index-only scans** (covering indexes where the index satisfies the SELECT list without touching the table) — deferred to v3
- **WAL mode** — still rollback journal only in v2

---

## Relationship to existing specs

- **Extends:** `storage-sqlite.md` (adds IX-1 and IX-2), `sql-backend.md`
  (adds `IndexDef` and index methods), `mini-sqlite-python.md` (adds advisor),
  `sql-planner.md` (adds index selection), `sql-vm.md` (adds query events)
- **Superseded by:** nothing — this is the forward spec; v1 remains valid for
  the subset it covers
- **Future:** `storage-sqlite-v3.md` will add composite indexes, statistics,
  and the first learned advisor implementation
