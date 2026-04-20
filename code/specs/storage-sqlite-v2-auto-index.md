# storage-sqlite v2 — Automatic Index Building

## Overview

This document specifies v2 of the `storage-sqlite` / `mini-sqlite` stack: a
system that **watches incoming queries and automatically builds B-tree indexes
without the user ever writing `CREATE INDEX`**.

The database is a silent observer. Every time a `SELECT` runs a full table
scan because of a `WHERE` clause, it takes a note. After enough evidence
accumulates on the same column, it quietly builds a B-tree index and starts
using it. The user sees faster queries and does nothing.

### Who decides?

The decision of *when* to build or drop an index is separated from the
mechanics of *how*. A pluggable `IndexPolicy` interface owns the decision.
The default `HitCountPolicy` is a simple hit-count heuristic. Any other
policy — rule-based, statistical, or learned — can be dropped in at runtime
by implementing the same interface. The infrastructure does not know or care
what logic the policy uses.

This keeps the mechanical layers (B-tree pages, backend interface, query
events, advisor) completely independent of any particular decision strategy,
now or in the future.

---

## What changes relative to v1

v1 is complete and byte-compatible with real SQLite. v2 adds on top of it:

| Component | v1 state | v2 addition |
|---|---|---|
| `storage_sqlite.btree` | table B-trees only | + index B-tree page types (0x0A / 0x02) |
| `storage_sqlite.index_tree` | absent | new module — `IndexTree` CRUD |
| `storage_sqlite.backend` | table scan only | + `create_index`, `drop_index`, `scan_index`, `list_indexes` |
| `sql_backend` | no index interface | + `IndexDef`, index methods on `Backend` |
| `sql_vm` | executes, returns result | + emits a `QueryEvent` after each SELECT scan |
| `mini_sqlite.policy` | absent | new module — `IndexPolicy` protocol + `HitCountPolicy` |
| `mini_sqlite.advisor` | absent | new module — `IndexAdvisor` (event consumer + actuator) |
| `mini_sqlite.connection` | passes backend to vm | + wires `IndexAdvisor` into every execute |
| `sql_planner` | always full scan | + index selection for equality / range predicates |

Everything else (pager, record, freelist, schema, varint, header, lexer,
parser, optimizer, codegen) is **untouched**.

---

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│  Application                                              │
│  conn.execute("SELECT * FROM orders WHERE user_id = ?")   │
└────────────────────────┬─────────────────────────────────┘
                         │
                         ▼
┌──────────────────────────────────────────────────────────┐
│  mini_sqlite.Connection                                   │
│                                                           │
│  1. run SQL through pipeline → QueryResult                │
│  2. pass QueryEvent to IndexAdvisor                       │
│  3. if advisor says "build index X" → call create_index   │
└───────┬──────────────────────────┬────────────────────────┘
        │ SQL pipeline             │ index DDL
        ▼                          ▼
┌───────────────┐        ┌─────────────────────────────────┐
│  sql-vm       │        │  IndexAdvisor                   │
│               │        │                                  │
│  executes     │        │  • receives QueryEvents          │
│  plan; uses   │        │  • forwards signals to policy    │
│  available    │        │  • calls create_index / drop_    │
│  indexes      │        │    index when policy says so     │
└───────┬───────┘        └───────────┬─────────────────────┘
        │                            │ policy.on_scan() /
        │                            │ policy.should_drop()
        │                            ▼
        │                ┌─────────────────────────────────┐
        │                │  IndexPolicy (protocol)         │
        │                │                                  │
        │                │  Default: HitCountPolicy        │
        │                │  Pluggable: any implementation  │
        │                └─────────────────────────────────┘
        │
        ▼
┌──────────────────────────────────────────────────────────┐
│  sql_backend.Backend (SqliteFileBackend)                  │
│                                                           │
│  create_index / drop_index / scan_index / list_indexes    │
│                                                           │
│  storage_sqlite.IndexTree — index B-tree pages           │
└──────────────────────────────────────────────────────────┘
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

Indexes are stored in `sqlite_schema` exactly as real SQLite stores them:

```
type     = 'index'
name     = <index name>
tbl_name = <table name>
rootpage = <root page of the index B-tree>
sql      = 'CREATE INDEX <name> ON <table> (<col>, ...)'
           or NULL for auto-created indexes (matching sqlite3 convention
           for internal indexes on UNIQUE / PRIMARY KEY constraints)
```

This means auto-created indexes are visible to the real `sqlite3` CLI and
survive a `sqlite3` `VACUUM` or `.schema` inspection.

---

### Component 3 — Query event system (`sql_vm`)

After every successful `SELECT` that performs a scan, `sql_vm` emits a
`QueryEvent` to a registered listener.

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

```python
# sql_vm public API addition
def set_event_listener(listener: Callable[[QueryEvent], None] | None) -> None:
    """Register a callback to receive QueryEvents after each SELECT scan.

    Pass None to remove the listener. The listener is called synchronously
    before execute() returns, so it must be fast (no blocking I/O).
    """
```

`QueryEvent` is only emitted for SELECT statements that produce a scan node.
INSERT / UPDATE / DELETE do not emit events in v2.

---

### Component 4 — Index policy (`mini_sqlite.policy`)

The `IndexPolicy` interface owns the decision of when to create or drop an
index. It receives workload signals and returns simple yes/no answers.

```python
class IndexPolicy(Protocol):
    """Decides which indexes to build and drop based on query observations.

    Implementations receive signals from the IndexAdvisor and return
    decisions. They do not interact with the backend directly — the advisor
    acts on their decisions.

    The default implementation is HitCountPolicy. Any other strategy can be
    plugged in by implementing this protocol.
    """

    def on_full_scan(
        self,
        table: str,
        column: str,
        event: QueryEvent,
    ) -> bool:
        """Called after each full-table scan that filtered on *column*.

        Return True to request that an index be created on (table, column).
        The advisor will act on a True return — implementations should only
        return True when they are confident an index is warranted.
        """

    def on_index_used(
        self,
        index_name: str,
        table: str,
        column: str,
        event: QueryEvent,
    ) -> None:
        """Called when a query used an existing index.

        Allows the policy to track utilization and inform drop decisions.
        No return value — use should_drop() for drop decisions.
        """

    def should_drop(self, index_name: str, table: str, column: str) -> bool:
        """Return True if an auto-created index should be dropped.

        Called periodically by the advisor. Return True only when the
        index is clearly no longer beneficial.
        """
```

#### Default implementation: `HitCountPolicy`

```python
class HitCountPolicy:
    """Create an index after N full-table scans on the same column.
    Drop an index that has not been used in the last M queries.

    This is the simplest useful policy. It is correct and predictable.
    It makes no assumptions about data distribution, query frequency
    trends, or workload stationarity.

    Parameters
    ----------
    create_threshold : int
        Number of full-table scans on a column before requesting an index.
        Default 10.
    cold_window : int
        Number of query events without index use before requesting a drop.
        Default 100.
    """

    def __init__(
        self,
        *,
        create_threshold: int = 10,
        cold_window: int = 100,
    ) -> None: ...

    def on_full_scan(self, table: str, column: str, event: QueryEvent) -> bool:
        # Increment hit_count[(table, column)].
        # Return True when hit_count >= create_threshold.
        ...

    def on_index_used(self, index_name: str, table: str, column: str,
                      event: QueryEvent) -> None:
        # Record last-used event index for drop tracking.
        ...

    def should_drop(self, index_name: str, table: str, column: str) -> bool:
        # Return True if the index has not appeared in used_index for the
        # last cold_window query events.
        ...
```

---

### Component 5 — Index advisor (`mini_sqlite.advisor`)

The `IndexAdvisor` is the coordinator between the query event stream and the
backend. It holds a reference to a `Backend` and an `IndexPolicy`, receives
`QueryEvent` objects from the VM, and calls `create_index` / `drop_index`
when the policy asks for it.

```python
class IndexAdvisor:
    """Coordinates query observations with index lifecycle management.

    Receives QueryEvents, forwards signals to the policy, and acts on
    the policy's decisions by calling create_index / drop_index on the
    backend. The advisor is policy-agnostic — it does not know or care
    what decision logic the policy uses.
    """

    def __init__(
        self,
        backend: Backend,
        policy: IndexPolicy | None = None,  # defaults to HitCountPolicy()
        *,
        max_auto_indexes: int = 20,
    ) -> None: ...

    def set_policy(self, policy: IndexPolicy | None) -> None:
        """Swap the active policy at runtime.

        The new policy takes effect on the next query event. Existing
        indexes are not affected by a policy change.
        If None, reverts to HitCountPolicy with default parameters.
        """

    def on_query_event(self, event: QueryEvent) -> None:
        """Process one query event. May trigger create_index or drop_index."""
```

#### Policy swap contract

Swapping the policy at runtime has well-defined semantics:

- Indexes already created remain in place (they live in the file, not the policy)
- The new policy starts fresh with no hit-count history from the old policy
- The old policy is discarded and its state is not transferred
- Indexes created by the old policy may later be dropped by the new policy
  if `should_drop` returns True

This makes policies **stateless with respect to the advisor** — each policy
owns its own internal counters and can be reasoned about in isolation.

#### Auto-index naming

Auto-created index names follow a deterministic pattern:

```
auto_{table}_{column}
# examples:
auto_orders_user_id
auto_users_email
```

If a name would collide with an existing index, a numeric suffix is appended:

```
auto_orders_user_id_2
```

#### Index creation is transactional

Each `create_index` call is wrapped in a `begin_transaction` / `commit`
cycle so the backfill is atomic and durable. If the backfill fails, the
advisor logs the failure, resets the hit count via the policy, and does not
retry until the threshold is crossed again.

---

### Component 6 — Planner index selection (`sql_planner`)

When the planner builds a `Scan` node for a table, it checks whether any
available index covers the predicate columns. If so, it substitutes an
`IndexScan` node.

#### Index-eligible predicates (v2 subset)

| Predicate form | Eligible? |
|---|---|
| `col = literal` | ✅ |
| `col = ?` (parameter) | ✅ |
| `col > literal`, `col >= literal` | ✅ |
| `col < literal`, `col <= literal` | ✅ |
| `col BETWEEN a AND b` | ✅ |
| `col IN (v1, v2, …)` | ✅ (union of point lookups) |
| `col IS NULL` | ❌ deferred |
| `col LIKE 'prefix%'` | ❌ deferred |
| Multi-column compound predicates | ❌ deferred (composite indexes later) |

#### Index selection algorithm (v2 — first match, no cost model)

```
for each Scan(table, predicate) in the logical plan:
    for each index in list_indexes(table):
        if index.columns[0] appears in predicate:
            replace Scan with IndexScan(index, range_from_predicate)
            break
```

No cost model in v2 — any matching index is used. A future phase adds
selectivity estimates to prefer the best index when multiple candidates
exist.

#### `IndexScan` logical plan node

```python
@dataclass
class IndexScan:
    table: str
    index_name: str
    column: str
    lo: SqlValue | None       # lower bound key (None = unbounded)
    hi: SqlValue | None       # upper bound key (None = unbounded)
    lo_inclusive: bool
    hi_inclusive: bool
    residual: Expr | None     # remaining predicate applied after index scan
```

The `IndexScan` node compiles to an `INDEX_SCAN` opcode in `sql_codegen`
and executes via `backend.scan_index` in `sql_vm`.

---

## End-to-end example

```python
import mini_sqlite

conn = mini_sqlite.connect("shop.db")

conn.execute("CREATE TABLE orders (id INTEGER PRIMARY KEY, user_id INTEGER, total REAL)")
for i in range(10_000):
    conn.execute("INSERT INTO orders VALUES (?, ?, ?)", (i, i % 500, float(i)))
conn.commit()

# First 9 queries: full table scans. Hit count climbs to 9.
for _ in range(9):
    conn.execute("SELECT * FROM orders WHERE user_id = ?", (42,)).fetchall()

# 10th query crosses the threshold. Advisor silently creates
# auto_orders_user_id and all subsequent queries use it.
rows = conn.execute("SELECT * FROM orders WHERE user_id = ?", (42,)).fetchall()

# The index is visible to sqlite3 too.
import sqlite3
with sqlite3.connect("shop.db") as db:
    print(db.execute(
        "SELECT name FROM sqlite_schema WHERE type='index'"
    ).fetchall())
    # [('auto_orders_user_id',)]
```

### Swapping the policy

```python
from mini_sqlite.policy import HitCountPolicy

# Aggressive policy: build indexes after just 3 hits.
conn._advisor.set_policy(HitCountPolicy(create_threshold=3, cold_window=50))

# Any future policy (rule-based, statistical, etc.) fits the same slot.
# conn._advisor.set_policy(MyCustomPolicy())
```

---

## Phased build order

| Phase | Deliverable | Packages touched |
|---|---|---|
| IX-1 | `IndexTree` — index B-tree page types, insert / lookup / range_scan / delete | `storage-sqlite` |
| IX-2 | Backend interface extension — `IndexDef`, `create_index`, `drop_index`, `list_indexes`, `scan_index` in `sql_backend` + `SqliteFileBackend` | `sql-backend`, `storage-sqlite` |
| IX-3 | `CREATE INDEX` / `DROP INDEX` DDL wired end-to-end through the pipeline | `sql-planner`, `sql-codegen`, `sql-vm`, `mini-sqlite` |
| IX-4 | Query event system — `QueryEvent`, `set_event_listener` in `sql_vm` | `sql-vm` |
| IX-5 | `IndexPolicy` protocol, `HitCountPolicy`, `IndexAdvisor`, wired into `mini_sqlite.Connection` | `mini-sqlite` |
| IX-6 | Planner index selection — `IndexScan` node, first-match substitution | `sql-planner`, `sql-codegen`, `sql-vm` |

Each phase is a separate feature branch and PR. Do not start IX-2 until
IX-1's tests are fully green.

---

## Testing strategy

### IX-1: `IndexTree` unit tests
- Insert N keys, scan in order → keys returned sorted
- Point lookup: present and absent keys
- Range scan: open/closed bounds
- Delete: remove a key, adjacent keys intact
- Splits: enough insertions to force root split and interior splits
- Overflow: index a long TEXT value requiring overflow pages
- `free_all`: all pages returned to freelist after call

### IX-2: Backend index tests
- `create_index` backfills existing rows correctly
- `scan_index` equality and range
- `drop_index` removes B-tree and `sqlite_schema` row
- `list_indexes` filtered by table
- `IndexAlreadyExists` and `IndexNotFound` raised correctly
- Index row visible to real `sqlite3` (oracle test)
- `sqlite3`-created index readable by `scan_index` (reverse oracle)

### IX-3: DDL pipeline tests
- `CREATE INDEX idx ON t (col)` end-to-end
- `DROP INDEX idx` end-to-end
- `CREATE INDEX IF NOT EXISTS` is idempotent
- `DROP INDEX IF EXISTS` is idempotent
- Errors translate correctly to PEP 249 exceptions

### IX-4: Query event tests
- Listener receives event after SELECT with WHERE
- `rows_scanned` matches actual scan count
- `used_index` is None on full scan, set on index scan
- INSERT / UPDATE / DELETE do not emit events
- Removing listener stops events

### IX-5: Advisor + policy tests
- Hit count below threshold → no index created
- Hit count at threshold → index created
- Column queried after index exists → `used_index` set → count not incremented
- `max_auto_indexes` cap respected
- Name collision → numeric suffix appended
- Failed backfill → advisor resets and does not retry immediately
- `set_policy()` swap → new policy takes effect, existing indexes retained
- Custom `IndexPolicy` implementation accepted

### IX-6: Planner index selection tests
- `SELECT … WHERE col = ?` uses index after creation
- `SELECT … WHERE col > ?` uses index for range scan
- Full scan used when no index covers the predicate column
- `QueryEvent.used_index` set correctly when index is used

---

## Non-goals (v2)

- **Composite / multi-column indexes** — deferred to v3
- **UNIQUE indexes** — `IndexDef.unique` field reserved but not enforced in v2
- **`ANALYZE` / statistics-based cost model** — deferred to v3 (requires
  selectivity estimates and histogram maintenance)
- **Index-only scans** (covering indexes where the SELECT list is fully
  satisfied by the index without touching the table) — deferred to v3
- **WAL mode** — still rollback journal only in v2
- **Multi-process locking** — still single-process only

---

## Relationship to existing specs

- **Extends:** `storage-sqlite.md` (IX-1, IX-2), `sql-backend.md` (index
  interface), `mini-sqlite-python.md` (policy + advisor), `sql-planner.md`
  (index selection), `sql-vm.md` (query events)
- **Forward compatibility:** the `IndexPolicy` interface is the stable
  extension point for any future decision strategy. Nothing else needs to
  change when a new policy is introduced.
