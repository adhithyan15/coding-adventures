"""
PredicatePushdown — move Filter nodes closer to the scans they reference.

Why push?
---------

A filter that eliminates 99% of rows is best applied *before* expensive
operations — joins that multiply row counts, sorts that touch every row,
projections that evaluate expressions per row. The rule of thumb is
"cheap first, expensive later," and pushing filters is the most effective
way to honour it in a relational-algebra plan.

What we do
----------

For each Filter node, we split its predicate on ``AND`` into conjuncts.
Each conjunct is an independent predicate: ``a AND b`` matches a row iff
both ``a`` and ``b`` do. Conjuncts can be pushed individually — some may
be eligible, some not. The conjuncts that stay behind are re-AND'd and
left at the original level.

Where we push
-------------

- Through ``Project``: if the conjunct only references input columns (not
  columns computed by the projection), push below the Project.
- Through ``Join``: if the conjunct's columns all come from one side of
  the join, push to that side. Conjuncts referencing both sides stay on
  top (they become join-equivalent predicates the join algorithm can
  consume).
- Through ``Sort``: always safe — sorting doesn't change which rows exist.
- Through ``Distinct``: always safe — the rows that survive filtering are
  the same set regardless of dedup order.

Where we do *not* push
----------------------

- Through ``Aggregate`` — the filter above an aggregate is a HAVING
  clause (or a Filter the planner put there by mistake). HAVING
  references aggregate outputs that don't exist below the Aggregate.
- Through ``Limit`` — ``LIMIT 5 + Filter`` yields different rows than
  ``Filter + LIMIT 5``.
- Through ``Union`` — not handled in v1. A Union below a Filter gets
  its own Filter copy by distribution; that rewrite is a future pass.

Column scoping
--------------

Each Scan exposes columns under its alias (or table name). A qualified
reference ``e.salary`` is "from side X" iff ``e`` is in X's alias set.
We compute each subtree's alias set once and consult it per conjunct.
"""

from __future__ import annotations

from sql_planner import (
    Aggregate,
    Begin,
    Between,
    BinaryExpr,
    BinaryOp,
    CaseExpr,
    Column,
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
    JoinKind,
    Like,
    Literal,
    LogicalPlan,
    NotIn,
    NotLike,
    Project,
    Rollback,
    Scan,
    Sort,
    UnaryExpr,
    Union,
)
from sql_planner.plan import Limit


class PredicatePushdown:
    """Pushes Filter conjuncts as close to their scans as possible."""

    name = "PredicatePushdown"

    def __call__(self, plan: LogicalPlan) -> LogicalPlan:
        return _push(plan)


def _push(p: LogicalPlan) -> LogicalPlan:
    match p:
        case Filter(input=inner, predicate=pred):
            return _push_filter(_push(inner), pred)
        case Project(input=inner, items=items):
            return Project(input=_push(inner), items=items)
        case Join(left=l, right=r, kind=k, condition=cond):
            return Join(left=_push(l), right=_push(r), kind=k, condition=cond)
        case Aggregate(input=inner, group_by=gb, aggregates=aggs):
            return Aggregate(input=_push(inner), group_by=gb, aggregates=aggs)
        case Having(input=inner, predicate=pred):
            return Having(input=_push(inner), predicate=pred)
        case Sort(input=inner, keys=keys):
            return Sort(input=_push(inner), keys=keys)
        case Limit(input=inner, count=c, offset=o):
            return Limit(input=_push(inner), count=c, offset=o)
        case Distinct(input=inner):
            return Distinct(input=_push(inner))
        case Union(left=l, right=r, all=a):
            return Union(left=_push(l), right=_push(r), all=a)
        case Intersect(left=l, right=r, all=a):
            return Intersect(left=_push(l), right=_push(r), all=a)
        case Except(left=l, right=r, all=a):
            return Except(left=_push(l), right=_push(r), all=a)
        case DerivedTable(query=q, alias=alias, columns=cols):
            # Recurse into the inner query; predicates from the outer query
            # are NOT pushed inside a derived table (they reference aliases
            # that only exist in the outer scope).
            return DerivedTable(query=_push(q), alias=alias, columns=cols)
        case Begin() | Commit() | Rollback():
            return p
        case _:
            return p


def _push_filter(inner: LogicalPlan, predicate: Expr) -> LogicalPlan:
    """Given the already-pushed child ``inner`` and a predicate to apply
    above it, push what can be pushed and wrap the rest in a Filter."""
    conjuncts = _split_conjuncts(predicate)
    keep, pushed = _distribute_conjuncts(inner, conjuncts)
    # ``pushed`` has already been applied inside ``inner``; ``keep`` is the
    # subset that couldn't move.
    if not keep:
        return pushed
    return Filter(input=pushed, predicate=_combine_and(keep))


def _distribute_conjuncts(
    tree: LogicalPlan, conjuncts: list[Expr]
) -> tuple[list[Expr], LogicalPlan]:
    """Return (conjuncts that couldn't be pushed, rewritten tree with the rest applied)."""
    match tree:
        # Through Project — safe if the conjunct doesn't reference any
        # expression the projection introduced. In this v1 we conservatively
        # push only when every column referenced in the conjunct already
        # appears as a bare Column in the Project's input alias set.
        case Project(input=child, items=items):
            child_aliases = _alias_set(child)
            eligible, stuck = _split_by_scope(conjuncts, child_aliases)
            if not eligible:
                return conjuncts, tree
            inner_keep, new_child = _distribute_conjuncts(child, eligible)
            new_child = _wrap_with_keeps(new_child, inner_keep)
            return stuck, Project(input=new_child, items=items)

        # Through Sort — always safe.
        case Sort(input=child, keys=keys):
            inner_keep, new_child = _distribute_conjuncts(child, conjuncts)
            new_child = _wrap_with_keeps(new_child, inner_keep)
            return [], Sort(input=new_child, keys=keys)

        # Through Distinct — always safe.
        case Distinct(input=child):
            inner_keep, new_child = _distribute_conjuncts(child, conjuncts)
            new_child = _wrap_with_keeps(new_child, inner_keep)
            return [], Distinct(input=new_child)

        # Through Join — each conjunct assigned to the side(s) whose
        # alias set contains its columns. Conjuncts that reference both
        # sides stay above the join.
        #
        # Outer-join safety: pushing a right-side predicate inside a LEFT
        # OUTER JOIN corrupts null-padding semantics — the filter fires
        # before the join, eliminating right rows that would have produced
        # null-padded left rows, and then the WHERE on NULL matches them
        # wrongly. Only push to a side when the join kind does not null-pad
        # that side:
        #   LEFT  JOIN → left side OK to push, right side NOT OK
        #   RIGHT JOIN → right side OK to push, left side NOT OK
        #   FULL  JOIN → neither side OK
        #   INNER / CROSS → both sides OK
        case Join(left=l, right=r, kind=k, condition=cond):
            left_aliases = _alias_set(l)
            right_aliases = _alias_set(r)
            can_push_left = k in (JoinKind.INNER, JoinKind.CROSS, JoinKind.LEFT)
            can_push_right = k in (JoinKind.INNER, JoinKind.CROSS, JoinKind.RIGHT)
            left_push: list[Expr] = []
            right_push: list[Expr] = []
            stuck: list[Expr] = []
            for c in conjuncts:
                cols = _column_aliases(c)
                if can_push_left and cols and cols.issubset(left_aliases):
                    left_push.append(c)
                elif can_push_right and cols and cols.issubset(right_aliases):
                    right_push.append(c)
                else:
                    stuck.append(c)
            new_l: LogicalPlan = l
            new_r: LogicalPlan = r
            if left_push:
                inner_keep, new_l_inner = _distribute_conjuncts(l, left_push)
                new_l = _wrap_with_keeps(new_l_inner, inner_keep)
            if right_push:
                inner_keep_r, new_r_inner = _distribute_conjuncts(r, right_push)
                new_r = _wrap_with_keeps(new_r_inner, inner_keep_r)
            return stuck, Join(left=new_l, right=new_r, kind=k, condition=cond)

        # Scan / IndexScan / EmptyResult / others — can't descend further.
        case Scan() | IndexScan() | EmptyResult():
            return conjuncts, tree

        # Aggregate, Having, Limit, Union — do not push through.
        case _:
            return conjuncts, tree


def _wrap_with_keeps(tree: LogicalPlan, keeps: list[Expr]) -> LogicalPlan:
    if not keeps:
        return tree
    return Filter(input=tree, predicate=_combine_and(keeps))


# --------------------------------------------------------------------------
# Helpers — conjunct splitting, alias scoping, column-alias extraction
# --------------------------------------------------------------------------


def _split_conjuncts(expr: Expr) -> list[Expr]:
    """Turn ``a AND b AND c`` into ``[a, b, c]``. Non-AND stays a singleton."""
    if isinstance(expr, BinaryExpr) and expr.op is BinaryOp.AND:
        return _split_conjuncts(expr.left) + _split_conjuncts(expr.right)
    return [expr]


def _combine_and(exprs: list[Expr]) -> Expr:
    """Re-AND a list. Left-associative; precondition: exprs is non-empty."""
    out = exprs[0]
    for e in exprs[1:]:
        out = BinaryExpr(op=BinaryOp.AND, left=out, right=e)
    return out


def _alias_set(plan: LogicalPlan) -> set[str]:
    """Every ``table`` or ``alias`` that identifies a Scan reachable from this subtree."""
    out: set[str] = set()
    _walk_aliases(plan, out)
    return out


def _walk_aliases(p: LogicalPlan, out: set[str]) -> None:
    match p:
        case Scan(table=t, alias=a) | IndexScan(table=t, alias=a):
            out.add(a or t)
        case DerivedTable(query=_, alias=a, columns=_):
            # A derived table exposes its alias to the outer query; we do NOT
            # descend into the inner plan — its aliases are inner scope only.
            out.add(a)
        case Filter(input=inner) | Project(input=inner) | Aggregate(input=inner) \
                | Having(input=inner) | Sort(input=inner) | Limit(input=inner) \
                | Distinct(input=inner):
            _walk_aliases(inner, out)
        case Join(left=l, right=r) | Union(left=l, right=r):
            _walk_aliases(l, out)
            _walk_aliases(r, out)
        case _:
            pass


def _column_aliases(expr: Expr) -> set[str]:
    """The set of table-aliases referenced by columns inside ``expr``.

    A qualified Column contributes its ``table`` field. A bare Column
    contributes nothing usable here — but by this point the planner has
    resolved every Column, so we expect qualified references only. Any
    bare column we still see makes this conjunct unpushable (conservative).
    """
    out: set[str] = set()
    _walk_column_aliases(expr, out, unknown_holder := [False])
    if unknown_holder[0]:
        out.add("__unknown__")  # poison: conjunct referencing unresolved cols can't push
    return out


def _split_by_scope(
    conjuncts: list[Expr], scope: set[str]
) -> tuple[list[Expr], list[Expr]]:
    """Return (eligible, stuck) — eligible conjuncts have all column aliases in scope."""
    eligible: list[Expr] = []
    stuck: list[Expr] = []
    for c in conjuncts:
        cols = _column_aliases(c)
        if cols and cols.issubset(scope):
            eligible.append(c)
        elif not cols:
            # No column references — a pure constant predicate (ConstantFolding
            # should have collapsed these, but be safe). Push anywhere.
            eligible.append(c)
        else:
            stuck.append(c)
    return eligible, stuck


def _walk_column_aliases(expr: Expr, out: set[str], unknown: list[bool]) -> None:
    match expr:
        case Column(table=t, col=_):
            if t is None:
                unknown[0] = True
            else:
                out.add(t)
        case BinaryExpr(_, left, right):
            _walk_column_aliases(left, out, unknown)
            _walk_column_aliases(right, out, unknown)
        case UnaryExpr(_, operand):
            _walk_column_aliases(operand, out, unknown)
        case IsNull(operand=operand) | IsNotNull(operand=operand):
            _walk_column_aliases(operand, out, unknown)
        case Between(operand=op, low=lo, high=hi):
            _walk_column_aliases(op, out, unknown)
            _walk_column_aliases(lo, out, unknown)
            _walk_column_aliases(hi, out, unknown)
        case In(operand=op, values=vs) | NotIn(operand=op, values=vs):
            _walk_column_aliases(op, out, unknown)
            for v in vs:
                _walk_column_aliases(v, out, unknown)
        case Like(operand=op) | NotLike(operand=op):
            _walk_column_aliases(op, out, unknown)
        case FunctionCall(_, args):
            for a in args:
                if a.value is not None:
                    _walk_column_aliases(a.value, out, unknown)
        case CaseExpr(whens=whens, else_=else_):
            for cond, result in whens:
                _walk_column_aliases(cond, out, unknown)
                _walk_column_aliases(result, out, unknown)
            if else_ is not None:
                _walk_column_aliases(else_, out, unknown)
        case Literal():
            pass
        case _:
            # Aggregates / Wildcards never belong in pushable conjuncts;
            # mark the conjunct unpushable.
            unknown[0] = True
