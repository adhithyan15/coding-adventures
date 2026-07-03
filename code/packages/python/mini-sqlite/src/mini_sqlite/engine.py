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

import contextlib
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
    AlterTableStmt,
    CreateIndexStmt,
    CreateTableStmt,
    CreateTriggerStmt,
    CreateViewStmt,
    DeleteStmt,
    DropIndexStmt,
    DropTableStmt,
    DropTriggerStmt,
    DropViewStmt,
    IndexScan,
    InsertSelectStmt,
    InsertValuesStmt,
    ReleaseSavepointStmt,
    RollbackToStmt,
    SavepointStmt,
    Scan,
    UpdateStmt,
    plan,
)
from sql_planner.expr import Wildcard as _Wildcard
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
from sql_planner.plan import (
    children as _plan_children,
)
from sql_vm import QueryEvent, QueryResult, execute  # noqa: F401 — QueryEvent re-exported

from .adapter import to_statement
from .binding import substitute
from .errors import (
    READONLY_ERROR_MESSAGE,
    OperationalError,
    ProgrammingError,
    translate,
)

# Statement types that SQLite considers "writes" — anything that mutates
# the database file (rows OR schema).  When ``PRAGMA query_only = 1`` is
# active on a connection, attempting any of these raises
# ``OperationalError: attempt to write a readonly database`` (SQLITE_READONLY,
# code 8) — matching the reference engine's stance.
#
# NOT included (these are deliberately allowed under query_only):
#   * SelectStmt / UnionStmt / IntersectStmt / ExceptStmt — pure reads
#   * BeginStmt / CommitStmt / RollbackStmt — transaction control
#   * SavepointStmt / ReleaseSavepointStmt / RollbackToStmt — savepoint
#     control (SQLite lets these through; the savepoint just brackets
#     no writes)
#   * PRAGMA, VACUUM/ANALYZE/REINDEX/EXPLAIN, ATTACH/DETACH — intercepted
#     *before* parsing, so they never reach this gate at all.  This is
#     important: ``PRAGMA query_only = 0`` must still work to lift the
#     gate, and SQLite permits it.
_WRITE_STMT_TYPES = (
    InsertValuesStmt,
    InsertSelectStmt,
    UpdateStmt,
    DeleteStmt,
    CreateTableStmt,
    DropTableStmt,
    CreateIndexStmt,
    DropIndexStmt,
    CreateViewStmt,
    DropViewStmt,
    CreateTriggerStmt,
    DropTriggerStmt,
    AlterTableStmt,
)

if TYPE_CHECKING:
    from .advisor import IndexAdvisor


def _collect_scan_tables(node: LogicalPlan) -> list[str]:
    """Walk a plan tree and return table names from Scan nodes in document order.

    Used by the CTAS empty-source fallback to expand ``SELECT *`` wildcards
    into concrete column names when the source table holds no rows (so the VM
    never emits any and ``QueryResult.columns`` stays empty).
    """
    if isinstance(node, Scan):
        return [node.table]
    result: list[str] = []
    for child in _plan_children(node):
        result.extend(_collect_scan_tables(child))
    return result


def _ctas_infer_columns(
    backend: Backend,
    sel_sql: str,
    view_defs: dict[str, Any] | None,
) -> tuple[str, ...]:
    """Derive CTAS output column names by planning *sel_sql* without executing it.

    Called only when the source SELECT returned no rows (empty table), meaning
    ``QueryResult.columns`` is ``()``.  The plan's ``Project`` node carries
    alias information that lets us recover the correct column names.

    Column-name rules (matching VM behaviour for non-empty tables):

    * Explicit ``AS alias`` → use the alias.
    * ``SELECT *`` wildcard → expand to the backend's column names for each
      scanned table, in scan order.
    * Any other unnamed expression (``x * 2``, ``1``, …) → ``'?'``, matching
      the ``'?'`` sentinel the VM emits for computed columns without an alias.
    """
    _ast = parse_sql(sel_sql)
    _stmt = to_statement(_ast, view_defs=view_defs or {})
    _lp = plan(_stmt, backend_as_schema_provider(backend))
    _opt = optimize(_lp)
    if not isinstance(_opt, Project):
        return ()
    cols: list[str] = []
    for _item in _opt.items:
        if isinstance(_item.expr, _Wildcard):
            for _tbl in _collect_scan_tables(_opt.input):
                with contextlib.suppress(Exception):
                    cols.extend(c.name for c in backend.columns(_tbl))
        elif _item.alias is not None:
            cols.append(_item.alias)
        else:
            cols.append("?")
    return tuple(cols)


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
        # EXPLAIN QUERY PLAN <stmt>: parse + plan the inner statement and
        # return a four-column row set (id, parent, notused, detail) that
        # mirrors SQLite's output format.  Walked here instead of inside
        # the normal pipeline so we never code-generate or execute the
        # underlying statement.
        if re.match(r"\s*EXPLAIN\s+QUERY\s+PLAN\b", bound, re.IGNORECASE):
            inner = re.sub(
                r"^\s*EXPLAIN\s+QUERY\s+PLAN\s+",
                "",
                bound,
                count=1,
                flags=re.IGNORECASE,
            )
            return _run_explain_query_plan(backend, inner, view_defs=view_defs)
        # VACUUM / ANALYZE / REINDEX are no-ops in mini-sqlite.  SQLite uses
        # VACUUM to rebuild the database file and ANALYZE to collect statistics
        # for the query planner; neither concept applies to our in-memory /
        # file-backed stack.  We silently succeed so migration scripts and ORM
        # setup routines that call these statements don't crash.
        # Bare EXPLAIN (without QUERY PLAN) emits SQLite's VDBE bytecode —
        # we don't expose our internal IR, so we silently return an empty
        # result rather than crash.
        if re.match(
            r"\s*(VACUUM|ANALYZE|REINDEX|EXPLAIN\b)\b",
            bound,
            re.IGNORECASE,
        ):
            return QueryResult(rows_affected=0)
        # ATTACH DATABASE — accepted as a no-op but schema alias is tracked.
        #
        # Real SQLite multi-database support requires per-statement schema
        # routing (e.g. ``SELECT * FROM aux.t``) which mini-sqlite does not
        # currently implement.  We accept ATTACH so ORM/migration code
        # that opens, attaches, queries, then detaches doesn't crash.
        # Queries that reference attached databases (e.g. ``aux.users``) will
        # still fail because the planner cannot resolve the schema prefix.
        #
        # We record the alias name so that a subsequent DETACH of the same
        # alias succeeds (as SQLite would after a real attach).
        _attach_m = re.match(
            r"\s*ATTACH\b.*\bAS\s+([^\s;]+)",
            bound,
            re.IGNORECASE | re.DOTALL,
        )
        if _attach_m:
            _alias = _attach_m.group(1).strip("\"'`[]").lower()
            _schemas = _ATTACHED_SCHEMAS.setdefault(id(backend), set())
            if len(_schemas) >= 10:  # mirrors SQLite's SQLITE_MAX_ATTACHED default
                raise OperationalError("too many attached databases - max 10")
            _schemas.add(_alias)
            return QueryResult(rows_affected=0)
        # DETACH DATABASE — raise SQLite-compatible errors.
        #
        # SQLite raises specific OperationalError messages depending on the
        # schema name:
        #   DETACH main           → "cannot detach database main"
        #   DETACH <not attached> → "no such database: <name>"
        #   DETACH <attached>     → success (no-op in mini-sqlite)
        _detach_m = re.match(
            r"\s*DETACH\s+(?:DATABASE\s+)?(\S+)",
            bound,
            re.IGNORECASE,
        )
        if _detach_m:
            _schema = _detach_m.group(1).strip("\"'`[]").lower()
            if _schema == "main":
                raise OperationalError("cannot detach database main")
            if _schema in _ATTACHED_SCHEMAS.get(id(backend), set()):
                _ATTACHED_SCHEMAS[id(backend)].discard(_schema)
                return QueryResult(rows_affected=0)
            raise OperationalError(f"no such database: {_detach_m.group(1).strip('\"\'`[]')}")
        # Normalise TEMP/TEMPORARY before parsing.
        #
        # SQLite allows "CREATE TEMP TABLE …" and "CREATE TEMPORARY TABLE …"
        # as aliases for "CREATE TABLE …".  TEMP and TEMPORARY cannot be hard
        # keywords in the SQL grammar because they are commonly used as table
        # names (e.g. "INSERT INTO temp …").  Instead we strip the modifier
        # here — after parameter substitution, before the parser — so the
        # grammar stays clean and `temp` remains a valid identifier everywhere.
        #
        # The regex matches the pattern at any position in the statement and
        # removes just the TEMP / TEMPORARY word between CREATE and TABLE/VIEW.
        bound = re.sub(
            r"(?i)\b(CREATE)\s+(?:TEMP|TEMPORARY)\s+(TABLE|VIEW)\b",
            r"\1 \2",
            bound,
        )
        # CREATE TABLE … AS SELECT … (CTAS)
        #
        # SQLite's CTAS creates a new table whose column names come from the
        # SELECT's output.  Declared types are copied from source-column
        # declarations for bare column references; expression columns get an
        # empty declared type (BLOB affinity — the permissive SQLite default).
        #
        # Mini-sqlite handles CTAS as a pre-parse interception because the SQL
        # grammar only accepts CREATE TABLE with an explicit column list.  The
        # steps mirror what SQLite does internally:
        #
        #   1. Execute the source SELECT to obtain column names and rows.
        #   2. CREATE TABLE dst (col1, col2, …) — names only, no declared types.
        #   3. Bulk-INSERT every source row.
        #
        # ``IF NOT EXISTS`` is honoured: if the table already exists the whole
        # statement is a no-op (rows are NOT inserted into the existing table).
        #
        # Column type inheritance from source declarations is a known
        # limitation: all destination columns get BLOB affinity regardless of
        # the source schema.  Dynamic typing means query results are still
        # correct; only ``PRAGMA table_info`` shows empty types.
        _ctas_m = re.match(
            r"\s*CREATE\s+TABLE\s+(IF\s+NOT\s+EXISTS\s+)?(\S+)\s+AS\s+(.+)",
            bound,
            re.IGNORECASE | re.DOTALL,
        )
        if _ctas_m:
            if bool(_pragma_get(backend, "query_only")):
                raise OperationalError(READONLY_ERROR_MESSAGE)
            _ctas_ine = bool(_ctas_m.group(1))
            _ctas_tbl = _ctas_m.group(2).strip("\"'`[]")
            _ctas_sel = _ctas_m.group(3).strip()
            _ctas_kw: dict[str, Any] = dict(
                advisor=advisor,
                view_defs=view_defs,
                check_registry=check_registry,
                fk_child=fk_child,
                fk_parent=fk_parent,
                savepoints=savepoints,
                trigger_executor=trigger_executor,
                trigger_depth=trigger_depth,
                user_functions=user_functions,
            )
            # Step 1 — execute the source SELECT.
            _ctas_src = run(backend, _ctas_sel, **_ctas_kw)
            # Step 2 — create the destination table.
            # Every column gets BLOB affinity — the permissive SQLite default
            # for expression columns.  Type propagation from source column
            # declarations is a known limitation (PRAGMA table_info shows
            # BLOB rather than the source column's declared type).
            #
            # When the source table is empty the VM emits no rows, leaving
            # QueryResult.columns as ().  In that case we plan the SELECT to
            # recover the output column names without executing it.
            _ctas_col_names = _ctas_src.columns or _ctas_infer_columns(
                backend, _ctas_sel, view_defs
            )
            # Sanitise column names: the VM uses '?' for unnamed computed
            # columns, but '?' is a qmark placeholder in SQL; replace it with
            # a positional synthetic name.  Double-quote identifiers so the
            # grammar strips the delimiters and stores the bare name — backtick
            # quoting stores names WITH the backtick characters.
            _safe_col_names = [
                c
                if re.match(r"^[A-Za-z_][A-Za-z0-9_]*$", c)
                else f"col_{i}"
                for i, c in enumerate(_ctas_col_names)
            ]
            _ctas_cols = ", ".join(f'"{n}" BLOB' for n in _safe_col_names)
            try:
                run(backend, f'CREATE TABLE "{_ctas_tbl}" ({_ctas_cols})', **_ctas_kw)
            except OperationalError as _ctas_e:
                if _ctas_ine and "already exists" in str(_ctas_e).lower():
                    return QueryResult(rows_affected=0)
                raise
            # Step 3 — bulk-insert source rows.
            if _ctas_src.rows:
                _ctas_ph = ", ".join("?" * len(_safe_col_names))
                for _ctas_row in _ctas_src.rows:
                    run(
                        backend,
                        f'INSERT INTO "{_ctas_tbl}" VALUES ({_ctas_ph})',
                        _ctas_row,
                        **_ctas_kw,
                    )
            return QueryResult(rows_affected=len(_ctas_src.rows))
        ast = parse_sql(bound)
        stmt = to_statement(ast, view_defs=view_defs)

        # ``PRAGMA query_only = 1`` puts the connection into read-only mode.
        # SQLite rejects any write (DML or DDL) with
        # ``OperationalError: attempt to write a readonly database``
        # (SQLITE_READONLY).  The gate must fire BEFORE the CREATE VIEW /
        # DROP VIEW intercept below (CREATE VIEW is a write under SQLite's
        # rules) and before planning so we never spin up a write program
        # only to discard it.
        #
        # PRAGMAs, ATTACH/DETACH, VACUUM/ANALYZE/REINDEX/EXPLAIN, and
        # BEGIN/COMMIT/ROLLBACK/SAVEPOINT are not subject to this gate —
        # the first four are intercepted before parsing, and the
        # transaction-control statements are intentionally not in
        # ``_WRITE_STMT_TYPES`` (SQLite permits them under query_only).
        # In particular, ``PRAGMA query_only = 0`` must still succeed so
        # callers can lift the gate without re-opening the connection.
        if isinstance(stmt, _WRITE_STMT_TYPES) and bool(
            _pragma_get(backend, "query_only")
        ):
            raise OperationalError(READONLY_ERROR_MESSAGE)

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
        # CREATE INDEX on expressions is accepted-and-ignored.  Mini-sqlite
        # parses ``CREATE INDEX idx ON t(LOWER(name))`` so ORM/migration
        # tools don't error, but doesn't create an actual index (the storage
        # backend can't index a synthetic ``__expr_N`` column name).  Bare-
        # column CREATE INDEX still goes through normally.
        if isinstance(stmt, CreateIndexStmt) and any(
            c.startswith("__expr_") for c in stmt.columns
        ):
            # Silently no-op the index creation.  Returning rows_affected=0
            # matches SQLite's behaviour for successful DDL.
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
        # Honour PRAGMA foreign_keys on this connection.  Mini-sqlite
        # defaults to ON (deviation from SQLite, which defaults to OFF);
        # the per-connection PRAGMA state can override.  When OFF, the
        # VM's FK validation short-circuits for every INSERT/UPDATE/
        # DELETE.
        fk_setting = _pragma_get(backend, "foreign_keys")
        fk_enabled = bool(fk_setting) if fk_setting is not None else True
        result = execute(
            program,
            backend,
            check_registry=check_registry,
            fk_child=fk_child,
            fk_parent=fk_parent,
            fk_enabled=fk_enabled,
            event_cb=event_cb,
            filtered_columns=_filtered,
            trigger_executor=_trigger_executor,
            trigger_depth=trigger_depth,
            user_functions=user_functions or None,
        )
        # Update the connection-state globals consulted by changes(),
        # total_changes(), and last_insert_rowid().  Only DML statements
        # (with non-empty rows_affected) bump these counters — SELECTs do not.
        # The total_changes counter is updated within mini-sqlite's process,
        # not strictly per-connection; this is a known simplification.
        from sql_vm.scalar_functions import _TOTAL_CHANGES, set_connection_state
        if result.rows_affected is not None and result.rows_affected > 0:
            set_connection_state(
                changes=result.rows_affected,
                total_changes=_TOTAL_CHANGES + result.rows_affected,
                last_insert_rowid=(
                    int(result.last_inserted_rowid)
                    if getattr(result, "last_inserted_rowid", None)
                    else None
                ),
            )
        return result
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

# Matches one of these PRAGMA forms:
#   PRAGMA name                       — read query
#   PRAGMA name('arg')                — read with table-name argument
#   PRAGMA name("arg")                — same, double-quoted
#   PRAGMA name = <value>             — write; value is a signed integer literal
#                                       or one of ON / OFF / TRUE / FALSE / YES / NO
#                                       or a bare identifier (journal_mode = wal)
_PRAGMA_RE = re.compile(
    r"""
    \s* PRAGMA \s+
    (?P<name>[A-Za-z_][A-Za-z0-9_]*)   # pragma name
    (?:                                  # optional argument or assignment
        \s* \(
            \s* ["']? (?P<arg>[A-Za-z_0-9][A-Za-z0-9_]*) ["']? \s*
        \)
        |
        \s* = \s* (?P<set_value>-?\d+|[A-Za-z_][A-Za-z0-9_]*)
    )?
    \s* ;? \s* $
    """,
    re.IGNORECASE | re.VERBOSE,
)


# ---------------------------------------------------------------------------
# EXPLAIN QUERY PLAN — walk the optimised LogicalPlan and emit one row per
# "interesting" plan node.  The output mirrors SQLite's four-column layout:
#
#   id      | parent  | notused | detail
#   --------+---------+---------+--------------------------------
#   integer | integer | always  | human-readable description, e.g.
#           |         | 0       | "SCAN t" or "SEARCH t USING INDEX ix"
#
# We deliberately do NOT emit a row for every plan node — SQLite skips
# pure transforms (Filter, Project) and only surfaces the data-source and
# big algorithmic choices (sorts, group-by, distinct).  Our detail-string
# generator returns ``None`` for nodes we want to elide; the walker then
# passes the elided node's parent_id down to its children.
# ---------------------------------------------------------------------------


def _format_index_bounds(node: object) -> str:
    """Render an :class:`IndexScan`'s bounds as a SQLite-style detail suffix.

    The output is a comma-free ``AND``-joined sequence of ``col<op>?``
    fragments matching real SQLite's EXPLAIN QUERY PLAN format::

        x = 5             → "x=?"
        x = 1 AND y = 2   → "x=? AND y=?"
        x > 5             → "x>?"
        x BETWEEN 1 AND 5 → "x>? AND x<?"
        x = 1 AND y > 2   → "x=? AND y>?"

    Edge cases:

    * Returns ``""`` if the IndexScan has no columns or no bounds at all
      (e.g. a covering-index full scan with no WHERE constraints).
    * The function probes ``lo`` / ``hi`` defensively because a future
      planner may produce per-column tuples of varying length; missing
      entries are treated as "no bound on this column".
    """
    cols = getattr(node, "columns", None) or ()
    if not cols:
        return ""
    lo = getattr(node, "lo", None)
    hi = getattr(node, "hi", None)
    fragments: list[str] = []
    for i, col in enumerate(cols):
        lo_val = lo[i] if lo is not None and i < len(lo) else None
        hi_val = hi[i] if hi is not None and i < len(hi) else None
        if lo_val is not None and hi_val is not None and lo_val == hi_val:
            # Both bounds equal → equality constraint.
            fragments.append(f"{col}=?")
        else:
            if lo_val is not None:
                fragments.append(f"{col}>?")
            if hi_val is not None:
                fragments.append(f"{col}<?")
    return " AND ".join(fragments)


def _explain_detail(node: LogicalPlan) -> str | None:
    """Return SQLite-style EXPLAIN QUERY PLAN detail text for *node*, or None.

    Returning None means the node is a pure transform (e.g. Filter,
    Project, Limit, Having) that doesn't appear as its own row in
    SQLite's output; its children are reparented to the elided node's
    parent so the id/parent topology still matches.
    """
    # Late-imported to avoid a top-level cycle with the planner module.
    from sql_planner.plan import (
        Aggregate as _Agg,
    )
    from sql_planner.plan import (
        Delete as _Del,
    )
    from sql_planner.plan import (
        Distinct as _Dist,
    )
    from sql_planner.plan import (
        IndexScan as _Ix,
    )
    from sql_planner.plan import (
        Insert as _Ins,
    )
    from sql_planner.plan import (
        Scan as _Scan,
    )
    from sql_planner.plan import (
        Sort as _Sort,
    )
    from sql_planner.plan import (
        Update as _Upd,
    )
    from sql_planner.plan import (
        WindowAgg as _Win,
    )

    if isinstance(node, _Scan):
        # ``SCAN <table>`` or ``SCAN <table> AS <alias>`` — the alias is
        # included when distinct from the table name so the user can tell
        # apart self-joins.
        if node.alias and node.alias != node.table:
            return f"SCAN {node.table} AS {node.alias}"
        return f"SCAN {node.table}"
    if isinstance(node, _Ix):
        # ``SEARCH <table> [AS <alias>] USING INDEX <name> (col=?...)``
        # mirroring SQLite's output:
        #
        #   x = 5            → "(x=?)"
        #   x > 5            → "(x>?)"
        #   x BETWEEN 1 AND 5 → "(x>? AND x<?)"
        #   x = 1 AND y = 2  → "(x=? AND y=?)"
        #   x = 1 AND y > 2  → "(x=? AND y>?)"
        #
        # SQLite's detail string omits inclusivity markers (``>=`` and
        # ``<=`` both render as ``>`` and ``<`` here).
        base = f"SEARCH {node.table}"
        if node.alias and node.alias != node.table:
            base += f" AS {node.alias}"
        base += f" USING INDEX {node.index_name}"
        suffix = _format_index_bounds(node)
        return base + (f" ({suffix})" if suffix else "")
    if isinstance(node, _Agg):
        return "USE TEMP B-TREE FOR GROUP BY"
    if isinstance(node, _Sort):
        return "USE TEMP B-TREE FOR ORDER BY"
    if isinstance(node, _Dist):
        return "USE TEMP B-TREE FOR DISTINCT"
    if isinstance(node, _Win):
        return "USE TEMP B-TREE FOR WINDOW FUNCTION"
    if isinstance(node, DerivedTable):
        alias = node.alias or "subq"
        return f"SCAN SUBQUERY {alias}"
    if isinstance(node, (_Ins, _Upd, _Del)):
        # DML uses the target table directly; SQLite shows the table
        # name (no row for the action itself, but we approximate).
        return f"SCAN {node.table}"
    # All other nodes (Filter, Project, Join, Having, Limit, Union/Intersect/
    # Except, Begin/Commit/Rollback, CreateTable, etc.) elided.
    return None


def _run_explain_query_plan(
    backend: Backend,
    sql: str,
    *,
    view_defs: dict | None = None,
) -> QueryResult:
    """Plan *sql* and return a ``(id, parent, notused, detail)`` row set.

    The inner statement is parsed and planned but never executed.  No
    side effects on the backend.  Designed for the ``EXPLAIN QUERY PLAN``
    diagnostic path, not for general query handling.
    """
    ast = parse_sql(sql)
    stmt = to_statement(ast, view_defs=view_defs)
    logical = plan(stmt, backend_as_schema_provider(backend))
    optimized = optimize(logical)

    rows: list[tuple[int, int, int, str]] = []
    counter = [0]  # mutable in closure — pre-increment to get the next id

    def walk(node: LogicalPlan, parent_id: int) -> None:
        detail = _explain_detail(node)
        if detail is not None:
            counter[0] += 1
            my_id = counter[0]
            rows.append((my_id, parent_id, 0, detail))
            child_parent = my_id
        else:
            # Elided node: children inherit this node's parent.
            child_parent = parent_id
        for child in _plan_children(node):
            walk(child, child_parent)

    walk(optimized, 0)

    return QueryResult(
        columns=("id", "parent", "notused", "detail"),
        rows=tuple(rows),
    )


# ---------------------------------------------------------------------------


def _fk_find_pk(table: str, backend: Backend) -> str:
    """Return the PRIMARY KEY column name for *table*, falling back to ``'id'``.

    Used by ``PRAGMA foreign_key_check`` when a ``REFERENCES`` clause
    omits the parent column (``REFERENCES p`` instead of
    ``REFERENCES p(col)``).  Same algorithm sql-vm uses internally.
    """
    try:
        cols = backend.columns(table)
    except Exception:  # noqa: BLE001 — parent table missing → conventional fallback
        return "id"
    for c in cols:
        if getattr(c, "primary_key", False):
            return c.name
    return "id"


def _fk_row_exists(
    table: str, col: str, value: object, backend: Backend
) -> bool:
    """Return True if any row in *table* has *col* == *value*.

    Walks the table linearly via ``backend.scan`` — fine for diagnostic
    PRAGMAs but ``O(N*M)`` for the full ``foreign_key_check`` sweep.
    """
    try:
        cur = backend.scan(table)
    except Exception:  # noqa: BLE001 — parent table missing → every row is a violation
        return False
    while True:
        row = cur.next()
        if row is None:
            return False
        if row.get(col) == value:
            return True


def _format_pragma_default(value: object) -> str:
    """Render a column's stored default value as the SQL-literal text that
    ``PRAGMA table_info.dflt_value`` reports.

    SQLite's ``dflt_value`` column returns the literal source text that
    appeared after ``DEFAULT`` — so ``DEFAULT 'x'`` reads back as
    ``"'x'"`` (with the single quotes!), ``DEFAULT 42`` reads back as
    ``"42"``, ``DEFAULT NULL`` reads back as ``"NULL"``, and so on.
    Mini-sqlite's adapter parses the literal into a Python value at
    CREATE TABLE time; this helper re-encodes it back to source-text
    form so the pragma surfaces the same string sqlite3 does.

    Bytes get the ``X'hex...'`` blob-literal form; bools collapse to
    ``'0'`` / ``'1'`` (SQLite stores booleans as integers internally).
    """
    if value is None:
        return "NULL"
    if isinstance(value, bool):
        return "1" if value else "0"
    if isinstance(value, (int, float)):
        return repr(value)
    if isinstance(value, (bytes, bytearray)):
        return "X'" + bytes(value).hex().upper() + "'"
    # Default: treat as text — escape embedded single quotes by doubling.
    text = str(value).replace("'", "''")
    return f"'{text}'"


# Boolean PRAGMA value parser.  SQLite accepts a wide range of representations
# for "true / false" — integers (1/0, also any non-zero), and the keywords
# ON, OFF, TRUE, FALSE, YES, NO (case-insensitive).  Returns:
#   * True / False if the value can be parsed as a boolean.
#   * None if the value is unrecognised — caller decides how to handle.
def _parse_bool_pragma(s: str) -> bool | None:
    s_lower = s.strip().lower()
    if s_lower in ("1", "on", "true", "yes"):
        return True
    if s_lower in ("0", "off", "false", "no"):
        return False
    # Numeric: SQLite treats any non-zero integer as TRUE.
    try:
        return bool(int(s_lower))
    except ValueError:
        return None


# In-process storage for "settable" PRAGMAs that don't have real semantics in
# mini-sqlite but must round-trip a value to match SQLite's behaviour.
#
# Real SQLite stores these in the database header or in the connection's
# runtime configuration.  Mini-sqlite is in-memory and doesn't have most of
# those concepts; we keep the value in a per-process dict so that
# ``PRAGMA foo = 5; PRAGMA foo;`` returns ``5`` within the same process.
#
# Each entry maps the PRAGMA name to its current value and the column type.
# Default values mirror SQLite's defaults so that a fresh read of any
# unmodified PRAGMA returns the same value SQLite would.
_PRAGMA_DEFAULTS: dict[str, tuple[object, str]] = {
    # name              (default_value,    column_type)
    # SQLite defaults this to 0 (FK enforcement OFF).  Mini-sqlite has
    # historically enforced FKs unconditionally; rather than break that
    # behaviour, we default to 1 (ON) so the pragma's read value matches
    # the enforcement default.  ``PRAGMA foreign_keys = OFF`` still
    # works — the VM consults this setting on every INSERT/UPDATE/DELETE.
    "foreign_keys":     (1,                "integer"),  # ON by default in mini-sqlite
    "recursive_triggers": (0,              "integer"),
    "case_sensitive_like": (0,             "integer"),
    "legacy_alter_table": (0,              "integer"),
    "defer_foreign_keys": (0,              "integer"),
    "secure_delete":    (0,                "integer"),
    "temp_store":       (0,                "integer"),  # 0=default file-based
    "synchronous":      (2,                "integer"),  # 2=FULL (SQLite default)
    "cache_size":       (-2000,            "integer"),  # negative = kibibytes
    "auto_vacuum":      (0,                "integer"),  # 0=NONE
    "application_id":   (0,                "integer"),
    "page_size":        (4096,             "integer"),
    "page_count":       (0,                "integer"),  # mini-sqlite has no pages
    "freelist_count":   (0,                "integer"),
    "encoding":         ("UTF-8",          "text"),
    "journal_mode":     ("memory",         "text"),     # mini-sqlite is in-memory
    "locking_mode":     ("normal",         "text"),
    # Additional PRAGMAs that applications written for real SQLite often probe
    # defensively.  Mini-sqlite has no on-disk file, no WAL log, and no
    # thread pool, so most of these are accept-and-store cosmetic state — the
    # value round-trips but otherwise has no effect on execution.  Defaults
    # mirror SQLite's documented defaults so a fresh read matches the oracle.
    "reverse_unordered_selects": (0,       "integer"),  # bool; off by default
    "cell_size_check":  (0,                "integer"),  # bool; off by default
    "fullfsync":        (0,                "integer"),  # bool; off by default
    "wal_autocheckpoint": (1000,           "integer"),  # pages between autocheckpoints
    "journal_size_limit": (-1,             "integer"),  # bytes; -1 = no limit
    "threads":          (0,                "integer"),  # worker threads in sort/index
    # writable_schema — when ON, SQLite allows direct UPDATE/INSERT/DELETE
    # against sqlite_master, which lets you rename/repair the schema.
    # Mini-sqlite synthesises sqlite_master on-the-fly and does not honour
    # writes to it, but we still round-trip the PRAGMA's value so ORMs and
    # migration tools that toggle it during repair flows don't trip on
    # unrecognised PRAGMA.
    "writable_schema":  (0,                "integer"),  # bool; off by default
    # read_uncommitted — selects whether SQLite uses the "READ UNCOMMITTED"
    # isolation level (shared-cache mode only).  Mini-sqlite has no
    # shared cache so the flag has no semantic effect, but ORMs probe
    # it defensively before deciding whether to issue an explicit
    # ``PRAGMA read_uncommitted = 0`` for a clean baseline.
    "read_uncommitted": (0,                "integer"),  # bool; off by default
    # query_only — when ON, SQLite rejects writes with
    # ``OperationalError: attempt to write a readonly database``
    # (SQLITE_READONLY).  Mini-sqlite honours this as of 2.16.0:
    # any DML (INSERT/UPDATE/DELETE) or DDL (CREATE/DROP/ALTER)
    # statement is rejected at the run-loop gate (see
    # ``_WRITE_STMT_TYPES`` and the check in ``run()`` above).
    # SELECTs, PRAGMAs, and transaction control still flow normally,
    # so ``PRAGMA query_only = 0`` can always lift the gate.
    "query_only":       (0,                "integer"),  # bool; off by default
}

# Per-connection PRAGMA state.  Keyed by the backend object's id() so each
# connection has its own values.  WeakValueDictionary would be ideal but
# Backend isn't hashable in all implementations; we settle for id-keyed dict.
# Entries must be explicitly evicted when a connection closes (see
# ``_pragma_clear``) to prevent id-reuse pollution: CPython can allocate a
# new backend at the same address as a just-freed one, causing the new
# connection to inherit the old connection's PRAGMA state.
_PRAGMA_STATE: dict[int, dict[str, object]] = {}

# Per-connection set of virtually-attached schema aliases.  Populated when
# ATTACH succeeds (no-op) so that a subsequent DETACH of the same alias also
# succeeds (instead of raising "no such database").  Same id-reuse caveat as
# _PRAGMA_STATE — must be cleared in ``_pragma_clear``.
_ATTACHED_SCHEMAS: dict[int, set[str]] = {}


def _pragma_get(backend: Backend, name: str) -> object:
    """Return the current value of *name* for this backend, defaulting to the
    SQLite-compatible default if never set."""
    state = _PRAGMA_STATE.setdefault(id(backend), {})
    if name in state:
        return state[name]
    return _PRAGMA_DEFAULTS[name][0]


def _pragma_set(backend: Backend, name: str, value: object) -> None:
    """Store *value* for *name* on this backend."""
    state = _PRAGMA_STATE.setdefault(id(backend), {})
    state[name] = value


def _pragma_clear(backend: Backend) -> None:
    """Remove all per-connection PRAGMA and attached-schema state for *backend*.

    Called by :meth:`~mini_sqlite.connection.Connection.close` to prevent
    stale state from leaking into a later connection whose backend object
    happens to be allocated at the same memory address.
    """
    _PRAGMA_STATE.pop(id(backend), None)
    _ATTACHED_SCHEMAS.pop(id(backend), None)


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

    ``PRAGMA user_version``
        Read the user-defined integer at byte offset 60 of the database
        header.  Returns one row ``(user_version,)`` of an int.  A fresh
        database returns 0.

    ``PRAGMA user_version = <int>``
        Write *<int>* into the user_version field.  Must fit in u32
        (0 ≤ v ≤ 2³² − 1).  Produces an empty result.

    ``PRAGMA schema_version``
        Read-only — returns one row ``(schema_version,)`` of an int.
        The schema cookie is bumped automatically on every DDL operation.
    """
    m = _PRAGMA_RE.match(sql)
    if m is None:
        raise ProgrammingError(f"invalid PRAGMA syntax: {sql!r}")
    name = m.group("name").lower()
    arg = m.group("arg")  # may be None
    set_value = m.group("set_value")  # may be None — assignment form

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
                # SQLite's ``notnull`` reports the *explicit* NOT NULL
                # declaration, not the implicit one from PRIMARY KEY.
                # ``id INTEGER PRIMARY KEY`` → notnull=0;
                # ``id INTEGER PRIMARY KEY NOT NULL`` → notnull=1.
                # The adapter (mini-sqlite 1.97+) only sets the raw
                # ``not_null`` flag for explicit declarations, so the
                # raw field is the correct source.
                not_null = int(col.not_null)
                pk = int(col.primary_key)
                type_name = col.type_name
                dflt = _format_pragma_default(col.default) if col.has_default() else None
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
        # SQLite's index_list emits five columns:
        #
        #   seq      INTEGER  — 0-based position in the list
        #   name     TEXT     — index name
        #   unique   INTEGER  — 1 if UNIQUE, 0 otherwise
        #   origin   TEXT     — 'c' (CREATE INDEX) | 'u' (UNIQUE column
        #                       constraint) | 'pk' (PRIMARY KEY constraint).
        #                       Mini-sqlite distinguishes user CREATE INDEX
        #                       from auto-indexes by the ``sqlite_autoindex_``
        #                       name prefix; we map ``sqlite_autoindex_*`` →
        #                       'u' (UNIQUE column constraint) since the
        #                       backend's auto-indexes back UNIQUE/PK
        #                       columns interchangeably.
        #   partial  INTEGER  — 1 if a WHERE clause; mini-sqlite does not
        #                       support partial indexes, so always 0.
        rows = []
        for seq, idx in enumerate(indexes):
            origin = "u" if idx.name.startswith("sqlite_autoindex_") else "c"
            rows.append((seq, idx.name, int(idx.unique), origin, 0))
        return QueryResult(
            columns=("seq", "name", "unique", "origin", "partial"),
            rows=tuple(rows),
        )

    if name == "index_info":
        # PRAGMA index_info(<index-name>) — for each column the index
        # covers, return a row (seqno, cid, name):
        #
        #   seqno  INTEGER  — 0-based position in the index key
        #   cid    INTEGER  — column id in the parent table (0-based)
        #   name   TEXT     — column name
        #
        # Returns an empty result if the index doesn't exist (matches
        # SQLite's behaviour — no error, just zero rows).
        if not arg:
            raise ProgrammingError("PRAGMA index_info requires an index name")
        idx = None
        try:
            # list_indexes() without a table arg returns all indexes;
            # we filter by name here.
            for candidate in backend.list_indexes():
                if candidate.name == arg:
                    idx = candidate
                    break
        except Exception:  # noqa: BLE001 — backend may not implement
            pass
        if idx is None:
            return QueryResult(columns=("seqno", "cid", "name"), rows=())
        # Resolve each indexed column name to its 0-based cid in the
        # parent table.  ``backend.columns`` returns ColumnDef objects
        # in declaration order, so the position is the cid.
        try:
            parent_cols = backend.columns(idx.table)
        except Exception:  # noqa: BLE001
            parent_cols = []
        col_to_cid = {
            (getattr(c, "name", c)): i for i, c in enumerate(parent_cols)
        }
        info_rows = tuple(
            (seqno, col_to_cid.get(col_name, -1), col_name)
            for seqno, col_name in enumerate(idx.columns)
        )
        return QueryResult(
            columns=("seqno", "cid", "name"),
            rows=info_rows,
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

    if name == "foreign_key_check":
        # SQLite: scan every (or one named) child table; for each row,
        # for each declared FOREIGN KEY, verify the referenced parent
        # row exists.  Returns one row per violation::
        #
        #   table   TEXT    — the child table where the bad row lives
        #   rowid   INTEGER — the bad row's rowid
        #   parent  TEXT    — the referenced parent table
        #   fkid    INTEGER — the FK's position in foreign_key_list (0-based)
        #
        # Rules:
        #   * NULL child values pass unconditionally (SQL "unknown
        #     reference is not an error" rule).
        #   * If no ``parent_col`` was declared (i.e. ``REFERENCES p``),
        #     resolve to the parent's first PRIMARY KEY column.
        #   * If the parent table is missing, every non-NULL child row
        #     is a violation.
        viol_rows: list[tuple] = []
        if fk_child:
            # Optional table-name filter: ``PRAGMA foreign_key_check(t)``
            # restricts scanning to one child table.
            table_filter = arg or None
            for child_table, fks in fk_child.items():
                if table_filter and child_table != table_filter:
                    continue
                try:
                    parent_pk_cache: dict[str, str] = {}
                    cur = backend.scan(child_table)
                except Exception:  # noqa: BLE001 — table missing → skip
                    continue
                while True:
                    row = cur.next()
                    if row is None:
                        break
                    rowid = getattr(cur, "rowid", lambda: None)()
                    for fk_id, (child_col, parent_table, parent_col) in enumerate(fks):
                        value = row.get(child_col)
                        if value is None:
                            continue
                        # Resolve target column (cached per parent table
                        # to avoid repeated PK lookups).
                        if parent_col is None:
                            if parent_table not in parent_pk_cache:
                                parent_pk_cache[parent_table] = _fk_find_pk(
                                    parent_table, backend
                                )
                            ref_col = parent_pk_cache[parent_table]
                        else:
                            ref_col = parent_col
                        if not _fk_row_exists(
                            parent_table, ref_col, value, backend
                        ):
                            viol_rows.append(
                                (child_table, rowid, parent_table, fk_id)
                            )
        return QueryResult(
            columns=("table", "rowid", "parent", "fkid"),
            rows=tuple(viol_rows),
        )

    if name == "table_list":
        tables = backend.tables()
        return QueryResult(
            columns=("schema", "name", "type"),
            rows=tuple(("main", t, "table") for t in tables),
        )

    if name == "user_version":
        if set_value is not None:
            try:
                backend.set_user_version(int(set_value))
            except AttributeError as e:
                # Backend without u32-header support (e.g. InMemoryBackend
                # in some configurations) — surface as Unsupported rather
                # than the bare AttributeError.
                raise ProgrammingError(
                    "backend does not support PRAGMA user_version write"
                ) from e
            except ValueError as e:
                raise ProgrammingError(str(e)) from e
            return QueryResult(rows_affected=0)
        try:
            v = backend.get_user_version()
        except AttributeError:
            v = 0  # backend has no header — return 0 by convention
        return QueryResult(columns=("user_version",), rows=((v,),))

    if name == "schema_version":
        # Read-only.  Ignores any "= value" form (matches sqlite3 silently).
        try:
            v = backend.get_schema_version()
        except AttributeError:
            v = 0
        return QueryResult(columns=("schema_version",), rows=((v,),))

    # ------------------------------------------------------------------------
    # database_list — one row per attached database.  Mini-sqlite has no
    # ATTACH support, so only "main" exists.
    # ------------------------------------------------------------------------
    if name == "database_list":
        return QueryResult(
            columns=("seq", "name", "file"),
            rows=((0, "main", ""),),
        )

    # ------------------------------------------------------------------------
    # collation_list — sqlite3 reports BINARY, RTRIM, NOCASE.  Mini-sqlite
    # only implements BINARY (the default).  We still report all three so
    # that introspection code that expects them doesn't break.
    # ------------------------------------------------------------------------
    if name == "collation_list":
        return QueryResult(
            columns=("seq", "name"),
            rows=((0, "RTRIM"), (1, "NOCASE"), (2, "BINARY")),
        )

    # ------------------------------------------------------------------------
    # compile_options — SQLite reports the compile-time options used to
    # build the binary.  We return a representative list so introspection
    # code that just wants "is JSON enabled?" gets a sensible answer.
    # ------------------------------------------------------------------------
    if name == "compile_options":
        return QueryResult(
            columns=("compile_options",),
            rows=(
                ("ENABLE_JSON1",),
                ("ENABLE_FTS5",),  # we don't really support it but report it
                ("ENABLE_RTREE",),  # ditto
                ("THREADSAFE=0",),
            ),
        )

    # ------------------------------------------------------------------------
    # function_list — one row per registered scalar/aggregate function.
    # Real SQLite returns (name, builtin, type, enc, narg, flags).  We
    # report the registered scalar functions from sql_vm.scalar_functions
    # plus the well-known aggregates.
    # ------------------------------------------------------------------------
    if name == "function_list":
        from sql_vm.scalar_functions import _REGISTRY  # type: ignore[attr-defined]
        rows: list[tuple] = []
        # All scalar functions registered in the VM.  We report narg=-1 (variadic)
        # because the registry doesn't currently track arity per function.
        for fname in sorted(_REGISTRY.keys()):
            rows.append((fname, 1, "s", "utf8", -1, 0x800))
        # Known aggregates.
        for agg in ("count", "sum", "avg", "min", "max", "group_concat", "total"):
            rows.append((agg, 1, "a", "utf8", -1, 0x800))
        return QueryResult(
            columns=("name", "builtin", "type", "enc", "narg", "flags"),
            rows=tuple(rows),
        )

    # ------------------------------------------------------------------------
    # module_list — virtual-table modules.  We report none; this is a
    # legitimate state for SQLite builds without virtual tables.
    # ------------------------------------------------------------------------
    if name == "module_list":
        return QueryResult(columns=("name",), rows=())

    # ------------------------------------------------------------------------
    # pragma_list — the catalog of supported PRAGMA names.  Apps and ORMs
    # probe this to learn what's safe to call; the rows are intentionally
    # informational (the response shape is just ``(name,)`` per row), so
    # mini-sqlite advertises only the PRAGMAs it actually implements
    # rather than mirroring SQLite's full list (which would be a lie).
    # The catalog is built from:
    #   * the dedicated handlers above (table_info, table_list, …)
    #   * the writable scalars in _PRAGMA_DEFAULTS (foreign_keys, …)
    #   * the read-only health checks (integrity_check, quick_check)
    # …and sorted alphabetically so iteration is stable across runs.
    # ------------------------------------------------------------------------
    if name == "pragma_list":
        supported = {
            "case_sensitive_like",
            "collation_list",
            "compile_options",
            "data_version",
            "database_list",
            "foreign_key_check",
            "foreign_key_list",
            "function_list",
            "index_info",
            "index_list",
            "integrity_check",
            "module_list",
            "optimize",
            "pragma_list",
            "quick_check",
            "schema_version",
            "table_info",
            "table_list",
            "user_version",
        }
        supported.update(_PRAGMA_DEFAULTS.keys())
        return QueryResult(
            columns=("name",),
            rows=tuple((n,) for n in sorted(supported)),
        )

    # ------------------------------------------------------------------------
    # data_version — a counter that bumps every time another *connection*
    # writes to the database file.  Within a single connection it stays
    # fixed.  Mini-sqlite has no shared backing file (each connection
    # owns its in-memory store), so the counter has no real semantics —
    # we return the SQLite baseline value of 1, which matches a freshly-
    # opened ``:memory:`` connection in stdlib sqlite3.
    # ------------------------------------------------------------------------
    if name == "data_version":
        return QueryResult(columns=("data_version",), rows=((1,),))

    # ------------------------------------------------------------------------
    # Boolean / scalar settable PRAGMAs — read/write round-trip.
    #
    # Most of these have no real effect in mini-sqlite (we don't have pages,
    # WAL, etc.), but apps and ORMs commonly read/write them and expect the
    # value to round-trip.  We store the value per-connection in _PRAGMA_STATE
    # so that the assignment is observable.
    #
    # Bool-valued PRAGMAs accept ON/OFF/1/0/TRUE/FALSE/YES/NO as input but
    # the read form always returns the integer 0 or 1 (matches SQLite).
    # ------------------------------------------------------------------------
    # case_sensitive_like is special in SQLite: write-only.  Reads always
    # return an empty result.  Accept any boolean value on write; otherwise
    # do nothing (we don't currently propagate the flag to LIKE evaluation).
    if name == "case_sensitive_like":
        if set_value is not None:
            parsed = _parse_bool_pragma(set_value)
            if parsed is None:
                raise ProgrammingError(
                    f"invalid boolean value for PRAGMA {name}: {set_value!r}"
                )
            _pragma_set(backend, name, int(parsed))
        return QueryResult(columns=(), rows=())

    _BOOL_PRAGMAS = {
        "foreign_keys",
        "recursive_triggers",
        "legacy_alter_table",
        "defer_foreign_keys",
        "secure_delete",
        # Accept-and-store cosmetic flags (no semantic effect in mini-sqlite):
        "reverse_unordered_selects",  # planner hint, no scrambling implemented
        "cell_size_check",            # btree integrity flag, no btree in mini
        "fullfsync",                  # fsync mode, no disk I/O in mini
        # ``writable_schema`` round-trips but mini-sqlite ignores it on the
        # write side — the schema catalog is synthesised at query time, not
        # stored in a writable sqlite_master table.  Tools that read this
        # PRAGMA defensively (e.g. to skip a repair flow) still see the
        # expected value.
        "writable_schema",
        # ``read_uncommitted`` controls SQLite's shared-cache isolation
        # level.  Mini-sqlite has no shared cache, so this is purely a
        # round-tripped value with no semantic effect.
        "read_uncommitted",
        # ``query_only`` is enforced as of mini-sqlite 2.16.0 — when ON
        # the run-loop gate in ``run()`` rejects any DML or DDL with
        # ``OperationalError: attempt to write a readonly database``
        # (SQLITE_READONLY).  The PRAGMA's value still round-trips here
        # so callers can read it back and so ``PRAGMA query_only = 0``
        # always lifts the gate.
        "query_only",
    }
    _INT_PRAGMAS = {
        "temp_store",
        "synchronous",
        "cache_size",
        "auto_vacuum",
        "application_id",
        "page_size",
        "page_count",
        "freelist_count",
        # Accept-and-store integer-valued flags (no semantic effect):
        "wal_autocheckpoint",  # WAL autocheckpoint threshold (no WAL in mini)
        "journal_size_limit",  # journal size cap in bytes (no journal in mini)
        "threads",             # worker-thread pool size (no threading in mini)
    }
    _TEXT_PRAGMAS = {
        "encoding",
        "journal_mode",
        "locking_mode",
    }

    if name in _BOOL_PRAGMAS:
        if set_value is not None:
            parsed = _parse_bool_pragma(set_value)
            if parsed is None:
                raise ProgrammingError(f"invalid boolean value for PRAGMA {name}: {set_value!r}")
            _pragma_set(backend, name, int(parsed))
            return QueryResult(rows_affected=0)
        v = _pragma_get(backend, name)
        return QueryResult(columns=(name,), rows=((int(bool(v)),),))

    if name in _INT_PRAGMAS:
        if set_value is not None:
            try:
                iv = int(set_value)
            except ValueError as e:
                raise ProgrammingError(
                    f"invalid integer value for PRAGMA {name}: {set_value!r}"
                ) from e
            # page_size / page_count / freelist_count are read-only in real
            # SQLite (set is silently ignored after the database is created).
            # We mirror that: silently swallow the assignment.
            if name not in ("page_size", "page_count", "freelist_count"):
                _pragma_set(backend, name, iv)
            # Most ``PRAGMA name = value`` forms return an empty result, but
            # a few echo the new value back as a one-row scalar.  This is a
            # SQLite quirk documented per-PRAGMA: ``wal_autocheckpoint``,
            # ``journal_size_limit``, and ``threads`` all echo on set;
            # ``application_id``, ``user_version``, and the cache/temp/
            # synchronous family stay silent.
            if name in {"wal_autocheckpoint", "journal_size_limit", "threads"}:
                return QueryResult(columns=(name,), rows=((iv,),))
            return QueryResult(rows_affected=0)
        v = _pragma_get(backend, name)
        return QueryResult(columns=(name,), rows=((v,),))

    if name in _TEXT_PRAGMAS:
        if set_value is not None:
            # journal_mode is special: SQLite only allows specific values
            # depending on storage type.  For in-memory databases (which is
            # what mini-sqlite currently is) journal_mode is locked to
            # 'memory' — assignments to other modes are silently rejected.
            # The write form still returns a one-row result of the *current*
            # (possibly unchanged) value, matching SQLite.
            if name == "journal_mode":
                # Reject anything other than 'memory' to stay byte-compatible
                # with sqlite3's behaviour on :memory: databases.  File-backed
                # backends are not yet distinguished — when they are, this
                # branch should consult backend.is_in_memory() or similar.
                requested = set_value.lower()
                current = _pragma_get(backend, name)
                if requested == "memory":
                    _pragma_set(backend, name, requested)
                # else: silently reject; current value unchanged.
                return QueryResult(columns=(name,), rows=((current,),))
            if name == "locking_mode":
                _pragma_set(backend, name, set_value.lower())
                return QueryResult(columns=(name,), rows=((set_value.lower(),),))
            _pragma_set(backend, name, set_value.lower())
            return QueryResult(rows_affected=0)
        v = _pragma_get(backend, name)
        return QueryResult(columns=(name,), rows=((v,),))

    # ------------------------------------------------------------------------
    # Maintenance pragmas — return SQLite-compatible "ok" / empty results.
    #
    # These pragmas trigger heavy operations in real SQLite:
    #
    #   PRAGMA optimize                  — analyses statistics, maybe rebuilds
    #                                       indexes.  Returns no rows.
    #   PRAGMA optimize(N)               — same with bitmask of optimisation
    #                                       flags (N=0 disables, ignore in mini).
    #   PRAGMA integrity_check           — full structural + constraint scan.
    #                                       Returns 'ok' row, or one row per
    #                                       error found.
    #   PRAGMA integrity_check(N)        — same but reports at most N errors.
    #   PRAGMA integrity_check('table')  — restrict the check to one table.
    #   PRAGMA quick_check               — like integrity_check but skips the
    #                                       UNIQUE / foreign-key checks.
    #
    # Mini-sqlite holds everything in memory; corruption isn't possible in the
    # ways SQLite worries about (page-level B-tree integrity, partial writes,
    # etc.).  We unconditionally report `'ok'` for the *_check pragmas — that
    # matches the result every healthy real SQLite database returns — and
    # treat `optimize` as a no-op.
    # ------------------------------------------------------------------------
    if name == "optimize":
        # Always succeeds with empty result.  Both `PRAGMA optimize` and
        # `PRAGMA optimize(N)` for any N produce no rows in real SQLite when
        # the database is in good shape.
        return QueryResult(columns=(), rows=())

    if name in ("integrity_check", "quick_check"):
        # Report 'ok' as a single-row, single-column result — the canonical
        # successful response from real SQLite.  The "max errors" argument
        # form (PRAGMA integrity_check(N)) and the "single-table" argument
        # form (PRAGMA integrity_check('table')) both produce the same 'ok'
        # row when there are no errors.
        return QueryResult(columns=("integrity_check",), rows=(("ok",),))

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
