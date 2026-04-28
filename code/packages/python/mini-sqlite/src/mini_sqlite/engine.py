"""
Thin pipeline orchestrator: SQL text → QueryResult.

Lives between the facade (Connection/Cursor) and the stack of processing
packages. Centralising the call sequence here means ``connection.py`` and
``cursor.py`` never import planner/optimizer/codegen/vm directly — they
just ask the engine for a result.

Exception policy: every exception raised by any pipeline layer is funneled
through :func:`translate` so the caller only ever sees PEP 249 classes.

Index advisor integration
-------------------------

:func:`run` accepts an optional ``advisor`` keyword argument.  When
provided, the engine calls :meth:`~mini_sqlite.advisor.IndexAdvisor.observe_plan`
on the optimised plan *before* code generation.  This lets the advisor
observe the planner's index-scan choices (or lack thereof) and create
auto-indexes when the :class:`~mini_sqlite.policy.IndexPolicy` threshold
is reached.
"""

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import replace
from typing import TYPE_CHECKING, Any

from sql_backend import Backend, backend_as_schema_provider
from sql_codegen import compile as codegen_compile
from sql_optimizer import optimize
from sql_parser import parse_sql
from sql_planner import (
    AggregateExpr,
    IndexScan,
    InsertValuesStmt,
    Scan,
    plan,
)
from sql_planner.plan import (
    Aggregate,
    DerivedTable,
    Distinct,
    Except,
    Filter,
    Having,
    Intersect,
    Join,
    LogicalPlan,
    Project,
    Sort,
    Union,
)
from sql_planner.plan import (
    Limit as PlanLimit,
)
from sql_vm import QueryEvent, QueryResult, execute  # noqa: F401 — QueryEvent re-exported

from .adapter import to_statement
from .binding import substitute
from .errors import ProgrammingError, translate

if TYPE_CHECKING:
    from .advisor import IndexAdvisor


def run(
    backend: Backend,
    sql: str,
    parameters: Sequence[Any] = (),
    *,
    advisor: IndexAdvisor | None = None,
) -> QueryResult:
    """Execute a single SQL statement and return the :class:`QueryResult`.

    ``parameters`` is an ordered sequence matching the ``?`` placeholders
    in ``sql``. Empty for un-parameterised statements.

    ``advisor``, when provided, receives the optimised plan via
    :meth:`~mini_sqlite.advisor.IndexAdvisor.observe_plan` so it can
    auto-create indexes based on observed query patterns.
    """
    bound = substitute(sql, parameters)
    try:
        ast = parse_sql(bound)
        stmt = to_statement(ast)
        # ``INSERT INTO t VALUES (...)`` without an explicit column list
        # means "all columns, in declaration order" — the downstream
        # pipeline expects the list to be populated explicitly, so we
        # resolve it here using the backend's schema.
        if isinstance(stmt, InsertValuesStmt) and stmt.columns is None:
            cols = backend.columns(stmt.table)
            # ``backend.columns`` returns ``ColumnDef`` objects on some
            # backends and bare strings on others; normalise to names.
            names = tuple(getattr(c, "name", c) for c in cols)
            stmt = replace(stmt, columns=names)
        logical = plan(stmt, backend_as_schema_provider(backend))
        optimized = optimize(logical)
        # Notify the advisor about the query plan *before* code generation.
        # This lets the advisor observe which columns were filtered without an
        # index, and create one if the policy threshold has been reached.
        if advisor is not None:
            advisor.observe_plan(optimized)
        program = codegen_compile(_flatten_project_over_aggregate(optimized))
        # Extract scan metadata from the plan so the QueryEvent is populated
        # with the correct table and filtered columns without requiring the VM
        # to parse the predicate structure.
        _table, _filtered = _extract_scan_info(optimized)
        # Only emit QueryEvents for SELECT-type plans.  _extract_scan_info
        # returns an empty string for DML and DDL statements (UPDATE, DELETE,
        # INSERT, CREATE TABLE, …).  We suppress the callback for those so
        # the advisor's cold-window counter only advances on SELECT scans —
        # consistent with the spec language "N consecutive SELECT scans".
        event_cb = (
            advisor.on_query_event
            if (advisor is not None and _table)
            else None
        )
        return execute(
            program,
            backend,
            event_cb=event_cb,
            filtered_columns=_filtered,
        )
    except ProgrammingError:
        # Already-translated errors raised from our own code pass through.
        raise
    except Exception as e:  # noqa: BLE001 — boundary translation point
        raise translate(e) from e


def _flatten_project_over_aggregate(p: LogicalPlan) -> LogicalPlan:
    """Rewrite ``Project(Aggregate(...))`` into a bare ``Aggregate`` with
    the Project items' aliases baked into the aggregate output names.

    The planner wraps every SELECT in a Project for schema uniformity,
    but the codegen's aggregate path expects the Aggregate node at the
    top of the read core. We detect the pattern and strip the Project,
    re-labelling each aggregate alias so the result set columns come
    out with the user-facing names.

    Wrappers (Sort, Distinct, Limit) pass through — we only rewrite the
    Project/Aggregate pair.

    The function also recurses into child plans such as
    :class:`~sql_planner.plan.DerivedTable`, :class:`~sql_planner.plan.Filter`,
    :class:`~sql_planner.plan.Join`, :class:`~sql_planner.plan.Union`, etc.
    so that nested queries inside derived tables are also normalised before
    codegen sees them.
    """
    # ------------------------------------------------------------------
    # First, recursively normalise all child plans so that any
    # Project(Aggregate) pattern inside a derived table or set operation
    # is fixed before we process the outer plan.
    # ------------------------------------------------------------------
    p = _flatten_children(p)

    # ------------------------------------------------------------------
    # Walk down through ordering/limit wrappers looking for Project.
    # ------------------------------------------------------------------
    stack: list[LogicalPlan] = []
    cur: LogicalPlan = p
    while isinstance(cur, (Sort, Distinct, PlanLimit)):
        stack.append(cur)
        cur = cur.input
    if not isinstance(cur, Project):
        return p

    # Determine whether the inner plan is a bare Aggregate or Having(Aggregate).
    having_node: Having | None = None
    if isinstance(cur.input, Having) and isinstance(cur.input.input, Aggregate):
        having_node = cur.input
        aggregate: Aggregate = cur.input.input
    elif isinstance(cur.input, Aggregate):
        aggregate = cur.input
    else:
        return p

    project: Project = cur

    # Pair each aggregate slot with the projection item that consumes it.
    # The planner emits one AggregateItem per AggregateExpr in the SELECT
    # list; positions line up by left-to-right appearance.
    agg_items = list(aggregate.aggregates)
    renamed = list(agg_items)

    # Column projection items over group_by already surface under the
    # column's own name from the Aggregate — no rewrite needed for those.
    # Only AggregateExpr items need their Aggregate slot alias updated.
    for item in project.items:
        if isinstance(item.expr, AggregateExpr):
            for idx, ai in enumerate(agg_items):
                if (
                    ai.func == item.expr.func
                    and ai.arg == item.expr.arg
                    and ai.distinct == item.expr.distinct
                ):
                    renamed[idx] = replace(renamed[idx], alias=item.alias or ai.alias)
                    break

    new_aggregate = replace(aggregate, aggregates=tuple(renamed))

    # Re-wrap Having if it was present, then Sort/Distinct/Limit stack.
    out: LogicalPlan = (
        replace(having_node, input=new_aggregate) if having_node is not None else new_aggregate
    )
    for wrap in reversed(stack):
        out = replace(wrap, input=out)
    return out


def _flatten_children(p: LogicalPlan) -> LogicalPlan:
    """Recursively apply :func:`_flatten_project_over_aggregate` to child plans.

    This ensures that plans embedded inside :class:`~sql_planner.plan.DerivedTable`
    (and set-operation siblings) are normalised before the parent plan is
    processed.  Without this, ``SELECT … FROM (SELECT agg … GROUP BY …) AS dt``
    would fail because the inner ``Project(Aggregate(...))`` is never rewritten.
    """
    match p:
        case DerivedTable(query=inner):
            return replace(p, query=_flatten_project_over_aggregate(inner))
        case Filter(input=inner):
            return replace(p, input=_flatten_children(inner))
        case Project(input=inner):
            return replace(p, input=_flatten_children(inner))
        case Sort(input=inner):
            return replace(p, input=_flatten_children(inner))
        case Distinct(input=inner):
            return replace(p, input=_flatten_children(inner))
        case PlanLimit(input=inner):
            return replace(p, input=_flatten_children(inner))
        case Having(input=inner):
            return replace(p, input=_flatten_children(inner))
        case Aggregate(input=inner):
            return replace(p, input=_flatten_children(inner))
        case Join(left=l, right=r):
            return replace(p, left=_flatten_children(l), right=_flatten_children(r))
        case Union(left=l, right=r):
            return replace(p, left=_flatten_project_over_aggregate(l),
                           right=_flatten_project_over_aggregate(r))
        case Intersect(left=l, right=r):
            return replace(p, left=_flatten_project_over_aggregate(l),
                           right=_flatten_project_over_aggregate(r))
        case Except(left=l, right=r):
            return replace(p, left=_flatten_project_over_aggregate(l),
                           right=_flatten_project_over_aggregate(r))
        case _:
            # Leaf nodes (Scan, Insert, Delete, Update, Create, Drop, Begin,
            # Commit, Rollback) have no child plans to recurse into.
            return p


def _extract_scan_info(plan: LogicalPlan) -> tuple[str, list[str]]:
    """Return ``(table, filtered_columns)`` for the primary scan in *plan*.

    Walks the plan tree looking for the first ``Filter(Scan(t))`` or
    ``IndexScan(t)`` pattern and returns the table name plus the column names
    that appear in the filter predicate.

    Used to pre-populate :class:`~sql_vm.QueryEvent` fields without requiring
    the VM to parse the predicate structure at execution time.

    Returns ``("", [])`` for plans that have no scan (e.g. DDL statements).
    """
    match plan:
        case IndexScan(table=t, columns=cols):
            # IndexScan: the filter columns are the matched index columns.
            return t, list(cols)
        case Filter(input=Scan(table=t), predicate=pred):
            from .advisor import _filter_columns  # local import to avoid circularity
            alias = t
            cols = _filter_columns(pred, alias)
            return t, cols
        case Filter(input=inner):
            return _extract_scan_info(inner)
        case Scan(table=t):
            return t, []
        case (
            Project(input=inner)
            | Distinct(input=inner)
            | Sort(input=inner)
            | PlanLimit(input=inner)
            | Having(input=inner)
            | Aggregate(input=inner)
        ):
            return _extract_scan_info(inner)
        case DerivedTable():
            # Don't recurse into subqueries — focus on the outermost scan.
            return "", []
        case Join(left=lhs):
            # For JOINs, use the left (driving) table.
            return _extract_scan_info(lhs)
        case _:
            return "", []
