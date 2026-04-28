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
    AlterTableStmt,
    BeginStmt,
    CommitStmt,
    CreateIndexStmt,
    CreateTableStmt,
    DeleteStmt,
    DerivedTableRef,
    DropIndexStmt,
    DropTableStmt,
    ExceptStmt,
    InsertSelectStmt,
    InsertValuesStmt,
    IntersectStmt,
    JoinClause,
    JoinKind,
    RecursiveCTERef,
    RollbackStmt,
    SelectItem,
    SelectStmt,
    Statement,
    TableRef,
    UnionStmt,
    UpdateStmt,
)
from .ast import (
    Assignment as AstAssignment,
)
from .ast import (
    SortKey as AstSortKey,
)
from .errors import (
    AmbiguousColumn,
    InternalError,
    InvalidAggregate,
    UnknownColumn,
    UnsupportedStatement,
)
from .expr import (
    AggregateExpr,
    Between,
    BinaryExpr,
    BinaryOp,
    CaseExpr,
    Column,
    ExistsSubquery,
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
    WindowFuncExpr,
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
        case UnionStmt():
            return _plan_union(ast, schema)
        case IntersectStmt():
            return _plan_intersect(ast, schema)
        case ExceptStmt():
            return _plan_except(ast, schema)
        case InsertValuesStmt():
            return _plan_insert(ast, schema)
        case InsertSelectStmt():
            return _plan_insert_select(ast, schema)
        case UpdateStmt():
            return _plan_update(ast, schema)
        case DeleteStmt():
            return _plan_delete(ast, schema)
        case CreateTableStmt():
            return _plan_create_table(ast)
        case DropTableStmt():
            return _plan_drop_table(ast)
        case AlterTableStmt():
            return _plan_alter_table(ast)
        case CreateIndexStmt():
            return _plan_create_index(ast)
        case DropIndexStmt():
            return _plan_drop_index(ast)
        case BeginStmt():
            return P.Begin()
        case CommitStmt():
            return P.Commit()
        case RollbackStmt():
            return P.Rollback()
    # Exhaustiveness — every Statement variant is matched above.
    raise UnsupportedStatement(kind=type(ast).__name__)


def plan_all(asts: list[Statement], schema: SchemaProvider) -> list[P.LogicalPlan]:
    """Translate a list of statements. Short helper for multi-statement scripts."""
    return [plan(a, schema) for a in asts]


# --------------------------------------------------------------------------
# SELECT planning
# --------------------------------------------------------------------------


def _plan_select(
    stmt: SelectStmt,
    schema: SchemaProvider,
    *,
    working_set: tuple[str, tuple[str, ...]] | None = None,
) -> P.LogicalPlan:
    # 1. Build the scan / join tree from FROM + JOINs, and the column scope
    #    it exposes. Column resolution later qualifies bare references
    #    against this scope.
    tree, scope = _build_from_tree(stmt.from_, stmt.joins, schema, working_set=working_set)

    # 2. WHERE — single Filter above the scan tree. Aggregates are forbidden
    #    inside WHERE (SQL forbids this — WHERE runs per-row before grouping).
    if stmt.where is not None:
        where = _resolve(stmt.where, scope, schema)
        if contains_aggregate(where):
            raise InvalidAggregate(message="aggregate function not allowed in WHERE clause")
        # IX-6: If the WHERE predicate can be served by a B-tree index on the
        # scan's base table, substitute Filter(Scan) with IndexScan.  This
        # only applies when the tree is a bare Scan (not a Join or
        # DerivedTable) — composite sources can't be accelerated this way.
        if isinstance(tree, P.Scan):
            idx_node = _try_index_scan(where, tree, schema)
            tree = idx_node if idx_node is not None else P.Filter(input=tree, predicate=where)
        else:
            tree = P.Filter(input=tree, predicate=where)

    # 3. GROUP BY + aggregates — zero-or-one Aggregate node. We emit one if:
    #    (a) GROUP BY is non-empty, or
    #    (b) any SELECT item or HAVING predicate uses an aggregate.
    resolved_items = tuple(
        P.ProjectionItem(expr=_resolve(it.expr, scope, schema), alias=_derive_alias(it))
        for it in stmt.items
    )
    having = _resolve(stmt.having, scope, schema) if stmt.having is not None else None
    order_by = tuple(
        P.SortKey(
            expr=_resolve(k.expr, scope, schema),
            descending=k.descending,
            nulls_first=k.nulls_first,
        )
        for k in stmt.order_by
    )

    has_agg_in_select = any(contains_aggregate(i.expr) for i in resolved_items)
    has_agg_in_having = having is not None and contains_aggregate(having)
    has_agg_in_order = any(contains_aggregate(k.expr) for k in order_by)
    if stmt.group_by or has_agg_in_select or has_agg_in_having or has_agg_in_order:
        group_by = tuple(_resolve(g, scope, schema) for g in stmt.group_by)
        aggregates = _collect_aggregates(resolved_items, having, order_by)
        tree = P.Aggregate(input=tree, group_by=group_by, aggregates=aggregates)

    # 4. HAVING — Filter-like node, but distinct from Filter for the
    #    optimizer's sake (see Having docstring in plan.py).
    if having is not None:
        tree = P.Having(input=tree, predicate=having)

    # 5. Window functions — if any SELECT item is a WindowFuncExpr, emit a
    #    WindowAgg node instead of a plain Project.
    #
    #    Design: the inner Project materialises all columns needed by the
    #    window expressions (non-window SELECT items + any extra dependency
    #    columns).  WindowAgg post-processes the materialised result buffer to
    #    append the window function output columns.
    win_items = [it for it in resolved_items if isinstance(it.expr, WindowFuncExpr)]
    if win_items:
        specs: list[P.WindowFuncSpec] = []
        for i, it in enumerate(win_items):
            wf: WindowFuncExpr = it.expr  # type: ignore[assignment]
            alias = it.alias or f"window_{i + 1}"
            specs.append(
                P.WindowFuncSpec(
                    func=wf.func,
                    arg_expr=wf.arg,
                    partition_by=wf.partition_by,
                    order_by=wf.order_by,
                    alias=alias,
                )
            )

        non_win_items = [it for it in resolved_items if not isinstance(it.expr, WindowFuncExpr)]

        # Track which (table, col) pairs are already covered by the non-window
        # projection so we don't add redundant extra columns.
        covered: set[tuple[str | None, str]] = {
            (it.expr.table, it.expr.col)
            for it in non_win_items
            if isinstance(it.expr, Column)
        }
        extra: list[P.ProjectionItem] = []
        for spec in specs:
            for dep in _win_spec_columns(spec):
                key = (dep.table, dep.col)
                if key not in covered:
                    covered.add(key)
                    extra.append(P.ProjectionItem(expr=dep, alias=dep.col))

        inner_items = tuple(non_win_items) + tuple(extra)
        inner_projection = P.Project(input=tree, items=inner_items)

        # Final output: non-window item names first, then window alias names.
        non_win_out = tuple(
            it.alias if it.alias is not None
            else (it.expr.col if isinstance(it.expr, Column) else f"column_{i + 1}")
            for i, it in enumerate(non_win_items)
        )
        output_cols = non_win_out + tuple(s.alias for s in specs)

        tree = P.WindowAgg(
            input=inner_projection,
            specs=tuple(specs),
            output_cols=output_cols,
        )
    else:
        # 5 (normal path). Projection — always present.
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


def _win_spec_columns(spec: P.WindowFuncSpec) -> list[Column]:
    """Collect all Column references from a WindowFuncSpec's expressions."""
    from .expr import collect_columns as _cc
    cols: list[Column] = []
    if spec.arg_expr is not None:
        cols.extend(_cc(spec.arg_expr))
    for e in spec.partition_by:
        cols.extend(_cc(e))
    for e, _ in spec.order_by:
        cols.extend(_cc(e))
    return cols


def _build_from_tree(
    root: TableRef | DerivedTableRef | RecursiveCTERef,
    joins: tuple[JoinClause, ...],
    schema: SchemaProvider,
    *,
    working_set: tuple[str, tuple[str, ...]] | None = None,
) -> tuple[P.LogicalPlan, Scope]:
    """Build a nested Join tree out of FROM + JOIN clauses. Return tree + scope.

    ``working_set`` is ``(cte_name, columns)`` when planning the recursive body
    of a WITH RECURSIVE CTE — any ``TableRef(cte_name)`` becomes a
    :class:`P.WorkingSetScan` rather than a backend :class:`P.Scan`.
    """
    scope: Scope = {}
    if isinstance(root, RecursiveCTERef):
        # Plan anchor normally; derive its output schema.
        anchor_plan = _plan_select(root.anchor, schema)
        anchor_cols = _output_columns(anchor_plan)
        # Plan recursive with working_set so the self-reference becomes WorkingSetScan.
        ws_alias = root.alias or root.name
        recursive_plan = _plan_select(
            root.recursive, schema, working_set=(root.name, anchor_cols)
        )
        _add_to_scope(scope, ws_alias, list(anchor_cols))
        tree: P.LogicalPlan = P.RecursiveCTE(
            anchor=anchor_plan,
            recursive=recursive_plan,
            alias=ws_alias,
            columns=anchor_cols,
            union_all=root.union_all,
        )
    elif isinstance(root, DerivedTableRef):
        inner_plan = _plan_select(root.select, schema)
        cols = _output_columns(inner_plan)
        _add_to_scope(scope, root.alias, list(cols))
        tree = P.DerivedTable(
            query=inner_plan, alias=root.alias, columns=cols
        )
    else:
        # Plain TableRef — check if it's a working-set self-reference first.
        if working_set is not None and root.table == working_set[0]:
            ws_alias = root.alias or root.table
            _add_to_scope(scope, ws_alias, list(working_set[1]))
            tree = P.WorkingSetScan(alias=ws_alias, columns=working_set[1])
        else:
            root_cols = schema.columns(root.table)
            _add_to_scope(scope, root.alias or root.table, root_cols)
            tree = P.Scan(table=root.table, alias=root.alias)

    for j in joins:
        if isinstance(j.right, DerivedTableRef):
            inner_plan = _plan_select(j.right.select, schema)
            cols = _output_columns(inner_plan)
            _add_to_scope(scope, j.right.alias, list(cols))
            right_node: P.LogicalPlan = P.DerivedTable(
                query=inner_plan, alias=j.right.alias, columns=cols
            )
        elif isinstance(j.right, TableRef) and (
            working_set is not None and j.right.table == working_set[0]
        ):
            # Working-set self-reference in a JOIN (e.g. INNER JOIN cte ON ...)
            ws_alias = j.right.alias or j.right.table
            _add_to_scope(scope, ws_alias, list(working_set[1]))
            right_node = P.WorkingSetScan(alias=ws_alias, columns=working_set[1])
        else:
            right_cols = schema.columns(j.right.table)  # type: ignore[union-attr]
            _add_to_scope(scope, j.right.alias or j.right.table, right_cols)  # type: ignore[union-attr]
            right_node = P.Scan(table=j.right.table, alias=j.right.alias)  # type: ignore[union-attr]
        # ON clause is resolved against the merged scope so it can reference
        # columns from both sides.
        condition = _resolve(j.on, scope, schema) if j.on is not None else None
        _validate_join(j, condition)
        tree = P.Join(
            left=tree,
            right=right_node,
            kind=j.kind,
            condition=condition,
        )
    return tree, scope


def _output_columns(plan: P.LogicalPlan) -> tuple[str, ...]:
    """Return the ordered output column names of a finished plan tree.

    Used to compute the schema of a derived table (subquery in FROM) at
    planning time, so the outer query can resolve column references against
    it.  Walks downward through transparent wrapper nodes (Sort, Limit,
    Distinct, Having) until it reaches a Project node whose items carry
    explicit aliases or column names.

    Raises :class:`UnsupportedStatement` for ``SELECT *`` inside a derived
    table — we can't know the column list without executing the query.
    """
    # Walk through purely decorative wrapper nodes that don't change columns.
    node = plan
    while isinstance(node, (P.Sort, P.Limit, P.Distinct, P.Having)):
        node = node.input  # type: ignore[union-attr]

    if isinstance(node, P.Project):
        cols: list[str] = []
        for i, item in enumerate(node.items, start=1):
            if item.alias is not None:
                cols.append(item.alias)
            elif isinstance(item.expr, P.ProjectionItem):
                # Shouldn't happen, but guard anyway.
                cols.append(f"column_{i}")
            else:
                # No alias — we use the expr-level alias from _derive_alias.
                # _derive_alias already ran and stored the result in item.alias,
                # so reaching here means the item has no natural name.
                cols.append(f"column_{i}")
        return tuple(cols)

    raise UnsupportedStatement(
        kind="SELECT * in derived table (cannot infer column names without schema)"
    )


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


def _resolve(
    expr: Expr,
    scope: Scope,
    schema: SchemaProvider | None = None,
) -> Expr:
    """Qualify bare :class:`Column` references against ``scope``.

    Returns a new expression tree (because expressions are frozen). For
    expressions that contain no columns, this is equivalent to returning
    the input unchanged.

    ``schema`` is only required when the expression tree may contain
    :class:`~sql_planner.expr.ExistsSubquery` nodes — the planner needs it
    to plan the inner SELECT independently.  All other expression types ignore
    this parameter.
    """
    match expr:
        case Literal() | Wildcard():
            return expr
        case Column(table, col):
            return _resolve_column(table, col, scope)
        case BinaryExpr(op, left, right):
            return BinaryExpr(
                op=op,
                left=_resolve(left, scope, schema),
                right=_resolve(right, scope, schema),
            )
        case UnaryExpr(op, operand):
            return UnaryExpr(op=op, operand=_resolve(operand, scope, schema))
        case FunctionCall(name, args):
            new_args = tuple(
                a if a.star or a.value is None
                else type(a)(star=False, value=_resolve(a.value, scope, schema))
                for a in args
            )
            return FunctionCall(name=name, args=new_args)
        case IsNull(operand):
            return IsNull(operand=_resolve(operand, scope, schema))
        case IsNotNull(operand):
            return IsNotNull(operand=_resolve(operand, scope, schema))
        case Between(operand, low, high):
            return Between(
                operand=_resolve(operand, scope, schema),
                low=_resolve(low, scope, schema),
                high=_resolve(high, scope, schema),
            )
        case In(operand, values):
            return In(
                operand=_resolve(operand, scope, schema),
                values=tuple(_resolve(v, scope, schema) for v in values),
            )
        case NotIn(operand, values):
            return NotIn(
                operand=_resolve(operand, scope, schema),
                values=tuple(_resolve(v, scope, schema) for v in values),
            )
        case Like(operand, pattern):
            return Like(operand=_resolve(operand, scope, schema), pattern=pattern)
        case NotLike(operand, pattern):
            return NotLike(operand=_resolve(operand, scope, schema), pattern=pattern)
        case AggregateExpr(func, arg, distinct):
            if arg.star or arg.value is None:
                return expr
            new_arg = type(arg)(star=False, value=_resolve(arg.value, scope, schema))
            return AggregateExpr(func=func, arg=new_arg, distinct=distinct)
        case CaseExpr(whens, else_):
            return CaseExpr(
                whens=tuple(
                    (_resolve(cond, scope, schema), _resolve(result, scope, schema))
                    for cond, result in whens
                ),
                else_=_resolve(else_, scope, schema) if else_ is not None else None,
            )
        case ExistsSubquery(query=stmt):
            # Plan the inner SELECT independently using the same schema but
            # without sharing the outer scope — no correlated subqueries.
            # After this call, query holds a LogicalPlan ready for codegen.
            if schema is None:
                raise InternalError(message="schema required to plan EXISTS subquery")
            inner_plan = _plan_select(stmt, schema)  # type: ignore[arg-type]
            return ExistsSubquery(query=inner_plan)
        case WindowFuncExpr(func, arg, partition_by, order_by):
            new_arg = _resolve(arg, scope, schema) if arg is not None else None
            new_partition_by = tuple(_resolve(e, scope, schema) for e in partition_by)
            new_order_by = tuple(
                (_resolve(e, scope, schema), desc) for e, desc in order_by
            )
            return WindowFuncExpr(
                func=func,
                arg=new_arg,
                partition_by=new_partition_by,
                order_by=new_order_by,
            )
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
            case CaseExpr(whens, else_):
                for cond, result in whens:
                    collect_in(cond)
                    collect_in(result)
                if else_ is not None:
                    collect_in(else_)
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
        P.Assignment(column=a.column, value=_resolve(a.value, scope, schema))
        for a in stmt.assignments
    )
    predicate = _resolve(stmt.where, scope, schema) if stmt.where is not None else None
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
    predicate = _resolve(stmt.where, scope, schema) if stmt.where is not None else None
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


def _plan_alter_table(stmt: AlterTableStmt) -> P.LogicalPlan:
    return P.AlterTable(table=stmt.table, column=stmt.column)


def _plan_create_index(stmt: CreateIndexStmt) -> P.LogicalPlan:
    """CREATE INDEX — no schema lookup needed (backend validates at execution)."""
    return P.CreateIndex(
        name=stmt.name,
        table=stmt.table,
        columns=stmt.columns,
        unique=stmt.unique,
        if_not_exists=stmt.if_not_exists,
    )


def _plan_drop_index(stmt: DropIndexStmt) -> P.LogicalPlan:
    """DROP INDEX — no schema lookup needed (backend validates at execution)."""
    return P.DropIndex(name=stmt.name, if_exists=stmt.if_exists)


# --------------------------------------------------------------------------
# Set-operation planning (UNION / INTERSECT / EXCEPT)
# --------------------------------------------------------------------------


def _plan_union(stmt: UnionStmt, schema: SchemaProvider) -> P.LogicalPlan:
    """Plan UNION [ALL]: recurse into both sub-queries, wrap in Union.

    The left side may itself be a set-operation statement (e.g. when the
    user writes ``A UNION B UNION C`` the adapter builds
    ``UnionStmt(UnionStmt(A, B), C)``).  We therefore dispatch through the
    top-level ``plan()`` function rather than assuming ``_plan_select``.
    """
    left = plan(stmt.left, schema)
    right = _plan_select(stmt.right, schema)
    return P.Union(left=left, right=right, all=stmt.all)


def _plan_intersect(stmt: IntersectStmt, schema: SchemaProvider) -> P.LogicalPlan:
    """Plan INTERSECT [ALL]: recurse into both sub-queries, wrap in Intersect.

    Left side may be another set-operation statement; dispatch through
    ``plan()`` to handle chaining.
    """
    left = plan(stmt.left, schema)
    right = _plan_select(stmt.right, schema)
    return P.Intersect(left=left, right=right, all=stmt.all)


def _plan_except(stmt: ExceptStmt, schema: SchemaProvider) -> P.LogicalPlan:
    """Plan EXCEPT [ALL]: recurse into both sub-queries, wrap in Except.

    Left side may be another set-operation statement; dispatch through
    ``plan()`` to handle chaining.
    """
    left = plan(stmt.left, schema)
    right = _plan_select(stmt.right, schema)
    return P.Except(left=left, right=right, all=stmt.all)


# --------------------------------------------------------------------------
# INSERT … SELECT planning
# --------------------------------------------------------------------------


def _plan_insert_select(stmt: InsertSelectStmt, schema: SchemaProvider) -> P.LogicalPlan:
    """Plan INSERT INTO t (cols) SELECT …

    The target table and columns are validated the same way as INSERT VALUES.
    The sub-query is planned as a regular SELECT — column resolution runs
    against the *source* table(s), not the target table.
    """
    table_cols = schema.columns(stmt.table)
    if stmt.columns is not None:
        for c in stmt.columns:
            if c not in table_cols:
                raise UnknownColumn(table=stmt.table, column=c)
    sub_plan = _plan_select(stmt.select, schema)
    return P.Insert(
        table=stmt.table,
        columns=stmt.columns,
        source=P.InsertSource(query=sub_plan),
    )


# --------------------------------------------------------------------------
# IX-6 / IX-8: Index-scan selection
# --------------------------------------------------------------------------
#
# When the planner builds a Filter(Scan(t)) and the schema provider knows
# about indexes on ``t``, it tries to replace the node pair with an
# IndexScan.  The IndexScan materialises matching rows directly from the
# B-tree — no full scan needed.
#
# Supported predicate shapes per index column ``col``:
#
#   col = literal          equality          → lo=(v,), hi=(v,), both inclusive
#   col > literal          open lower bound  → lo=(v,) (excl), hi=None
#   col >= literal         closed lower      → lo=(v,) (incl), hi=None
#   col < literal          open upper bound  → lo=None, hi=(v,) (excl)
#   col <= literal         closed upper      → lo=None, hi=(v,) (incl)
#   literal < col          reversed GT       → lo=(literal,) (excl), hi=None
#   literal <= col         reversed GTE      → lo=(literal,) (incl), hi=None
#   literal > col          reversed LT       → lo=None, hi=(literal,) (excl)
#   literal >= col         reversed LTE      → lo=None, hi=(literal,) (incl)
#   col BETWEEN lo AND hi  closed range      → lo=(lo_lit,), hi=(hi_lit,)
#   pred1 AND pred2        compound: extract one half, other becomes residual
#
# Multi-column composite indexes (IX-8):
#
#   a = v1 AND b = v2      → lo=(v1,v2), hi=(v1,v2), both inclusive
#   a = v1 AND b > v2      → lo=(v1,v2) excl, hi=(v1,) incl
#   a = v1 AND b < v2      → lo=(v1,) incl, hi=(v1,v2) excl
#   a = v1 AND b BETWEEN l AND h  → lo=(v1,l), hi=(v1,h), both incl
#
# Any other predicate shape falls through to Filter(Scan).


class _MultiColBounds:
    """Result of multi-column index prefix matching for a predicate.

    ``matched_cols`` is the ordered list of index columns that were
    successfully bound against the predicate (always >= 1).  ``lo`` / ``hi``
    are tuples of bound values (one per matched column), or ``None`` for an
    unbounded side.  ``residual`` is any predicate fragment not consumed by
    the index.
    """
    __slots__ = ("matched_cols", "lo", "hi", "lo_inclusive", "hi_inclusive", "residual")

    def __init__(
        self,
        matched_cols: list[str],
        lo: tuple[object, ...] | None,
        hi: tuple[object, ...] | None,
        lo_inclusive: bool,
        hi_inclusive: bool,
        residual: Expr | None,
    ) -> None:
        self.matched_cols = matched_cols
        self.lo = lo
        self.hi = hi
        self.lo_inclusive = lo_inclusive
        self.hi_inclusive = hi_inclusive
        self.residual = residual


def _try_index_scan(
    predicate: Expr,
    scan: P.Scan,
    schema: SchemaProvider,
) -> P.IndexScan | None:
    """Try to replace ``Filter(Scan(scan.table))`` with an ``IndexScan``.

    Asks the schema provider for indexes on ``scan.table``.  For each index
    whose leading columns appear in ``predicate`` in a matchable position,
    computes a :class:`_MultiColBounds` result.  The *best* matching index
    (most predicate columns covered) wins.  Ties are broken by the order
    ``list_indexes`` returns indexes (deterministic).

    Returns ``None`` if no index can serve the predicate, or if the schema
    provider does not expose a ``list_indexes`` method.

    IX-8 change: ``_try_index_scan`` now evaluates *all* indexes and picks
    the one that covers the most predicate columns, rather than returning
    the first match.  A two-column composite index covering both ``a`` and
    ``b`` is preferred over a single-column index on ``a`` alone when the
    predicate has constraints on both ``a`` and ``b``.
    """
    list_indexes_fn = getattr(schema, "list_indexes", None)
    if list_indexes_fn is None:
        return None

    alias = scan.alias or scan.table
    indexes = list_indexes_fn(scan.table)

    best_n: int = 0
    best_node: P.IndexScan | None = None

    for idx in indexes:
        if not idx.columns:
            continue
        result = _extract_multi_column_bounds(predicate, alias, list(idx.columns))
        if result is None:
            continue
        n = len(result.matched_cols)
        if n > best_n:
            best_n = n
            best_node = P.IndexScan(
                table=scan.table,
                alias=scan.alias,
                index_name=idx.name,
                columns=tuple(result.matched_cols),
                lo=result.lo,
                hi=result.hi,
                lo_inclusive=result.lo_inclusive,
                hi_inclusive=result.hi_inclusive,
                residual=result.residual,
            )
    return best_node


# Return type: (lo_scalar, lo_inclusive, hi_scalar, hi_inclusive, residual) | None
# lo_scalar / hi_scalar are *single* SqlValue scalars, not tuples.
# _extract_multi_column_bounds wraps these into tuples for IndexScan.
_BoundsResult = tuple[object | None, bool, object | None, bool, Expr | None]


def _extract_multi_column_bounds(
    predicate: Expr,
    alias: str,
    index_cols: list[str],
) -> _MultiColBounds | None:
    """Extract multi-column range bounds for a composite index prefix.

    Walks ``index_cols`` in order, trying to bind each leading column to a
    constraint in ``predicate``.  Binding rules:

    - An **equality** constraint on column ``c`` (``c = literal``) is a
      *prefix-extending* match: after binding ``c``, we recurse into the
      residual predicate to try binding the next column.
    - A **range** constraint (GT, GTE, LT, LTE, BETWEEN) on column ``c``
      stops the prefix extension — no further columns can be added to the
      composite bound (subsequent columns are not constrained by range
      B-tree semantics).
    - If column ``c`` has no constraint, the whole attempt fails.

    Returns ``None`` if *no* column in ``index_cols`` can be matched.

    Examples (index columns ``["a", "b"]``)::

        predicate: a = 1 AND b = 2
          → _MultiColBounds(["a","b"], lo=(1,2), hi=(1,2), both incl, residual=None)

        predicate: a = 1 AND b > 5
          → _MultiColBounds(["a","b"], lo=(1,5) excl, hi=(1,) incl, residual=None)

        predicate: a = 1 AND b < 5
          → _MultiColBounds(["a","b"], lo=(1,) incl, hi=(1,5) excl, residual=None)

        predicate: a > 3
          → _MultiColBounds(["a"], lo=(3,) excl, hi=None, residual=None)

        predicate: a = 1
          → _MultiColBounds(["a"], lo=(1,) incl, hi=(1,) incl, residual=None)
    """
    if not index_cols:
        return None

    # ---- Step 1: extract bounds for the first column ---------------------
    result = _extract_index_bounds(predicate, alias, index_cols[0])
    if result is None:
        return None

    lo0, lo_incl0, hi0, hi_incl0, residual0 = result

    # ---- Step 2: determine whether the first column is an exact equality -
    # An equality match (lo == hi, both inclusive) is the only case that
    # allows extending the composite bound to the next column.
    is_eq = (
        lo0 is not None
        and hi0 is not None
        and lo0 == hi0
        and lo_incl0
        and hi_incl0
    )

    # ---- Step 3: single-column stop cases --------------------------------
    # Stop extending if:
    #   (a) the match is a range, not an equality  →  can't use next col
    #   (b) there is only one index column
    #   (c) the residual after extracting col 0 is None  →  nothing left to
    #       extract the next column from
    if not is_eq or len(index_cols) == 1 or residual0 is None:
        lo_t = (lo0,) if lo0 is not None else None
        hi_t = (hi0,) if hi0 is not None else None
        return _MultiColBounds(
            matched_cols=[index_cols[0]],
            lo=lo_t,
            hi=hi_t,
            lo_inclusive=lo_incl0,
            hi_inclusive=hi_incl0,
            residual=residual0,
        )

    # ---- Step 4: try to extend with the next column ----------------------
    eq_val = lo0   # lo0 == hi0 for an equality match
    next_result = _extract_multi_column_bounds(residual0, alias, index_cols[1:])

    if next_result is None:
        # The residual couldn't bind the next column — stay single-column.
        return _MultiColBounds(
            matched_cols=[index_cols[0]],
            lo=(eq_val,),
            hi=(eq_val,),
            lo_inclusive=True,
            hi_inclusive=True,
            residual=residual0,
        )

    # ---- Step 5: prepend eq_val to the next column's bounds --------------
    #
    # For composite B-tree prefix comparison, the backend uses
    # ``sort_key[:len(lo_sort)]``.  Therefore:
    #
    # • If next.lo is None (unbounded below for the next column), the
    #   composite lower bound is just ``(eq_val,)`` inclusive — start of the
    #   sub-tree where the first column equals eq_val.
    #
    # • If next.hi is None (unbounded above), the composite upper bound is
    #   ``(eq_val,)`` inclusive — end of the sub-tree.
    #
    # • Otherwise, prepend eq_val to get the full composite bound tuple.
    if next_result.lo is not None:
        new_lo: tuple[object, ...] | None = (eq_val,) + next_result.lo
        new_lo_incl = next_result.lo_inclusive
    else:
        new_lo = (eq_val,)
        new_lo_incl = True

    if next_result.hi is not None:
        new_hi: tuple[object, ...] | None = (eq_val,) + next_result.hi
        new_hi_incl = next_result.hi_inclusive
    else:
        new_hi = (eq_val,)
        new_hi_incl = True

    return _MultiColBounds(
        matched_cols=[index_cols[0]] + next_result.matched_cols,
        lo=new_lo,
        hi=new_hi,
        lo_inclusive=new_lo_incl,
        hi_inclusive=new_hi_incl,
        residual=next_result.residual,
    )


def _extract_index_bounds(
    predicate: Expr,
    alias: str,
    col: str,
) -> _BoundsResult | None:
    """Extract B-tree range bounds for column ``alias.col`` from ``predicate``.

    Returns a 5-tuple ``(lo, lo_inclusive, hi, hi_inclusive, residual)``
    when the predicate (or part of it) can be served by a range scan on
    ``col``.  The ``residual`` is the remaining predicate that must still be
    evaluated row-by-row after the index scan (may be ``None``).

    Returns ``None`` when the predicate cannot be used for an index scan.
    """

    def is_our_col(expr: Expr) -> bool:
        """True when *expr* is a Column reference to ``alias.col``."""
        return isinstance(expr, Column) and expr.table == alias and expr.col == col

    if isinstance(predicate, Between) and (
        is_our_col(predicate.operand)
        and isinstance(predicate.low, Literal)
        and isinstance(predicate.high, Literal)
    ):
            return (predicate.low.value, True, predicate.high.value, True, None)

    if isinstance(predicate, BinaryExpr):
        op = predicate.op
        lhs = predicate.left
        rhs = predicate.right

        # ---- Equality -------------------------------------------------------
        if op == BinaryOp.EQ:
            if is_our_col(lhs) and isinstance(rhs, Literal):
                return (rhs.value, True, rhs.value, True, None)
            if isinstance(lhs, Literal) and is_our_col(rhs):
                return (lhs.value, True, lhs.value, True, None)

        # ---- Simple range: col OP literal -----------------------------------
        elif op == BinaryOp.GT:
            if is_our_col(lhs) and isinstance(rhs, Literal):
                return (rhs.value, False, None, True, None)
            if isinstance(lhs, Literal) and is_our_col(rhs):
                # literal > col  →  col < literal
                return (None, True, lhs.value, False, None)

        elif op == BinaryOp.GTE:
            if is_our_col(lhs) and isinstance(rhs, Literal):
                return (rhs.value, True, None, True, None)
            if isinstance(lhs, Literal) and is_our_col(rhs):
                # literal >= col  →  col <= literal
                return (None, True, lhs.value, True, None)

        elif op == BinaryOp.LT:
            if is_our_col(lhs) and isinstance(rhs, Literal):
                return (None, True, rhs.value, False, None)
            if isinstance(lhs, Literal) and is_our_col(rhs):
                # literal < col  →  col > literal
                return (lhs.value, False, None, True, None)

        elif op == BinaryOp.LTE:
            if is_our_col(lhs) and isinstance(rhs, Literal):
                return (None, True, rhs.value, True, None)
            if isinstance(lhs, Literal) and is_our_col(rhs):
                # literal <= col  →  col >= literal
                return (lhs.value, True, None, True, None)

        # ---- Compound AND: try both halves ----------------------------------
        elif op == BinaryOp.AND:
            left_bounds = _extract_index_bounds(lhs, alias, col)
            if left_bounds is not None:
                lo, lo_incl, hi, hi_incl, inner_res = left_bounds
                residual = _combine_residuals(inner_res, rhs)
                return (lo, lo_incl, hi, hi_incl, residual)
            right_bounds = _extract_index_bounds(rhs, alias, col)
            if right_bounds is not None:
                lo, lo_incl, hi, hi_incl, inner_res = right_bounds
                residual = _combine_residuals(inner_res, lhs)
                return (lo, lo_incl, hi, hi_incl, residual)

    return None


def _combine_residuals(inner: Expr | None, outer: Expr) -> Expr | None:
    """Combine *inner* residual (from a sub-extraction) with *outer* sibling.

    When we extract bounds from one arm of an AND, the other arm becomes the
    residual.  If that other arm itself had an inner residual (from deeper
    nesting), we AND them together.

    ``None`` means no residual — just return *outer* unchanged.
    """
    if inner is None:
        return outer
    return BinaryExpr(op=BinaryOp.AND, left=inner, right=outer)


# Keep imports from being marked unused — these are re-exported for callers.
_ = AstAssignment
_ = AstSortKey
