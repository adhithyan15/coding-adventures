"""
IndexAdvisor — automatic B-tree index creation from observed query plans.
=========================================================================

The advisor sits between the SQL engine and the backend.  Every time a
query is executed the engine calls :meth:`IndexAdvisor.observe_plan` with
the *optimised* logical plan.  The advisor walks the plan tree looking for
``Filter(Scan)`` patterns — a full table scan with a filter applied — and
records which columns appear in those filters.  Once a column's hit count
reaches the policy threshold the advisor calls ``backend.create_index``.

Why plan-level observation instead of execution-level?
-------------------------------------------------------

We observe the *plan*, not the execution, for two reasons:

1. **Simplicity**: the plan tells us exactly which columns are in the
   filter without requiring the VM to emit telemetry during execution.

2. **Accuracy**: the plan is consulted *before* execution starts, so we
   can react on the very query that triggers the threshold — not only on
   the next one.

The trade-off is that we see *potential* filter columns, not actually
evaluated ones.  In practice this is fine — if the planner put a ``Filter``
node there, the column is definitely being filtered.

Index naming convention
-----------------------

Auto-created indexes use the name ``auto_{table}_{column}``.  This prefix
makes it easy to identify and optionally drop them.

If a user-created index already covers the column (first column of any
existing index for that table), the advisor skips creation to avoid
redundancy.

Lifecycle
---------

One ``IndexAdvisor`` is created per :class:`~mini_sqlite.connection.Connection`.
If the user calls :meth:`Connection.set_policy` the connection replaces the
advisor's policy in-place; the hit-count table is preserved so accumulated
observations are not lost.
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
        # this column in a filter position.
        self._hits: dict[tuple[str, str], int] = {}

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
        """
        _walk(plan, self._record)

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

    def _maybe_create_index(self, table: str, column: str) -> None:
        """Create ``auto_{table}_{column}`` if no index already covers it."""
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


# --------------------------------------------------------------------------
# Plan-tree walker — finds filter patterns and extracts column references.
# --------------------------------------------------------------------------


def _walk(
    plan: LogicalPlan,
    callback: Callable[[str, str, str | None], None],
) -> None:
    """Walk *plan* depth-first, invoking *callback* for each filter pattern.

    Recognised patterns:

    ``Filter(Scan(t), predicate)``
        → extract column names from ``predicate``; call
        ``callback(t, col, None)`` for each.

    ``IndexScan(t, column, index_name)``
        → call ``callback(t, column, index_name)`` to signal "already indexed".

    All other nodes are recursed into without triggering a callback.
    """
    match plan:
        case Filter(input=Scan(table=table, alias=alias), predicate=pred):
            # A full-table scan with a filter — the interesting case.
            tbl_alias = alias or table
            cols = _filter_columns(pred, tbl_alias)
            for col in cols:
                callback(table, col, None)
            # Still recurse in case of nested queries (shouldn't happen in
            # this position, but be defensive).

        case IndexScan(table=table, column=col, index_name=idx_name):
            # The planner already picked an index — note it without
            # incrementing hit counts.
            callback(table, col, idx_name)

        case Filter(input=inner, predicate=_):
            # Filter over something other than a Scan (e.g. Filter over Join).
            # Recurse into the inner plan.
            _walk(inner, callback)

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
            _walk(inner, callback)

        case DerivedTable(query=inner):
            _walk(inner, callback)

        case Join(left=lhs, right=rhs):
            _walk(lhs, callback)
            _walk(rhs, callback)

        case (
            Union(left=lhs, right=rhs)
            | Intersect(left=lhs, right=rhs)
            | Except(left=lhs, right=rhs)
        ):
            _walk(lhs, callback)
            _walk(rhs, callback)

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
