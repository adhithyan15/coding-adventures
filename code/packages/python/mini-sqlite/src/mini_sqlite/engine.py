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

import re
from collections.abc import Mapping, Sequence
from dataclasses import replace
from typing import TYPE_CHECKING, Any

from sql_backend import Backend, backend_as_schema_provider
from sql_backend.schema import ColumnDef as BackendColumnDef
from sql_codegen import compile as codegen_compile
from sql_optimizer import optimize
from sql_parser import parse_sql
from sql_planner import (
    AggregateExpr,
    CreateViewStmt,
    DropViewStmt,
    IndexScan,
    InsertValuesStmt,
    ReleaseSavepointStmt,
    RollbackToStmt,
    SavepointStmt,
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
    parameters: Sequence[Any] | Mapping[str, Any] = (),
    *,
    advisor: IndexAdvisor | None = None,
    check_registry: dict | None = None,
    fk_child: dict | None = None,
    fk_parent: dict | None = None,
    view_defs: dict | None = None,
    savepoints: list[str] | None = None,
    trigger_executor: Any | None = None,
    trigger_depth: int = 0,
    user_functions: dict | None = None,
) -> QueryResult:
    """Execute a single SQL statement and return the :class:`QueryResult`.

    ``parameters`` follows PEP 249 paramstyle:

    * a ``Sequence`` (tuple, list, …) → qmark style; each ``?`` in *sql*
      consumes the next positional value.
    * a ``Mapping`` (dict, …) → named style; each ``:identifier`` in *sql*
      is replaced by ``parameters[identifier]``.

    Empty for un-parameterised statements.

    ``advisor``, when provided, receives the optimised plan via
    :meth:`~mini_sqlite.advisor.IndexAdvisor.observe_plan` so it can
    auto-create indexes based on observed query patterns.

    ``view_defs``, when provided, is a live ``dict[str, SelectStmt]`` owned
    by the :class:`~mini_sqlite.connection.Connection`.  It is passed to the
    adapter so that view names in FROM/JOIN clauses are expanded inline.
    ``CREATE VIEW`` and ``DROP VIEW`` statements update this dict directly;
    they never reach the planner or VM.
    """
    bound = substitute(sql, parameters)
    try:
        # PRAGMA statements are intercepted before parsing — they query backend
        # metadata and return formatted rows without going through the planner.
        if re.match(r"\s*PRAGMA\b", bound, re.IGNORECASE):
            return _run_pragma(backend, bound, fk_child=fk_child)
        ast = parse_sql(bound)
        stmt = to_statement(ast, view_defs=view_defs)

        # CREATE VIEW / DROP VIEW are intercepted here — the planner and VM
        # never see them.  We update the connection's view registry and return
        # an empty DDL result immediately.
        if isinstance(stmt, CreateViewStmt):
            if view_defs is not None:
                if stmt.name in view_defs:
                    if stmt.if_not_exists:
                        pass  # IF NOT EXISTS: silently skip duplicate
                    else:
                        raise ProgrammingError(f"view already exists: {stmt.name}")
                else:
                    view_defs[stmt.name] = stmt.query
            return QueryResult(rows_affected=0)
        if isinstance(stmt, DropViewStmt):
            if view_defs is not None:
                if stmt.name in view_defs:
                    del view_defs[stmt.name]
                elif not stmt.if_exists:
                    raise ProgrammingError(f"no such view: {stmt.name}")
            return QueryResult(rows_affected=0)
        # SAVEPOINT / RELEASE / ROLLBACK TO are intercepted here.
        # The planner and VM never see them — the engine calls the backend
        # directly and keeps the connection's savepoints list in sync.
        if isinstance(stmt, SavepointStmt):
            backend.create_savepoint(stmt.name)
            if savepoints is not None:
                savepoints.append(stmt.name)
            return QueryResult(rows_affected=0)
        if isinstance(stmt, ReleaseSavepointStmt):
            backend.release_savepoint(stmt.name)
            if savepoints is not None and stmt.name in savepoints:
                idx = len(savepoints) - 1 - savepoints[::-1].index(stmt.name)
                del savepoints[idx:]
            return QueryResult(rows_affected=0)
        if isinstance(stmt, RollbackToStmt):
            backend.rollback_to_savepoint(stmt.name)
            if savepoints is not None and stmt.name in savepoints:
                idx = len(savepoints) - 1 - savepoints[::-1].index(stmt.name)
                del savepoints[idx + 1:]
            return QueryResult(rows_affected=0)
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
        # Build a trigger executor on the first (top-level) call; re-use the
        # caller-supplied one for recursive trigger body executions.
        _trigger_executor = trigger_executor
        if _trigger_executor is None:
            _trigger_executor = _make_trigger_executor(
                backend=backend,
                check_registry=check_registry,
                fk_child=fk_child,
                fk_parent=fk_parent,
                view_defs=view_defs,
                user_functions=user_functions,
            )
        return execute(
            program,
            backend,
            check_registry=check_registry,
            fk_child=fk_child,
            fk_parent=fk_parent,
            event_cb=event_cb,
            filtered_columns=_filtered,
            trigger_executor=_trigger_executor,
            trigger_depth=trigger_depth,
            user_functions=user_functions or None,
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


# --------------------------------------------------------------------------
# PRAGMA handler — returns backend metadata as a QueryResult.
# --------------------------------------------------------------------------

# Matches: PRAGMA name  or  PRAGMA name('arg')  or  PRAGMA name("arg")
_PRAGMA_RE = re.compile(
    r"""
    \s* PRAGMA \s+
    (?P<name>[A-Za-z_][A-Za-z0-9_]*)   # pragma name
    (?:                                  # optional argument
        \s* \(
            \s* ["']? (?P<arg>[A-Za-z_][A-Za-z0-9_]*) ["']? \s*
        \)
    )?
    \s* ;? \s* $
    """,
    re.IGNORECASE | re.VERBOSE,
)


def _run_pragma(backend: Backend, sql: str, *, fk_child: dict | None = None) -> QueryResult:
    """Handle a PRAGMA statement by querying backend metadata.

    Supported pragmas (matching SQLite output format):

    ``PRAGMA table_info('t')``
        One row per column: ``(cid, name, type, notnull, dflt_value, pk)``.

    ``PRAGMA index_list('t')``
        One row per index on table *t*: ``(seq, name, unique)``.

    ``PRAGMA foreign_key_list('t')``
        One row per FK on table *t*:
        ``(id, seq, table, from, to, on_update, on_delete, match)``.

    ``PRAGMA table_list``
        One row per table in the schema: ``(schema, name, type)``.
    """
    m = _PRAGMA_RE.match(sql)
    if m is None:
        raise ProgrammingError(f"invalid PRAGMA syntax: {sql!r}")
    name = m.group("name").lower()
    arg = m.group("arg")  # may be None

    if name == "table_info":
        if not arg:
            raise ProgrammingError("PRAGMA table_info requires a table name")
        try:
            cols = backend.columns(arg)
        except Exception:  # noqa: BLE001 — unknown table returns empty
            return QueryResult(
                columns=("cid", "name", "type", "notnull", "dflt_value", "pk"),
                rows=(),
            )
        rows = []
        for i, col in enumerate(cols):
            if isinstance(col, BackendColumnDef):
                not_null = int(col.effective_not_null())
                pk = int(col.primary_key)
                type_name = col.type_name
                dflt = col.default if col.has_default() else None
                name_str = col.name
            else:
                not_null = 0
                pk = 0
                type_name = "TEXT"
                dflt = None
                name_str = str(col)
            rows.append((i, name_str, type_name, not_null, dflt, pk))
        return QueryResult(
            columns=("cid", "name", "type", "notnull", "dflt_value", "pk"),
            rows=tuple(rows),
        )

    if name == "index_list":
        if not arg:
            raise ProgrammingError("PRAGMA index_list requires a table name")
        try:
            indexes = backend.list_indexes(table=arg)
        except Exception:  # noqa: BLE001
            indexes = []
        return QueryResult(
            columns=("seq", "name", "unique"),
            rows=tuple((seq, idx.name, int(idx.unique)) for seq, idx in enumerate(indexes)),
        )

    if name == "foreign_key_list":
        if not arg:
            raise ProgrammingError("PRAGMA foreign_key_list requires a table name")
        fk_rows = []
        if fk_child:
            for fk_id, (from_col, ref_table, ref_col) in enumerate(fk_child.get(arg, [])):
                fk_rows.append((
                    fk_id, 0, ref_table, from_col,
                    ref_col or "", "NO ACTION", "NO ACTION", "NONE",
                ))
        return QueryResult(
            columns=("id", "seq", "table", "from", "to", "on_update", "on_delete", "match"),
            rows=tuple(fk_rows),
        )

    if name == "table_list":
        tables = backend.tables()
        return QueryResult(
            columns=("schema", "name", "type"),
            rows=tuple(("main", t, "table") for t in tables),
        )

    # Unknown PRAGMA — return empty result rather than error, matching SQLite.
    return QueryResult(columns=(), rows=())


# --------------------------------------------------------------------------
# Trigger executor — fires trigger body SQL with NEW/OLD value injection.
# --------------------------------------------------------------------------


def _split_body_sql(body: str) -> list[str]:
    """Split a trigger body SQL string on ' ; ' separators into individual statements."""
    return [s.strip() for s in body.split(" ; ") if s.strip()]


# Matches ``NEW . col`` or ``OLD . col`` (with any surrounding whitespace)
# as generated by the adapter's _node_to_sql helper.
_PSEUDO_REF_RE = re.compile(r"\b(NEW|OLD)\s*\.\s*(\w+)", re.IGNORECASE)


def _inject_pseudo_refs(
    sql: str,
    new_row: dict | None,
    old_row: dict | None,
) -> tuple[str, list[Any]]:
    """Replace ``NEW.col`` / ``OLD.col`` references with ``?`` placeholders.

    Returns ``(rewritten_sql, ordered_params)`` so the body statement can be
    executed as a parameterised query with the actual row values inline rather
    than requiring a live cursor scan of a pseudo-table.

    Replacement is strictly left-to-right so parameter order matches the
    placeholder order the binding layer expects.
    """
    params: list[Any] = []

    def _replace(m: re.Match) -> str:
        pseudo = m.group(1).upper()
        col = m.group(2)
        row = new_row if pseudo == "NEW" else old_row
        params.append(row.get(col) if row else None)
        return "?"

    rewritten = _PSEUDO_REF_RE.sub(_replace, sql)
    return rewritten, params


def _make_trigger_executor(
    *,
    backend: Backend,
    check_registry: dict | None,
    fk_child: dict | None,
    fk_parent: dict | None,
    view_defs: dict | None,
    user_functions: dict | None = None,
) -> Any:
    """Return a callable suitable for passing as ``trigger_executor`` to :func:`execute`.

    The returned executor rewrites each body statement so that ``NEW.col``
    and ``OLD.col`` references are replaced with ``?`` placeholders bound to
    the actual row values.  This avoids the need to create temporary tables
    and keeps each body statement purely data-driven.

    Nested trigger firings (triggers within trigger bodies) are handled by
    passing the same executor recursively; the depth counter is forwarded so
    the VM's recursion guard stays accurate.

    ``new_row`` is ``None`` for ``DELETE`` triggers; ``old_row`` is ``None``
    for ``INSERT`` triggers.
    """

    def executor(defn: Any, new_row: dict | None, old_row: dict | None, depth: int) -> None:
        for stmt_sql in _split_body_sql(defn.body):
            rewritten, params = _inject_pseudo_refs(stmt_sql, new_row, old_row)
            run(
                backend,
                rewritten,
                params,
                check_registry=check_registry,
                fk_child=fk_child,
                fk_parent=fk_parent,
                view_defs=view_defs,
                trigger_executor=executor,
                trigger_depth=depth,
                user_functions=user_functions,
            )

    return executor
