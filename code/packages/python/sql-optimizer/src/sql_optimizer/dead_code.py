"""
DeadCodeElimination — remove provably-empty subtrees and pointless filters.

Driven by what ConstantFolding produced. If the folder reduced a
predicate to the literal ``FALSE`` or to ``NULL`` (which in WHERE is
treated as FALSE), the filter cannot pass any rows and the whole subtree
below it is dead. Likewise ``LIMIT 0``.

We also drop filters whose predicate folded to ``TRUE`` — they accept
every row and therefore do nothing.

``EmptyResult`` propagation
---------------------------

Once we produce an :class:`EmptyResult`, we try to propagate it upward:

- ``Project(EmptyResult)`` → ``EmptyResult`` (with projected schema)
- ``Filter(EmptyResult)`` → ``EmptyResult``
- ``Sort(EmptyResult)`` / ``Limit(EmptyResult)`` / ``Distinct(EmptyResult)``
  → ``EmptyResult``
- ``Join`` with an ``EmptyResult`` side:
    * INNER or CROSS → the whole join is empty
    * LEFT / RIGHT / FULL → propagation is more delicate; in v1 we do
      not propagate through outer joins to avoid breaking semantics
      (a LEFT JOIN with empty right still produces left rows with NULLs).
- ``Union(EmptyResult, x)`` → ``x``
- ``Union(x, EmptyResult)`` → ``x``
- ``Aggregate(EmptyResult)`` — v1 leaves this alone because an empty
  input to ``SELECT COUNT(*) FROM t`` must still produce one row (0).
"""

from __future__ import annotations

from sql_planner import (
    Aggregate,
    Begin,
    Column,
    Commit,
    DerivedTable,
    Distinct,
    EmptyResult,
    Except,
    Filter,
    Having,
    IndexScan,
    Intersect,
    Join,
    JoinKind,
    Literal,
    LogicalPlan,
    Project,
    ProjectionItem,
    Rollback,
    Scan,
    Sort,
    Union,
    Wildcard,
)
from sql_planner.plan import Limit


def _schema_from_items(items: tuple[ProjectionItem, ...]) -> tuple[str, ...]:
    """Derive output column names from a Project's items.

    This mirrors ``_schema_of`` / ``_projection_name`` in the codegen.  We
    need it here so that ``Project(EmptyResult())`` can preserve the output
    schema in the resulting ``EmptyResult``, even when the inner node was
    produced by ``Limit(count=0)`` — which itself cannot know the projected
    schema at the time it fires.

    Rules (same as the codegen):
    - If *any* item is a ``Wildcard`` the schema is unknown at optimise time
      (it depends on the runtime table definition), so we return ``()``.
    - ``alias`` takes precedence over the expression name.
    - A plain ``Column`` reference contributes its column name.
    - Anything else produces the empty string ``""`` as a placeholder; the
      codegen will assign a positional name (``column_N``) at emit time, so
      the exact string does not matter here — only whether it is non-empty.
    """
    for item in items:
        if isinstance(item.expr, Wildcard):
            return ()
    result: list[str] = []
    for item in items:
        if item.alias is not None:
            result.append(item.alias)
        elif isinstance(item.expr, Column):
            result.append(item.expr.col)
        else:
            result.append("")
    return tuple(result)


def _schema_of_plan(p: LogicalPlan) -> tuple[str, ...]:
    """Walk a plan tree to derive the output column names.

    Used when ``Limit(count=0)`` needs to produce an ``EmptyResult`` that
    carries the correct column schema.  The ``inner`` plan at that point is
    the non-eliminated subtree (e.g. ``Sort(Project(...))``).  We walk down
    through ``Sort``, ``Distinct``, ``Limit``, and ``Having`` nodes (all of
    which preserve their inner schema) until we reach a ``Project``.

    Returns an empty tuple when the schema is unknowable (e.g. ``Wildcard``
    in the projection, or a bare ``Scan`` without schema info).
    """
    match p:
        case Project(items=items):
            return _schema_from_items(items)
        case Sort(input=inner) | Distinct(input=inner) | Having(input=inner) | Limit(input=inner):
            return _schema_of_plan(inner)
        case EmptyResult(columns=cols):
            return cols
        case _:
            return ()


class DeadCodeElimination:
    """Fold FALSE filters to EmptyResult; drop TRUE filters; collapse empties."""

    name = "DeadCodeElimination"

    def __call__(self, plan: LogicalPlan) -> LogicalPlan:
        return _eliminate(plan)


def _eliminate(p: LogicalPlan) -> LogicalPlan:
    match p:
        case Scan() | IndexScan() | EmptyResult():
            return p

        case Filter(input=inner, predicate=pred):
            inner = _eliminate(inner)
            if isinstance(pred, Literal):
                if pred.value is True:
                    return inner
                if pred.value is False or pred.value is None:
                    return EmptyResult()
            if isinstance(inner, EmptyResult):
                return EmptyResult()
            return Filter(input=inner, predicate=pred)

        case Project(input=inner, items=items):
            inner = _eliminate(inner)
            if isinstance(inner, EmptyResult):
                # Prefer the schema from inner.columns when the EmptyResult
                # was already annotated (e.g. by a nested Project elimination).
                # Fall back to deriving the schema from our own items when
                # inner.columns is empty — which happens when LIMIT 0 produced
                # a bare EmptyResult() before this Project had a chance to
                # annotate it.
                cols = inner.columns if inner.columns else _schema_from_items(items)
                return EmptyResult(columns=cols)
            return Project(input=inner, items=items)

        case Sort(input=inner, keys=keys):
            inner = _eliminate(inner)
            if isinstance(inner, EmptyResult):
                return inner
            return Sort(input=inner, keys=keys)

        case Limit(input=inner, count=c, offset=o):
            inner = _eliminate(inner)
            if c == 0:
                # Preserve the output schema so that cursor.description is
                # correct even when the query returns zero rows.  We derive the
                # schema from the inner plan rather than leaving it empty —
                # without this the codegen emits SetResultSchema(()) and the
                # PEP 249 cursor sees description=() instead of the expected
                # column tuple.
                return EmptyResult(columns=_schema_of_plan(inner))
            if isinstance(inner, EmptyResult):
                return inner
            return Limit(input=inner, count=c, offset=o)

        case Distinct(input=inner):
            inner = _eliminate(inner)
            if isinstance(inner, EmptyResult):
                return inner
            return Distinct(input=inner)

        case Having(input=inner, predicate=pred):
            inner = _eliminate(inner)
            if isinstance(inner, EmptyResult):
                return EmptyResult()
            return Having(input=inner, predicate=pred)

        case Aggregate(input=inner, group_by=gb, aggregates=aggs):
            # Do NOT elide Aggregate on empty input — SELECT COUNT(*)
            # over an empty table still returns one row (0).
            return Aggregate(input=_eliminate(inner), group_by=gb, aggregates=aggs)

        case Join(left=lft, right=rgt, kind=k, condition=cond):
            lft = _eliminate(lft)
            rgt = _eliminate(rgt)
            if k in (JoinKind.INNER, JoinKind.CROSS) and (
                isinstance(lft, EmptyResult) or isinstance(rgt, EmptyResult)
            ):
                return EmptyResult()
            return Join(left=lft, right=rgt, kind=k, condition=cond)

        case Union(left=lft, right=rgt, all=a):
            lft = _eliminate(lft)
            rgt = _eliminate(rgt)
            if isinstance(lft, EmptyResult):
                return rgt
            if isinstance(rgt, EmptyResult):
                return lft
            return Union(left=lft, right=rgt, all=a)

        case Intersect(left=lft, right=rgt, all=a):
            lft = _eliminate(lft)
            rgt = _eliminate(rgt)
            # Either side empty → intersection is empty.
            if isinstance(lft, EmptyResult) or isinstance(rgt, EmptyResult):
                return EmptyResult()
            return Intersect(left=lft, right=rgt, all=a)

        case Except(left=lft, right=rgt, all=a):
            lft = _eliminate(lft)
            rgt = _eliminate(rgt)
            # Left empty → result is empty regardless of right.
            if isinstance(lft, EmptyResult):
                return EmptyResult()
            # Right empty → left passes through unchanged (no rows to subtract).
            if isinstance(rgt, EmptyResult):
                return lft
            return Except(left=lft, right=rgt, all=a)

        case DerivedTable(query=q, alias=alias, columns=cols):
            return DerivedTable(query=_eliminate(q), alias=alias, columns=cols)

        case Begin() | Commit() | Rollback():
            return p

        case _:
            return p
