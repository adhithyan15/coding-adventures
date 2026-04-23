"""
IndexAdvisor — automatic B-tree index lifecycle management.
===========================================================

The advisor sits between the SQL engine and the backend.  It has two hooks:

1. **:meth:`observe_plan`** — called *before* execution with the optimised
   logical plan.  The advisor walks the plan tree looking for
   ``Filter(Scan)`` patterns (full table scans with a filter applied) and
   records which columns appear in those filters.  Once a column's hit count
   reaches the policy threshold the advisor calls ``backend.create_index``.

2. **:meth:`on_query_event`** — called *after* execution with a
   :class:`~sql_vm.QueryEvent`.  The advisor uses this to track how
   recently each auto-created index was actually used and to drop indexes
   that have gone cold according to the active policy's :meth:`should_drop`
   method.

Why plan-level observation instead of execution-level for creation?
--------------------------------------------------------------------

We observe the *plan*, not the execution, for two reasons:

1. **Simplicity**: the plan tells us exactly which columns are in the
   filter without requiring the VM to emit telemetry during execution.

2. **Accuracy**: the plan is consulted *before* execution starts, so we
   can react on the very query that triggers the threshold — not only on
   the next one.

The trade-off is that we see *potential* filter columns, not actually
evaluated ones.  In practice this is fine — if the planner put a ``Filter``
node there, the column is definitely being filtered.

Drop logic requires execution-level data
----------------------------------------

Dropping indexes correctly requires knowing which indexes were *actually
used at runtime*.  The :meth:`on_query_event` hook receives a
:class:`~sql_vm.QueryEvent` after each SELECT scan that contains
``used_index`` — the name of the index that was used, or ``None`` for a
full table scan.  The advisor uses this to maintain a per-index
``queries_since_last_use`` counter and requests a drop when the policy
threshold is reached.

Index naming convention
-----------------------

Auto-created indexes use the name ``auto_{table}_{column}``.  This prefix
makes it easy to identify them and ensures the advisor never accidentally
drops user-created indexes (which do not start with ``auto_``).

If a user-created index already covers the column (first column of any
existing index for that table), the advisor skips creation to avoid
redundancy.

Lifecycle
---------

One ``IndexAdvisor`` is created per :class:`~mini_sqlite.connection.Connection`.
If the user calls :meth:`Connection.set_policy` the connection replaces the
advisor's policy in-place; the hit-count table and drop-tracking state are
preserved so accumulated observations are not lost.
"""

from __future__ import annotations

import contextlib
from collections.abc import Callable

from sql_backend.backend import Backend
from sql_backend.errors import IndexAlreadyExists
from sql_backend.index import IndexDef
from sql_planner import Between as PlanBetween
from sql_planner import (
    BinaryExpr,
    BinaryOp,
    Column,
    Expr,
    Filter,
    IndexScan,
    Literal,
    LogicalPlan,
    Scan,
)
from sql_planner import In as PlanIn
from sql_planner import NotIn as PlanNotIn
from sql_planner.plan import (
    Aggregate,
    DerivedTable,
    Distinct,
    Except,
    Having,
    Intersect,
    Join,
    Project,
    Sort,
    Union,
)
from sql_planner.plan import Limit as PlanLimit
from sql_vm import QueryEvent

from .policy import HitCountPolicy, IndexPolicy


class IndexAdvisor:
    """Observes optimised query plans and auto-creates indexes when warranted.

    Parameters
    ----------
    backend:
        The database backend.  Used for ``list_indexes`` (to avoid
        duplicates) and ``create_index`` (to materialise new indexes).
    policy:
        Decision policy.  Defaults to :class:`~mini_sqlite.policy.HitCountPolicy`
        with its default threshold (3).  May be replaced at any time via
        :attr:`policy`.

    Example::

        advisor = IndexAdvisor(backend)
        for sql in repeated_queries:
            plan = optimize(plan_sql(sql))
            advisor.observe_plan(plan)
        # After threshold queries on the same column, the index now exists.
        assert any(
            idx.columns[0] == "user_id"
            for idx in backend.list_indexes("orders")
        )
    """

    def __init__(
        self,
        backend: Backend,
        policy: IndexPolicy | None = None,
    ) -> None:
        self._backend = backend
        self._policy: IndexPolicy = policy or HitCountPolicy()
        # hit counts: (table_name, column_name) → how many times we've seen
        # this column in a filter position (single-column tracking).
        self._hits: dict[tuple[str, str], int] = {}
        # pair hit counts: (table_name, col_a, col_b) → how many times we've
        # seen *both* columns filtered in the same query (composite tracking,
        # IX-8).  Only ordered pairs where col_a < col_b in scan order are
        # recorded (first column seen in _filter_columns order, second column
        # next).
        self._pair_hits: dict[tuple[str, str, str], int] = {}
        # Drop-tracking state.
        # _query_count: total SELECT scans observed via on_query_event.
        # _last_use[index_name]: _query_count value when the index was last
        #     seen in a QueryEvent.used_index.
        # _created_at[index_name]: _query_count value when the advisor created
        #     the index (baseline for "never used since creation").
        self._query_count: int = 0
        self._last_use: dict[str, int] = {}
        self._created_at: dict[str, int] = {}
        # Metadata for auto-created indexes: name → (table, columns_tuple).
        # Populated when _maybe_create_index or _maybe_create_composite_index
        # succeeds.  Used by the drop loop to properly reset hit counts when
        # an index is dropped, without relying on name-parsing heuristics.
        self._auto_index_meta: dict[str, tuple[str, tuple[str, ...]]] = {}

    # ------------------------------------------------------------------
    # Policy swap.
    # ------------------------------------------------------------------

    @property
    def policy(self) -> IndexPolicy:
        """The active index-creation policy."""
        return self._policy

    @policy.setter
    def policy(self, new_policy: IndexPolicy) -> None:
        """Replace the policy (hit counts are preserved)."""
        self._policy = new_policy

    # ------------------------------------------------------------------
    # Public observation hook.
    # ------------------------------------------------------------------

    def observe_plan(self, plan: LogicalPlan) -> None:
        """Walk *plan* and record filter-column observations.

        Called by the engine after each ``optimize()`` before code
        generation.  Mutates internal hit-count state and may trigger
        ``create_index`` calls on the backend.

        The method is idempotent with respect to index creation: if an
        index already exists for a ``(table, column)`` pair, the advisor
        skips creation even if the policy says "yes".

        IX-8: pair callbacks are processed *before* single-column callbacks
        within each :class:`~sql_planner.plan.Filter` node.  This ordering
        ensures that when both the pair threshold and the single-column
        threshold are reached simultaneously, the composite index is created
        first and the subsequent single-column check for the leading column
        finds an existing covering index (and skips the single).
        """
        _walk(plan, self._record, self._record_pair)

    def on_query_event(self, event: QueryEvent) -> None:
        """Process one :class:`~sql_vm.QueryEvent` emitted after a SELECT scan.

        Called by the engine after each ``vm.execute()`` call that performed
        a table or index scan.  Advances the query counter, updates the
        last-use timestamp for the used index (if any), then checks all
        tracked auto-created indexes against the policy's
        :meth:`~mini_sqlite.policy.IndexPolicy.should_drop` method.

        Drop semantics
        --------------
        An index is a candidate for dropping when:

        - Its name starts with ``auto_`` (user-created indexes are never
          touched automatically).
        - The policy implements ``should_drop`` (detected via ``hasattr``).
        - ``policy.should_drop(name, table, col, queries_since_last_use)``
          returns ``True``.

        Failures during ``drop_index`` are swallowed — the advisor continues
        running; the index simply stays in place until the next opportunity.
        """
        self._query_count += 1

        # Record index utilisation.
        if event.used_index is not None and event.used_index.startswith("auto_"):
            self._last_use[event.used_index] = self._query_count

        # Check whether the policy wants to drop any cold auto-indexes.
        should_drop_fn = getattr(self._policy, "should_drop", None)
        if not callable(should_drop_fn):
            return

        # Collect all auto-indexes we know about: those we created plus any
        # that were already tracked in _last_use from previous events.
        candidates = set(self._created_at) | set(self._last_use)
        for idx_name in list(candidates):
            last = self._last_use.get(idx_name, self._created_at.get(idx_name, 0))
            queries_since = self._query_count - last

            # Resolve table + columns from stored metadata when available.
            # Fall back to name-parsing for indexes whose metadata was not
            # captured (e.g. single-column indexes created before IX-8).
            meta = self._auto_index_meta.get(idx_name)
            if meta is not None:
                table, columns = meta
                # Pass the first column to should_drop for the column arg
                # (the spec argument is informational; HitCountPolicy ignores it).
                column = columns[0] if columns else ""
            else:
                # Legacy path: parse auto_{table}_{column} by splitting on "_".
                parts = idx_name.split("_", 2)  # ["auto", table, column]
                if len(parts) != 3:
                    continue
                _, table, column = parts
                columns = (column,)

            if should_drop_fn(idx_name, table, column, queries_since):
                try:
                    self._backend.drop_index(idx_name, if_exists=True)
                except Exception:  # noqa: BLE001 — drop failures are non-fatal
                    continue
                # Remove from tracking so the advisor doesn't re-check it.
                self._created_at.pop(idx_name, None)
                self._last_use.pop(idx_name, None)
                self._auto_index_meta.pop(idx_name, None)
                # Reset single-column hit counts so the index can be
                # re-created if the workload pattern returns.
                for col in columns:
                    self._hits.pop((table, col), None)
                # Reset pair hit counts for composite indexes (2-column).
                if len(columns) == 2:
                    self._pair_hits.pop((table, columns[0], columns[1]), None)

    def on_query_event(self, event: QueryEvent) -> None:
        """Process one :class:`~sql_vm.QueryEvent` emitted after a SELECT scan.

        Called by the engine after each ``vm.execute()`` call that performed
        a table or index scan.  Advances the query counter, updates the
        last-use timestamp for the used index (if any), then checks all
        tracked auto-created indexes against the policy's
        :meth:`~mini_sqlite.policy.IndexPolicy.should_drop` method.

        Drop semantics
        --------------
        An index is a candidate for dropping when:

        - Its name starts with ``auto_`` (user-created indexes are never
          touched automatically).
        - The policy implements ``should_drop`` (detected via ``hasattr``).
        - ``policy.should_drop(name, table, col, queries_since_last_use)``
          returns ``True``.

        Failures during ``drop_index`` are swallowed — the advisor continues
        running; the index simply stays in place until the next opportunity.
        """
        self._query_count += 1

        # Record index utilisation.
        if event.used_index is not None and event.used_index.startswith("auto_"):
            self._last_use[event.used_index] = self._query_count

        # Check whether the policy wants to drop any cold auto-indexes.
        should_drop_fn = getattr(self._policy, "should_drop", None)
        if not callable(should_drop_fn):
            return

        # Collect all auto-indexes we know about: those we created plus any
        # that were already tracked in _last_use from previous events.
        candidates = set(self._created_at) | set(self._last_use)
        for idx_name in list(candidates):
            last = self._last_use.get(idx_name, self._created_at.get(idx_name, 0))
            queries_since = self._query_count - last
            # Derive table and column from the naming convention
            # auto_{table}_{column}.  If the name doesn't follow the
            # convention we skip it rather than guessing.
            parts = idx_name.split("_", 2)  # ["auto", table, column]
            if len(parts) != 3:
                continue
            _, table, column = parts
            if should_drop_fn(idx_name, table, column, queries_since):
                try:
                    self._backend.drop_index(idx_name, if_exists=True)
                except Exception:  # noqa: BLE001 — drop failures are non-fatal
                    continue
                # Remove from tracking so the advisor doesn't re-check it.
                self._created_at.pop(idx_name, None)
                self._last_use.pop(idx_name, None)
                # Also reset the hit count so the index can be re-created if
                # the workload pattern returns.
                key = (table, column)
                self._hits.pop(key, None)

    # ------------------------------------------------------------------
    # Internal callbacks.
    # ------------------------------------------------------------------

    def _record(self, table: str, column: str, used_index: str | None) -> None:
        """Record a single filter observation and maybe create an index.

        ``used_index`` is non-None when the plan already used an ``IndexScan``
        for this column — in that case we don't increment the hit count
        (the index already exists and is being used; no action needed).
        """
        if used_index is not None:
            # The planner already selected an index — no work to do.
            return
        key = (table, column)
        self._hits[key] = self._hits.get(key, 0) + 1
        hit_count = self._hits[key]
        if not self._policy.should_create(table, column, hit_count):
            return
        self._maybe_create_index(table, column)

    def _record_pair(self, table: str, col_a: str, col_b: str) -> None:
        """Record a co-filtered column pair and maybe create a composite index.

        Called for each ordered pair of columns observed in the same
        ``Filter(Scan(t))`` predicate (IX-8).  The pair ``(col_a, col_b)``
        reflects the column order as returned by :func:`_filter_columns` —
        col_a is the first-encountered column, col_b the second.

        Uses ``policy.should_create(table, "{col_a}_{col_b}", hit_count)``
        to decide when to act.  Since :class:`~mini_sqlite.policy.HitCountPolicy`
        only checks ``hit_count >= threshold``, the synthetic column string
        is purely informational.
        """
        key = (table, col_a, col_b)
        self._pair_hits[key] = self._pair_hits.get(key, 0) + 1
        hit_count = self._pair_hits[key]
        # Use a synthetic column name to represent the pair in the policy call.
        if not self._policy.should_create(table, f"{col_a}_{col_b}", hit_count):
            return
        self._maybe_create_composite_index(table, col_a, col_b)

    def _maybe_create_index(self, table: str, column: str) -> None:
        """Create ``auto_{table}_{column}`` if no index already covers it.

        Skips creation when any existing index already has ``column`` as its
        leading column — including a composite index that starts with
        ``column`` (e.g. ``auto_t_column_other``), which already accelerates
        queries that filter only on ``column``.
        """
        try:
            existing = self._backend.list_indexes(table)
        except Exception:  # noqa: BLE001 — backend errors are non-fatal here
            return
        # Skip if any existing index already has this column as its first key.
        for idx in existing:
            if idx.columns and idx.columns[0] == column:
                return
        name = f"auto_{table}_{column}"
        idx_def = IndexDef(
            name=name,
            table=table,
            columns=[column],
            unique=False,
            auto=True,
        )
        with contextlib.suppress(IndexAlreadyExists):
            self._backend.create_index(idx_def)
            # Record creation time so the drop loop can compute
            # queries_since_last_use from the moment the index was born.
            self._created_at[name] = self._query_count
            self._auto_index_meta[name] = (table, (column,))

    def _maybe_create_composite_index(
        self,
        table: str,
        col_a: str,
        col_b: str,
    ) -> None:
        """Create ``auto_{table}_{col_a}_{col_b}`` if no covering index exists.

        Skips creation when any index on the table already has ``col_a`` as
        its leading column.  The rationale: if a single-column index on
        ``col_a`` (or a composite starting with ``col_a``) already exists and
        is being used, adding another composite index would be redundant while
        ``col_a``-only queries remain dominant.

        If no index covers the leading column, a new two-column composite
        index is created.  This replaces the need for two separate
        single-column indexes when both columns are always filtered together.
        """
        try:
            existing = self._backend.list_indexes(table)
        except Exception:  # noqa: BLE001 — backend errors are non-fatal here
            return
        # Skip if any existing index already has col_a as its leading column.
        # This includes both single-column ``auto_t_col_a`` and composite
        # ``auto_t_col_a_col_b`` indexes.
        for idx in existing:
            if idx.columns and idx.columns[0] == col_a:
                return
        name = f"auto_{table}_{col_a}_{col_b}"
        idx_def = IndexDef(
            name=name,
            table=table,
            columns=[col_a, col_b],
            unique=False,
            auto=True,
        )
        with contextlib.suppress(IndexAlreadyExists):
            self._backend.create_index(idx_def)
            self._created_at[name] = self._query_count
            self._auto_index_meta[name] = (table, (col_a, col_b))


# --------------------------------------------------------------------------
# Plan-tree walker — finds filter patterns and extracts column references.
# --------------------------------------------------------------------------


def _walk(
    plan: LogicalPlan,
    callback: Callable[[str, str, str | None], None],
    pair_callback: Callable[[str, str, str], None] | None = None,
) -> None:
    """Walk *plan* depth-first, invoking *callback* for each filter pattern.

    Recognised patterns:

    ``Filter(Scan(t), predicate)``
        → extract column names from ``predicate``; for each column call
        ``callback(t, col, None)``.

        IX-8: if *pair_callback* is provided and two or more indexable
        columns are present in the same predicate, also call
        ``pair_callback(t, col_a, col_b)`` for each ordered pair.  Pairs
        are processed **before** single-column callbacks so that a composite
        index created by a pair callback is visible to the subsequent
        single-column checks (which then skip the leading column as already
        covered).

    ``IndexScan(t, columns, index_name)``
        → call ``callback(t, col, index_name)`` for each column in
        ``columns`` to signal "already indexed" — this prevents the hit
        count from accumulating for columns that already benefit from an
        index.

    All other nodes are recursed into without triggering a callback.
    """
    match plan:
        case Filter(input=Scan(table=table, alias=alias), predicate=pred):
            # A full-table scan with a filter — the interesting case.
            tbl_alias = alias or table
            cols = _filter_columns(pred, tbl_alias)
            # IX-8: process pairs FIRST so that a newly-created composite
            # index is visible to the subsequent single-column callbacks.
            if pair_callback is not None and len(cols) >= 2:
                for i in range(len(cols)):
                    for j in range(i + 1, len(cols)):
                        pair_callback(table, cols[i], cols[j])
            for col in cols:
                callback(table, col, None)

        case IndexScan(table=table, columns=idx_cols, index_name=idx_name):
            # The planner already picked an index — note each covered column
            # without incrementing hit counts (no new index needed).
            for col in idx_cols:
                callback(table, col, idx_name)

        case Filter(input=inner, predicate=_):
            # Filter over something other than a Scan (e.g. Filter over Join).
            # Recurse into the inner plan.
            _walk(inner, callback, pair_callback)

        case Scan():
            # Bare scan with no filter above it — nothing to record.
            pass

        case (
            Project(input=inner)
            | Distinct(input=inner)
            | Sort(input=inner)
            | PlanLimit(input=inner)
            | Having(input=inner)
            | Aggregate(input=inner)
        ):
            _walk(inner, callback, pair_callback)

        case DerivedTable(query=inner):
            _walk(inner, callback, pair_callback)

        case Join(left=lhs, right=rhs):
            _walk(lhs, callback, pair_callback)
            _walk(rhs, callback, pair_callback)

        case (
            Union(left=lhs, right=rhs)
            | Intersect(left=lhs, right=rhs)
            | Except(left=lhs, right=rhs)
        ):
            _walk(lhs, callback, pair_callback)
            _walk(rhs, callback, pair_callback)

        case _:
            # Leaf nodes (Begin, Commit, Rollback, Insert, Update, Delete,
            # CreateTable, DropTable, CreateIndex, DropIndex) — nothing to do.
            pass


def _filter_columns(predicate: Expr, alias: str) -> list[str]:
    """Extract column names referenced in *predicate* for the given alias.

    Only includes columns that are compared against a ``Literal`` value in
    a position usable by a B-tree index (equality, range, BETWEEN, IN).
    Columns referenced only inside OR sub-predicates or complex expressions
    are excluded — an index on such a column would be unlikely to help.

    This is intentionally conservative: we'd rather under-create indexes
    than create ones that don't pay off.

    Supported filter shapes (indexable):
    - ``col = literal``   / ``literal = col``
    - ``col < / <= / > / >= literal``  (and reversed)
    - ``col BETWEEN lo AND hi``
    - ``col IN (v1, v2, ...)``
    - ``cond1 AND cond2``  (recurse into both halves)
    """
    out: list[str] = []
    _extract_indexable_columns(predicate, alias, out)
    return list(dict.fromkeys(out))  # deduplicate while preserving order


def _is_indexed_col(expr: Expr, alias: str) -> str | None:
    """Return the column name if *expr* is ``Column(alias, col)``, else None."""
    if isinstance(expr, Column) and expr.table == alias:
        return expr.col
    return None


def _extract_indexable_columns(pred: Expr, alias: str, out: list[str]) -> None:
    """Recursively collect indexable columns into *out*."""
    if isinstance(pred, BinaryExpr):
        op = pred.op
        left, right = pred.left, pred.right

        if op == BinaryOp.AND:
            # AND: recurse into both halves — each half may produce columns.
            _extract_indexable_columns(left, alias, out)
            _extract_indexable_columns(right, alias, out)
            return

        if op in (BinaryOp.EQ, BinaryOp.LT, BinaryOp.LTE, BinaryOp.GT, BinaryOp.GTE):
            # Equality / range: one side must be our column, the other a literal.
            col = _is_indexed_col(left, alias)
            if col is not None and isinstance(right, Literal):
                out.append(col)
                return
            col = _is_indexed_col(right, alias)
            if col is not None and isinstance(left, Literal):
                out.append(col)
                return

    if isinstance(pred, PlanBetween):
        col = _is_indexed_col(pred.operand, alias)
        if col is not None and isinstance(pred.low, Literal) and isinstance(pred.high, Literal):
            out.append(col)
            return

    if isinstance(pred, (PlanIn, PlanNotIn)):
        col = _is_indexed_col(pred.operand, alias)
        if col is not None and all(isinstance(v, Literal) for v in pred.values):
            out.append(col)
            return
