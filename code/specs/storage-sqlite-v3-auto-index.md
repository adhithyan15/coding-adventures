# storage-sqlite v3 — Index Lifecycle Management

## Overview

v3 completes the automatic index lifecycle that v2 started. v2 creates indexes
automatically but never removes them. That asymmetry has a real cost: a workload
that queries by `user_id` for a month and then switches to querying by `email`
will accumulate an unused index on `user_id` that adds write overhead forever.

v3 closes the loop by teaching the advisor to **drop cold indexes** — indexes
that have not been used in a configurable window of queries. It also introduces
**composite (multi-column) index support**: the planner and advisor learn to
recognize `col_a = ? AND col_b > ?` as a candidate for a single two-column
index rather than two separate single-column indexes.

### Guiding principles

1. **No silent data loss.** The advisor only drops *auto-created* indexes
   (those whose name starts with `auto_`). User-created indexes are never
   touched automatically.
2. **Explicit beats implicit for policy.** Drop decisions use the same
   pluggable `IndexPolicy` pattern as create decisions. Custom policies can
   override both.
3. **Observation drives decisions.** Drop logic requires knowing which indexes
   were *actually used at runtime*, not just which ones the planner considered.
   This necessitates the query-event instrumentation that was deferred from v2.

---

## What changes relative to v2

| Component | v2 state | v3 addition |
|---|---|---|
| `sql_vm` | executes plans, no events | + emits `QueryEvent` after each SELECT scan |
| `mini_sqlite.policy` | `should_create` only | + `should_drop(index_name, table, column) -> bool` |
| `mini_sqlite.advisor` | creates indexes only | + drop loop: checks should_drop after each query |
| `mini_sqlite.policy.HitCountPolicy` | `threshold` only | + `cold_window` parameter |
| `sql_planner` | single-column IndexScan | + multi-column IndexScan for compound AND predicates |
| `sql_codegen` / `sql_vm` | single-column scan_index | + multi-column lo/hi key support |

Everything else (pager, record, freelist, schema, btree, index_tree, backend
interface, optimizer, parser, lexer) is **untouched**.

---

## Architecture

```
┌────────────────────────────────────────────────────────────────┐
│  mini_sqlite.Connection                                         │
│                                                                 │
│  1. parse + plan + optimize → LogicalPlan                       │
│  2. advisor.observe_plan(plan) → may trigger create_index       │
│  3. codegen + vm.run(plan, event_cb=advisor.on_query_event)     │
│     └─ vm calls event_cb with QueryEvent after each SELECT scan │
│  4. advisor.on_query_event(event) → may trigger drop_index      │
└────────────────────────────────────────────────────────────────┘
```

The advisor has two hooks:

- **`observe_plan(plan)`** — called before execution; drives *creation*.
- **`on_query_event(event)`** — called after execution; drives *drops*.

Both hooks are synchronous and fast. No I/O in the critical path beyond the
existing `list_indexes` and `create_index` / `drop_index` backend calls.

---

## Component specifications

### Component 1 — Query event system (`sql_vm`)

After every successful `SELECT` that performs a scan (full or index), the VM
calls an optional event callback registered by the caller.

```python
@dataclass
class QueryEvent:
    table: str                    # primary table being scanned
    filtered_columns: list[str]   # columns that appeared in WHERE predicates
    rows_scanned: int             # rows examined (full-scan count or index range)
    rows_returned: int            # rows in the result set
    used_index: str | None        # name of the index used, or None (full scan)
    duration_us: int              # wall-clock microseconds
```

The VM's `run()` method gains an optional `event_cb` parameter:

```python
def run(
    plan: LogicalPlan,
    *,
    event_cb: Callable[[QueryEvent], None] | None = None,
) -> QueryResult:
    """Execute plan. If event_cb is given, call it after each SELECT scan."""
```

`QueryEvent` is emitted for SELECT statements that produce a scan node.
INSERT / UPDATE / DELETE / DDL do not emit events.

`filtered_columns` is populated from the WHERE predicate columns that were
actually present in the executing plan node — the same columns the advisor
observes from the plan in its `observe_plan` hook. These two views are
consistent: if the planner put a filter there, the VM sees the same columns.

---

### Component 2 — Policy extension: `should_drop`

The `IndexPolicy` protocol gains a second method:

```python
@runtime_checkable
class IndexPolicy(Protocol):
    def should_create(self, table: str, column: str, hit_count: int) -> bool:
        """Return True when an index on table.column should be created."""
        ...

    def should_drop(
        self,
        index_name: str,
        table: str,
        column: str,
        queries_since_last_use: int,
    ) -> bool:
        """Return True when the auto-created index should be dropped.

        index_name: the auto-created index name (e.g. "auto_orders_user_id")
        table, column: the table and first column of the index
        queries_since_last_use: how many SELECT scans have run since this
            index was last seen in a QueryEvent.used_index

        The advisor only calls this for auto-created indexes (name starts
        with "auto_"). User-created indexes are never passed to should_drop.
        """
        ...
```

Existing v2 custom policies that only implement `should_create` remain valid
because the advisor calls `should_drop` only when the policy also implements
it. The advisor detects whether the policy supports drop via:

```python
hasattr(policy, "should_drop") and callable(policy.should_drop)
```

This keeps backward compatibility: a v2-style policy that only defines
`should_create` will never have its (absent) `should_drop` called.

---

### Component 3 — `HitCountPolicy` v3

`HitCountPolicy` adds a `cold_window` parameter:

```python
class HitCountPolicy:
    """Create an index after N full-table scans; drop it after M cold queries.

    Parameters
    ----------
    threshold : int
        Number of full-table scans on a column before requesting an index.
        Default 3.
    cold_window : int
        Number of SELECT scans (any table) without seeing this index used
        before requesting a drop. 0 disables automatic dropping (default).

    When cold_window = 0 (the default), HitCountPolicy behaves identically
    to its v2 form — no indexes are ever dropped.

    Setting cold_window > 0 enables the drop cycle. For example,
    cold_window=100 means: if an auto-created index has not appeared in
    QueryEvent.used_index for 100 consecutive SELECT scans, request a drop.

    The query counter advances on every QueryEvent received by
    advisor.on_query_event(), regardless of which table is involved.
    This is intentional: the window reflects overall query activity, not
    per-table activity, so that a table that is rarely queried does not
    have its index dropped prematurely.
    """

    def __init__(
        self,
        threshold: int = 3,
        *,
        cold_window: int = 0,
    ) -> None:
        if threshold < 1:
            raise ValueError(f"threshold must be >= 1, got {threshold!r}")
        if cold_window < 0:
            raise ValueError(f"cold_window must be >= 0, got {cold_window!r}")
        self._threshold = threshold
        self._cold_window = cold_window
        # last_used_at[(index_name)] = query counter at last use
        self._last_used_at: dict[str, int] = {}
        self._query_count: int = 0

    @property
    def threshold(self) -> int: ...

    @property
    def cold_window(self) -> int: ...

    def should_create(self, table: str, column: str, hit_count: int) -> bool:
        # Unchanged from v2.
        ...

    def should_drop(
        self,
        index_name: str,
        table: str,
        column: str,
        queries_since_last_use: int,
    ) -> bool:
        # Return True if cold_window > 0 and
        # queries_since_last_use >= cold_window.
        ...
```

The `_last_used_at` dict and `_query_count` are updated by the advisor — not
by the policy directly. The advisor calls:
- `policy.notify_index_used(index_name)` after each `QueryEvent` where
  `used_index` is not None — records the current query count for the index.
- `policy.notify_query()` after each `QueryEvent` — advances the counter.

Wait — these "notify" methods would be policy-specific. Instead, the advisor
derives `queries_since_last_use` itself and passes it into `should_drop`. The
policy implementation may keep its own per-index counters internally or rely
purely on the `queries_since_last_use` argument. `HitCountPolicy` uses the
argument directly: it just compares against `cold_window`.

---

### Component 4 — Advisor v3

The advisor gains the `on_query_event` hook and the drop loop.

```python
class IndexAdvisor:
    def __init__(
        self,
        backend: Backend,
        policy: IndexPolicy | None = None,
    ) -> None: ...

    @property
    def policy(self) -> IndexPolicy: ...

    @policy.setter
    def policy(self, new_policy: IndexPolicy) -> None:
        """Replace the policy. Hit counts are preserved; drop counters reset."""
        ...

    def observe_plan(self, plan: LogicalPlan) -> None:
        """Walk plan for Filter(Scan) patterns; may create indexes."""
        ...

    def on_query_event(self, event: QueryEvent) -> None:
        """Process one query event. Updates use-tracking; may drop indexes.

        Called by the engine after vm.run() for every SELECT scan.

        If event.used_index is not None, records that the index was used.
        After recording, checks all auto-created indexes against the policy's
        should_drop. Drops any index for which the policy returns True.
        """
        ...
```

#### Drop loop design

After each `on_query_event` call:

1. Advance an internal query counter (`_query_count += 1`).
2. If `event.used_index` is not None and it names an auto-created index,
   record `_last_use[event.used_index] = _query_count`.
3. For every auto-created index tracked in `_last_use` (and any that were
   created by this advisor but have never been used, using `_created_at`):
   - Compute `queries_since_last_use = _query_count - _last_use.get(idx, _created_at[idx])`.
   - If `policy.should_drop(idx, table, col, queries_since_last_use)` returns
     True, call `backend.drop_index(idx, if_exists=True)` and remove it from
     the tracking dicts.

The advisor only drops indexes whose names start with `auto_`. Indexes
created by the user (via explicit `CREATE INDEX`) are never in the advisor's
`_created_at` dict and are never evaluated for dropping.

#### Policy swap in v3

When the policy is swapped via `advisor.policy = new_policy`:

- Hit counts (`_hits`) are preserved — the new policy will see the same
  accumulated counts on the next `observe_plan` call.
- Drop tracking state (`_query_count`, `_last_use`, `_created_at`) is
  preserved in the advisor and is independent of the policy.
- The new policy starts fresh with its own internal state (e.g., a new
  `HitCountPolicy`'s `_last_used_at` dict starts empty).

---

### Component 5 — Composite index advisor (planner + advisor)

v3 extends the advisor and planner to recognise compound `AND` predicates as
candidates for a single multi-column index.

#### Multi-column index naming

```
auto_{table}_{col1}_{col2}
# example:
auto_orders_user_id_status
```

#### Advisor: compound predicate extraction

When the advisor walks a `Filter(Scan(t), pred)` node, it already recurses
into `AND` sub-predicates and collects multiple columns (e.g., `user_id`
*and* `status`). In v2 each column is recorded and evaluated independently.
In v3 the advisor also considers the *set* of columns together:

```
single columns observed: {user_id, status}
compound pair hits:      {(user_id, status): N}
```

When a compound pair's hit count crosses a (configurable) threshold, the
advisor creates a two-column index instead of two single-column indexes. The
advisor avoids creating a compound index if a single-column index on the
leading column already exists and is being used (it may be sufficient).

For v3 the advisor only considers **pairs** (two columns). Three or more
column composites are deferred to v4.

#### Planner: multi-column index selection

The planner's index selection step is extended:

```
for each Scan(table, predicate) in the logical plan:
    best = None
    for each index in list_indexes(table):
        matched_cols = prefix_match(index.columns, predicate)
        if len(matched_cols) > len(best.matched_cols if best else []):
            best = (index, matched_cols, range_from_predicate)
    if best:
        replace Scan with IndexScan(best.index, best.range, residual)
```

*Prefix match*: the index is eligible if the predicate covers the leading
column(s) of the index in order (the standard B-tree prefix rule). A
two-column index `(a, b)` is chosen over a single-column index `(a)` if the
predicate filters on both `a` and `b` — more columns matched means less
post-filter work.

The `residual` on the `IndexScan` carries any predicate columns not covered by
the chosen index (e.g., if the index is on `(a, b)` but the predicate also
filters on `c`, the `c` condition becomes the residual filter applied row by
row after the index scan).

#### `IndexScan` node extension

```python
@dataclass
class IndexScan:
    table: str
    index_name: str
    columns: list[str]              # now a list (was a single column in v2)
    lo: list[SqlValue] | None       # multi-column lower bound
    hi: list[SqlValue] | None       # multi-column upper bound
    lo_inclusive: bool
    hi_inclusive: bool
    residual: Expr | None
```

The single-column `column: str` field from v2 is replaced by `columns:
list[str]`. Single-column indexes produce a list of length 1, which is
backward-compatible with all existing VM and codegen paths (they already
pass lists to `scan_index`).

---

## End-to-end examples

### Index drop

```python
conn = mini_sqlite.connect(":memory:", auto_index=True)
from mini_sqlite.policy import HitCountPolicy
# Create after 3 hits; drop after 50 queries without use.
conn.set_policy(HitCountPolicy(threshold=3, cold_window=50))

conn.execute("CREATE TABLE orders (id INTEGER PRIMARY KEY, user_id INTEGER)")
# ... insert rows ...
conn.commit()

# 3 queries on user_id → index created.
for _ in range(3):
    conn.execute("SELECT * FROM orders WHERE user_id = ?", (1,)).fetchall()

# Switch workload: query by id only for 50+ queries.
for i in range(55):
    conn.execute("SELECT * FROM orders WHERE id = ?", (i,)).fetchall()

# auto_orders_user_id was cold for 55 queries ≥ cold_window=50 → dropped.
assert not any(
    idx.name == "auto_orders_user_id"
    for idx in conn._advisor._backend.list_indexes("orders")
)
```

### Composite index

```python
conn = mini_sqlite.connect(":memory:", auto_index=True)
conn.execute("CREATE TABLE orders (id INTEGER PRIMARY KEY, user_id INTEGER, status TEXT)")
# ... insert rows ...
conn.commit()

# 3 queries filtering on both user_id AND status.
for _ in range(3):
    conn.execute(
        "SELECT * FROM orders WHERE user_id = ? AND status = ?",
        (1, "shipped"),
    ).fetchall()

# Advisor creates a two-column index.
indexes = conn._advisor._backend.list_indexes("orders")
assert any(
    idx.columns == ["user_id", "status"]
    for idx in indexes
)
```

---

## Phased build order

| Phase | Deliverable | Packages touched |
|---|---|---|
| IX-7 | `QueryEvent` emitted by `sql_vm`; `should_drop` on `IndexPolicy`; `cold_window` on `HitCountPolicy`; `on_query_event` on `IndexAdvisor`; engine wires event callback | `sql-vm`, `mini-sqlite` |
| IX-8 | Composite (multi-column) index advisor + planner prefix-match selection + `IndexScan.columns` list | `mini-sqlite`, `sql-planner`, `sql-codegen`, `sql-vm` |

Each phase is a separate feature branch and PR.

---

## Testing strategy

### IX-7: Drop logic tests (`mini-sqlite/tests/test_tier3_drop.py`)

**`HitCountPolicy` with `cold_window`:**
- `cold_window=0` (default): `should_drop` always returns False
- `should_drop` returns True when `queries_since_last_use >= cold_window`
- `should_drop` returns False when `queries_since_last_use < cold_window`
- `cold_window < 0` raises `ValueError`

**`QueryEvent`:**
- VM emits `QueryEvent` after a full-table scan SELECT
- VM emits `QueryEvent` after an index-scan SELECT with `used_index` set
- VM does NOT emit `QueryEvent` for INSERT / UPDATE / DELETE / DDL
- `rows_scanned` matches actual scan count
- `filtered_columns` matches the WHERE predicate columns

**`IndexAdvisor.on_query_event`:**
- Index used → `should_drop` not triggered while `queries_since_last_use < cold_window`
- Index unused for `cold_window` queries → `drop_index` called on backend
- Advisor only drops `auto_`-prefixed indexes; user-created indexes are untouched
- `drop_index` failure is non-fatal (advisor continues)
- Policy without `should_drop` → drop loop is skipped entirely

**Integration: create then drop cycle:**
- Connect with `HitCountPolicy(threshold=3, cold_window=10)`.
- Run 3 queries on `col_a` → index created.
- Run 10 queries on a different column (no index used).
- Index is automatically dropped.
- Run 3 more queries on `col_a` → index re-created.

### IX-8: Composite index tests (`mini-sqlite/tests/test_tier3_composite.py`)

**Advisor:**
- Compound AND predicate on `(col_a, col_b)` seen 3 times → two-column index
  `auto_t_col_a_col_b` created.
- Single-column index on leading column already exists → composite not created
  (single-column is sufficient while leading-column queries dominate).
- Columns observed in different queries on different subsets → single-column
  indexes created independently (no spurious composite).

**Planner:**
- Query `WHERE a = ? AND b > ?` with composite index `(a, b)` → `IndexScan`
  with `columns=["a", "b"]`.
- Query `WHERE a = ?` alone with composite index `(a, b)` → `IndexScan` with
  `columns=["a"]` (leading prefix match).
- Query `WHERE b = ?` alone with composite index `(a, b)` → full table scan
  (non-leading column not eligible).
- Composite index preferred over single-column when both exist and query
  covers both columns.

---

## Non-goals (v3)

- **Three-or-more column composite indexes** — deferred to v4.
- **UNIQUE index enforcement** — `IndexDef.unique` field reserved but not
  enforced. Deferred to a dedicated uniqueness spec.
- **`ANALYZE` / statistics-based cost model** — first-match / prefix-length
  heuristic only. Deferred to v4.
- **Index-only scans** — deferred to v4.
- **WAL mode** — still rollback journal only.
- **Multi-process locking** — still single-process only.
- **Partial indexes** (`CREATE INDEX ... WHERE condition`) — deferred.
- **Descending indexes** — all indexes are ascending (standard SQLite default).

---

## Relationship to existing specs

- **Extends:** `storage-sqlite-v2-auto-index.md` (all v2 infrastructure reused
  verbatim; this spec adds the drop and composite layers on top)
- **Forward compatibility:** the `IndexPolicy` protocol extension (adding
  `should_drop`) is backward compatible — v2 policies without `should_drop`
  continue to work. The `IndexScan.columns: list[str]` change is backward
  compatible — single-column indexes produce a length-1 list, which all
  existing VM and codegen paths already handle.
