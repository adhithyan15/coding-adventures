"""
LimitPushdown — attach a ``scan_limit`` hint to scans where safe.

A hint, not a guarantee
-----------------------

The VM always enforces the real Limit at the correct level. Pushed hints
let a backend short-circuit early — e.g., stop reading after N rows —
but when a Filter sits between the Limit and the Scan, the backend may
need to read more than N raw rows before N pass the filter. The real
Limit above the Filter is still the arbiter.

Where we push
-------------

- Through ``Project`` — projecting 10 rows is projecting the first 10.
- Through ``Filter`` — with the caveat above; the hint becomes a hint.

Where we don't
--------------

- Through ``Sort`` — ``ORDER BY x LIMIT 5`` vs ``LIMIT 5 + ORDER BY x``
  are different queries.
- Through ``Aggregate`` — aggregates consume all input rows.
- Through ``Join`` — a join may duplicate or drop rows.
- Through ``Distinct`` — dedup needs every row first.
"""

from __future__ import annotations

from sql_planner import (
    Aggregate,
    Begin,
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
    LogicalPlan,
    Project,
    Rollback,
    Scan,
    Sort,
    Union,
)
from sql_planner.plan import Limit


class LimitPushdown:
    """Annotate Scans with ``scan_limit`` when a Limit is above them."""

    name = "LimitPushdown"

    def __call__(self, plan: LogicalPlan) -> LogicalPlan:
        return _push(plan)


def _push(p: LogicalPlan) -> LogicalPlan:
    match p:
        case Limit(input=inner, count=c, offset=o):
            # We only push the count component. If offset is set, pushing
            # is still safe (the backend reads at most count+offset raw
            # rows to cover both); but in v1 we keep it conservative —
            # only push when offset is None or 0.
            pushed_count = c if (o is None or o == 0) and c is not None else None
            new_inner = _attach(inner, pushed_count) if pushed_count is not None else _push(inner)
            return Limit(input=new_inner, count=c, offset=o)
        case Filter(input=inner, predicate=pred):
            return Filter(input=_push(inner), predicate=pred)
        case Project(input=inner, items=items):
            return Project(input=_push(inner), items=items)
        case Sort(input=inner, keys=keys):
            return Sort(input=_push(inner), keys=keys)
        case Aggregate(input=inner, group_by=gb, aggregates=aggs):
            return Aggregate(input=_push(inner), group_by=gb, aggregates=aggs)
        case Having(input=inner, predicate=pred):
            return Having(input=_push(inner), predicate=pred)
        case Distinct(input=inner):
            return Distinct(input=_push(inner))
        case Join(left=l, right=r, kind=k, condition=cond):
            return Join(left=_push(l), right=_push(r), kind=k, condition=cond)
        case Union(left=l, right=r, all=a):
            return Union(left=_push(l), right=_push(r), all=a)
        case Intersect(left=l, right=r, all=a):
            # Do not push Limit through INTERSECT — the semantics change.
            return Intersect(left=_push(l), right=_push(r), all=a)
        case Except(left=l, right=r, all=a):
            # Do not push Limit through EXCEPT — same reasoning.
            return Except(left=_push(l), right=_push(r), all=a)
        case DerivedTable(query=q, alias=alias, columns=cols):
            # Recurse into the inner plan; the outer LIMIT does NOT push
            # inside the derived table — the inner query has its own shape.
            return DerivedTable(query=_push(q), alias=alias, columns=cols)
        case Begin() | Commit() | Rollback():
            return p
        case _:
            return p


def _attach(p: LogicalPlan, limit: int) -> LogicalPlan:
    """Thread ``limit`` through safe passes-through until it reaches a Scan."""
    match p:
        case Scan(
            table=t, alias=a, required_columns=rc, scan_limit=existing
        ):
            # Preserve tighter hints if one is already set.
            new_limit = min(existing, limit) if existing is not None else limit
            return Scan(
                table=t, alias=a, required_columns=rc, scan_limit=new_limit
            )
        case IndexScan():
            # IndexScan already constrains which rows are returned via its lo/hi
            # bounds — no need to further annotate with scan_limit.
            return p
        case Project(input=inner, items=items):
            return Project(input=_attach(inner, limit), items=items)
        case Filter(input=inner, predicate=pred):
            return Filter(input=_attach(inner, limit), predicate=pred)
        case _:
            # Stop propagation — everything else is not safe to hint past.
            return _push(p)


# Silence an unused import warning.
_ = EmptyResult
