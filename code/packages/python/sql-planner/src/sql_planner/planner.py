"""
The planner core — AST → LogicalPlan
====================================

This module implements the translation from a structured SQL AST (see
:mod:`sql_planner.ast`) to a LogicalPlan tree (see :mod:`sql_planner.plan`).

Planning a SELECT statement
---------------------------

The planning order is bottom-up, matching the execution order of a SQL
pipeline:

1. Build the FROM / JOIN scan tree (the data sources).
2. Wrap it in a Filter for the WHERE clause.
3. Wrap it in an Aggregate if there is GROUP BY or any aggregate in
   SELECT / HAVING / ORDER BY.
4. Wrap it in a Having for HAVING.
5. Wrap it in a Project for the SELECT list.
6. Wrap it in a Distinct if SELECT DISTINCT.
7. Wrap it in a Sort for ORDER BY.
8. Wrap it in a Limit for LIMIT / OFFSET.

Steps 2–8 are only emitted when the corresponding clause is present. The
result is a tree where each node is a pure relational-algebra transform
and the leaves are :class:`Scan` nodes.

Column resolution
-----------------

After building the scan tree, the planner knows which tables (and aliases)
are in scope. It walks every expression — in SELECT, WHERE, GROUP BY,
HAVING, ORDER BY — and qualifies every bare :class:`Column` reference:

- If the column name appears in exactly one in-scope table, the planner
  fills in that table's alias (or name if unaliased).
- If it appears in more than one, the planner raises
  :class:`AmbiguousColumn`.
- If it appears in none, the planner raises :class:`UnknownColumn`.

Qualified references (``e.salary``) are validated — the planner checks
that ``e`` is a known alias and that ``e``'s table has a ``salary``
column — but passed through otherwise unchanged.

DML / DDL
---------

INSERT / UPDATE / DELETE / CREATE / DROP planning is mostly structural:
the planner just copies the statement into the equivalent LogicalPlan
node. The interesting bit for UPDATE / DELETE is that expressions still
need column resolution (against the single target table).
"""

from __future__ import annotations

from . import plan as P
from .ast import (
    Assignment as AstAssignment,
)
from .ast import (
    CreateTableStmt,
    DeleteStmt,
    DropTableStmt,
    InsertValuesStmt,
    JoinClause,
    JoinKind,
    SelectItem,
    SelectStmt,
    Statement,
    TableRef,
    UpdateStmt,
)
from .ast import (
    SortKey as AstSortKey,
)
from .errors import (
    AmbiguousColumn,
    InvalidAggregate,
    UnknownColumn,
    UnsupportedStatement,
)
from .expr import (
    AggregateExpr,
    Between,
    BinaryExpr,
    Column,
    Expr,
    FunctionCall,
    In,
    IsNotNull,
    IsNull,
    Like,
    Literal,
    NotIn,
    NotLike,
    UnaryExpr,
    Wildcard,
    contains_aggregate,
)
from .schema_provider import SchemaProvider

# A scope maps an alias (or table name if unaliased) to the ordered list of
# columns that alias exposes. We use a dict so lookups during column
# resolution are O(1).
Scope = dict[str, list[str]]


def plan(ast: Statement, schema: SchemaProvider) -> P.LogicalPlan:
    """Translate a single statement into a LogicalPlan.

    Dispatches on statement type. The per-statement helpers handle column
    resolution and clause-to-node wrapping.
    """
    match ast:
        case SelectStmt():
            return _plan_select(ast, schema)
        case InsertValuesStmt():
            return _plan_insert(ast, schema)
        case UpdateStmt():
            return _plan_update(ast, schema)
        case DeleteStmt():
            return _plan_delete(ast, schema)
        case CreateTableStmt():
            return _plan_create_table(ast)
        case DropTableStmt():
            return _plan_drop_table(ast)
    # Exhaustiveness — every Statement variant is matched above.
    raise UnsupportedStatement(kind=type(ast).__name__)


def plan_all(asts: list[Statement], schema: SchemaProvider) -> list[P.LogicalPlan]:
    """Translate a list of statements. Short helper for multi-statement scripts."""
    return [plan(a, schema) for a in asts]


# --------------------------------------------------------------------------
# SELECT planning
# --------------------------------------------------------------------------


def _plan_select(stmt: SelectStmt, schema: SchemaProvider) -> P.LogicalPlan:
    # 1. Build the scan / join tree from FROM + JOINs, and the column scope
    #    it exposes. Column resolution later qualifies bare references
    #    against this scope.
    tree, scope = _build_from_tree(stmt.from_, stmt.joins, schema)

    # 2. WHERE — single Filter above the scan tree. Aggregates are forbidden
    #    inside WHERE (SQL forbids this — WHERE runs per-row before grouping).
    if stmt.where is not None:
        where = _resolve(stmt.where, scope)
        if contains_aggregate(where):
            raise InvalidAggregate(message="aggregate function not allowed in WHERE clause")
        tree = P.Filter(input=tree, predicate=where)

    # 3. GROUP BY + aggregates — zero-or-one Aggregate node. We emit one if:
    #    (a) GROUP BY is non-empty, or
    #    (b) any SELECT item or HAVING predicate uses an aggregate.
    resolved_items = tuple(
        P.ProjectionItem(expr=_resolve(it.expr, scope), alias=_derive_alias(it))
        for it in stmt.items
    )
    having = _resolve(stmt.having, scope) if stmt.having is not None else None
    order_by = tuple(
        P.SortKey(
            expr=_resolve(k.expr, scope),
            descending=k.descending,
            nulls_first=k.nulls_first,
        )
        for k in stmt.order_by
    )

    has_agg_in_select = any(contains_aggregate(i.expr) for i in resolved_items)
    has_agg_in_having = having is not None and contains_aggregate(having)
    has_agg_in_order = any(contains_aggregate(k.expr) for k in order_by)
    if stmt.group_by or has_agg_in_select or has_agg_in_having or has_agg_in_order:
        group_by = tuple(_resolve(g, scope) for g in stmt.group_by)
        aggregates = _collect_aggregates(resolved_items, having, order_by)
        tree = P.Aggregate(input=tree, group_by=group_by, aggregates=aggregates)

    # 4. HAVING — Filter-like node, but distinct from Filter for the
    #    optimizer's sake (see Having docstring in plan.py).
    if having is not None:
        tree = P.Having(input=tree, predicate=having)

    # 5. Projection — always present. A SELECT always has at least one item.
    tree = P.Project(input=tree, items=resolved_items)

    # 6. DISTINCT — simple wrapper.
    if stmt.distinct:
        tree = P.Distinct(input=tree)

    # 7. ORDER BY — single Sort node.
    if order_by:
        tree = P.Sort(input=tree, keys=order_by)

    # 8. LIMIT / OFFSET — only emit if either is set; a Limit with both None
    #    would be a no-op.
    if stmt.limit is not None and (stmt.limit.count is not None or stmt.limit.offset is not None):
        tree = P.Limit(input=tree, count=stmt.limit.count, offset=stmt.limit.offset)

    return tree


def _build_from_tree(
    root: TableRef,
    joins: tuple[JoinClause, ...],
    schema: SchemaProvider,
) -> tuple[P.LogicalPlan, Scope]:
    """Build a nested Join tree out of FROM + JOIN clauses. Return tree + scope."""
    # Ensure the root table exists.
    root_cols = schema.columns(root.table)
    scope: Scope = {}
    _add_to_scope(scope, root.alias or root.table, root_cols)
    tree: P.LogicalPlan = P.Scan(table=root.table, alias=root.alias)

    for j in joins:
        right_cols = schema.columns(j.right.table)
        _add_to_scope(scope, j.right.alias or j.right.table, right_cols)
        right_scan: P.LogicalPlan = P.Scan(table=j.right.table, alias=j.right.alias)
        # ON clause is resolved against the merged scope so it can reference
        # columns from both sides.
        condition = _resolve(j.on, scope) if j.on is not None else None
        _validate_join(j, condition)
        tree = P.Join(
            left=tree,
            right=right_scan,
            kind=j.kind,
            condition=condition,
        )
    return tree, scope


def _validate_join(clause: JoinClause, condition: Expr | None) -> None:
    """Cross joins must have no condition; every other join kind must have one."""
    if clause.kind == JoinKind.CROSS:
        if condition is not None:
            raise UnsupportedStatement(kind="CROSS JOIN with ON condition")
    else:
        if condition is None:
            raise UnsupportedStatement(kind=f"{clause.kind} JOIN without ON condition")


def _add_to_scope(scope: Scope, alias: str, columns: list[str]) -> None:
    """Register a table's columns under the given alias in the scope.

    Duplicate aliases are a planner error (same alias used twice in FROM)
    but we treat it as UnsupportedStatement rather than a dedicated variant
    — it's a niche failure mode, not a common one worth its own error type.
    """
    if alias in scope:
        raise UnsupportedStatement(kind=f"duplicate alias: {alias}")
    scope[alias] = columns


# --------------------------------------------------------------------------
# Expression resolution
# --------------------------------------------------------------------------


def _resolve(expr: Expr, scope: Scope) -> Expr:
    """Qualify bare :class:`Column` references against ``scope``.

    Returns a new expression tree (because expressions are frozen). For
    expressions that contain no columns, this is equivalent to returning
    the input unchanged.
    """
    match expr:
        case Literal() | Wildcard():
            return expr
        case Column(table, col):
            return _resolve_column(table, col, scope)
        case BinaryExpr(op, left, right):
            return BinaryExpr(op=op, left=_resolve(left, scope), right=_resolve(right, scope))
        case UnaryExpr(op, operand):
            return UnaryExpr(op=op, operand=_resolve(operand, scope))
        case FunctionCall(name, args):
            new_args = tuple(
                a if a.star or a.value is None
                else type(a)(star=False, value=_resolve(a.value, scope))
                for a in args
            )
            return FunctionCall(name=name, args=new_args)
        case IsNull(operand):
            return IsNull(operand=_resolve(operand, scope))
        case IsNotNull(operand):
            return IsNotNull(operand=_resolve(operand, scope))
        case Between(operand, low, high):
            return Between(
                operand=_resolve(operand, scope),
                low=_resolve(low, scope),
                high=_resolve(high, scope),
            )
        case In(operand, values):
            return In(
                operand=_resolve(operand, scope),
                values=tuple(_resolve(v, scope) for v in values),
            )
        case NotIn(operand, values):
            return NotIn(
                operand=_resolve(operand, scope),
                values=tuple(_resolve(v, scope) for v in values),
            )
        case Like(operand, pattern):
            return Like(operand=_resolve(operand, scope), pattern=pattern)
        case NotLike(operand, pattern):
            return NotLike(operand=_resolve(operand, scope), pattern=pattern)
        case AggregateExpr(func, arg, distinct):
            if arg.star or arg.value is None:
                return expr
            new_arg = type(arg)(star=False, value=_resolve(arg.value, scope))
            return AggregateExpr(func=func, arg=new_arg, distinct=distinct)
    raise AmbiguousColumn(column="<internal>", tables=[])  # unreachable


def _resolve_column(table: str | None, col: str, scope: Scope) -> Column:
    if table is not None:
        if table not in scope:
            raise UnknownColumn(table=table, column=col)
        if col not in scope[table]:
            raise UnknownColumn(table=table, column=col)
        return Column(table=table, col=col)
    # Bare column reference — find which tables have it.
    owners = [t for t, cols in scope.items() if col in cols]
    if not owners:
        raise UnknownColumn(table=None, column=col)
    if len(owners) > 1:
        raise AmbiguousColumn(column=col, tables=owners)
    return Column(table=owners[0], col=col)


# --------------------------------------------------------------------------
# Aggregate collection
# --------------------------------------------------------------------------


def _collect_aggregates(
    items: tuple[P.ProjectionItem, ...],
    having: Expr | None,
    order_by: tuple[P.SortKey, ...],
) -> tuple[P.AggregateItem, ...]:
    """Pull AggregateExprs out of SELECT / HAVING / ORDER BY into Aggregate.aggregates.

    We don't rewrite the source expressions to reference the aggregated
    result by alias — codegen can navigate the tree and emit the right
    opcodes. This keeps the planner simple; the optimizer can de-duplicate
    later.
    """
    seen: list[tuple[P.AggregateItem, str]] = []  # (item, alias) for de-dup
    counter = 0

    def collect_in(expr: Expr) -> None:
        nonlocal counter
        match expr:
            case AggregateExpr(func, arg, distinct):
                alias = f"_agg_{counter}"
                counter += 1
                seen.append((
                    P.AggregateItem(func=func, arg=arg, alias=alias, distinct=distinct),
                    alias,
                ))
            case BinaryExpr(_, left, right):
                collect_in(left)
                collect_in(right)
            case UnaryExpr(_, operand):
                collect_in(operand)
            case FunctionCall(_, args):
                for a in args:
                    if a.value is not None:
                        collect_in(a.value)
            case IsNull(operand) | IsNotNull(operand):
                collect_in(operand)
            case Between(operand, low, high):
                collect_in(operand)
                collect_in(low)
                collect_in(high)
            case In(operand, values) | NotIn(operand, values):
                collect_in(operand)
                for v in values:
                    collect_in(v)
            case Like(operand, _) | NotLike(operand, _):
                collect_in(operand)
            case _:
                pass

    for it in items:
        collect_in(it.expr)
    if having is not None:
        collect_in(having)
    for k in order_by:
        collect_in(k.expr)
    return tuple(item for item, _ in seen)


# --------------------------------------------------------------------------
# Alias derivation
# --------------------------------------------------------------------------


def _derive_alias(item: SelectItem) -> str | None:
    """Preserve the user's explicit alias. For bare column refs, use the column name.

    Everything else returns None; codegen assigns a positional name
    (``column_1``, ``column_2``) when emitting the result schema.
    """
    if item.alias is not None:
        return item.alias
    if isinstance(item.expr, Column):
        return item.expr.col
    if isinstance(item.expr, FunctionCall):
        return item.expr.name.lower()
    if isinstance(item.expr, AggregateExpr):
        return item.expr.func.value.lower()
    return None


# --------------------------------------------------------------------------
# DML / DDL planning
# --------------------------------------------------------------------------


def _plan_insert(stmt: InsertValuesStmt, schema: SchemaProvider) -> P.LogicalPlan:
    # Validate the target table and, if a column list is given, each column.
    table_cols = schema.columns(stmt.table)
    if stmt.columns is not None:
        for c in stmt.columns:
            if c not in table_cols:
                raise UnknownColumn(table=stmt.table, column=c)
    return P.Insert(
        table=stmt.table,
        columns=stmt.columns,
        source=P.InsertSource(values=stmt.rows),
    )


def _plan_update(stmt: UpdateStmt, schema: SchemaProvider) -> P.LogicalPlan:
    cols = schema.columns(stmt.table)
    scope: Scope = {stmt.table: cols}
    for a in stmt.assignments:
        if a.column not in cols:
            raise UnknownColumn(table=stmt.table, column=a.column)
    resolved_assignments = tuple(
        P.Assignment(column=a.column, value=_resolve(a.value, scope))
        for a in stmt.assignments
    )
    predicate = _resolve(stmt.where, scope) if stmt.where is not None else None
    if predicate is not None and contains_aggregate(predicate):
        raise InvalidAggregate(message="aggregate function not allowed in UPDATE WHERE clause")
    return P.Update(
        table=stmt.table,
        assignments=resolved_assignments,
        predicate=predicate,
    )


def _plan_delete(stmt: DeleteStmt, schema: SchemaProvider) -> P.LogicalPlan:
    cols = schema.columns(stmt.table)
    scope: Scope = {stmt.table: cols}
    predicate = _resolve(stmt.where, scope) if stmt.where is not None else None
    if predicate is not None and contains_aggregate(predicate):
        raise InvalidAggregate(message="aggregate function not allowed in DELETE WHERE clause")
    return P.Delete(table=stmt.table, predicate=predicate)


def _plan_create_table(stmt: CreateTableStmt) -> P.LogicalPlan:
    # DDL needs no schema lookup — the table doesn't exist yet.
    return P.CreateTable(
        table=stmt.table,
        columns=stmt.columns,
        if_not_exists=stmt.if_not_exists,
    )


def _plan_drop_table(stmt: DropTableStmt) -> P.LogicalPlan:
    return P.DropTable(table=stmt.table, if_exists=stmt.if_exists)


# Keep imports from being marked unused — these are re-exported for callers.
_ = AstAssignment
_ = AstSortKey
