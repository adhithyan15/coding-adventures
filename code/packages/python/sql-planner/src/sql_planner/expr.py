"""
Expression IR
=============

Expressions appear throughout the pipeline: in WHERE / HAVING predicates, in
SELECT projection items, in INSERT VALUES lists, in UPDATE assignments, and
inside aggregate function arguments. The planner, optimizer, and codegen
stages all speak in terms of the types defined here.

We use an **algebraic data type** encoded as a sealed hierarchy of frozen
dataclasses, each with a discriminant attribute. That shape gives us:

- Structural equality for free (frozen+eq dataclasses) — makes test
  assertions short and non-brittle.
- A single ``Expr`` union type for signatures, so callers match on
  ``isinstance`` (or, for the VM later, a dispatch table keyed on type).
- Immutability: expression trees can be shared between plan nodes
  without defensive copying. The optimizer relies on this when rewriting
  trees.

Why not a single Expr class with a "kind" enum?
-----------------------------------------------

Because every "kind" carries different fields. A ``BinaryOp`` has two
sub-expressions and an operator; a ``Literal`` has one value. If we packed
them all into one class, every field would have to be optional — and every
site that reads an expression would defensively check for None. Separating
the cases pushes the type system to enforce structure at construction time.

NULL in comparisons
-------------------

Three-valued logic (TRUE / FALSE / NULL) is enforced by the VM, not the
planner. The planner only *builds* expression trees; the VM evaluates them
per SQL semantics. We do not try to "normalize" ``x IS NULL`` vs ``x = NULL``
at plan time because they mean different things in SQL — the first is
``TRUE`` when x is null, the second is always ``NULL``.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum

from sql_backend.values import SqlValue


class BinaryOp(Enum):
    """Binary operators in expressions. Naming matches the spec exactly."""

    EQ = "="
    NOT_EQ = "<>"
    LT = "<"
    LTE = "<="
    GT = ">"
    GTE = ">="
    AND = "AND"
    OR = "OR"
    ADD = "+"
    SUB = "-"
    MUL = "*"
    DIV = "/"
    MOD = "%"


class UnaryOp(Enum):
    """Unary operators."""

    NOT = "NOT"
    NEG = "-"


class AggFunc(Enum):
    """SQL aggregate functions we support. Add more (stddev, group_concat) later."""

    COUNT = "COUNT"
    SUM = "SUM"
    AVG = "AVG"
    MIN = "MIN"
    MAX = "MAX"


# ---- Expr variants --------------------------------------------------------
#
# Every variant is a frozen dataclass. We give each one a slots layout for a
# small memory win on large plan trees, and freeze=True so they hash and
# can be deduplicated by the optimizer's constant-folding pass.

@dataclass(frozen=True, slots=True)
class Literal:
    """A compile-time constant — 42, 'hello', NULL, TRUE."""

    value: SqlValue


@dataclass(frozen=True, slots=True)
class Column:
    """A column reference, optionally qualified by table or alias.

    After planning, every Column that can be disambiguated is qualified:
    ``salary`` in a single-table query becomes ``Column(table="employees",
    col="salary")``. Ambiguous references raise AmbiguousColumn during
    planning and never make it into a finished LogicalPlan.
    """

    table: str | None
    col: str


@dataclass(frozen=True, slots=True)
class BinaryExpr:
    """Binary operator applied to two sub-expressions."""

    op: BinaryOp
    left: Expr
    right: Expr


@dataclass(frozen=True, slots=True)
class UnaryExpr:
    """Unary operator applied to one sub-expression."""

    op: UnaryOp
    operand: Expr


@dataclass(frozen=True, slots=True)
class FuncArg:
    """Argument to a function call. ``star=True`` represents ``*`` (for COUNT(*))."""

    star: bool = False
    value: Expr | None = None

    def __post_init__(self) -> None:
        # Exactly one of star / value must be set.
        if self.star == (self.value is not None):
            raise ValueError("FuncArg must set exactly one of star or value")


@dataclass(frozen=True, slots=True)
class FunctionCall:
    """Generic function call — scalar (e.g. UPPER, LOWER) or user-defined.

    Aggregate functions use :class:`AggregateExpr` instead. Mixing them in
    one node would force downstream code to re-discriminate on every visit.
    """

    name: str
    args: tuple[FuncArg, ...] = field(default_factory=tuple)


@dataclass(frozen=True, slots=True)
class IsNull:
    """``expr IS NULL`` — TRUE iff the operand evaluates to NULL."""

    operand: Expr


@dataclass(frozen=True, slots=True)
class IsNotNull:
    """``expr IS NOT NULL`` — TRUE iff the operand evaluates to a non-NULL value."""

    operand: Expr


@dataclass(frozen=True, slots=True)
class Between:
    """``expr BETWEEN low AND high`` — inclusive on both ends."""

    operand: Expr
    low: Expr
    high: Expr


@dataclass(frozen=True, slots=True)
class In:
    """``expr IN (v1, v2, ...)``."""

    operand: Expr
    values: tuple[Expr, ...]


@dataclass(frozen=True, slots=True)
class NotIn:
    """``expr NOT IN (v1, v2, ...)``."""

    operand: Expr
    values: tuple[Expr, ...]


@dataclass(frozen=True, slots=True)
class Like:
    """``expr LIKE 'pattern'`` — SQL pattern matching with % and _ wildcards."""

    operand: Expr
    pattern: str


@dataclass(frozen=True, slots=True)
class NotLike:
    """``expr NOT LIKE 'pattern'``."""

    operand: Expr
    pattern: str


@dataclass(frozen=True, slots=True)
class Wildcard:
    """The ``*`` in ``SELECT *`` — expanded by codegen against the backend schema."""


@dataclass(frozen=True, slots=True)
class CaseExpr:
    """Searched CASE — ``CASE WHEN c1 THEN v1 [WHEN c2 THEN v2 ...] [ELSE e] END``.

    The adapter normalizes *simple* CASE (``CASE x WHEN v THEN r ...``) into
    searched form by wrapping each WHEN value in a ``BinaryExpr(EQ, operand, v)``.
    The planner and all downstream stages only ever see searched CASE, which
    keeps the codegen and VM simple.

    How CASE evaluates (SQL standard)
    ----------------------------------

    Conditions are tested left-to-right.  The result of the first WHEN branch
    whose condition evaluates to TRUE is returned.  If no WHEN matches, the
    ELSE expression is used.  When there is no ELSE and no WHEN matches, the
    result is NULL — so ``else_=None`` is semantically ``ELSE NULL``.

    Codegen strategy
    ----------------

    CASE compiles to a conditional-jump chain using existing
    ``JumpIfFalse`` and ``Jump`` instructions.  After the end label, exactly
    one value sits on the stack.  No new VM instructions are needed::

        compile(c1)           ; push condition
        JumpIfFalse(when2_lbl); pop condition, skip if false
        compile(v1)           ; push result
        Jump(end_lbl)
        Label(when2_lbl)
        ...
        Label(else_lbl)
        compile(e)            ; ELSE branch, or LoadConst(None) if absent
        Label(end_lbl)
    """

    whens: tuple[tuple[Expr, Expr], ...]  # (condition, result) pairs, ≥1
    else_: Expr | None = None             # None ⇒ ELSE NULL


@dataclass(frozen=True, slots=True)
class AggregateExpr:
    """An aggregate function — ``COUNT(*)``, ``SUM(salary)``, ``COUNT(DISTINCT email)``.

    Aggregates can only appear in SELECT lists, HAVING predicates, and
    ORDER BY keys. The planner rejects aggregates in WHERE (SQL forbids
    this — WHERE runs before grouping, so aggregates make no sense there).
    """

    func: AggFunc
    arg: FuncArg  # star=True for COUNT(*)
    distinct: bool = False


@dataclass(frozen=True, slots=True)
class ExistsSubquery:
    """``EXISTS (subquery)`` — TRUE iff the inner query returns at least one row.

    Lifecycle
    ---------
    This node is created by the adapter with ``query`` holding a raw
    ``SelectStmt``.  The planner's ``_resolve()`` replaces ``query`` with
    the compiled ``LogicalPlan`` before passing the expression to codegen.

    ``query`` is typed as ``object`` to break the circular import between
    this module and ``sql_planner.plan`` (which itself imports ``Expr``).
    Callers narrow the type at the appropriate pipeline stage.

    NOT EXISTS
    ----------
    ``NOT EXISTS (...)`` is represented as
    ``UnaryExpr(op=UnaryOp.NOT, operand=ExistsSubquery(...))``.
    No ``negated`` field is needed — the existing ``UnaryOp.NOT`` instruction
    handles inversion at runtime without any extra complexity here.
    """

    query: object  # SelectStmt before _resolve; LogicalPlan after _resolve


@dataclass(frozen=True, slots=True)
class WindowFuncExpr:
    """A window (analytic) function: ``func([arg]) OVER (PARTITION BY … ORDER BY …)``.

    Window functions differ from aggregate functions:
    - They do *not* collapse rows — one output row per input row.
    - They operate over a "window" of related rows defined by the OVER clause.
    - They may appear only in SELECT lists (not WHERE, GROUP BY, or HAVING).

    Fields
    ------
    func:
        Function name in lower-case (``"row_number"``, ``"sum"``, …).
        The planner normalises to lower-case; codegen maps to :class:`WinFunc`.
    arg:
        The single expression argument, or ``None`` for arg-free functions
        (``ROW_NUMBER``, ``RANK``, ``DENSE_RANK``).  ``COUNT(*)`` passes
        ``arg=None`` with ``func="count_star"``.
    partition_by:
        Tuple of partition key expressions.  Rows with equal partition keys
        are placed in the same window group.  Empty tuple means the whole
        result set is one partition.
    order_by:
        Tuple of ``(expr, descending)`` sort keys within each partition.
        Required for ranking functions; optional for aggregating functions.

    Lifecycle
    ---------
    Created by the adapter with raw ``Column`` references.  ``_resolve()``
    in the planner qualifies each expression against the current scope,
    producing a fully-resolved tree that codegen can emit directly.
    """

    func: str                                    # e.g. "row_number", "sum"
    arg: "Expr | None"                           # None for arg-free funcs
    partition_by: tuple["Expr", ...] = ()
    order_by: tuple[tuple["Expr", bool], ...] = ()  # (expr, descending)


# The type union every non-specialized consumer should match on. Order
# doesn't matter for correctness, but we keep the union sorted to help
# anyone eyeballing pattern matches.
Expr = (
    Literal
    | Column
    | BinaryExpr
    | UnaryExpr
    | FunctionCall
    | IsNull
    | IsNotNull
    | Between
    | In
    | NotIn
    | Like
    | NotLike
    | Wildcard
    | CaseExpr
    | AggregateExpr
    | ExistsSubquery
    | WindowFuncExpr
)


def contains_aggregate(expr: Expr) -> bool:
    """Return True if ``expr`` contains an :class:`AggregateExpr` anywhere.

    Used by the planner to reject aggregates in WHERE and to detect implicit
    aggregation (SELECT COUNT(*) FROM t with no GROUP BY).
    """
    match expr:
        case AggregateExpr():
            return True
        case BinaryExpr(_, left, right):
            return contains_aggregate(left) or contains_aggregate(right)
        case UnaryExpr(_, operand):
            return contains_aggregate(operand)
        case FunctionCall(_, args):
            return any(a.value is not None and contains_aggregate(a.value) for a in args)
        case IsNull(operand) | IsNotNull(operand):
            return contains_aggregate(operand)
        case Between(operand, low, high):
            return (
                contains_aggregate(operand)
                or contains_aggregate(low)
                or contains_aggregate(high)
            )
        case In(operand, values) | NotIn(operand, values):
            return contains_aggregate(operand) or any(contains_aggregate(v) for v in values)
        case Like(operand, _) | NotLike(operand, _):
            return contains_aggregate(operand)
        case CaseExpr(whens, else_):
            if any(
                contains_aggregate(cond) or contains_aggregate(result)
                for cond, result in whens
            ):
                return True
            return else_ is not None and contains_aggregate(else_)
        case ExistsSubquery():
            # The inner query is independently scoped; from the outer
            # expression's perspective EXISTS is a boolean atom, not an
            # aggregate.
            return False
        case WindowFuncExpr():
            # Window functions are handled by a separate WindowAgg plan node.
            # They are not aggregates from the planner's perspective.
            return False
        case _:
            return False


def collect_columns(expr: Expr) -> list[Column]:
    """Return every ``Column`` reference inside ``expr``, in tree-walk order.

    Used by projection-pruning in the optimizer (to know which columns an
    expression really needs) and by column resolution (to find all
    references that need qualification).
    """
    out: list[Column] = []
    _collect_columns(expr, out)
    return out


def _collect_columns(expr: Expr, out: list[Column]) -> None:
    match expr:
        case Column():
            out.append(expr)
        case BinaryExpr(_, left, right):
            _collect_columns(left, out)
            _collect_columns(right, out)
        case UnaryExpr(_, operand):
            _collect_columns(operand, out)
        case FunctionCall(_, args):
            for a in args:
                if a.value is not None:
                    _collect_columns(a.value, out)
        case IsNull(operand) | IsNotNull(operand):
            _collect_columns(operand, out)
        case Between(operand, low, high):
            _collect_columns(operand, out)
            _collect_columns(low, out)
            _collect_columns(high, out)
        case In(operand, values) | NotIn(operand, values):
            _collect_columns(operand, out)
            for v in values:
                _collect_columns(v, out)
        case Like(operand, _) | NotLike(operand, _):
            _collect_columns(operand, out)
        case CaseExpr(whens, else_):
            for cond, result in whens:
                _collect_columns(cond, out)
                _collect_columns(result, out)
            if else_ is not None:
                _collect_columns(else_, out)
        case AggregateExpr(_, arg, _):
            if arg.value is not None:
                _collect_columns(arg.value, out)
        case ExistsSubquery():
            # Inner query columns are independently scoped — not visible to
            # projection-pruning or column-resolution in the outer query.
            pass
        case WindowFuncExpr(_, arg, partition_by, order_by):
            if arg is not None:
                _collect_columns(arg, out)
            for e in partition_by:
                _collect_columns(e, out)
            for e, _ in order_by:
                _collect_columns(e, out)
        case _:
            pass
