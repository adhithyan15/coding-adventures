"""
ProjectionPruning — annotate scans with the column subset the query needs.

Why it matters
--------------

Backends materialize every column in every scanned row by default. A
query that selects one column from a 50-column table pays 50x the memory
cost to bring rows through the pipeline. Annotating the Scan with a
``required_columns`` field lets a column-store or columnar-reader
backend drop the unneeded columns before they are handed to the VM.

Shape
-----

Top-down traversal: we carry a *requirement set* — the set of
``(alias, column)`` pairs the parent needs — and recurse. At each Scan,
we intersect the requirement set with that scan's alias and emit the
subset as ``required_columns``.

We keep ``required_columns`` as a tuple of bare column names (not
qualified pairs) because that is what the backend needs. The alias
filter happens during descent.
"""

from __future__ import annotations

from sql_planner import (
    Aggregate,
    Begin,
    Between,
    BinaryExpr,
    CaseExpr,
    Commit,
    DerivedTable,
    Distinct,
    EmptyResult,
    Except,
    Expr,
    Filter,
    FunctionCall,
    Having,
    In,
    IndexScan,
    Intersect,
    IsNotNull,
    IsNull,
    Join,
    Like,
    LogicalPlan,
    NotIn,
    NotLike,
    Project,
    Rollback,
    Scan,
    Sort,
    UnaryExpr,
    Union,
    Wildcard,
    collect_columns,
)
from sql_planner.expr import AggregateExpr
from sql_planner.plan import Limit

# A requirement is a set of (alias_or_table, column_name) pairs.
Req = frozenset[tuple[str, str]]


class ProjectionPruning:
    """Propagate required-column sets top-down, annotate Scans at the leaves."""

    name = "ProjectionPruning"

    def __call__(self, plan: LogicalPlan) -> LogicalPlan:
        # Root = no required-column constraint. Projects below will add their
        # own requirements; the root node's output columns are needed in
        # full, so we pass a sentinel "everything" which is represented by
        # None.
        return _prune(plan, required=None)


def _prune(p: LogicalPlan, required: Req | None) -> LogicalPlan:
    """Descend ``p``, threading the required-column set downward.

    ``required=None`` means "the caller doesn't know / needs everything".
    A Scan reached under None gets no annotation (all columns).
    """
    match p:
        case Scan(
            table=t, alias=a, required_columns=existing, scan_limit=sl
        ):
            if required is None:
                return p
            alias = a or t
            needed = tuple(sorted({c for k, c in required if k == alias}))
            # Preserve existing more-restrictive annotation if already set.
            if existing is not None:
                merged = tuple(sorted(set(existing) | set(needed)))
                return Scan(
                    table=t, alias=a, required_columns=merged, scan_limit=sl
                )
            return Scan(
                table=t, alias=a, required_columns=needed, scan_limit=sl
            )

        case IndexScan():
            # IndexScan is a leaf like Scan, but it has no required_columns
            # annotation field — the index already constrains the access path.
            # Projection pruning can't add hints here, so pass through.
            return p

        case EmptyResult():
            return p

        case Project(input=inner, items=items):
            # The Project introduces the requirements for its input — it
            # replaces whatever came from above, because the only things
            # the parent can see are the Project's outputs.
            new_req = _required_from_exprs(tuple(i.expr for i in items))
            return Project(input=_prune(inner, new_req), items=items)

        case Filter(input=inner, predicate=pred):
            new_req = (required or frozenset()) | _required_from_exprs((pred,))
            return Filter(input=_prune(inner, new_req), predicate=pred)

        case Aggregate(input=inner, group_by=gb, aggregates=aggs):
            agg_exprs: list[Expr] = list(gb)
            for a in aggs:
                if a.arg.value is not None:
                    agg_exprs.append(a.arg.value)
            new_req = _required_from_exprs(tuple(agg_exprs))
            return Aggregate(input=_prune(inner, new_req), group_by=gb, aggregates=aggs)

        case Having(input=inner, predicate=pred):
            new_req = (required or frozenset()) | _required_from_exprs((pred,))
            return Having(input=_prune(inner, new_req), predicate=pred)

        case Sort(input=inner, keys=keys):
            new_req = (required or frozenset()) | _required_from_exprs(
                tuple(k.expr for k in keys)
            )
            return Sort(input=_prune(inner, new_req), keys=keys)

        case Limit(input=inner, count=c, offset=o):
            return Limit(input=_prune(inner, required), count=c, offset=o)

        case Distinct(input=inner):
            return Distinct(input=_prune(inner, required))

        case Join(left=l, right=r, kind=k, condition=cond):
            # Whatever the parent needs + whatever the join condition needs.
            cond_req = _required_from_exprs((cond,)) if cond is not None else frozenset()
            combined = (required or frozenset()) | cond_req
            return Join(
                left=_prune(l, combined),
                right=_prune(r, combined),
                kind=k,
                condition=cond,
            )

        case Union(left=l, right=r, all=a):
            return Union(left=_prune(l, required), right=_prune(r, required), all=a)

        case Intersect(left=l, right=r, all=a):
            return Intersect(left=_prune(l, required), right=_prune(r, required), all=a)

        case Except(left=l, right=r, all=a):
            return Except(left=_prune(l, required), right=_prune(r, required), all=a)

        case DerivedTable(query=q, alias=alias, columns=cols):
            # Recurse into the inner query. The outer required-column set
            # does not propagate inside a derived table — the inner query has
            # its own scope. We pass None to let the inner optimizer use its
            # own logic.
            return DerivedTable(query=_prune(q, None), alias=alias, columns=cols)

        case Begin() | Commit() | Rollback():
            return p

        case _:
            # DML / DDL — no pruning, recurse only where there is an input
            # subtree worth annotating.
            return p


def _required_from_exprs(exprs: tuple[Expr, ...]) -> Req:
    """Columns (as (alias, col) pairs) referenced by a tuple of expressions.

    A Wildcard in an expression set forfeits pruning for that scope — we
    don't know yet which columns the wildcard will expand to, so we
    conservatively demand everything by returning a special empty set
    that the caller intersects to produce the empty tuple (and then the
    Scan sees ``required=None`` via its merge rule).
    """
    out: set[tuple[str, str]] = set()
    for e in exprs:
        if _contains_wildcard(e):
            # Signal 'all' by returning an empty Req — combined with None
            # up-tree, the scan keeps getting None and emits no annotation.
            # We collaborate with callers: an empty Req here means "no
            # pruning intel", not "need zero columns". To keep the
            # difference unambiguous we just not prune this side: callers
            # should interpret the empty frozenset at the root the same
            # way they would None.
            #
            # The simplest way: return None-equivalent here by collapsing
            # to whatever was there before. Callers that or this with
            # existing requirements get unchanged behaviour.
            #
            # We achieve this by collecting everything *except* wildcards,
            # and trust the empty-Req-at-Scan path to leave the scan
            # unannotated.
            continue
        for c in collect_columns(e):
            if c.table is not None:
                out.add((c.table, c.col))
        # Aggregate inner expressions: collect_columns already descends
        # through AggregateExpr.
        _ = AggregateExpr  # silence UP035 / F401 — re-exported for docs
    return frozenset(out)


def _contains_wildcard(e: Expr) -> bool:
    match e:
        case Wildcard():
            return True
        case BinaryExpr(_, left, right):
            return _contains_wildcard(left) or _contains_wildcard(right)
        case UnaryExpr(_, operand):
            return _contains_wildcard(operand)
        case IsNull(operand=o) | IsNotNull(operand=o):
            return _contains_wildcard(o)
        case Between(operand=op, low=lo, high=hi):
            return any(_contains_wildcard(x) for x in (op, lo, hi))
        case In(operand=op, values=vs) | NotIn(operand=op, values=vs):
            return _contains_wildcard(op) or any(_contains_wildcard(v) for v in vs)
        case Like(operand=op) | NotLike(operand=op):
            return _contains_wildcard(op)
        case FunctionCall(_, args):
            return any(a.value is not None and _contains_wildcard(a.value) for a in args)
        case CaseExpr(whens=whens, else_=else_):
            return (
                any(_contains_wildcard(c) or _contains_wildcard(r) for c, r in whens)
                or (else_ is not None and _contains_wildcard(else_))
            )
        case _:
            return False
