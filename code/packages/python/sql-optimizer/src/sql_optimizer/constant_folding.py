"""
ConstantFolding — evaluate constant sub-expressions at plan time.

Why fold at plan time at all? Because every row the VM executes pays the
cost of evaluating constant sub-expressions like ``1 + 1`` over and over.
Folding once, in the optimizer, saves that cost N times for an N-row
scan. It also enables downstream passes: ``Filter(1 = 2)`` only becomes
``Filter(FALSE)`` after ConstantFolding, which only then lets
DeadCodeElimination replace the whole subtree with ``EmptyResult``.

Traversal
---------

Bottom-up, post-order: we fold a node's children before the node itself.
That means a parent like ``1 + (2 + 3)`` folds ``2 + 3 → 5`` first, then
``1 + 5 → 6``.

NULL semantics
--------------

SQL uses three-valued logic. Most folded operators propagate NULL:
``NULL + x``, ``NULL * 0``, ``NULL = NULL`` all fold to NULL. Boolean
operators are partial exceptions because of short-circuiting:
``TRUE OR anything`` is TRUE even if ``anything`` is NULL; ``FALSE AND
anything`` is FALSE. We replicate SQL's three-valued truth tables
directly.

What we do *not* fold
---------------------

- ``1 / 0`` — leave alone so the VM raises the runtime error with a
  source position.
- Any expression involving a Column or an AggregateExpr — those aren't
  constants.
- FunctionCalls — scalar functions like UPPER are backend-defined and
  we don't evaluate them here. A future pass could fold
  ``UPPER('hi') → 'HI'`` if we pull the kernel list into the optimizer.
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
    Delete,
    DerivedTable,
    Distinct,
    EmptyResult,
    Except,
    Expr,
    Filter,
    Having,
    In,
    IndexScan,
    Insert,
    InsertSource,
    Intersect,
    IsNotNull,
    IsNull,
    Join,
    Like,
    Literal,
    LogicalPlan,
    NotIn,
    NotLike,
    Project,
    ProjectionItem,
    Rollback,
    Scan,
    Sort,
    UnaryExpr,
    UnaryOp,
    Union,
    Update,
)
from sql_planner.plan import Assignment as PlanAssignment
from sql_planner.plan import Limit, SortKey


class ConstantFolding:
    """Pure-function pass over expression trees inside a plan."""

    name = "ConstantFolding"

    def __call__(self, plan: LogicalPlan) -> LogicalPlan:
        return _fold_plan(plan)


def _fold_plan(p: LogicalPlan) -> LogicalPlan:
    match p:
        case Scan() | EmptyResult():
            return p
        case Filter(input=inner, predicate=pred):
            return Filter(input=_fold_plan(inner), predicate=_fold_expr(pred))
        case Project(input=inner, items=items):
            new_items = tuple(
                ProjectionItem(expr=_fold_expr(it.expr), alias=it.alias) for it in items
            )
            return Project(input=_fold_plan(inner), items=new_items)
        case Join(left=l, right=r, kind=k, condition=cond):
            return Join(
                left=_fold_plan(l),
                right=_fold_plan(r),
                kind=k,
                condition=_fold_expr(cond) if cond is not None else None,
            )
        case Aggregate(input=inner, group_by=gb, aggregates=aggs):
            return Aggregate(
                input=_fold_plan(inner),
                group_by=tuple(_fold_expr(e) for e in gb),
                aggregates=aggs,
            )
        case Having(input=inner, predicate=pred):
            return Having(input=_fold_plan(inner), predicate=_fold_expr(pred))
        case Sort(input=inner, keys=keys):
            new_keys = tuple(
                SortKey(
                    expr=_fold_expr(k.expr),
                    descending=k.descending,
                    nulls_first=k.nulls_first,
                )
                for k in keys
            )
            return Sort(input=_fold_plan(inner), keys=new_keys)
        case Limit(input=inner, count=c, offset=o):
            return Limit(input=_fold_plan(inner), count=c, offset=o)
        case Distinct(input=inner):
            return Distinct(input=_fold_plan(inner))
        case Union(left=l, right=r, all=a):
            return Union(left=_fold_plan(l), right=_fold_plan(r), all=a)
        case Intersect(left=l, right=r, all=a):
            return Intersect(left=_fold_plan(l), right=_fold_plan(r), all=a)
        case Except(left=l, right=r, all=a):
            return Except(left=_fold_plan(l), right=_fold_plan(r), all=a)
        case DerivedTable(query=q, alias=alias, columns=cols):
            return DerivedTable(query=_fold_plan(q), alias=alias, columns=cols)
        case Begin() | Commit() | Rollback():
            return p  # transaction control — nothing to fold
        case Insert(table=t, columns=cols, source=src, on_conflict=oc, returning=ret):
            new_src = _fold_insert_source(src)
            new_ret = tuple(_fold_expr(r) for r in ret)
            return Insert(table=t, columns=cols, source=new_src, on_conflict=oc, returning=new_ret)
        case Update(table=t, assignments=asgs, predicate=pred, returning=ret):
            return Update(
                table=t,
                assignments=tuple(
                    PlanAssignment(column=a.column, value=_fold_expr(a.value))
                    for a in asgs
                ),
                predicate=_fold_expr(pred) if pred is not None else None,
                returning=tuple(_fold_expr(r) for r in ret),
            )
        case Delete(table=t, predicate=pred, returning=ret):
            return Delete(
                table=t,
                predicate=_fold_expr(pred) if pred is not None else None,
                returning=tuple(_fold_expr(r) for r in ret),
            )
        case IndexScan(
            table=t, alias=a, index_name=iname, columns=cols,
            lo=lo, hi=hi, lo_inclusive=lo_incl, hi_inclusive=hi_incl,
            residual=residual,
        ):
            # Fold the residual predicate (the non-index part of the WHERE).
            new_res = _fold_expr(residual) if residual is not None else None
            if new_res is residual:
                return p
            return IndexScan(
                table=t, alias=a, index_name=iname, columns=cols,
                lo=lo, hi=hi, lo_inclusive=lo_incl, hi_inclusive=hi_incl,
                residual=new_res,
            )
        case _:
            return p  # CreateTable, DropTable, CreateIndex, DropIndex — nothing to fold


def _fold_insert_source(src: InsertSource) -> InsertSource:
    if src.values is not None:
        return InsertSource(
            values=tuple(tuple(_fold_expr(v) for v in row) for row in src.values)
        )
    return InsertSource(query=_fold_plan(src.query))  # type: ignore[arg-type]


# --------------------------------------------------------------------------
# Expression-level folding
# --------------------------------------------------------------------------


def _fold_expr(e: Expr) -> Expr:
    match e:
        case Literal() | Column():
            return e
        case BinaryExpr(op=op, left=left, right=right):
            return _fold_binary(op, _fold_expr(left), _fold_expr(right))
        case UnaryExpr(op=op, operand=operand):
            return _fold_unary(op, _fold_expr(operand))
        case IsNull(operand=operand):
            folded = _fold_expr(operand)
            if isinstance(folded, Literal):
                return Literal(value=folded.value is None)
            return IsNull(operand=folded)
        case IsNotNull(operand=operand):
            folded = _fold_expr(operand)
            if isinstance(folded, Literal):
                return Literal(value=folded.value is not None)
            return IsNotNull(operand=folded)
        case Between(operand=op, low=lo, high=hi):
            return Between(
                operand=_fold_expr(op),
                low=_fold_expr(lo),
                high=_fold_expr(hi),
            )
        case In(operand=op, values=vs):
            return In(
                operand=_fold_expr(op),
                values=tuple(_fold_expr(v) for v in vs),
            )
        case NotIn(operand=op, values=vs):
            return NotIn(
                operand=_fold_expr(op),
                values=tuple(_fold_expr(v) for v in vs),
            )
        case Like(operand=op, pattern=p):
            return Like(operand=_fold_expr(op), pattern=p)
        case NotLike(operand=op, pattern=p):
            return NotLike(operand=_fold_expr(op), pattern=p)
        case CaseExpr(whens=whens, else_=else_):
            # Fold each branch, but don't try to short-circuit at plan time
            # — short-circuit evaluation of CASE is the VM's responsibility.
            return CaseExpr(
                whens=tuple((_fold_expr(cond), _fold_expr(result)) for cond, result in whens),
                else_=_fold_expr(else_) if else_ is not None else None,
            )
        case _:
            # Wildcard, FunctionCall, AggregateExpr — not folded.
            return e


def _fold_binary(op: BinaryOp, left: Expr, right: Expr) -> Expr:
    # Boolean simplification — try short-circuits before demanding both sides
    # be literals. ``TRUE OR unknown`` is TRUE per SQL three-valued logic.
    if op is BinaryOp.AND:
        simp = _simplify_and(left, right)
        if simp is not None:
            return simp
    if op is BinaryOp.OR:
        simp = _simplify_or(left, right)
        if simp is not None:
            return simp

    if not (isinstance(left, Literal) and isinstance(right, Literal)):
        return BinaryExpr(op=op, left=left, right=right)
    lv, rv = left.value, right.value

    # NULL propagation for non-boolean ops. Boolean AND/OR handled above.
    if op not in (BinaryOp.AND, BinaryOp.OR) and (lv is None or rv is None):
        # Comparisons with NULL yield NULL (not FALSE). Arithmetic too.
        return Literal(value=None)

    try:
        return Literal(value=_apply_binary(op, lv, rv))
    except (ZeroDivisionError, TypeError):
        # Don't fold errors; leave them for the VM to raise at runtime with
        # full source position.
        return BinaryExpr(op=op, left=left, right=right)


def _apply_binary(op: BinaryOp, lv: object, rv: object) -> object:
    match op:
        case BinaryOp.EQ:
            return lv == rv
        case BinaryOp.NOT_EQ:
            return lv != rv
        case BinaryOp.LT:
            return lv < rv  # type: ignore[operator]
        case BinaryOp.LTE:
            return lv <= rv  # type: ignore[operator]
        case BinaryOp.GT:
            return lv > rv  # type: ignore[operator]
        case BinaryOp.GTE:
            return lv >= rv  # type: ignore[operator]
        case BinaryOp.ADD:
            return lv + rv  # type: ignore[operator]
        case BinaryOp.SUB:
            return lv - rv  # type: ignore[operator]
        case BinaryOp.MUL:
            return lv * rv  # type: ignore[operator]
        case BinaryOp.DIV:
            # Integer-style division matches SQL: 7/2 = 3 for integers.
            if isinstance(lv, int) and isinstance(rv, int) and not isinstance(lv, bool):
                return lv // rv
            return lv / rv  # type: ignore[operator]
        case BinaryOp.MOD:
            return lv % rv  # type: ignore[operator]
        case BinaryOp.CONCAT:
            # SQL || string concatenation. Both sides are strings (the SQL type
            # system guarantees this; if they aren't, fall through to TypeError
            # and let the VM raise a proper TypeMismatch at runtime).
            return str(lv) + str(rv)  # type: ignore[operator]
        case BinaryOp.AND | BinaryOp.OR:
            # Unreachable — handled by _simplify_and / _simplify_or above.
            raise AssertionError("unreachable")


def _simplify_and(left: Expr, right: Expr) -> Expr | None:
    """SQL three-valued AND:

    ========  =====  =====  =====
    AND       TRUE   FALSE  NULL
    --------  -----  -----  -----
    TRUE      TRUE   FALSE  NULL
    FALSE     FALSE  FALSE  FALSE
    NULL      NULL   FALSE  NULL
    ========  =====  =====  =====
    """
    if isinstance(left, Literal) and left.value is False:
        return Literal(value=False)
    if isinstance(right, Literal) and right.value is False:
        return Literal(value=False)
    if isinstance(left, Literal) and left.value is True:
        return right
    if isinstance(right, Literal) and right.value is True:
        return left
    if isinstance(left, Literal) and isinstance(right, Literal):
        # Both are literals but neither is TRUE/FALSE — one is NULL.
        return Literal(value=None)
    return None


def _simplify_or(left: Expr, right: Expr) -> Expr | None:
    """SQL three-valued OR:

    ========  =====  =====  =====
    OR        TRUE   FALSE  NULL
    --------  -----  -----  -----
    TRUE      TRUE   TRUE   TRUE
    FALSE     TRUE   FALSE  NULL
    NULL      TRUE   NULL   NULL
    ========  =====  =====  =====
    """
    if isinstance(left, Literal) and left.value is True:
        return Literal(value=True)
    if isinstance(right, Literal) and right.value is True:
        return Literal(value=True)
    if isinstance(left, Literal) and left.value is False:
        return right
    if isinstance(right, Literal) and right.value is False:
        return left
    if isinstance(left, Literal) and isinstance(right, Literal):
        return Literal(value=None)
    return None


def _fold_unary(op: UnaryOp, operand: Expr) -> Expr:
    if not isinstance(operand, Literal):
        return UnaryExpr(op=op, operand=operand)
    v = operand.value
    if v is None:
        return Literal(value=None)
    if op is UnaryOp.NOT:
        return Literal(value=not v)
    if op is UnaryOp.NEG:
        return Literal(value=-v)  # type: ignore[operator]
    return UnaryExpr(op=op, operand=operand)  # pragma: no cover
