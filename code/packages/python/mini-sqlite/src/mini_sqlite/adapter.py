"""
Parser-ASTNode → planner-Statement adapter
==========================================

The `sql-parser` package produces a generic ``ASTNode`` tree keyed by
grammar rule names (``select_stmt``, ``select_list``, ``expr``, ...). The
`sql-planner` package consumes a typed ``Statement`` tree (``SelectStmt``,
``InsertValuesStmt``, ``CreateTableStmt``, ...) whose shape matches a
compiler-textbook AST — no syntactic noise, just semantics.

This module is the single place in the pipeline that knows both shapes.
Everything above it sees only typed Statements; everything below sees
only generic ASTNodes.

The translation is a mostly-mechanical tree walk:

1. Descend to the ``statement`` node.
2. Dispatch by the grammar rule name of its sole child.
3. For each statement shape, extract the pieces we care about from the
   children list and construct the matching dataclass. Keywords, commas,
   and parentheses are skipped — the grammar has them for parse-time
   disambiguation, but they carry no semantic weight.

Expressions are translated by walking the `expr → or_expr → and_expr →
not_expr → comparison → additive → multiplicative → unary → primary`
precedence tower bottom-up. Each level either passes through (when its
only child is the next level) or builds a combining expression.

Placeholders (``?``) are preserved as a sentinel ``_Placeholder`` Literal;
the binding layer substitutes them with real values before planning.
"""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass
from typing import cast

from lang_parser import ASTNode
from lexer import Token
from sql_backend.schema import NO_DEFAULT
from sql_backend.schema import ColumnDef as BackendColumnDef
from sql_planner import (
    AggFunc,
    AggregateExpr,
    AlterTableStmt,
    Assignment,
    BeginStmt,
    Between,
    BinaryExpr,
    BinaryOp,
    CaseExpr,
    Column,
    CommitStmt,
    CreateIndexStmt,
    CreateTableStmt,
    CreateTriggerStmt,
    CreateViewStmt,
    DeleteStmt,
    DerivedTableRef,
    DropIndexStmt,
    DropTableStmt,
    DropTriggerStmt,
    DropViewStmt,
    ExceptStmt,
    ExistsSubquery,
    FuncArg,
    FunctionCall,
    In,
    InsertSelectStmt,
    InsertValuesStmt,
    InSubquery,
    IntersectStmt,
    IsNotNull,
    IsNull,
    JoinClause,
    JoinKind,
    Like,
    Limit,
    Literal,
    NotIn,
    NotInSubquery,
    NotLike,
    RecursiveCTERef,
    ReleaseSavepointStmt,
    RollbackStmt,
    RollbackToStmt,
    SavepointStmt,
    ScalarSubquery,
    SelectItem,
    SelectStmt,
    SortKey,
    Statement,
    TableRef,
    UnaryExpr,
    UnaryOp,
    UnionStmt,
    UpdateStmt,
    Wildcard,
    WindowFuncExpr,
)
from sql_planner.expr import Expr

from .errors import ProgrammingError

# --------------------------------------------------------------------------
# Placeholder sentinel. Retained inside Literal nodes until the binding
# layer replaces each one with a user-supplied value.
# --------------------------------------------------------------------------


@dataclass(frozen=True)
class _Placeholder:
    """Stand-in for an unbound ``?`` placeholder in the AST.

    We reuse ``Literal`` as the carrier so the tree stays shape-valid at
    the type level. The binding pass walks the tree, finds every literal
    whose ``value`` is a ``_Placeholder``, and rewrites it.
    """

    index: int  # 0-based position in the statement, left-to-right


# --------------------------------------------------------------------------
# Public entry point.
# --------------------------------------------------------------------------


def to_statement(
    ast: ASTNode,
    view_defs: dict[str, SelectStmt] | None = None,
) -> Statement:
    """Convert a parsed ``program`` ASTNode to a planner ``Statement``.

    The grammar's top rule is ``program = statement { ";" statement } [";"]``.
    We currently require exactly one statement per execute() call — matching
    both sqlite3's semantics and our own spec — so the driver slices on ``;``
    before calling us. Here we just walk down past ``program`` and
    ``statement`` to the actual statement node.

    ``view_defs`` maps each view name to its defining ``SelectStmt`` so that
    bare table references that name a view can be expanded inline, exactly
    like non-recursive CTEs.
    """
    prog = _child_node(ast, "program") if ast.rule_name != "program" else ast
    statement = _only_child_node(prog, "statement")
    return _stmt_dispatch(statement, view_defs=view_defs)


# --------------------------------------------------------------------------
# Statement dispatch.
# --------------------------------------------------------------------------


def _stmt_dispatch(
    stmt: ASTNode,
    view_defs: dict[str, SelectStmt] | None = None,
) -> Statement:
    # ``statement`` has exactly one child, which is the real statement node.
    inner = _single_child(stmt)
    if not isinstance(inner, ASTNode):
        raise ProgrammingError(f"unexpected statement shape: {inner}")
    match inner.rule_name:
        case "query_stmt":
            return _query_stmt(inner, view_defs=view_defs)
        case "select_stmt":
            # Legacy: old grammar emitted select_stmt directly under statement.
            return _select(inner, view_defs=view_defs)
        case "insert_stmt":
            return _insert(inner)
        case "replace_stmt":
            # REPLACE INTO t ... is syntactic sugar for INSERT OR REPLACE INTO t ...
            return _insert(inner, default_conflict="REPLACE")
        case "update_stmt":
            return _update(inner)
        case "delete_stmt":
            return _delete(inner)
        case "alter_table_stmt":
            return _alter_table(inner)
        case "create_table_stmt":
            return _create_table(inner)
        case "drop_table_stmt":
            return _drop_table(inner)
        case "create_index_stmt":
            return _create_index(inner)
        case "drop_index_stmt":
            return _drop_index(inner)
        case "create_view_stmt":
            return _create_view(inner)
        case "drop_view_stmt":
            return _drop_view(inner)
        case "create_trigger_stmt":
            return _create_trigger(inner)
        case "drop_trigger_stmt":
            return _drop_trigger(inner)
        case "begin_stmt":
            return BeginStmt()
        case "commit_stmt":
            return CommitStmt()
        case "rollback_stmt":
            return RollbackStmt()
        case "savepoint_stmt":
            return _savepoint(inner)
        case "release_stmt":
            return _release_savepoint(inner)
        case "rollback_to_stmt":
            return _rollback_to(inner)
    raise ProgrammingError(f"unsupported statement: {inner.rule_name}")


# --------------------------------------------------------------------------
# QUERY (SELECT + set operations via query_stmt).
# --------------------------------------------------------------------------


def _query_stmt(
    node: ASTNode,
    ctes: dict[str, SelectStmt | RecursiveCTERef] | None = None,
    view_defs: dict[str, SelectStmt] | None = None,
) -> Statement:
    """Translate ``query_stmt = [ with_clause ] select_stmt { set_op_clause }`` to a Statement.

    When a ``with_clause`` is present, each ``cte_def`` is parsed into a
    ``SelectStmt`` (non-recursive) or ``RecursiveCTERef`` (WITH RECURSIVE) and
    stored by name.  The resulting dict is passed into ``_select`` so that bare
    table references matching a CTE name are substituted at parse time.

    A ``query_stmt`` wraps a bare ``select_stmt`` with zero or more
    UNION/INTERSECT/EXCEPT tails.  When no tails are present this is
    equivalent to a plain SELECT; otherwise we build a left-associative
    set-operation tree.
    """
    # Accumulate CTEs: outer dict (if any) merged with any new WITH clause.
    active_ctes: dict[str, SelectStmt | RecursiveCTERef] = dict(ctes) if ctes else {}
    with_node = _maybe_child(node, "with_clause")
    if with_node is not None:
        # Check whether the WITH clause carries the RECURSIVE keyword.
        is_recursive = any(_is_keyword(c, "RECURSIVE") for c in with_node.children)

        for cte_node in _child_nodes(with_node, "cte_def"):
            name_tok = _first_token(cte_node, kind="NAME")
            if name_tok is None:
                raise ProgrammingError("cte_def: missing CTE name")
            cte_name = name_tok.value
            inner_q = _child_node(cte_node, "query_stmt")

            if is_recursive and _child_nodes(inner_q, "set_op_clause"):
                # Recursive CTE: body is "anchor UNION [ALL] recursive_step".
                # Parse anchor with the CTEs accumulated so far.
                anchor_node = _child_node(inner_q, "select_stmt")
                anchor_stmt = _select(anchor_node, ctes=active_ctes)

                # Parse the recursive step WITHOUT this CTE in active_ctes so
                # the self-reference stays as a plain TableRef.  The planner's
                # working_set mechanism converts it to WorkingSetScan.
                ctes_without_self = {k: v for k, v in active_ctes.items() if k != cte_name}
                set_op_nodes = _child_nodes(inner_q, "set_op_clause")
                union_all = True
                rec_stmt: SelectStmt | None = None
                for sop in set_op_nodes:
                    op, all_flag, right_sel_node = _set_op_clause(sop)
                    if op == "UNION":
                        union_all = all_flag
                        rec_stmt = _select(right_sel_node, ctes=ctes_without_self)
                if rec_stmt is None:
                    raise ProgrammingError(
                        f"RECURSIVE CTE '{cte_name}' must have a UNION [ALL] recursive step"
                    )
                active_ctes[cte_name] = RecursiveCTERef(
                    name=cte_name,
                    anchor=anchor_stmt,
                    recursive=rec_stmt,
                    union_all=union_all,
                )
            else:
                inner_stmt = _query_stmt(inner_q, ctes=active_ctes, view_defs=view_defs)
                if not isinstance(inner_stmt, SelectStmt):
                    raise ProgrammingError(
                        f"CTE '{cte_name}' body must be a plain SELECT, not a set operation"
                    )
                # Make this CTE visible to subsequent CTEs and the main query.
                active_ctes[cte_name] = inner_stmt

    select_node = _child_node(node, "select_stmt")
    left: Statement = _select(select_node, ctes=active_ctes, view_defs=view_defs)
    set_ops = _child_nodes(node, "set_op_clause")
    for op_node in set_ops:
        op, all_flag, right_select_node = _set_op_clause(op_node)
        right_stmt = _select(right_select_node, ctes=active_ctes, view_defs=view_defs)
        # Build a left-associative tree: after the first iteration ``left``
        # will already be a UnionStmt/IntersectStmt/ExceptStmt.  The AST
        # types accept any set-op stmt on the left side, and the planner
        # dispatches through plan() rather than _plan_select() for the left
        # operand, so chaining works correctly.
        if op == "UNION":
            left = UnionStmt(left=left, right=right_stmt, all=all_flag)  # type: ignore[arg-type]
        elif op == "INTERSECT":
            left = IntersectStmt(left=left, right=right_stmt, all=all_flag)  # type: ignore[arg-type]
        elif op == "EXCEPT":
            left = ExceptStmt(left=left, right=right_stmt, all=all_flag)  # type: ignore[arg-type]
    return left


def _set_op_clause(node: ASTNode) -> tuple[str, bool, ASTNode]:
    """Extract (operator_name, all_flag, select_stmt_node) from a set_op_clause."""
    op: str | None = None
    all_flag = False
    select_node: ASTNode | None = None
    for c in node.children:
        if isinstance(c, Token) and _token_type(c) == "KEYWORD":
            kw = c.value.upper()
            if kw in ("UNION", "INTERSECT", "EXCEPT"):
                op = kw
            elif kw == "ALL":
                all_flag = True
        elif isinstance(c, ASTNode) and c.rule_name == "select_stmt":
            select_node = c
    if op is None or select_node is None:
        raise ProgrammingError("malformed set_op_clause")
    return op, all_flag, select_node


# --------------------------------------------------------------------------
# SELECT.
# --------------------------------------------------------------------------


def _select(
    node: ASTNode,
    ctes: dict[str, SelectStmt | RecursiveCTERef] | None = None,
    view_defs: dict[str, SelectStmt] | None = None,
) -> SelectStmt:
    state = _PlaceholderCounter()

    distinct = _has_keyword_child(node, "DISTINCT")
    items = _select_list(_child_node(node, "select_list"), state)

    # FROM + JOINs — FROM is optional (SELECT 1, SELECT UPPER('x'), etc.).
    #
    # USING desugaring is now deferred to the planner (see JoinClause.using),
    # so we no longer need to track a "current left alias" here.  Each
    # join_clause node is translated independently; the planner's
    # _build_from_tree resolves USING columns from the accumulated scope.
    from_node = _maybe_child(node, "table_ref")
    if from_node is not None:
        from_ref = _table_ref(from_node, ctes=ctes, view_defs=view_defs)
        joins = tuple(
            _join_clause(c, state, ctes=ctes, view_defs=view_defs)
            for c in _child_nodes(node, "join_clause")
        )
    else:
        from_ref = None
        joins = ()

    # WHERE / GROUP BY / HAVING / ORDER BY / LIMIT — all optional.
    where = _maybe_expr(node, "where_clause", state, skip=1)
    group_by = _group_clause(_maybe_child(node, "group_clause"), state)
    having = _maybe_expr(node, "having_clause", state, skip=1)
    order_by = _order_clause(_maybe_child(node, "order_clause"), state)
    limit = _limit_clause(_maybe_child(node, "limit_clause"))

    return SelectStmt(
        from_=from_ref,
        items=items,
        joins=joins,
        where=where,
        group_by=group_by,
        having=having,
        order_by=order_by,
        limit=limit,
        distinct=distinct,
    )


def _select_list(node: ASTNode, state: _PlaceholderCounter) -> tuple[SelectItem, ...]:
    # select_list = STAR | select_item { "," select_item }
    if any(_is_token(c, type_="STAR") for c in node.children):
        return (SelectItem(expr=Wildcard()),)
    items = []
    for c in node.children:
        if isinstance(c, ASTNode) and c.rule_name == "select_item":
            items.append(_select_item(c, state))
    return tuple(items)


def _select_item(node: ASTNode, state: _PlaceholderCounter) -> SelectItem:
    # select_item = expr [ "AS" NAME ]
    expr = _expr(_child_node(node, "expr"), state)
    alias = None
    for i, c in enumerate(node.children):
        if _is_keyword(c, "AS") and i + 1 < len(node.children):
            nxt = node.children[i + 1]
            if isinstance(nxt, Token):
                alias = nxt.value
            break
    return SelectItem(expr=expr, alias=alias)


def _table_ref(
    node: ASTNode,
    ctes: dict[str, SelectStmt | RecursiveCTERef] | None = None,
    view_defs: dict[str, SelectStmt] | None = None,
) -> TableRef | DerivedTableRef | RecursiveCTERef:
    """Translate a table_ref node.

    The grammar has two forms::

        table_ref = "(" query_stmt ")" "AS" NAME   -- derived table
                  | table_name [ "AS" NAME ]        -- plain table

    We detect the derived-table form by checking for a ``query_stmt`` child.
    When the plain-table form names a non-recursive CTE, we substitute a
    DerivedTableRef.  For recursive CTEs we return RecursiveCTERef (with the
    alias updated from the usage site) so the planner can build the correct
    fixed-point iteration plan.
    """
    # Derived-table form: "(" query_stmt ")" "AS" NAME
    q = _maybe_child(node, "query_stmt")
    if q is not None:
        inner_stmt = _query_stmt(q, ctes=ctes, view_defs=view_defs)
        if not isinstance(inner_stmt, SelectStmt):
            raise ProgrammingError("derived table must be a plain SELECT, not a set operation")
        # The mandatory alias comes after the "AS" keyword.
        alias: str | None = None
        found_as = False
        for c in node.children:
            if _is_keyword(c, "AS"):
                found_as = True
            elif found_as and isinstance(c, Token) and _token_type(c) == "NAME":
                alias = c.value
                break
        if alias is None:
            raise ProgrammingError("derived table requires an alias (AS <name>)")
        return DerivedTableRef(select=inner_stmt, alias=alias)

    # Plain table form: table_name [ "AS" NAME | NAME ]
    #
    # The alias is optional.  Two syntactic forms are accepted:
    #   FROM employees AS e   — classic form with AS
    #   FROM employees e      — shorthand form without AS
    # NAME tokens never match SQL keywords (WHERE, JOIN, ON, etc.), so a bare
    # NAME token following the table_name ASTNode is unambiguously an alias.
    tn = _child_node(node, "table_name")
    parts = [c.value for c in tn.children if isinstance(c, Token) and _token_type(c) == "NAME"]
    table = parts[-1]  # schema.table → we ignore the schema qualifier
    alias = None
    saw_table_name = False
    for i, c in enumerate(node.children):
        if isinstance(c, ASTNode) and c.rule_name == "table_name":
            saw_table_name = True
        elif saw_table_name and _is_keyword(c, "AS") and i + 1 < len(node.children):
            nxt = node.children[i + 1]
            if isinstance(nxt, Token) and _token_type(nxt) == "NAME":
                alias = nxt.value
            break
        elif saw_table_name and isinstance(c, Token) and _token_type(c) == "NAME":
            # Alias written without AS (e.g. FROM employees e)
            alias = c.value
            break

    # CTE substitution: if the table name matches a known CTE, replace it.
    if ctes and table in ctes:
        entry = ctes[table]
        if isinstance(entry, RecursiveCTERef):
            # Propagate alias from the usage site (the CTE name is used as the
            # effective alias when no explicit alias is given).
            return RecursiveCTERef(
                name=entry.name,
                anchor=entry.anchor,
                recursive=entry.recursive,
                union_all=entry.union_all,
                alias=alias if alias is not None else table,
            )
        return DerivedTableRef(select=entry, alias=alias if alias is not None else table)

    # View substitution: expand named views into inline derived tables, exactly
    # like non-recursive CTEs.  CTEs take priority (checked above first).
    if view_defs and table in view_defs:
        return DerivedTableRef(
            select=view_defs[table],
            alias=alias if alias is not None else table,
        )

    return TableRef(table=table, alias=alias)


def _join_clause(
    node: ASTNode,
    state: _PlaceholderCounter,
    ctes: dict[str, SelectStmt | RecursiveCTERef] | None = None,
    view_defs: dict[str, SelectStmt] | None = None,
) -> JoinClause:
    # join_clause = [ join_type ] "JOIN" table_ref
    #               [ "ON" expr | "USING" "(" NAME { "," NAME } ")" ]
    #
    # USING desugaring is deferred to the planner (see JoinClause.using and
    # _build_from_tree).  NATURAL JOIN is forwarded as JoinKind.NATURAL for
    # the same reason — schema access is needed and only available in the
    # planner.
    jt = _maybe_child(node, "join_type")
    kind = _join_kind(jt) if jt is not None else JoinKind.INNER
    right = _table_ref(_child_node(node, "table_ref"), ctes=ctes, view_defs=view_defs)

    # USING (col1, col2, ...) — deferred resolution.
    #
    # We collect the column names and pass them as ``using=`` on JoinClause.
    # The planner expands them into the proper ON expression during
    # ``_build_from_tree``, where both the accumulated join scope and the
    # backend schema are available.
    #
    # We intentionally do NOT try to build the ON expression here in the
    # adapter, because in a chained join like:
    #
    #     a JOIN b USING (x) JOIN c USING (y)
    #
    # when the second USING is parsed, the adapter only knows that the
    # "current left table" is ``b`` (the most recently joined table).  But
    # ``y`` may live in ``a``, not ``b``.  The planner, which has already
    # added both ``a`` and ``b`` to the scope by the time it processes the
    # second join clause, can find the right table.
    if _has_keyword_child(node, "USING"):
        using_started = False
        col_names: list[str] = []
        for c in node.children:
            if _is_keyword(c, "USING"):
                using_started = True
                continue
            if using_started and isinstance(c, Token) and _token_type(c) == "NAME":
                col_names.append(c.value)
        return JoinClause(kind=kind, right=right, using=tuple(col_names))

    # Plain "ON expr" — or no condition at all (CROSS / NATURAL).
    expr_node = _maybe_child(node, "expr")
    on = _expr(expr_node, state) if expr_node is not None else None
    return JoinClause(kind=kind, right=right, on=on)


def _join_kind(node: ASTNode) -> str:
    # join_type = "CROSS" | "INNER" | "NATURAL" | "LEFT" ... | "RIGHT" ... | "FULL" ...
    # Look at the first keyword token to identify the join kind.
    for c in node.children:
        if isinstance(c, Token) and _token_type(c) == "KEYWORD":
            kw = c.value.upper()
            if kw == "CROSS":
                return JoinKind.CROSS
            if kw == "INNER":
                return JoinKind.INNER
            if kw == "NATURAL":
                return JoinKind.NATURAL
            if kw == "LEFT":
                return JoinKind.LEFT
            if kw == "RIGHT":
                return JoinKind.RIGHT
            if kw == "FULL":
                return JoinKind.FULL
    return JoinKind.INNER  # grammar requires one of the above; default safeguard


def _group_clause(
    node: ASTNode | None, state: _PlaceholderCounter
) -> tuple[Expr, ...]:
    if node is None:
        return ()
    # group_clause = "GROUP" "BY" column_ref { "," column_ref }
    return tuple(
        _column_ref_to_expr(c)
        for c in node.children
        if isinstance(c, ASTNode) and c.rule_name == "column_ref"
    )


def _order_clause(
    node: ASTNode | None, state: _PlaceholderCounter
) -> tuple[SortKey, ...]:
    if node is None:
        return ()
    keys: list[SortKey] = []
    for c in node.children:
        if isinstance(c, ASTNode) and c.rule_name == "order_item":
            keys.append(_order_item(c, state))
    return tuple(keys)


def _order_item(node: ASTNode, state: _PlaceholderCounter) -> SortKey:
    # order_item = expr [ "ASC" | "DESC" ]
    expr = _expr(_child_node(node, "expr"), state)
    descending = _has_keyword_child(node, "DESC")
    return SortKey(expr=expr, descending=descending)


def _limit_clause(node: ASTNode | None) -> Limit | None:
    if node is None:
        return None
    # limit_clause = "LIMIT" NUMBER [ "OFFSET" NUMBER ]
    numbers: list[int] = []
    has_offset_keyword = False
    for c in node.children:
        if isinstance(c, Token):
            if _token_type(c) == "NUMBER":
                numbers.append(int(c.value))
            elif _token_type(c) == "KEYWORD" and c.value.upper() == "OFFSET":
                has_offset_keyword = True
    count = numbers[0] if numbers else None
    offset = numbers[1] if has_offset_keyword and len(numbers) > 1 else None
    return Limit(count=count, offset=offset)


# --------------------------------------------------------------------------
# INSERT / UPDATE / DELETE.
# --------------------------------------------------------------------------


def _returning_exprs(
    node: ASTNode, state: _PlaceholderCounter
) -> tuple[Expr, ...]:
    """Parse a returning_clause child of a DML statement node.

    ``returning_clause = 'RETURNING' expr { ',' expr }``

    Returns an empty tuple when no returning_clause child is present.
    """
    ret_node = _maybe_child(node, "returning_clause")
    if ret_node is None:
        return ()
    return tuple(
        _expr(c, state)
        for c in ret_node.children
        if isinstance(c, ASTNode) and c.rule_name == "expr"
    )


def _conflict_action(node: ASTNode) -> str | None:
    """Extract the conflict resolution action from an optional ``conflict_clause`` child.

    ``conflict_clause = "OR" ( "REPLACE" | "IGNORE" | "ABORT" | "FAIL" | "ROLLBACK" )``

    Returns the action string in uppercase (e.g. ``"REPLACE"``) or ``None``
    when no ``conflict_clause`` is present.
    """
    cc = _maybe_child(node, "conflict_clause")
    if cc is None:
        return None
    # The second token in the conflict_clause is the action keyword.
    for child in cc.children:
        if isinstance(child, Token) and _token_type(child) == "KEYWORD":
            kw = child.value.upper()
            if kw in {"REPLACE", "IGNORE", "ABORT", "FAIL", "ROLLBACK"}:
                return kw
    return None


def _insert(
    node: ASTNode, default_conflict: str | None = None
) -> InsertValuesStmt | InsertSelectStmt:
    """Parse an ``insert_stmt`` or ``replace_stmt`` AST node.

    ``default_conflict`` is pre-set to ``"REPLACE"`` when called from the
    ``replace_stmt`` dispatch path (``REPLACE INTO \u2026`` shorthand).  For a
    regular ``insert_stmt`` the optional ``conflict_clause`` child is
    inspected instead and overrides ``default_conflict``.

    Grammar::

        insert_stmt  = "INSERT" [ conflict_clause ] "INTO" NAME
                       [ "(" NAME { "," NAME } ")" ]
                       insert_body [ returning_clause ] ;
        replace_stmt = "REPLACE" "INTO" NAME
                       [ "(" NAME { "," NAME } ")" ]
                       insert_body [ returning_clause ] ;
        insert_body  = "VALUES" row_value { "," row_value } | query_stmt ;
        conflict_clause = "OR" ( "REPLACE" | "IGNORE" | "ABORT" | "FAIL" | "ROLLBACK" ) ;
    """
    state = _PlaceholderCounter()
    # Conflict action: explicit clause overrides the default supplied by caller.
    on_conflict: str | None = _conflict_action(node) or default_conflict
    table_tok = _first_token(node, kind="NAME")
    assert table_tok is not None
    table = table_tok.value

    # Explicit column list: everything between LPAREN and RPAREN before insert_body.
    columns: tuple[str, ...] | None = None
    i = 0
    while i < len(node.children):
        c = node.children[i]
        if isinstance(c, ASTNode) and c.rule_name == "insert_body":
            break
        if _is_token(c, type_="LPAREN"):
            cols: list[str] = []
            j = i + 1
            while j < len(node.children) and not _is_token(node.children[j], type_="RPAREN"):
                child = node.children[j]
                if isinstance(child, Token) and _token_type(child) == "NAME":
                    cols.append(child.value)
                j += 1
            columns = tuple(cols)
        elif _is_keyword(c, "VALUES"):
            # Old grammar (pre insert_body): VALUES is at the stmt level.
            break
        i += 1

    # Check if we have an insert_body child (new grammar).
    insert_body_node = _maybe_child(node, "insert_body")
    returning = _returning_exprs(node, state)
    if insert_body_node is not None:
        # New grammar: insert_body = "VALUES" row_value ... | query_stmt
        q = _maybe_child(insert_body_node, "query_stmt")
        if q is not None:
            inner_stmt = _query_stmt(q)
            if not isinstance(inner_stmt, SelectStmt):
                raise ProgrammingError(
                    "INSERT \u2026 SELECT requires a plain SELECT, not a set operation"
                )
            return InsertSelectStmt(
                table=table, columns=columns, select=inner_stmt,
                on_conflict=on_conflict, returning=returning,
            )
        rows = tuple(_row_value(rv, state) for rv in _child_nodes(insert_body_node, "row_value"))
        return InsertValuesStmt(
            table=table, columns=columns, rows=rows,
            on_conflict=on_conflict, returning=returning,
        )

    # Old grammar fallback: row_value nodes directly under insert_stmt.
    rows = tuple(_row_value(rv, state) for rv in _child_nodes(node, "row_value"))
    return InsertValuesStmt(
        table=table, columns=columns, rows=rows,
        on_conflict=on_conflict, returning=returning,
    )


def _row_value(node: ASTNode, state: _PlaceholderCounter) -> tuple[Expr, ...]:
    return tuple(
        _expr(c, state) for c in node.children if isinstance(c, ASTNode) and c.rule_name == "expr"
    )


def _update(node: ASTNode) -> UpdateStmt:
    state = _PlaceholderCounter()
    # update_stmt = "UPDATE" NAME "SET" assignment { "," assignment } [where] [returning]
    table_tok = _first_token(node, kind="NAME")
    assert table_tok is not None
    table = table_tok.value

    assignments = tuple(
        _assignment(c, state)
        for c in node.children
        if isinstance(c, ASTNode) and c.rule_name == "assignment"
    )
    where = _maybe_expr(node, "where_clause", state, skip=1)
    returning = _returning_exprs(node, state)
    return UpdateStmt(table=table, assignments=assignments, where=where, returning=returning)


def _assignment(node: ASTNode, state: _PlaceholderCounter) -> Assignment:
    # assignment = NAME "=" expr
    col_tok = next(c for c in node.children if isinstance(c, Token) and _token_type(c) == "NAME")
    value = _expr(_child_node(node, "expr"), state)
    return Assignment(column=col_tok.value, value=value)


def _delete(node: ASTNode) -> DeleteStmt:
    state = _PlaceholderCounter()
    # delete_stmt = "DELETE" "FROM" NAME [where] [returning]
    table_tok = _first_token(node, kind="NAME")
    assert table_tok is not None
    where = _maybe_expr(node, "where_clause", state, skip=1)
    returning = _returning_exprs(node, state)
    return DeleteStmt(table=table_tok.value, where=where, returning=returning)


# --------------------------------------------------------------------------
# ALTER TABLE.
# --------------------------------------------------------------------------


def _alter_table(node: ASTNode) -> AlterTableStmt:
    # alter_table_stmt = "ALTER" "TABLE" NAME "ADD" [ "COLUMN" ] col_def ;
    table_tok = _first_token(node, kind="NAME")
    assert table_tok is not None
    col_node = _maybe_child(node, "col_def")
    assert col_node is not None, "alter_table_stmt: missing col_def"
    col = _col_def(col_node, _PlaceholderCounter())
    return AlterTableStmt(table=table_tok.value, column=col)


# --------------------------------------------------------------------------
# CREATE TABLE / DROP TABLE.
# --------------------------------------------------------------------------


def _create_table(node: ASTNode) -> CreateTableStmt:
    # create_table_stmt =
    #   "CREATE" "TABLE" ["IF" "NOT" "EXISTS"] NAME
    #   "(" col_def { "," col_def } ")"
    if_not_exists = _has_keyword_sequence(node, ("IF", "NOT", "EXISTS"))
    table_tok = _first_token(node, kind="NAME")
    assert table_tok is not None
    state = _PlaceholderCounter()
    cols = tuple(_col_def(c, state) for c in _child_nodes(node, "col_def"))
    return CreateTableStmt(table=table_tok.value, columns=cols, if_not_exists=if_not_exists)


def _col_def(node: ASTNode, state: _PlaceholderCounter | None = None) -> BackendColumnDef:
    # col_def = NAME NAME { col_constraint }
    names = [c for c in node.children if isinstance(c, Token) and _token_type(c) == "NAME"]
    col_name = names[0].value
    type_name = names[1].value.upper() if len(names) > 1 else "TEXT"

    not_null = False
    primary_key = False
    unique = False
    check_expression = None
    foreign_key: tuple[str, str | None] | None = None
    col_default = NO_DEFAULT   # "no DEFAULT clause" sentinel
    _state = state or _PlaceholderCounter()
    for c in _child_nodes(node, "col_constraint"):
        kw_seq = tuple(
            t.value.upper()
            for t in c.children
            if isinstance(t, Token) and _token_type(t) == "KEYWORD"
        )
        if kw_seq == ("NOT", "NULL"):
            not_null = True
        elif kw_seq == ("PRIMARY", "KEY"):
            primary_key = True
            not_null = True  # PRIMARY KEY implies NOT NULL.
        elif kw_seq == ("UNIQUE",):
            unique = True
        elif kw_seq[0:1] == ("CHECK",):
            expr_node = _maybe_child(c, "expr")
            if expr_node is not None:
                check_expression = _expr(expr_node, _state)
        elif kw_seq[0:1] == ("REFERENCES",):
            # Collect the NAME tokens: first is ref_table, second (if present) is ref_col.
            ref_names = [
                t.value
                for t in c.children
                if isinstance(t, Token) and _token_type(t) == "NAME"
            ]
            ref_table = ref_names[0] if ref_names else ""
            ref_col: str | None = ref_names[1] if len(ref_names) > 1 else None
            foreign_key = (ref_table, ref_col)
        elif kw_seq[0:1] == ("DEFAULT",):
            # col_constraint grammar: "DEFAULT" primary
            #
            # We evaluate scalar literal defaults at parse time.  The grammar's
            # ``primary`` production covers NUMBER, STRING, NULL, TRUE, FALSE, and
            # parenthesised expressions.  We parse the ``primary`` node via _primary
            # and, if the result is a plain Literal, store the Python value as the
            # column's default.  Non-literal expressions (e.g. DEFAULT (CURRENT_TIMESTAMP),
            # DEFAULT (1+1)) are left as NO_DEFAULT and evaluated at INSERT time in
            # a future increment — this covers the overwhelming majority of real-world
            # column defaults.
            primary_node = _maybe_child(c, "primary")
            if primary_node is not None:
                try:
                    default_expr = _primary(primary_node, _state)
                    if isinstance(default_expr, Literal):
                        col_default = default_expr.value  # Python int|float|str|bool|None
                except Exception:  # noqa: BLE001 — malformed node; leave as NO_DEFAULT
                    pass
    return BackendColumnDef(
        name=col_name,
        type_name=type_name,
        not_null=not_null,
        primary_key=primary_key,
        unique=unique,
        default=col_default,
        check_expr=check_expression,
        foreign_key=foreign_key,
    )


def _drop_table(node: ASTNode) -> DropTableStmt:
    if_exists = _has_keyword_sequence(node, ("IF", "EXISTS"))
    table_tok = _first_token(node, kind="NAME")
    assert table_tok is not None
    return DropTableStmt(table=table_tok.value, if_exists=if_exists)


# --------------------------------------------------------------------------
# CREATE INDEX / DROP INDEX.
# --------------------------------------------------------------------------


def _create_index(node: ASTNode) -> CreateIndexStmt:
    """Translate ``create_index_stmt`` into :class:`CreateIndexStmt`.

    Grammar::

        create_index_stmt =
            "CREATE" [ "UNIQUE" ] "INDEX" [ "IF" "NOT" "EXISTS" ] NAME
            "ON" NAME "(" NAME { "," NAME } ")" ;

    NAME tokens appear in order:  index_name, table_name, col1, col2, ...
    All KEYWORD tokens are filtered out before collecting NAMEs.
    """
    unique = _has_keyword_child(node, "UNIQUE")
    if_not_exists = _has_keyword_sequence(node, ("IF", "NOT", "EXISTS"))

    # Collect NAME tokens, skipping keywords like INDEX, ON, IF, NOT, EXISTS.
    names = [
        c.value
        for c in node.children
        if isinstance(c, Token) and _token_type(c) == "NAME"
    ]
    if len(names) < 3:
        raise ProgrammingError(
            "create_index_stmt: expected index_name, table_name, and at least one column"
        )

    index_name = names[0]
    table_name = names[1]
    columns = tuple(names[2:])

    return CreateIndexStmt(
        name=index_name,
        table=table_name,
        columns=columns,
        unique=unique,
        if_not_exists=if_not_exists,
    )


def _drop_index(node: ASTNode) -> DropIndexStmt:
    """Translate ``drop_index_stmt`` into :class:`DropIndexStmt`.

    Grammar::

        drop_index_stmt = "DROP" "INDEX" [ "IF" "EXISTS" ] NAME ;

    The single NAME token is the index name.
    """
    if_exists = _has_keyword_sequence(node, ("IF", "EXISTS"))
    name_tok = _first_token(node, kind="NAME")
    if name_tok is None:
        raise ProgrammingError("drop_index_stmt: expected index name")
    return DropIndexStmt(name=name_tok.value, if_exists=if_exists)


# --------------------------------------------------------------------------
# CREATE VIEW / DROP VIEW.
# --------------------------------------------------------------------------


def _create_view(node: ASTNode) -> CreateViewStmt:
    """Translate ``create_view_stmt`` into :class:`CreateViewStmt`.

    Grammar::

        create_view_stmt = "CREATE" "VIEW" [ "IF" "NOT" "EXISTS" ] NAME "AS" query_stmt ;

    The view body is a full ``query_stmt`` (SELECT, WITH, set operations).
    Only plain SELECT bodies are accepted — the engine will reject set-op
    views when it tries to store them as a ``SelectStmt``.
    """
    if_not_exists = _has_keyword_sequence(node, ("IF", "NOT", "EXISTS"))
    name_tok = _first_token(node, kind="NAME")
    if name_tok is None:
        raise ProgrammingError("create_view_stmt: expected view name")
    q = _maybe_child(node, "query_stmt")
    if q is None:
        raise ProgrammingError("create_view_stmt: expected query body")
    inner_stmt = _query_stmt(q)
    if not isinstance(inner_stmt, SelectStmt):
        raise ProgrammingError("CREATE VIEW body must be a plain SELECT, not a set operation")
    return CreateViewStmt(name=name_tok.value, query=inner_stmt, if_not_exists=if_not_exists)


def _drop_view(node: ASTNode) -> DropViewStmt:
    """Translate ``drop_view_stmt`` into :class:`DropViewStmt`.

    Grammar::

        drop_view_stmt = "DROP" "VIEW" [ "IF" "EXISTS" ] NAME ;
    """
    if_exists = _has_keyword_sequence(node, ("IF", "EXISTS"))
    name_tok = _first_token(node, kind="NAME")
    if name_tok is None:
        raise ProgrammingError("drop_view_stmt: expected view name")
    return DropViewStmt(name=name_tok.value, if_exists=if_exists)


# --------------------------------------------------------------------------
# CREATE TRIGGER / DROP TRIGGER.
# --------------------------------------------------------------------------


def _node_to_sql(node: ASTNode) -> str:
    """Reconstruct SQL text from an ASTNode by flattening all token values.

    NEW and OLD are not keywords in our lexer — they arrive as NAME tokens.
    We uppercase them here so that references like ``new.col`` become
    ``NEW . col``, matching the temporary table names the trigger executor
    creates.

    STRING token values have their surrounding quotes stripped by the lexer;
    we re-add single quotes here (escaping any embedded quotes via SQL-standard
    doubling so the reconstructed text is re-parseable).
    """
    parts: list[str] = []
    for child in node.children:
        if isinstance(child, Token):
            tt = _token_type(child)
            val = child.value
            if tt == "NAME" and val.lower() in ("new", "old"):
                val = val.upper()
            elif tt == "STRING":
                # Re-wrap in single quotes; escape embedded quotes by doubling.
                val = "'" + val.replace("'", "''") + "'"
            parts.append(val)
        elif isinstance(child, ASTNode):
            parts.append(_node_to_sql(child))
    return " ".join(parts)


def _create_trigger(node: ASTNode) -> CreateTriggerStmt:
    """Translate ``create_trigger_stmt`` into :class:`CreateTriggerStmt`.

    Grammar::

        create_trigger_stmt =
            "CREATE" "TRIGGER" NAME
            ( "BEFORE" | "AFTER" ) ( "INSERT" | "UPDATE" | "DELETE" ) "ON" NAME
            "FOR" "EACH" "ROW"
            "BEGIN" trigger_body_stmt ";" { trigger_body_stmt ";" } "END" ;

    NAME tokens appear in order: trigger_name, table_name.
    KEYWORD tokens carry BEFORE/AFTER and INSERT/UPDATE/DELETE.
    """
    names = [c.value for c in node.children if isinstance(c, Token) and _token_type(c) == "NAME"]
    if len(names) < 2:
        raise ProgrammingError("create_trigger_stmt: expected trigger name and table name")
    trigger_name = names[0]
    table_name = names[1]

    keywords = [
        c.value.upper()
        for c in node.children
        if isinstance(c, Token) and _token_type(c) == "KEYWORD"
    ]
    timing = "BEFORE" if "BEFORE" in keywords else "AFTER"
    event = next((k for k in keywords if k in ("INSERT", "UPDATE", "DELETE")), None)
    if event is None:
        raise ProgrammingError("create_trigger_stmt: expected INSERT, UPDATE, or DELETE event")

    body_stmts = _child_nodes(node, "trigger_body_stmt")
    body_sql = " ; ".join(_node_to_sql(s) for s in body_stmts)

    return CreateTriggerStmt(
        name=trigger_name,
        timing=timing,
        event=event,
        table=table_name,
        body_sql=body_sql,
    )


def _drop_trigger(node: ASTNode) -> DropTriggerStmt:
    """Translate ``drop_trigger_stmt`` into :class:`DropTriggerStmt`.

    Grammar::

        drop_trigger_stmt = "DROP" "TRIGGER" [ "IF" "EXISTS" ] NAME ;
    """
    if_exists = _has_keyword_sequence(node, ("IF", "EXISTS"))
    name_tok = _first_token(node, kind="NAME")
    if name_tok is None:
        raise ProgrammingError("drop_trigger_stmt: expected trigger name")
    return DropTriggerStmt(name=name_tok.value, if_exists=if_exists)


# --------------------------------------------------------------------------
# SAVEPOINT / RELEASE / ROLLBACK TO.
# --------------------------------------------------------------------------


def _savepoint(node: ASTNode) -> SavepointStmt:
    """Translate ``savepoint_stmt`` into :class:`SavepointStmt`.

    Grammar::

        savepoint_stmt = "SAVEPOINT" NAME ;
    """
    name_tok = _first_token(node, kind="NAME")
    if name_tok is None:
        raise ProgrammingError("savepoint_stmt: expected savepoint name")
    return SavepointStmt(name=name_tok.value)


def _release_savepoint(node: ASTNode) -> ReleaseSavepointStmt:
    """Translate ``release_stmt`` into :class:`ReleaseSavepointStmt`.

    Grammar::

        release_stmt = "RELEASE" [ "SAVEPOINT" ] NAME ;
    """
    name_tok = _first_token(node, kind="NAME")
    if name_tok is None:
        raise ProgrammingError("release_stmt: expected savepoint name")
    return ReleaseSavepointStmt(name=name_tok.value)


def _rollback_to(node: ASTNode) -> RollbackToStmt:
    """Translate ``rollback_to_stmt`` into :class:`RollbackToStmt`.

    Grammar::

        rollback_to_stmt = "ROLLBACK" "TO" [ "SAVEPOINT" ] NAME ;
    """
    name_tok = _first_token(node, kind="NAME")
    if name_tok is None:
        raise ProgrammingError("rollback_to_stmt: expected savepoint name")
    return RollbackToStmt(name=name_tok.value)


# --------------------------------------------------------------------------
# Expressions. Walk the precedence tower.
# --------------------------------------------------------------------------


@dataclass
class _PlaceholderCounter:
    """Monotonic counter for placeholder positions. Left-to-right discovery order."""

    count: int = 0

    def next(self) -> int:
        n = self.count
        self.count += 1
        return n


def _expr(node: ASTNode, state: _PlaceholderCounter) -> Expr:
    # expr = or_expr
    return _or_expr(_child_node(node, "or_expr"), state)


def _or_expr(node: ASTNode, state: _PlaceholderCounter) -> Expr:
    # or_expr = and_expr { "OR" and_expr }
    return _left_assoc_binary(node, "and_expr", _and_expr, {"OR": BinaryOp.OR}, state)


def _and_expr(node: ASTNode, state: _PlaceholderCounter) -> Expr:
    # and_expr = not_expr { "AND" not_expr }
    return _left_assoc_binary(node, "not_expr", _not_expr, {"AND": BinaryOp.AND}, state)


def _not_expr(node: ASTNode, state: _PlaceholderCounter) -> Expr:
    # not_expr = "NOT" not_expr | comparison
    if _has_keyword_child(node, "NOT"):
        inner = _child_node(node, "not_expr")
        return UnaryExpr(op=UnaryOp.NOT, operand=_not_expr(inner, state))
    return _comparison(_child_node(node, "comparison"), state)


def _comparison(node: ASTNode, state: _PlaceholderCounter) -> Expr:
    """Comparison covers: bare additive, cmp_op, BETWEEN, IN, LIKE, IS NULL.

    Grammar: ``comparison = additive [cmp_op additive | "BETWEEN" ... | ...]``.
    If the only child is an ``additive``, we pass it through. Otherwise we
    inspect the following children to pick the right expression shape.
    """
    additives = [c for c in node.children if isinstance(c, ASTNode) and c.rule_name == "additive"]
    left = _additive(additives[0], state)

    # Bare additive → nothing to combine.
    if len(additives) == 1 and not any(
        isinstance(c, ASTNode) and c.rule_name == "cmp_op" for c in node.children
    ) and not _has_keyword_child(node, "BETWEEN") and not _has_keyword_child(node, "IN") \
       and not _has_keyword_child(node, "LIKE") and not _has_keyword_child(node, "GLOB") \
       and not _has_keyword_child(node, "IS"):
        return left

    # cmp_op form.
    cmp = _maybe_child(node, "cmp_op")
    if cmp is not None:
        op = _cmp_op_to_binop(cmp)
        right = _additive(additives[1], state)
        return BinaryExpr(op=op, left=left, right=right)

    # BETWEEN / NOT BETWEEN.
    if _has_keyword_child(node, "BETWEEN"):
        negated = _has_keyword_child(node, "NOT")
        low = _additive(additives[1], state)
        high = _additive(additives[2], state)
        expr: Expr = Between(operand=left, low=low, high=high)
        return UnaryExpr(op=UnaryOp.NOT, operand=expr) if negated else expr

    # IN / NOT IN.
    if _has_keyword_child(node, "IN"):
        negated = _has_keyword_child(node, "NOT")
        # New grammar wraps the list in an in_expr node (= query_stmt | value_list).
        in_expr_node = _maybe_child(node, "in_expr")
        if in_expr_node is not None:
            q = _maybe_child(in_expr_node, "query_stmt")
            if q is not None:
                # Subquery form: expr IN (SELECT ...)
                inner_stmt = _query_stmt(q)
                if not isinstance(inner_stmt, SelectStmt):
                    raise ProgrammingError("IN subquery must be a plain SELECT statement")
                if negated:
                    return NotInSubquery(operand=left, query=inner_stmt)
                return InSubquery(operand=left, query=inner_stmt)
            vl = _child_node(in_expr_node, "value_list")
        else:
            # Old grammar fallback: value_list directly under comparison.
            vl = _child_node(node, "value_list")
        values = tuple(
            _expr(c, state) for c in vl.children if isinstance(c, ASTNode) and c.rule_name == "expr"
        )
        if negated:
            return NotIn(operand=left, values=values)
        return In(operand=left, values=values)

    # LIKE / NOT LIKE — pattern is a string literal additive.
    if _has_keyword_child(node, "LIKE"):
        negated = _has_keyword_child(node, "NOT")
        pat_expr = _additive(additives[1], state)
        if not isinstance(pat_expr, Literal) or not isinstance(pat_expr.value, str):
            raise ProgrammingError("LIKE pattern must be a string literal")
        if negated:
            return NotLike(operand=left, pattern=pat_expr.value)
        return Like(operand=left, pattern=pat_expr.value)

    # GLOB / NOT GLOB — case-sensitive pattern match using Unix glob syntax.
    #
    # SQL:  string GLOB pattern
    # Internal: glob(pattern, string)  — same argument order as SQLite's C API.
    #
    # GLOB differs from LIKE in two ways:
    #   1. Case-sensitive (* matches any sequence, ? matches one character).
    #   2. The pattern argument is passed dynamically (not restricted to string
    #      literals), so GLOB can be used with column references or expressions
    #      as the pattern.  This is consistent with SQLite's behaviour.
    if _has_keyword_child(node, "GLOB"):
        negated = _has_keyword_child(node, "NOT")
        pat_expr = _additive(additives[1], state)
        glob_call: Expr = FunctionCall(
            name="glob",
            args=(FuncArg(value=pat_expr), FuncArg(value=left)),
        )
        if negated:
            return UnaryExpr(op=UnaryOp.NOT, operand=glob_call)
        return glob_call

    # IS NULL / IS NOT NULL.
    if _has_keyword_child(node, "IS"):
        if _has_keyword_child(node, "NOT"):
            return IsNotNull(operand=left)
        return IsNull(operand=left)

    return left


def _additive(node: ASTNode, state: _PlaceholderCounter) -> Expr:
    # additive = multiplicative { ("+"|"-"|"||") multiplicative }
    #
    # "||" is SQL string concatenation (same as Python's str + str but for any
    # type — the VM coerces both sides to str before joining them).  It has the
    # same precedence as arithmetic + and - because it is in the same grammar
    # level.  This matches SQLite, PostgreSQL, and the SQL standard.
    return _left_assoc_punct(
        node,
        "multiplicative",
        _multiplicative,
        {"PLUS": BinaryOp.ADD, "MINUS": BinaryOp.SUB, "CONCAT_OP": BinaryOp.CONCAT},
        state,
    )


def _multiplicative(node: ASTNode, state: _PlaceholderCounter) -> Expr:
    # multiplicative = unary { (STAR|"/"|"%") unary }
    return _left_assoc_punct(
        node,
        "unary",
        _unary,
        {"STAR": BinaryOp.MUL, "SLASH": BinaryOp.DIV, "PERCENT": BinaryOp.MOD},
        state,
    )


def _unary(node: ASTNode, state: _PlaceholderCounter) -> Expr:
    # unary = "-" unary | primary
    if any(_is_token(c, type_="MINUS") for c in node.children):
        inner = _child_node(node, "unary")
        return UnaryExpr(op=UnaryOp.NEG, operand=_unary(inner, state))
    return _primary(_child_node(node, "primary"), state)


def _primary(node: ASTNode, state: _PlaceholderCounter) -> Expr:
    # primary = NUMBER | STRING | NULL | TRUE | FALSE | function_call
    #         | column_ref | "(" expr ")" | "?"
    for c in node.children:
        if isinstance(c, Token):
            t = _token_type(c)
            if t == "NUMBER":
                return Literal(value=_parse_number(c.value))
            if t == "STRING":
                return Literal(value=_unquote_string(c.value))
            if t == "BLOB":
                # BLOB_HEX token value is e.g. x'deadbeef' — strip x' and '.
                hex_str = c.value[2:-1]
                return Literal(value=bytes.fromhex(hex_str))
            if t == "QMARK":
                idx = state.next()
                return Literal(value=cast(object, _Placeholder(index=idx)))  # type: ignore[arg-type]
            if t == "KEYWORD":
                kw = c.value.upper()
                if kw == "NULL":
                    return Literal(value=None)
                if kw == "TRUE":
                    return Literal(value=True)
                if kw == "FALSE":
                    return Literal(value=False)
                if kw == "EXISTS":
                    # EXISTS "(" query_stmt ")" — find the query_stmt sibling.
                    q = _maybe_child(node, "query_stmt")
                    if q is None:
                        raise ProgrammingError("EXISTS requires a subquery")
                    inner_stmt = _query_stmt(q)
                    if not isinstance(inner_stmt, SelectStmt):
                        raise ProgrammingError("EXISTS subquery must be a SELECT statement")
                    return ExistsSubquery(query=inner_stmt)
        elif isinstance(c, ASTNode):
            if c.rule_name == "cast_expr":
                return _cast_expr(c, state)
            if c.rule_name == "window_func_call":
                return _window_func_call(c, state)
            if c.rule_name == "function_call":
                return _function_call(c, state)
            if c.rule_name == "column_ref":
                return _column_ref_to_expr(c)
            if c.rule_name == "expr":
                return _expr(c, state)
            if c.rule_name == "case_expr":
                return _case_expr(c, state)
            if c.rule_name == "query_stmt":
                # Scalar subquery: "(" query_stmt ")" in expression position.
                inner_stmt = _query_stmt(c)
                if not isinstance(inner_stmt, SelectStmt):
                    raise ProgrammingError("scalar subquery must be a SELECT statement")
                return ScalarSubquery(query=inner_stmt)
    raise ProgrammingError("unrecognized primary expression")


def _function_call(node: ASTNode, state: _PlaceholderCounter) -> Expr:
    # function_call = NAME "(" (STAR | [value_list]) ")"
    name_tok = next(c for c in node.children if isinstance(c, Token) and _token_type(c) == "NAME")
    name = name_tok.value
    star = any(_is_token(c, type_="STAR") for c in node.children)
    vl = _maybe_child(node, "value_list")
    args: list[FuncArg] = []
    if star:
        args.append(FuncArg(star=True))
    elif vl is not None:
        for c in vl.children:
            if isinstance(c, ASTNode) and c.rule_name == "expr":
                args.append(FuncArg(value=_expr(c, state)))

    # Aggregate functions fold into AggregateExpr; everything else stays generic.
    upper = name.upper()
    agg_map = {
        "COUNT": AggFunc.COUNT,
        "SUM": AggFunc.SUM,
        "AVG": AggFunc.AVG,
        "MIN": AggFunc.MIN,
        "MAX": AggFunc.MAX,
    }
    if upper in agg_map:
        if len(args) != 1:
            raise ProgrammingError(f"{upper}: expected 1 argument, got {len(args)}")
        return AggregateExpr(func=agg_map[upper], arg=args[0])

    if upper == "GROUP_CONCAT":
        # GROUP_CONCAT(col)          — SQLite default separator ','
        # GROUP_CONCAT(col, sep)     — explicit string literal separator
        #
        # SQL:2003 §10.9 requires the separator to be a character-string
        # literal; we enforce that at parse time so the codegen can bake the
        # separator into the instruction stream rather than evaluating it
        # dynamically each time.
        if len(args) == 0 or len(args) > 2:
            raise ProgrammingError(
                "GROUP_CONCAT: expected 1 or 2 arguments "
                "(column [, separator_literal])"
            )
        separator: str | None = None
        if len(args) == 2:
            sep_expr = args[1].value
            if not isinstance(sep_expr, Literal) or not isinstance(sep_expr.value, str):
                raise ProgrammingError(
                    "GROUP_CONCAT: separator must be a string literal, "
                    f"got {type(sep_expr).__name__}"
                )
            separator = sep_expr.value
        return AggregateExpr(
            func=AggFunc.GROUP_CONCAT,
            arg=args[0],
            separator=separator,
        )

    return FunctionCall(name=name, args=tuple(args))


def _cast_expr(node: ASTNode, state: _PlaceholderCounter) -> Expr:
    """Translate a ``cast_expr`` node into a :class:`FunctionCall`.

    Grammar::

        cast_expr = "CAST" "(" expr "AS" NAME ")"

    ``CAST(expr AS type_name)`` is semantically equivalent to calling the
    scalar function ``cast(expr, 'type_name')`` — which is exactly how the
    built-in ``cast`` function is registered in :mod:`sql_vm.scalar_functions`.

    The type name (INTEGER, TEXT, REAL, BLOB, NUMERIC) is passed as a string
    literal so the VM receives a concrete type indicator at dispatch time.

    Example::

        CAST(price AS INTEGER)   →  FunctionCall("cast", (FuncArg(price), FuncArg("INTEGER")))
    """
    inner_expr = _expr(_child_node(node, "expr"), state)
    # Find the NAME token that follows the AS keyword inside this cast_expr node.
    type_name: str | None = None
    found_as = False
    for c in node.children:
        if _is_keyword(c, "AS"):
            found_as = True
        elif found_as and isinstance(c, Token) and _token_type(c) == "NAME":
            type_name = c.value.upper()
            break
    if type_name is None:
        raise ProgrammingError("CAST: missing type name after AS")
    return FunctionCall(
        name="cast",
        args=(FuncArg(value=inner_expr), FuncArg(value=Literal(value=type_name))),
    )


def _window_func_call(node: ASTNode, state: _PlaceholderCounter) -> WindowFuncExpr:
    """Translate a ``window_func_call`` node into a :class:`WindowFuncExpr`.

    Grammar::

        window_func_call = NAME "(" ( STAR | [ value_list ] ) ")" "OVER" "(" window_spec ")" ;
        window_spec      = [ partition_clause ] [ order_clause ] ;
        partition_clause = "PARTITION" "BY" expr { "," expr } ;
        order_clause     = "ORDER" "BY" order_item { "," order_item } ;
        order_item       = expr [ "ASC" | "DESC" ] ;

    Supported functions and their arg requirements:

    - Arg-free (no argument):   ROW_NUMBER, RANK, DENSE_RANK, PERCENT_RANK, CUME_DIST
    - COUNT(*) (star arg):      COUNT — maps to "count_star"
    - Single-arg:               SUM, COUNT(col), AVG, MIN, MAX, FIRST_VALUE, LAST_VALUE
    - Literal-arg:              NTILE(n) — n is an integer constant
    - Multi-arg:                LAG(col [, offset [, default]]),
                                LEAD(col [, offset [, default]]),
                                NTH_VALUE(col, n)

    All function names are normalised to lower-case.
    """
    # Extract the function name.
    name_tok = next(c for c in node.children if isinstance(c, Token) and _token_type(c) == "NAME")
    func_name = name_tok.value.lower()

    # Extract argument (star, value_list, or empty).
    star = any(_is_token(c, type_="STAR") for c in node.children)
    vl = _maybe_child(node, "value_list")
    arg: Expr | None = None
    extra_args_tuple: tuple[Expr, ...] = ()

    if star:
        # COUNT(*) OVER (...) → func="count_star", arg=None
        func_name = "count_star"
    elif vl is not None:
        exprs = [c for c in vl.children if isinstance(c, ASTNode) and c.rule_name == "expr"]
        if exprs:
            arg = _expr(exprs[0], state)
            # Multi-argument functions (LAG, LEAD, NTH_VALUE) carry extra args
            # beyond the first column reference.  We thread them through as a
            # tuple so the planner and codegen can normalise them into the
            # proper (offset, default) / (n,) shapes.
            if len(exprs) > 1:
                extra_args_tuple = tuple(_expr(e, state) for e in exprs[1:])
    # Arg-free ranking functions keep func_name as-is (row_number, rank, dense_rank).

    # Extract the window_spec node.
    ws = _maybe_child(node, "window_spec")

    # PARTITION BY clause.
    partition_exprs: list[Expr] = []
    if ws is not None:
        pc = _maybe_child(ws, "partition_clause")
        if pc is not None:
            partition_exprs = [
                _expr(c, state)
                for c in pc.children
                if isinstance(c, ASTNode) and c.rule_name == "expr"
            ]

    # ORDER BY clause — reuse the shared _order_items helper.
    order_keys: list[tuple[Expr, bool]] = []
    if ws is not None:
        oc = _maybe_child(ws, "order_clause")
        if oc is not None:
            for oi in _child_nodes(oc, "order_item"):
                oi_exprs = [
                    c for c in oi.children
                    if isinstance(c, ASTNode) and c.rule_name == "expr"
                ]
                if not oi_exprs:
                    continue
                oi_expr = _expr(oi_exprs[0], state)
                desc = any(
                    _is_token(c, type_="KEYWORD")
                    and isinstance(c, Token)
                    and c.value.upper() == "DESC"
                    for c in oi.children
                )
                order_keys.append((oi_expr, desc))

    return WindowFuncExpr(
        func=func_name,
        arg=arg,
        partition_by=tuple(partition_exprs),
        order_by=tuple(order_keys),
        extra_args=extra_args_tuple,
    )


def _case_expr(node: ASTNode, state: _PlaceholderCounter) -> CaseExpr:
    """Translate a ``case_expr`` node into a :class:`CaseExpr`.

    Grammar::

        case_expr   = "CASE" [ case_operand ] case_when { case_when } [ "ELSE" expr ] "END"
        case_operand = expr
        case_when   = "WHEN" expr "THEN" expr

    If ``case_operand`` is present this is a *simple* CASE: each WHEN value is
    turned into an equality comparison ``operand = when_value``.  Without an
    operand it is a *searched* CASE whose WHEN clauses are boolean predicates.
    The planner and all downstream stages see only the searched form.
    """
    # Optional simple-CASE operand.
    op_node = _maybe_child(node, "case_operand")
    operand = _expr(_child_node(op_node, "expr"), state) if op_node is not None else None

    # WHEN/THEN pairs.
    when_nodes = _child_nodes(node, "case_when")
    if not when_nodes:
        raise ProgrammingError("CASE requires at least one WHEN clause")
    whens: list[tuple[Expr, Expr]] = []
    for wn in when_nodes:
        # case_when = "WHEN" expr "THEN" expr  — exactly two expr children.
        exprs = [c for c in wn.children if isinstance(c, ASTNode) and c.rule_name == "expr"]
        if len(exprs) != 2:
            raise ProgrammingError("CASE WHEN requires exactly one condition and one result")
        cond_expr = _expr(exprs[0], state)
        result_expr = _expr(exprs[1], state)
        if operand is not None:
            # Normalize simple CASE: WHEN v THEN r → WHEN operand = v THEN r
            cond_expr = BinaryExpr(op=BinaryOp.EQ, left=operand, right=cond_expr)
        whens.append((cond_expr, result_expr))

    # Optional ELSE clause.
    else_expr: Expr | None = None
    for i, c in enumerate(node.children):
        if _is_keyword(c, "ELSE") and i + 1 < len(node.children):
            next_c = node.children[i + 1]
            if isinstance(next_c, ASTNode) and next_c.rule_name == "expr":
                else_expr = _expr(next_c, state)
            break

    return CaseExpr(whens=tuple(whens), else_=else_expr)


def _column_ref_to_expr(node: ASTNode) -> Column:
    # column_ref = NAME [ "." NAME ]
    names = [c for c in node.children if isinstance(c, Token) and _token_type(c) == "NAME"]
    if len(names) == 1:
        return Column(table=None, col=names[0].value)
    return Column(table=names[0].value, col=names[1].value)


# --------------------------------------------------------------------------
# Left-associative fold helpers.
# --------------------------------------------------------------------------


def _left_assoc_binary(
    node: ASTNode,
    child_rule: str,
    child_fn: Callable[[ASTNode, _PlaceholderCounter], Expr],
    keyword_map: dict[str, BinaryOp],
    state: _PlaceholderCounter,
) -> Expr:
    """Fold ``x OP y OP z`` into ``(x OP y) OP z`` using keyword operators."""
    children = node.children
    # Find the subexpressions in order.
    subs = [c for c in children if isinstance(c, ASTNode) and c.rule_name == child_rule]
    if len(subs) == 1:
        return child_fn(subs[0], state)
    result = child_fn(subs[0], state)
    # Operators appear between child_rule nodes; we step through children
    # and pair each keyword op with the next subexpression.
    i = 0
    sub_idx = 1
    while i < len(children):
        c = children[i]
        if isinstance(c, Token) and _token_type(c) == "KEYWORD":
            op = keyword_map.get(c.value.upper())
            if op is not None and sub_idx < len(subs):
                result = BinaryExpr(op=op, left=result, right=child_fn(subs[sub_idx], state))
                sub_idx += 1
        i += 1
    return result


def _left_assoc_punct(
    node: ASTNode,
    child_rule: str,
    child_fn: Callable[[ASTNode, _PlaceholderCounter], Expr],
    token_map: dict[str, BinaryOp],
    state: _PlaceholderCounter,
) -> Expr:
    """Same as above but for punctuation-based operators (+, -, *, /, %)."""
    children = node.children
    subs = [c for c in children if isinstance(c, ASTNode) and c.rule_name == child_rule]
    if len(subs) == 1:
        return child_fn(subs[0], state)
    result = child_fn(subs[0], state)
    sub_idx = 1
    for c in children:
        if isinstance(c, Token):
            op = token_map.get(_token_type(c))
            if op is not None and sub_idx < len(subs):
                result = BinaryExpr(op=op, left=result, right=child_fn(subs[sub_idx], state))
                sub_idx += 1
    return result


def _cmp_op_to_binop(node: ASTNode) -> BinaryOp:
    for c in node.children:
        if isinstance(c, Token):
            t = _token_type(c)
            v = c.value
            if t == "EQUALS" or v == "=":
                return BinaryOp.EQ
            if t == "NOT_EQUALS" or v in ("<>", "!="):
                return BinaryOp.NOT_EQ
            if v == "<":
                return BinaryOp.LT
            if v == ">":
                return BinaryOp.GT
            if v == "<=":
                return BinaryOp.LTE
            if v == ">=":
                return BinaryOp.GTE
    raise ProgrammingError("unrecognized cmp_op")


# --------------------------------------------------------------------------
# Token / node helpers. Guards against the stringly-typed TokenType layout
# (Token.type.name on some vendored lexers, Token.type as a plain string
# on others).
# --------------------------------------------------------------------------


def _token_type(t: Token) -> str:
    name = getattr(t.type, "name", None)
    return name if name is not None else str(t.type)


def _is_token(x: object, *, type_: str | None = None) -> bool:
    if not isinstance(x, Token):
        return False
    if type_ is None:
        return True
    return _token_type(x) == type_


def _is_keyword(x: object, kw: str) -> bool:
    return isinstance(x, Token) and _token_type(x) == "KEYWORD" and x.value.upper() == kw.upper()


def _has_keyword_child(node: ASTNode, kw: str) -> bool:
    return any(_is_keyword(c, kw) for c in node.children)


def _has_keyword_sequence(node: ASTNode, kws: tuple[str, ...]) -> bool:
    targets = [k.upper() for k in kws]
    sequence = [
        c.value.upper()
        for c in node.children
        if isinstance(c, Token) and _token_type(c) == "KEYWORD"
    ]
    for i in range(len(sequence) - len(targets) + 1):
        if sequence[i : i + len(targets)] == targets:
            return True
    return False


def _child_node(node: ASTNode, rule: str) -> ASTNode:
    for c in node.children:
        if isinstance(c, ASTNode) and c.rule_name == rule:
            return c
    raise ProgrammingError(f"expected child rule {rule!r} under {node.rule_name}")


def _only_child_node(node: ASTNode, rule: str) -> ASTNode:
    kids = [c for c in node.children if isinstance(c, ASTNode) and c.rule_name == rule]
    if len(kids) != 1:
        raise ProgrammingError(f"expected exactly one {rule!r} child, got {len(kids)}")
    return kids[0]


def _maybe_child(node: ASTNode, rule: str) -> ASTNode | None:
    for c in node.children:
        if isinstance(c, ASTNode) and c.rule_name == rule:
            return c
    return None


def _child_nodes(node: ASTNode, rule: str) -> list[ASTNode]:
    return [c for c in node.children if isinstance(c, ASTNode) and c.rule_name == rule]


def _single_child(node: ASTNode) -> ASTNode | Token:
    kids = [c for c in node.children if isinstance(c, (ASTNode, Token))]
    if not kids:
        raise ProgrammingError(f"{node.rule_name} has no children")
    # Statement nodes typically have one ASTNode child (the actual stmt) plus
    # possibly a trailing semicolon token that the program level strips — by
    # the time we get here the statement node's only meaningful child is the
    # inner stmt ASTNode.
    for k in kids:
        if isinstance(k, ASTNode):
            return k
    return kids[0]


def _maybe_expr(
    node: ASTNode, clause_rule: str, state: _PlaceholderCounter, skip: int = 0
) -> Expr | None:
    """Find a sub-clause like ``where_clause`` and extract its ``expr`` child."""
    clause = _maybe_child(node, clause_rule)
    if clause is None:
        return None
    inner = _child_node(clause, "expr")
    _ = skip  # unused but kept for API clarity: where_clause etc. start with a keyword.
    return _expr(inner, state)


def _first_token(node: ASTNode, *, kind: str) -> Token | None:
    for c in node.children:
        if isinstance(c, Token) and _token_type(c) == kind:
            return c
    return None


def _parse_number(s: str) -> int | float:
    if "." in s or "e" in s or "E" in s:
        return float(s)
    return int(s)


def _unquote_string(s: str) -> str:
    # The SQL lexer accepts backslash-escapes: 'O\'Brien', 'back\\slash'.
    # Strip the surrounding quotes and unescape any `\x` → `x` pair.
    if len(s) >= 2 and s[0] == s[-1] and s[0] in ("'", '"'):
        body = s[1:-1]
        out = []
        i = 0
        while i < len(body):
            if body[i] == "\\" and i + 1 < len(body):
                out.append(body[i + 1])
                i += 2
            else:
                out.append(body[i])
                i += 1
        return "".join(out)
    return s
