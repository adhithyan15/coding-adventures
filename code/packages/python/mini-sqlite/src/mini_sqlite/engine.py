"""
Thin pipeline orchestrator: SQL text → QueryResult.

Lives between the facade (Connection/Cursor) and the stack of processing
packages. Centralising the call sequence here means ``connection.py`` and
``cursor.py`` never import planner/optimizer/codegen/vm directly — they
just ask the engine for a result.

Exception policy: every exception raised by any pipeline layer is funneled
through :func:`translate` so the caller only ever sees PEP 249 classes.
"""

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import replace
from typing import Any

from sql_backend import Backend, backend_as_schema_provider
from sql_codegen import compile as codegen_compile
from sql_optimizer import optimize
from sql_parser import parse_sql
from sql_planner import (
    AggregateExpr,
    InsertValuesStmt,
    plan,
)
from sql_planner.plan import (
    Aggregate,
    Distinct,
    LogicalPlan,
    Project,
    Sort,
)
from sql_planner.plan import (
    Limit as PlanLimit,
)
from sql_vm import QueryResult, execute

from .adapter import to_statement
from .binding import substitute
from .errors import ProgrammingError, translate


def run(backend: Backend, sql: str, parameters: Sequence[Any] = ()) -> QueryResult:
    """Execute a single SQL statement and return the :class:`QueryResult`.

    ``parameters`` is an ordered sequence matching the ``?`` placeholders
    in ``sql``. Empty for un-parameterised statements.
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
        program = codegen_compile(_flatten_project_over_aggregate(optimized))
        return execute(program, backend)
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
    """
    # Walk down through ordering/limit wrappers looking for Project.
    stack: list[LogicalPlan] = []
    cur: LogicalPlan = p
    while isinstance(cur, (Sort, Distinct, PlanLimit)):
        stack.append(cur)
        cur = cur.input
    if not isinstance(cur, Project) or not isinstance(cur.input, Aggregate):
        return p
    project: Project = cur
    aggregate: Aggregate = cur.input

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

    # Re-wrap with any Sort/Distinct/Limit stack we peeled off. Rewrite
    # sort/limit/distinct inputs to point at the flattened Aggregate.
    out: LogicalPlan = new_aggregate
    for wrap in reversed(stack):
        out = replace(wrap, input=out)
    return out
