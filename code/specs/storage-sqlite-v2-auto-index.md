# storage-sqlite v2 — Automatic Index Building

## Overview

This document specifies v2 of the `storage-sqlite` / `mini-sqlite` stack: a
system that **watches incoming queries and automatically builds B-tree indexes
without the user ever writing `CREATE INDEX`**.

The database is a silent observer. Every time a query plan shows a full table
scan with a `WHERE` predicate, it takes a note. After enough evidence
accumulates on the same column, it quietly builds a B-tree index. The planner
notices the new index on the next query and uses it automatically. The user
sees faster queries and does nothing.

### Who decides?

The decision of *when* to build an index is separated from the mechanics of
*how*. A pluggable `IndexPolicy` interface owns the decision. The default
`HitCountPolicy` is a simple hit-count heuristic. Any other policy —
rule-based, statistical, or learned — can be dropped in at runtime by
implementing the same interface. The infrastructure does not know or care what
logic the policy uses.

This keeps the mechanical layers (B-tree pages, backend interface, advisor)
completely independent of any particular decision strategy, now or in the future.

### Design decision: plan-level observation

The advisor observes the *optimised logical plan* before the VM executes it,
rather than listening to events emitted during execution. This approach was
chosen because:

1. **Simplicity** — the plan explicitly encodes which columns are in filter
   positions; no VM instrumentation is needed.
2. **Accuracy** — the plan is inspected before execution starts, so an index
   can be created on the very query that crosses the threshold, not only on
   the next one.
3. **Decoupling** — the advisor is wired into the engine layer, not the VM.
   The VM itself is unchanged by v2.

The trade-off is that the advisor sees *planned* filter columns, not columns
that were actually evaluated at runtime. In practice this is fine — if the
planner placed a `Filter` node there, the column is definitively being
filtered. (Index drop logic, which requires knowing how often an index is
*actually used*, is deferred to v3.)

---

## What changes relative to v1

v1 is complete and byte-compatible with real SQLite. v2 adds on top of it:

| Component | v1 state | v2 addition |
|---|---|---|
| `storage_sqlite.btree` | table B-trees only | + index B-tree page types (0x0A / 0x02) |
| `storage_sqlite.index_tree` | absent | new module — `IndexTree` CRUD |
| `storage_sqlite.backend` | table scan only | + `create_index`, `drop_index`, `scan_index`, `list_indexes` |
| `sql_backend` | no index interface | + `IndexDef`, index methods on `Backend` |
| `sql_planner` | always full scan | + `IndexScan` node, first-match substitution |
| `sql_codegen` | no index ops | + `CreateIndex`, `DropIndex`, `OpenIndexScan` IR instructions |
| `sql_vm` | executes, returns result | + executes index DDL and `IndexScan` plans |
| `mini_sqlite.policy` | absent | new module — `IndexPolicy` protocol + `HitCountPolicy` |
| `mini_sqlite.advisor` | absent | new module — `IndexAdvisor` (plan observer + actuator) |
| `mini_sqlite.connection` | passes backend to vm | + wires `IndexAdvisor` into every execute |

Everything else (pager, record, freelist, schema, varint, header, lexer,
parser, optimizer) is **untouched**.

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
│  1. parse + plan + optimize → LogicalPlan                 │
│  2. advisor.observe_plan(plan) → may trigger create_index │
│  3. codegen + vm.run(plan) → QueryResult                  │
└───────┬──────────────────────────┬────────────────────────┘
        │ SQL pipeline             │ plan observation
        ▼                          ▼
┌───────────────┐        ┌─────────────────────────────────┐
│  sql-vm       │        │  IndexAdvisor                   │
│               │        │                                  │
│  executes     │        │  • walks plan for Filter(Scan)  │
│  plan; uses   │        │  • increments per-column hits   │
│  available    │        │  • calls policy.should_create() │
│  indexes      │        │  • calls create_index if yes    │
└───────┬───────┘        └───────────┬─────────────────────┘
        │                            │ should_create(table, col, n)
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
```

Auto-created indexes are visible to the real `sqlite3` CLI and survive a
`sqlite3` `VACUUM` or `.schema` inspection.

---

### Component 3 — Plan-level observation (`mini_sqlite.advisor`)

Instead of a runtime query-event system, the advisor inspects the **optimised
logical plan** before code generation. The engine calls
`advisor.observe_plan(plan)` once per statement, right after optimization and
before VM execution.

The advisor walks the plan tree for two structural patterns:

1. **`Filter(Scan(t), predicate)`** — a full table scan with a predicate.  
   The advisor extracts column names from the predicate (see *Indexable
   predicates* below), increments hit counts, and calls `should_create` on the
   policy.

2. **`IndexScan(t, column, index_name)`** — an index scan already chosen by
   the planner.  
   The advisor notes that the index is in use but does *not* increment hit
   counts (the index already exists; no action needed).

All other plan nodes are recursed into transparently.

#### Indexable predicates

The advisor extracts column names from predicates that a B-tree index could
satisfy:

| Predicate form | Indexable? |
|---|---|
| `col = literal` / `literal = col` | ✅ |
| `col < / <= / > / >= literal` (and reversed) | ✅ |
| `col BETWEEN lo AND hi` | ✅ |
| `col IN (v1, v2, …)` | ✅ |
| `predA AND predB` | ✅ (recurse into both halves) |
| `predA OR predB` | ❌ (or-predicates rarely benefit from a single index) |

OR sub-predicates and complex expressions are deliberately excluded — an index
on a column that only appears inside an `OR` is unlikely to help, and
over-creating indexes has a write-amplification cost.

---

### Component 4 — Index policy (`mini_sqlite.policy`)

The `IndexPolicy` interface owns the *create* decision. It receives a running
hit count and returns a boolean.

```python
@runtime_checkable
class IndexPolicy(Protocol):
    """Decides whether to create an index on a (table, column) pair.

    The advisor calls should_create() each time it observes a new hit for
    a (table, column) pair. Return True to trigger index creation; return
    False to wait for more evidence.

    hit_count is the running total of times the advisor has seen a filter
    on column within table. It includes the current observation — so the
    first call arrives with hit_count=1.
    """

    def should_create(self, table: str, column: str, hit_count: int) -> bool:
        """Return True when an index on table.column should be created."""
        ...
```

**v2 scope**: the policy handles *creation* only. Index drop logic (determining
when an auto-created index has become cold and should be removed) is deferred
to v3 where it will be designed alongside composite-index utilisation tracking.

#### Default implementation: `HitCountPolicy`

```python
class HitCountPolicy:
    """Create an index when a column's filter-hit count reaches threshold.

    Parameters
    ----------
    threshold : int
        Number of full-table scans on a column before requesting an index.
        Default 3.
    """

    def __init__(self, threshold: int = 3) -> None:
        if threshold < 1:
            raise ValueError(f"threshold must be >= 1, got {threshold!r}")
        self._threshold = threshold

    @property
    def threshold(self) -> int:
        """The configured hit-count threshold (read-only)."""
        return self._threshold

    def should_create(self, table: str, column: str, hit_count: int) -> bool:
        """Return True when hit_count has reached the configured threshold."""
        _ = table, column  # unused — decision is purely count-based
        return hit_count >= self._threshold
```

---

### Component 5 — Index advisor (`mini_sqlite.advisor`)

The `IndexAdvisor` coordinates plan observation with index creation. It holds a
reference to the `Backend` and the active `IndexPolicy`, and maintains a
per-`(table, column)` hit-count table.

```python
class IndexAdvisor:
    """Observes optimised query plans and auto-creates indexes when warranted.

    Parameters
    ----------
    backend:
        The database backend. Used for list_indexes (to avoid duplicates)
        and create_index (to materialise new indexes).
    policy:
        Decision policy. Defaults to HitCountPolicy(threshold=3). May be
        replaced at any time via the policy property.
    """

    def __init__(
        self,
        backend: Backend,
        policy: IndexPolicy | None = None,
    ) -> None: ...

    @property
    def policy(self) -> IndexPolicy:
        """The active index-creation policy."""
        ...

    @policy.setter
    def policy(self, new_policy: IndexPolicy) -> None:
        """Replace the policy. Hit counts accumulated so far are preserved."""
        ...

    def observe_plan(self, plan: LogicalPlan) -> None:
        """Walk plan and record filter-column observations.

        Called by the engine after each optimize() before code generation.
        Mutates internal hit-count state and may trigger create_index calls
        on the backend.

        Idempotent with respect to index creation: if an index already
        exists for a (table, column) pair, the advisor skips creation even
        if the policy says yes.
        """
        ...
```

#### Hit-count state and policy swaps

Hit counts are stored in the advisor (`_hits: dict[tuple[str, str], int]`),
not in the policy. This means:

- **Policy swaps preserve accumulated hit counts.** Replacing the policy via
  `advisor.policy = new_policy` retains the existing counter table. The new
  policy will see the same hit counts the next time `should_create` is called.
- **Only the decision logic changes** when the policy is swapped, not the
  observation history.

This is deliberately different from the original spec design (which proposed
hit counts living inside the policy). Housing counters in the advisor is a
cleaner separation: the advisor owns *state*, the policy owns *rules*.

#### Auto-index naming

Auto-created index names follow a deterministic pattern:

```
auto_{table}_{column}
# examples:
auto_orders_user_id
auto_users_email
```

If the first column of any existing index on the table already matches the
target column, the advisor skips creation to avoid redundancy (rather than
appending a numeric suffix). Numeric suffixes are a v3 concern for composite
index disambiguation.

#### Duplicate prevention

Before calling `backend.create_index`, the advisor calls `backend.list_indexes`
and skips creation if any existing index already has the target column as its
first key. `IndexAlreadyExists` from the backend (a race between two advisors
on the same file) is silently suppressed via `contextlib.suppress`.

---

### Component 6 — SQL pipeline additions (IX-3)

v2 wires `CREATE INDEX` and `DROP INDEX` DDL through the entire pipeline:

**`sql_planner`**: new AST nodes `CreateIndexStmt`, `DropIndexStmt`; new
logical plan nodes `CreateIndex`, `DropIndex`.

**`sql_codegen`**: new IR instructions `CreateIndex`, `DropIndex`,
`OpenIndexScan`; the compiler lowers `IndexScan` plan nodes to
`OpenIndexScan` with `lo`/`hi` bounds.

**`sql_vm`**: executes `CreateIndex` and `DropIndex` by delegating to the
backend; executes `OpenIndexScan` by calling `backend.scan_index` and
iterating rowids through a table fetch.

---

### Component 7 — Planner index selection (IX-6)

When the planner builds a `Scan` node for a table, it checks whether any
available index covers the predicate column. If so, it substitutes an
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
| Multi-column compound predicates | ✅ implemented in v3 (`storage-sqlite-v3-auto-index.md`) |

#### Index selection algorithm (v2 — first match, no cost model)

```
for each Scan(table, predicate) in the logical plan:
    for each index in list_indexes(table):
        if index.columns[0] appears in predicate:
            replace Scan with IndexScan(index, range_from_predicate)
            break
```

No cost model in v2 — any matching index is used. A future phase adds
selectivity estimates to prefer the best index when multiple candidates exist.

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

The `IndexScan` node compiles to an `OpenIndexScan` IR instruction in
`sql_codegen` and executes via `backend.scan_index` in `sql_vm`.

---

## End-to-end example

```python
import mini_sqlite

conn = mini_sqlite.connect("shop.db")

conn.execute("CREATE TABLE orders (id INTEGER PRIMARY KEY, user_id INTEGER, total REAL)")
for i in range(10_000):
    conn.execute("INSERT INTO orders VALUES (?, ?, ?)", (i, i % 500, float(i)))
conn.commit()

# First 2 queries: full table scans. Hit count climbs to 2.
for _ in range(2):
    conn.execute("SELECT * FROM orders WHERE user_id = ?", (42,)).fetchall()

# 3rd query crosses the default threshold of 3. Advisor silently creates
# auto_orders_user_id and all subsequent queries use it automatically.
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

# Aggressive policy: build indexes after just 1 hit.
conn.set_policy(HitCountPolicy(threshold=1))

# Any future policy (rule-based, statistical, etc.) fits the same slot.
# conn.set_policy(MyCustomPolicy())
```

Note: `Connection.set_policy(policy)` delegates to `advisor.policy = policy`.
Hit counts accumulated before the swap are preserved.

---

## Phased build order

Phases IX-1 and IX-2 were delivered as separate PRs. Phases IX-3 through IX-6
were tightly coupled (DDL, planner index selection, and advisor all interact
at the engine level) and were delivered together in a single PR.

| Phase | Deliverable | Packages touched | Status |
|---|---|---|---|
| IX-1 | `IndexTree` — index B-tree page types, insert / lookup / range_scan / delete | `storage-sqlite` | ✅ done |
| IX-2 | Backend interface extension — `IndexDef`, `create_index`, `drop_index`, `list_indexes`, `scan_index` in `sql_backend` + `SqliteFileBackend` | `sql-backend`, `storage-sqlite` | ✅ done |
| IX-3–6 | `CREATE INDEX` / `DROP INDEX` DDL + `IndexScan` planner + codegen + VM + `IndexPolicy` / `HitCountPolicy` / `IndexAdvisor` + `Connection.auto_index` | `sql-planner`, `sql-codegen`, `sql-vm`, `mini-sqlite` | ✅ done |

---

## Testing strategy

### IX-1: `IndexTree` unit tests (storage-sqlite/tests/test_index_tree.py)
- Insert N keys, scan in order → keys returned sorted
- Point lookup: present and absent keys
- Range scan: open/closed bounds
- Delete: remove a key, adjacent keys intact
- Splits: enough insertions to force root split and interior splits
- `free_all`: all pages returned to freelist after call

### IX-2: Backend index tests (storage-sqlite/tests/test_backend_index.py)
- `create_index` backfills existing rows correctly
- `scan_index` equality and range
- `drop_index` removes B-tree and `sqlite_schema` row
- `list_indexes` filtered by table
- `IndexAlreadyExists` and `IndexNotFound` raised correctly
- Index row visible to real `sqlite3` (oracle test)
- `sqlite3`-created index readable by `scan_index` (reverse oracle)

### IX-3: DDL pipeline tests (mini-sqlite/tests/test_tier2_features.py — TestCreateDropIndex)
- `CREATE INDEX idx ON t (col)` end-to-end
- `DROP INDEX idx` end-to-end
- `CREATE INDEX IF NOT EXISTS` is idempotent
- `DROP INDEX IF EXISTS` is idempotent
- Errors translate correctly to PEP 249 exceptions

### IX-4 (policy + advisor): (mini-sqlite/tests/test_tier2_features.py — TestHitCountPolicy, TestIndexAdvisor)
- `HitCountPolicy.should_create` returns False below threshold, True at/above it
- `threshold < 1` raises `ValueError`
- `observe_plan` on a `Filter(Scan)` increments hit count; creates index at threshold
- `observe_plan` on an `IndexScan` does not increment hit count
- `observe_plan` on a bare `Scan` (no filter) does not increment hit count
- Advisor skips creation if existing index already covers the column
- `IndexAlreadyExists` from backend is silently suppressed
- Custom `IndexPolicy` implementation accepted
- Policy swap preserves hit counts

### IX-5 (connection integration): (mini-sqlite/tests/test_tier2_features.py — TestConnectAutoIndex)
- `connect(auto_index=True)` (default) wires advisor
- `connect(auto_index=False)` disables advisor; `set_policy` is a no-op
- `set_policy(new_policy)` replaces policy on live connection

### IX-6 (planner index selection): covered by IX-4 advisor tests — after
  the advisor creates an index, re-running the same query produces an
  `IndexScan` plan rather than a `Filter(Scan)` plan.

---

## Non-goals (v2)

- **Index drop logic** — the advisor creates indexes but never drops them.
  Designing drop correctly requires knowing how often each index is *actually
  used at runtime*, which in turn requires either query-event instrumentation
  or periodic index utilisation queries. Deferred to v3 where it is designed
  alongside composite-index utilisation tracking.
- **Composite / multi-column indexes** — the advisor and planner handle
  single-column indexes only. `IndexDef.columns` is a list to accommodate
  future multi-column support; the advisor always passes a single-element list.
  Deferred to v3.
- **UNIQUE indexes** — `IndexDef.unique` field is present but not enforced.
  Deferred to v3.
- **`ANALYZE` / statistics-based cost model** — the planner picks the first
  matching index with no cost comparison. Deferred to v3.
- **Index-only scans** (covering indexes where the SELECT list is fully
  satisfied by the index without touching the table) — deferred to v3.
- **WAL mode** — still rollback journal only in v2.
- **Multi-process locking** — still single-process only.

---

## Relationship to existing specs

- **Extends:** `storage-sqlite.md` (IX-1, IX-2), `sql-backend.md` (index
  interface), `mini-sqlite-python.md` (policy + advisor), `sql-planner.md`
  (index selection), `sql-vm.md` (index DDL execution)
- **Forward compatibility:** the `IndexPolicy` interface is the stable
  extension point for any future decision strategy. v3 will extend it with
  a `should_drop` method and a `cold_window` parameter on `HitCountPolicy`.
