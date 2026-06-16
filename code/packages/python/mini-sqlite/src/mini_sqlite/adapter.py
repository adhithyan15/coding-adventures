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
from dataclasses import dataclass, field
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
    AstUpsertAssignment,
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
    ExcludedColumn,
    ExistsSubquery,
    FrameBound,
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
    UpsertClause,
    Wildcard,
    WindowFuncExpr,
    WinFrame,
)
from sql_planner.expr import Expr

from .errors import OperationalError, ProgrammingError

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


# Shorthand for the value type of the active-CTE dict.  A named CTE's
# body can be a plain SELECT, a set-op tree (UNION/INTERSECT/EXCEPT
# anywhere on the left spine), or a recursive reference token; all
# three plug into ``DerivedTableRef.select`` at substitution time.
# Aliased here purely so the dozen function signatures threading this
# dict around don't blow past the 100-column ruff limit.
_CTEBody = SelectStmt | UnionStmt | IntersectStmt | ExceptStmt | RecursiveCTERef


def _query_stmt(
    node: ASTNode,
    ctes: dict[str, _CTEBody] | None = None,
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
    active_ctes: dict[str, _CTEBody] = dict(ctes) if ctes else {}
    with_node = _maybe_child(node, "with_clause")
    if with_node is not None:
        # Check whether the WITH clause carries the RECURSIVE keyword.
        is_recursive = any(_is_keyword(c, "RECURSIVE") for c in with_node.children)

        for cte_node in _child_nodes(with_node, "cte_def"):
            name_tok = _first_token(cte_node, kind="NAME")
            if name_tok is None:
                raise ProgrammingError("cte_def: missing CTE name")
            cte_name = name_tok.value
            # Extract optional column alias list: WITH RECURSIVE cnt(n, m) AS (...)
            col_aliases = _cte_col_aliases(cte_node)
            inner_q = _child_node(cte_node, "query_stmt")

            if is_recursive and _child_nodes(inner_q, "set_op_clause"):
                # Recursive CTE: body is "anchor UNION [ALL] recursive_step".
                # The anchor can be either a SELECT or a VALUES expression
                # (SQLite accepts both — e.g. ``WITH RECURSIVE c(n) AS
                # (VALUES(1) UNION ALL SELECT n+1 FROM c WHERE n < 5)``
                # is the canonical "count from 1 to 5" pattern).
                #
                # Single-row VALUES yields a ``SelectStmt`` (a single SELECT
                # with literal columns), which fits ``RecursiveCTERef.anchor``
                # directly.  Multi-row VALUES yields a ``UnionStmt`` tree;
                # the planner's recursive-CTE anchor path expects a
                # ``SelectStmt`` (single base relation), so we reject the
                # multi-row case with a clear pointer to the workaround.
                anchor_values_node = _maybe_child(inner_q, "values_stmt")
                if anchor_values_node is not None:
                    values_anchor = _values_stmt(anchor_values_node)
                    if not isinstance(values_anchor, SelectStmt):
                        raise ProgrammingError(
                            f"RECURSIVE CTE {cte_name!r} anchor: multi-row "
                            f"VALUES is not yet supported (use a single "
                            f"VALUES row, or rewrite as ``SELECT … UNION "
                            f"ALL SELECT …`` for the anchor)"
                        )
                    anchor_stmt = values_anchor
                else:
                    anchor_node = _child_node(inner_q, "select_stmt")
                    anchor_stmt = _select(anchor_node, ctes=active_ctes)

                # Apply column aliases to the anchor's SELECT items so the
                # planner derives the right output-column names.  For example:
                #   WITH RECURSIVE cnt(n) AS (SELECT 1 UNION ALL SELECT n+1 ...)
                # renames the anchor's "1" column to "n", which makes the
                # recursive step's column reference "n" resolve correctly.
                if col_aliases:
                    anchor_stmt = _apply_cte_col_aliases(anchor_stmt, col_aliases)

                # Parse the recursive step WITHOUT this CTE in active_ctes so
                # the self-reference stays as a plain TableRef.  The planner's
                # working_set mechanism converts it to WorkingSetScan.
                ctes_without_self = {k: v for k, v in active_ctes.items() if k != cte_name}
                set_op_nodes = _child_nodes(inner_q, "set_op_clause")
                union_all = True
                rec_stmt: SelectStmt | None = None
                for sop in set_op_nodes:
                    op, all_flag, right_sel_node, right_rule = _set_op_clause(sop)
                    if op == "UNION":
                        union_all = all_flag
                        # The recursive step is required to be a SELECT
                        # (the grammar doesn't accept ``UNION ALL VALUES
                        # (n+1) FROM cte`` because VALUES can't reference
                        # row columns).  Reject the VALUES form with the
                        # same error SQLite would.
                        if right_rule == "values_stmt":
                            raise ProgrammingError(
                                f"RECURSIVE CTE '{cte_name}' recursive step "
                                f"cannot be a VALUES expression"
                            )
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
                # The body may be a plain SelectStmt or a set-op tree
                # (UnionStmt / IntersectStmt / ExceptStmt) — both are
                # accepted, matching SQLite.  Column aliases on the CTE
                # name are applied to the *leftmost* SelectStmt in the
                # tree (set-op output column names inherit from the
                # left operand, matching SQLite).
                if col_aliases:
                    inner_stmt = _apply_cte_col_aliases(inner_stmt, col_aliases)
                # Make this CTE visible to subsequent CTEs and the main query.
                active_ctes[cte_name] = inner_stmt

    # query_stmt branches: ( values_stmt | select_stmt ) { set_op_clause }
    # Either branch produces a left-hand Statement; set_op_clauses chain
    # additional SELECTs onto the right via UNION/INTERSECT/EXCEPT.
    values_node = _maybe_child(node, "values_stmt")
    left: Statement
    if values_node is not None:
        left = _values_stmt(values_node)
    else:
        select_node = _child_node(node, "select_stmt")
        left = _select(select_node, ctes=active_ctes, view_defs=view_defs)
    set_ops = _child_nodes(node, "set_op_clause")
    for op_node in set_ops:
        op, all_flag, right_node, right_rule = _set_op_clause(op_node)
        if right_rule == "values_stmt":
            right_stmt = _values_stmt(right_node)
        else:
            right_stmt = _select(right_node, ctes=active_ctes, view_defs=view_defs)
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

    # Compound ORDER BY / LIMIT — when the grammar parsed
    # ``SELECT a UNION ALL SELECT b ORDER BY x LIMIT N``,
    # the trailing ORDER BY/LIMIT got attached to the *rightmost*
    # ``SELECT b`` (because select_stmt's grammar still allows them
    # on every leg).  SQLite's documented semantics, though, is that
    # ORDER BY and LIMIT after a compound apply to the whole
    # compound, not just the last leg.  We reproduce that by:
    #
    #   1. detecting the rightmost SELECT's order_by/limit
    #   2. stripping them off that SELECT
    #   3. wrapping the entire compound in
    #          SELECT * FROM (compound) ORDER BY ... LIMIT ...
    #
    # …which is exactly the SQL the user would write if they wanted
    # to be explicit about the parenthesisation.  The wrapper makes
    # column names of the compound (inherited from the leftmost
    # SELECT) visible to the ORDER BY clause.
    if set_ops and isinstance(left, (UnionStmt, IntersectStmt, ExceptStmt)):
        rightmost = left.right
        if rightmost.order_by or rightmost.limit:
            # Strip order/limit from the rightmost SELECT.
            stripped_right = SelectStmt(
                items=rightmost.items,
                from_=rightmost.from_,
                joins=rightmost.joins,
                where=rightmost.where,
                group_by=rightmost.group_by,
                having=rightmost.having,
                # order_by and limit deliberately omitted — they get
                # hoisted onto the wrapper SELECT below.
                distinct=rightmost.distinct,
            )
            # Rebuild the compound with the stripped right operand.
            if isinstance(left, UnionStmt):
                compound = UnionStmt(left=left.left, right=stripped_right, all=left.all)
            elif isinstance(left, IntersectStmt):
                compound = IntersectStmt(left=left.left, right=stripped_right, all=left.all)
            else:
                compound = ExceptStmt(left=left.left, right=stripped_right, all=left.all)
            # Wrap in SELECT * FROM (compound) AS <synthetic> ORDER BY ... LIMIT ...
            # The sentinel alias starts with '<' so it cannot collide
            # with a user identifier (which the lexer restricts to
            # alphanumeric + underscore).
            wrapper_alias = f"<compound #{id(compound):x}>"
            left = SelectStmt(
                items=(SelectItem(expr=Wildcard(), alias=None),),
                from_=DerivedTableRef(select=compound, alias=wrapper_alias),
                order_by=rightmost.order_by,
                limit=rightmost.limit,
            )
    return left


def _set_op_clause(node: ASTNode) -> tuple[str, bool, ASTNode, str]:
    """Extract (operator_name, all_flag, body_node, body_rule) from a set_op_clause.

    The body may be a ``select_stmt`` (the usual case) or a
    ``values_stmt`` (``UNION ALL VALUES (1)``).  The caller dispatches
    on ``body_rule`` to call the right translator.

    SQLite compatibility note: only ``UNION ALL`` is valid.  SQLite parses
    ``INTERSECT ALL`` and ``EXCEPT ALL`` as syntax errors because neither the
    SQL-92 nor the SQLite dialect defines bag semantics for those two operators.
    We enforce the same restriction here so callers get the same
    ``OperationalError: near "ALL": syntax error`` they would from the real
    SQLite engine.
    """
    op: str | None = None
    all_flag = False
    body_node: ASTNode | None = None
    body_rule: str | None = None
    for c in node.children:
        if isinstance(c, Token) and _token_type(c) == "KEYWORD":
            kw = c.value.upper()
            if kw in ("UNION", "INTERSECT", "EXCEPT"):
                op = kw
            elif kw == "ALL":
                all_flag = True
        elif isinstance(c, ASTNode) and c.rule_name in ("select_stmt", "values_stmt"):
            body_node = c
            body_rule = c.rule_name
    if op is None or body_node is None or body_rule is None:
        raise ProgrammingError("malformed set_op_clause")
    # SQLite only supports UNION ALL.  INTERSECT ALL and EXCEPT ALL are not
    # part of the SQLite dialect (the grammar accepts them so the parser can
    # report a clean error rather than a confusing token-mismatch).
    if op in ("INTERSECT", "EXCEPT") and all_flag:
        raise OperationalError('near "ALL": syntax error')
    return op, all_flag, body_node, body_rule


# --------------------------------------------------------------------------
# VALUES — desugars a standalone ``VALUES (a,b),(c,d),…`` query into a
# left-deep UNION-ALL chain of single-row SELECTs.  This lets downstream
# layers see only constructs they already handle.
# --------------------------------------------------------------------------


def _values_stmt(node: ASTNode) -> Statement:
    """Translate ``values_stmt = "VALUES" row_value { "," row_value }`` to a Statement.

    SQLite's behaviour, which we match byte-for-byte:

    * Output columns are named ``column1``, ``column2``, …
      (1-indexed) when no explicit alias list is given.  Callers that
      *do* give aliases — e.g. ``WITH x(a, b) AS (VALUES (1, 2))`` —
      get the aliases applied by ``_apply_cte_col_aliases`` because
      that helper already walks down the left spine of a set-op tree
      to find the leftmost SELECT.  We only need to lay down the
      default names here.
    * UNION ALL (not plain UNION) so duplicate rows survive — matches
      SQLite's ``VALUES (1),(1)`` returning two rows.
    * Empty VALUES is rejected by the parser (the grammar requires at
      least one ``row_value``), so we don't need to handle that case.

    Implementation detail: we build a left-deep tree
    ``UNION ALL(UNION ALL(SELECT row0, SELECT row1), SELECT row2)`` so
    that the existing set-op tree machinery (column derivation, alias
    application, planner descent) sees exactly the shape it already
    handles.
    """
    row_nodes = _child_nodes(node, "row_value")
    if not row_nodes:
        raise ProgrammingError("VALUES list cannot be empty")

    # Fresh placeholder counter — matches the convention every other
    # top-level translator (_select, _insert, _update, _delete) uses:
    # each statement-shaped node starts its own ``?`` indexing.  A
    # nested VALUES inside a larger SELECT therefore can't share a
    # counter with the surrounding statement; that's a pre-existing
    # adapter-wide limitation, not specific to VALUES.
    state = _PlaceholderCounter()

    def _build_row(row_node: ASTNode) -> SelectStmt:
        # row_value = "(" expr { "," expr } ")"
        exprs = [
            _expr(c, state)
            for c in row_node.children
            if isinstance(c, ASTNode) and c.rule_name == "expr"
        ]
        if not exprs:
            raise ProgrammingError("VALUES row cannot be empty")
        # Name columns column1, column2, ... 1-indexed, matching SQLite.
        items = tuple(
            SelectItem(expr=e, alias=f"column{i + 1}")
            for i, e in enumerate(exprs)
        )
        return SelectStmt(items=items)

    selects = [_build_row(r) for r in row_nodes]
    if len(selects) == 1:
        return selects[0]
    # Fold left: ((s0 UNION ALL s1) UNION ALL s2) ...
    left: Statement = selects[0]
    for right in selects[1:]:
        left = UnionStmt(left=left, right=right, all=True)  # type: ignore[arg-type]
    return left


# --------------------------------------------------------------------------
# SELECT.
# --------------------------------------------------------------------------


def _extract_window_clause(node: ASTNode | None, state: _PlaceholderCounter) -> None:
    """Populate state.window_defs from a window_clause node (may be None).

    Grammar::

        window_clause = "WINDOW" NAME "AS" "(" window_spec ")"
                        { "," NAME "AS" "(" window_spec ")" } ;

    Each NAME → window_spec pair is stored in state.window_defs so that
    _window_func_call() can resolve OVER <name> references.
    """
    if node is None:
        return
    # Walk children collecting NAME / window_spec pairs.
    # Layout: WINDOW NAME AS ( window_spec ) [, NAME AS ( window_spec ) ...]
    children = node.children
    i = 0
    while i < len(children):
        c = children[i]
        # Skip WINDOW keyword and commas.
        if isinstance(c, Token) and _token_type(c) in ("KEYWORD", "COMMA"):
            i += 1
            continue
        # A NAME token starts a window definition.
        if isinstance(c, Token) and _token_type(c) == "NAME":
            win_name = c.value.upper()
            # Skip AS and LPAREN; find the window_spec node.
            j = i + 1
            while j < len(children):
                inner = children[j]
                if isinstance(inner, ASTNode) and inner.rule_name == "window_spec":
                    state.window_defs[win_name] = inner
                    i = j + 1
                    break
                j += 1
            else:
                i += 1
            continue
        i += 1


def _select(
    node: ASTNode,
    ctes: dict[str, _CTEBody] | None = None,
    view_defs: dict[str, SelectStmt] | None = None,
) -> SelectStmt:
    state = _PlaceholderCounter()

    # WINDOW clause must be populated before the select_list so that any
    # OVER <name> reference in the column expressions can resolve.
    _extract_window_clause(_maybe_child(node, "window_clause"), state)

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
    # select_item = expr [ [ "AS" ] NAME ]
    # SQLite allows bare alias without AS: SELECT 1 x  ≡  SELECT 1 AS x.
    # The grammar makes AS optional; the adapter handles both forms.
    # NAME never matches keywords (FROM, WHERE, …) so there is no ambiguity.
    expr = _expr(_child_node(node, "expr"), state)
    alias = None
    for i, c in enumerate(node.children):
        if _is_keyword(c, "AS"):
            # Full form: AS NAME
            if i + 1 < len(node.children):
                nxt = node.children[i + 1]
                if isinstance(nxt, Token):
                    alias = nxt.value
            break
        if isinstance(c, Token) and _token_type(c) == "NAME":
            # Bare alias without AS — direct NAME child of select_item
            alias = c.value
            break
    return SelectItem(expr=expr, alias=alias)


def _table_ref(
    node: ASTNode,
    ctes: dict[str, _CTEBody] | None = None,
    view_defs: dict[str, SelectStmt] | None = None,
) -> TableRef | DerivedTableRef | RecursiveCTERef:
    """Translate a table_ref node.

    The grammar has two forms::

        table_ref = "(" query_stmt ")" [ "AS" ] NAME   -- derived table
                  | table_name [ "AS" NAME | NAME ]     -- plain table

    We detect the derived-table form by checking for a ``query_stmt`` child.
    When the plain-table form names a non-recursive CTE, we substitute a
    DerivedTableRef.  For recursive CTEs we return RecursiveCTERef (with the
    alias updated from the usage site) so the planner can build the correct
    fixed-point iteration plan.

    Derived-table alias parsing
    ---------------------------
    SQLite accepts both ``(query) AS alias`` and ``(query) alias`` — the AS
    keyword is optional, matching standard SQL.  We find the alias by
    scanning for the first NAME token that appears after the closing
    parenthesis of the inner query (skipping any optional AS keyword in
    between).  An alias is still required (you can't have a bare ``(query)``
    in FROM); we raise if no NAME follows the closing paren.
    """
    # Derived-table form: "(" query_stmt ")" [ "AS" ] NAME
    q = _maybe_child(node, "query_stmt")
    if q is not None:
        inner_stmt = _query_stmt(q, ctes=ctes, view_defs=view_defs)
        # SQLite allows the inner query of a derived table to be a compound
        # query (UNION / INTERSECT / EXCEPT), not just a plain SELECT.  The
        # planner's :func:`_plan_derived_inner` dispatches on the statement
        # type, so we can pass through any of the four typed forms.  We
        # still reject anything else (INSERT, UPDATE, DDL, etc.) because the
        # surrounding ``FROM`` context only makes sense for query-producing
        # statements.
        if not isinstance(inner_stmt, SelectStmt | UnionStmt | IntersectStmt | ExceptStmt):
            raise ProgrammingError(
                "derived table inner query must be a SELECT or set operation"
            )
        # The alias is the first NAME token AFTER the closing parenthesis,
        # optionally preceded by an AS keyword.  Walk the children and grab
        # the NAME once we're past the ")" token.  SQLite allows the alias
        # to be omitted entirely (matching standard SQL), so we no longer
        # raise when the NAME is absent — the planner accepts ``alias=None``
        # and falls back to the inner query's unqualified column names for
        # scope resolution.
        alias: str | None = None
        past_close_paren = False
        for c in node.children:
            if isinstance(c, Token) and c.value == ")":
                past_close_paren = True
                continue
            if past_close_paren and isinstance(c, Token) and _token_type(c) == "NAME":
                alias = c.value
                break
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

    # Parse the optional ``index_hint`` child node.  The grammar:
    #
    #   index_hint = "INDEXED" "BY" NAME | "NOT" "INDEXED" ;
    #
    # Two mutually-exclusive forms.  ``INDEXED BY <name>`` pins the
    # planner to the named index; ``NOT INDEXED`` disables index
    # substitution for this scan.  Both flow through to :class:`TableRef`
    # and ultimately the planner's ``_try_index_scan``.
    index_hint: str | None = None
    not_indexed = False
    hint_node = _maybe_child(node, "index_hint")
    if hint_node is not None:
        if _has_keyword_child(hint_node, "NOT"):
            not_indexed = True
        else:
            # INDEXED BY NAME — the NAME is the only NAME-typed token
            # inside the hint node.
            name_tok = next(
                (c for c in hint_node.children
                 if isinstance(c, Token) and _token_type(c) == "NAME"),
                None,
            )
            if name_tok is not None:
                index_hint = name_tok.value
    return TableRef(
        table=table, alias=alias, index_hint=index_hint, not_indexed=not_indexed,
    )


def _join_clause(
    node: ASTNode,
    state: _PlaceholderCounter,
    ctes: dict[str, _CTEBody] | None = None,
    view_defs: dict[str, SelectStmt] | None = None,
) -> JoinClause:
    # join_clause has two forms (grammar):
    #   1. Explicit: [ join_type ] "JOIN" table_ref [ "ON" expr | "USING" (...) ]
    #   2. Comma:    "," table_ref  — implicit CROSS JOIN, no ON/USING condition.
    #
    # Detect the comma form by checking for a direct "," token child without a
    # "JOIN" keyword sibling.  When the comma form is present we immediately
    # return a CROSS JoinClause with no condition.
    has_join_kw = _has_keyword_child(node, "JOIN")
    if not has_join_kw:
        # Comma join — check for a "," token to be sure.
        is_comma_join = any(
            isinstance(c, Token) and c.value == ","
            for c in node.children
        )
        if is_comma_join:
            right = _table_ref(_child_node(node, "table_ref"), ctes=ctes, view_defs=view_defs)
            return JoinClause(kind=JoinKind.CROSS, right=right, on=None)

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


def _find_bare_collation(expr_node: ASTNode) -> str | None:
    """Walk down a single-spine ``expr`` AST to find an inner COLLATE.

    The grammar makes ``collated = bitwise [ "COLLATE" NAME ]`` the
    operand to ``comparison``.  When ORDER BY is followed by a bare
    expression like ``column1 COLLATE NOCASE``, the COLLATE is consumed
    by that inner ``collated`` rule before reaching the outer
    ``order_item``'s own COLLATE slot.  ``_comparison`` drops the
    collation on the bare-collated branch (because applying it via
    ``lower()``/``rtrim()`` there would change the column's display
    name).  That's the right call for SELECT-list expressions, but for
    ORDER BY we want the SortKey to carry the collation so the VM can
    apply the transform when building the sort key.

    This helper descends through the chain ``expr → or_expr → and_expr
    → not_expr → comparison → collated`` looking for the COLLATE
    keyword.  Stops at the first non-trivial wrapper (anything with
    multiple expression children, or any cmp_op / IN / LIKE / BETWEEN
    / IS / NOT keyword), since in those cases the collation belongs
    to the comparison context, not to a bare sort expression.
    """
    # The chain of rules we expect to traverse on a "bare" expression
    # (one with no binary operator, no comparison, no NOT, etc.).
    # We descend one level at a time; if we ever see anything other
    # than a single-child path, we bail.
    chain = (
        "expr",
        "or_expr",
        "and_expr",
        "not_expr",
        "comparison",
        "collated",
    )
    cursor: ASTNode | None = expr_node
    if cursor is None or cursor.rule_name != chain[0]:
        return None
    for next_rule in chain[1:]:
        if cursor is None:
            return None
        # The bare-expression branch has exactly one child of the next
        # rule.  If the comparison level has a cmp_op (etc.) we'd see
        # extra children — bail out and let _comparison handle the
        # collation as part of its rewrite.
        if next_rule == "collated":
            # At the comparison level: only descend to the inner
            # collated if there are no comparison operators present.
            has_op = (
                any(isinstance(c, ASTNode) and c.rule_name == "cmp_op" for c in cursor.children)
                or _has_keyword_child(cursor, "BETWEEN")
                or _has_keyword_child(cursor, "IN")
                or _has_keyword_child(cursor, "LIKE")
                or _has_keyword_child(cursor, "GLOB")
                or _has_keyword_child(cursor, "IS")
            )
            if has_op:
                return None
        # Look for an immediate child of next_rule.
        nxt: ASTNode | None = None
        for c in cursor.children:
            if isinstance(c, ASTNode) and c.rule_name == next_rule:
                if nxt is not None:
                    # Multiple children of this rule means a binary op
                    # at this level — not a bare expression.
                    return None
                nxt = c
        cursor = nxt
    # At ``collated``, look for the COLLATE keyword and the NAME after it.
    if cursor is None:
        return None
    if not _has_keyword_child(cursor, "COLLATE"):
        return None
    seen_collate = False
    for c in cursor.children:
        if _is_keyword(c, "COLLATE"):
            seen_collate = True
            continue
        if seen_collate and isinstance(c, Token) and _token_type(c) == "NAME":
            return c.value.upper()
    return None


def _order_item(node: ASTNode, state: _PlaceholderCounter) -> SortKey:
    # order_item = expr [ "COLLATE" NAME ] [ "ASC" | "DESC" ] [ "NULLS" NAME ]
    #
    # Direction defaults to ASC when no keyword is given.  NULL placement
    # defaults to ``None`` on SortKey, meaning "use SQLite default" (NULLs
    # first for ASC, NULLs last for DESC).  Explicit NULLS FIRST sets
    # ``nulls_first=True``; NULLS LAST sets ``nulls_first=False``.
    #
    # The NAME after NULLS must be either ``FIRST`` or ``LAST``
    # (case-insensitive).  We accept any NAME at the grammar level — it is
    # validated here — because making FIRST/LAST hard keywords would
    # forbid them as column names, which is impractical (``first_name``,
    # ``last`` are common identifiers).
    #
    # The optional ``COLLATE name`` clause names a comparison transform
    # (``BINARY`` / ``NOCASE`` / ``RTRIM`` in standard SQLite, plus any
    # user-registered ones).  We store the name verbatim on the SortKey
    # (upper-cased for consistency); the VM picks the matching transform
    # when building the sort key.  ``BINARY`` and ``None`` are
    # equivalent (the default).
    expr = _expr(_child_node(node, "expr"), state)
    descending = _has_keyword_child(node, "DESC")

    # Extract the COLLATE name (if any).  Two paths to find it:
    #
    #   1. ``order_item`` may carry an outer COLLATE/NAME pair directly
    #      (the grammar slot ``order_item = expr [ "COLLATE" NAME ] …``).
    #      This is rare in practice because the inner ``collated`` rule
    #      added in PR #3xxxx is matched greedily by the PEG parser, but
    #      it can still happen if the inner ``collated`` already saw a
    #      COLLATE and the user wrote a second one at the ORDER BY level
    #      (uncommon but legal).
    #
    #   2. More commonly, the COLLATE was consumed by the inner
    #      ``collated`` rule sitting under ``expr → … → comparison →
    #      collated``.  Walk the expr subtree to find that inner
    #      ``collated`` node and pull the COLLATE out of it.
    #
    # The latter path matters because ``_comparison`` drops the
    # collation when the collated is bare (no surrounding cmp_op /
    # BETWEEN / etc.).  That keeps the value semantics correct in
    # SELECT-list contexts but loses the sort-collation signal — which
    # we need to put back on the SortKey here.
    collation: str | None = None
    if _has_keyword_child(node, "COLLATE"):
        seen_collate = False
        for c in node.children:
            if _is_keyword(c, "COLLATE"):
                seen_collate = True
                continue
            if seen_collate and isinstance(c, Token) and _token_type(c) == "NAME":
                collation = c.value.upper()
                break
    if collation is None:
        collation = _find_bare_collation(_child_node(node, "expr"))

    nulls_first: bool | None = None
    if _has_keyword_child(node, "NULLS"):
        # Find the NAME token that follows the NULLS keyword.
        seen_nulls = False
        for c in node.children:
            if _is_keyword(c, "NULLS"):
                seen_nulls = True
                continue
            if seen_nulls and isinstance(c, Token) and _token_type(c) == "NAME":
                placement = c.value.upper()
                if placement == "FIRST":
                    nulls_first = True
                elif placement == "LAST":
                    nulls_first = False
                else:
                    raise ProgrammingError(
                        f"expected FIRST or LAST after NULLS in ORDER BY, "
                        f"got {c.value!r}"
                    )
                break
    return SortKey(
        expr=expr,
        descending=descending,
        nulls_first=nulls_first,
        collation=collation,
    )


def _signed_number(node: ASTNode) -> int:
    """Resolve a ``signed_number = [ "-" ] NUMBER`` node to a Python int.

    NUMBER tokens carry the raw source text; hex literals (``0x1F``)
    reach us as the original ``0x1F`` string, so we route through
    ``_parse_number`` which handles both decimal and hex spellings.
    Floats in LIMIT/OFFSET are rejected by SQLite at runtime, but the
    parser lets them through; we coerce to int and let the engine
    raise the same error sqlite3 would.

    SQLite documents a negative count as "no limit" — the meaning of
    the sign is preserved here and the caller (``_limit_clause``)
    translates a negative count to ``Limit.count=None`` (unbounded).
    """
    negative = False
    raw: str | None = None
    for c in node.children:
        if isinstance(c, Token):
            t = _token_type(c)
            if t == "MINUS":
                negative = True
            elif t == "NUMBER":
                raw = c.value
    if raw is None:
        raise ProgrammingError("signed_number missing NUMBER token")
    value = int(_parse_number(raw))
    return -value if negative else value


def _limit_clause(node: ASTNode | None) -> Limit | None:
    if node is None:
        return None
    # limit_clause = "LIMIT" signed_number
    #                [ "OFFSET" signed_number | "," signed_number ]
    #
    # Two trailing forms:
    #   * ``OFFSET N``  — count first, offset second (SQL standard)
    #   * ``, N``       — offset first, count second (MySQL-compatible)
    # We detect the comma form by the presence of a COMMA token and
    # swap the argument interpretation accordingly.
    signed_nums = [
        _signed_number(c)
        for c in node.children
        if isinstance(c, ASTNode) and c.rule_name == "signed_number"
    ]
    has_comma = any(
        isinstance(c, Token) and _token_type(c) == "COMMA" for c in node.children
    )

    if not signed_nums:
        return None

    if has_comma and len(signed_nums) == 2:
        # MySQL form: ``LIMIT offset, count``.  Swap so ``count`` and
        # ``offset`` carry their SQL-standard meaning downstream.
        offset_val, count_val = signed_nums[0], signed_nums[1]
    else:
        count_val = signed_nums[0]
        offset_val = signed_nums[1] if len(signed_nums) > 1 else None

    # SQLite: a negative count means "no limit" (unbounded).  Map that
    # to ``Limit.count=None`` which the planner/codegen already treat
    # as "do not emit LimitResult".
    count: int | None = None if count_val < 0 else count_val
    # Negative offsets are silently treated as zero in SQLite — match
    # that behaviour rather than passing the negative value through.
    offset: int | None
    if offset_val is None:
        offset = None
    elif offset_val < 0:
        offset = 0
    else:
        offset = offset_val
    return Limit(count=count, offset=offset)


# --------------------------------------------------------------------------
# INSERT / UPDATE / DELETE.
# --------------------------------------------------------------------------


def _returning_exprs(
    node: ASTNode, state: _PlaceholderCounter
) -> tuple[Expr, ...]:
    """Parse a returning_clause child of a DML statement node.

    Grammar (mini-sqlite 2.0+)::

        returning_clause = "RETURNING" returning_item { "," returning_item } ;
        returning_item   = "*" | expr ;

    The bare-``*`` form yields a :class:`Wildcard` sentinel; the
    planner expands it to every column of the target table at
    resolution time (same handling SELECT * uses).  Returns an empty
    tuple when no returning_clause child is present.
    """
    ret_node = _maybe_child(node, "returning_clause")
    if ret_node is None:
        return ()
    items: list[Expr] = []
    for c in ret_node.children:
        if isinstance(c, ASTNode) and c.rule_name == "returning_item":
            # ``*`` → Wildcard; ``expr`` → parsed expression.
            if any(_is_token(t, type_="STAR") for t in c.children):
                items.append(Wildcard())
            else:
                expr_node = _maybe_child(c, "expr")
                if expr_node is not None:
                    items.append(_expr(expr_node, state))
    return tuple(items)


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


def _upsert_clause(node: ASTNode, state: _PlaceholderCounter) -> UpsertClause | None:
    """Parse an optional ``upsert_clause`` child from an insert statement node.

    Grammar (from sql.grammar)::

        upsert_clause = "ON" "CONFLICT"
                        [ "(" NAME { "," NAME } ")" ]
                        ( "DO" "NOTHING"
                        | "DO" "UPDATE" "SET" upsert_assignment { "," upsert_assignment }
                          [ where_clause ] ) ;

        upsert_assignment = NAME "=" expr ;

    The optional trailing ``WHERE expr`` is SQLite's conditional-upsert
    extension: the DO UPDATE assignments fire only when the predicate is true.
    EXCLUDED column references inside that predicate are rewritten just like
    they are in assignment RHS expressions.

    Returns ``None`` when no ``upsert_clause`` child is present.

    EXCLUDED pseudo-table rewriting
    ---------------------------------
    The grammar parses ``EXCLUDED.col`` as a normal ``column_ref`` (two-part
    NAME.NAME), which the ``_expr`` helper turns into ``Column(table="EXCLUDED",
    col=col)``.  This function detects that sentinel and rewrites it to the
    dedicated ``ExcludedColumn(col=col)`` node so that the planner and codegen
    can pattern-match on it cleanly without string comparisons.
    """
    uc = _maybe_child(node, "upsert_clause")
    if uc is None:
        return None

    # Collect conflict target column names from optional "(NAME, ...)" part.
    conflict_target: list[str] = []
    in_target = False
    for child in uc.children:
        if _is_token(child, type_="LPAREN"):
            in_target = True
        elif _is_token(child, type_="RPAREN"):
            in_target = False
        elif in_target and isinstance(child, Token) and _token_type(child) == "NAME":
            conflict_target.append(child.value)

    # Determine action: DO NOTHING or DO UPDATE SET ...
    do_nothing = False
    assignments: list[AstUpsertAssignment] = []

    # Scan for NOTHING keyword (DO NOTHING branch)
    for child in uc.children:
        is_nothing_kw = (
            isinstance(child, Token)
            and _token_type(child) == "KEYWORD"
            and child.value.upper() == "NOTHING"
        )
        if is_nothing_kw:
            do_nothing = True
            break

    where_expr: Expr | None = None
    if not do_nothing:
        # DO UPDATE SET upsert_assignment { "," upsert_assignment } [ where_clause ]
        for child in uc.children:
            if isinstance(child, ASTNode) and child.rule_name == "upsert_assignment":
                # upsert_assignment = NAME "=" expr
                col_tok = _first_token(child, kind="NAME")
                assert col_tok is not None, "upsert_assignment missing column name"
                expr_node = _maybe_child(child, "expr")
                assert expr_node is not None, "upsert_assignment missing expr"
                raw_val = _expr(expr_node, state)
                # Rewrite Column(table="EXCLUDED", col=c) \u2192 ExcludedColumn(col=c)
                val = _rewrite_excluded(raw_val)
                assignments.append(AstUpsertAssignment(column=col_tok.value, value=val))

        # Optional trailing WHERE predicate (SQLite conditional-upsert).
        wc = _maybe_child(uc, "where_clause")
        if wc is not None:
            we = _maybe_child(wc, "expr")
            if we is not None:
                where_expr = _rewrite_excluded(_expr(we, state))

    return UpsertClause(
        conflict_target=tuple(conflict_target),
        do_nothing=do_nothing,
        assignments=tuple(assignments),
        where=where_expr,
    )


def _rewrite_excluded(expr: Expr) -> Expr:
    """Rewrite ``Column(table="EXCLUDED", col=c)`` to ``ExcludedColumn(col=c)``.

    The grammar's ``column_ref = NAME [ "." NAME ]`` rule parses ``EXCLUDED.col``
    as a two-part column reference.  The adapter turns that into a plain
    ``Column`` node; this helper post-processes the expression tree so that the
    ``EXCLUDED`` pseudo-table becomes the dedicated ``ExcludedColumn`` IR node.

    The table-name match is case-insensitive — SQLite accepts ``EXCLUDED``,
    ``excluded``, and ``Excluded`` interchangeably as the pseudo-table name in
    upsert assignments.

    All other expression types are returned unchanged.  We only need to descend
    into the top-level and binary/unary positions where EXCLUDED.col might
    appear inside a upsert assignment value.
    """
    match expr:
        case Column(table=t, col=c) if t is not None and t.upper() == "EXCLUDED":
            return ExcludedColumn(col=c)
        case BinaryExpr(op=op, left=left, right=right):
            return BinaryExpr(op=op, left=_rewrite_excluded(left), right=_rewrite_excluded(right))
        case UnaryExpr(op=uop, operand=inner):
            # WHERE predicates often use NOT / unary minus; descend through them
            # so EXCLUDED references nested inside (e.g., ``NOT excluded.flag``)
            # still get rewritten.
            return UnaryExpr(op=uop, operand=_rewrite_excluded(inner))
        case _:
            # For the upsert use-case, only literal values and EXCLUDED column
            # refs (possibly wrapped in binary/unary operators) are expected;
            # a full recursive walk over all Expr variants is overkill.
            return expr


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
                       insert_body [ upsert_clause ] [ returning_clause ] ;
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

    # Parse optional ON CONFLICT upsert clause.
    upsert = _upsert_clause(node, state)

    # Check if we have an insert_body child (new grammar).
    insert_body_node = _maybe_child(node, "insert_body")
    returning = _returning_exprs(node, state)
    if insert_body_node is not None:
        # New grammar:
        #   insert_body = "VALUES" row_value ...
        #              | "DEFAULT" "VALUES"
        #              | query_stmt
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
                upsert_clause=upsert,
            )
        # ``DEFAULT VALUES`` form \u2014 insert a single row consisting entirely
        # of column defaults.  Equivalent to ``INSERT INTO t () VALUES ()``.
        # Detected by the presence of a ``DEFAULT`` keyword child where no
        # ``row_value`` children appear.
        if _has_keyword_child(insert_body_node, "DEFAULT"):
            return InsertValuesStmt(
                table=table, columns=(), rows=((),),
                on_conflict=on_conflict, returning=returning,
                upsert_clause=upsert,
            )
        rows = tuple(_row_value(rv, state) for rv in _child_nodes(insert_body_node, "row_value"))
        return InsertValuesStmt(
            table=table, columns=columns, rows=rows,
            on_conflict=on_conflict, returning=returning,
            upsert_clause=upsert,
        )

    # Old grammar fallback: row_value nodes directly under insert_stmt.
    rows = tuple(_row_value(rv, state) for rv in _child_nodes(node, "row_value"))
    return InsertValuesStmt(
        table=table, columns=columns, rows=rows,
        on_conflict=on_conflict, returning=returning,
        upsert_clause=upsert,
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
    """Parse an ``alter_table_stmt`` node.

    Grammar (one of four forms)::

        alter_table_stmt = "ALTER" "TABLE" NAME (
              "ADD" [ "COLUMN" ] col_def
            | "RENAME" "TO" NAME
            | "RENAME" [ "COLUMN" ] NAME "TO" NAME
            | "DROP" [ "COLUMN" ] NAME
        )

    The first NAME is always the table being altered.  We dispatch on
    the second keyword (ADD / RENAME / DROP).  For RENAME we further
    dispatch on whether a ``TO`` keyword appears before any other NAME
    (RENAME TO new_name) or after a NAME (RENAME [COLUMN] old TO new).
    """
    table_tok = _first_token(node, kind="NAME")
    assert table_tok is not None
    table_name = table_tok.value

    # ADD [COLUMN] col_def — recognise via the col_def child.
    col_node = _maybe_child(node, "col_def")
    if col_node is not None:
        col = _col_def(col_node, _PlaceholderCounter())
        return AlterTableStmt(table=table_name, column=col)

    # The remaining forms have no col_def — dispatch on keywords.
    has_rename = _has_keyword_child(node, "RENAME")
    has_drop = _has_keyword_child(node, "DROP")
    has_to = _has_keyword_child(node, "TO")

    # Collect the NAME tokens after the table name in source order.
    names: list[str] = []
    seen_table = False
    for c in node.children:
        if isinstance(c, Token) and _token_type(c) == "NAME":
            if not seen_table:
                seen_table = True  # this is the table-name token
                continue
            names.append(c.value)

    if has_rename and has_to:
        # Two flavours:
        #   RENAME TO new_name           → exactly one extra NAME
        #   RENAME [COLUMN] old TO new   → exactly two extra NAMEs
        if len(names) == 1:
            return AlterTableStmt(table=table_name, rename_to=names[0])
        if len(names) == 2:
            old, new = names
            return AlterTableStmt(table=table_name, rename_column=(old, new))
        raise ProgrammingError(
            f"malformed ALTER TABLE RENAME: expected 1 or 2 NAMEs after "
            f"table, got {len(names)}"
        )

    if has_drop:
        # DROP [COLUMN] col_name — exactly one extra NAME.
        if len(names) != 1:
            raise ProgrammingError(
                f"malformed ALTER TABLE DROP COLUMN: expected 1 NAME, "
                f"got {len(names)}"
            )
        return AlterTableStmt(table=table_name, drop_column=names[0])

    raise ProgrammingError("alter_table_stmt: unrecognised operation")


# --------------------------------------------------------------------------
# CREATE TABLE / DROP TABLE.
# --------------------------------------------------------------------------


def _create_table(node: ASTNode) -> CreateTableStmt:
    # create_table_stmt =
    #   "CREATE" "TABLE" ["IF" "NOT" "EXISTS"] NAME
    #   "(" col_def { "," col_def } ")"
    #   [ table_options ]
    #
    # ``table_options = table_option {"," table_option}`` and
    # ``table_option = "STRICT" | "WITHOUT" NAME``.  We currently honour
    # ``STRICT`` (forwarded to the backend); ``WITHOUT ROWID`` is parsed
    # but silently ignored — mini-sqlite always uses a rowid table.
    if_not_exists = _has_keyword_sequence(node, ("IF", "NOT", "EXISTS"))
    table_tok = _first_token(node, kind="NAME")
    assert table_tok is not None
    state = _PlaceholderCounter()
    cols = tuple(_col_def(c, state) for c in _child_nodes(node, "col_def"))
    strict = False
    opts_node = _maybe_child(node, "table_options")
    if opts_node is not None:
        for opt in _child_nodes(opts_node, "table_option"):
            if _has_keyword_child(opt, "STRICT"):
                strict = True
                break
    return CreateTableStmt(
        table=table_tok.value,
        columns=cols,
        if_not_exists=if_not_exists,
        strict=strict,
    )


def _col_def(node: ASTNode, state: _PlaceholderCounter | None = None) -> BackendColumnDef:
    # col_def = NAME col_type { col_constraint }
    # col_type = NAME [ "(" NUMBER { "," NUMBER } ")" ]
    #
    # The column name is the first direct NAME child of col_def.  The type name
    # lives inside the col_type child node.  We ignore any length/precision
    # parameters inside col_type — e.g. VARCHAR(8) is treated as VARCHAR — and
    # apply SQLite's type-affinity rules to map the type to the backend's
    # internal representation.
    col_name_token = next(
        (c for c in node.children if isinstance(c, Token) and _token_type(c) == "NAME"), None
    )
    col_name = col_name_token.value if col_name_token else ""

    # Try the new col_type child node first, then fall back to the legacy
    # two-NAME layout for any grammars that haven't been regenerated yet.
    col_type_node = _maybe_child(node, "col_type")
    if col_type_node is not None:
        type_token = next(
            (
                c
                for c in col_type_node.children
                if isinstance(c, Token) and _token_type(c) == "NAME"
            ),
            None,
        )
        type_name = type_token.value.upper() if type_token else "TEXT"
    else:
        # Legacy: second NAME directly under col_def.
        names = [c for c in node.children if isinstance(c, Token) and _token_type(c) == "NAME"]
        type_name = names[1].value.upper() if len(names) > 1 else "TEXT"

    not_null = False
    primary_key = False
    autoincrement = False
    unique = False
    check_expression = None
    check_expr_text: str = ""
    foreign_key: tuple[str, str | None] | None = None
    col_default = NO_DEFAULT   # "no DEFAULT clause" sentinel
    collation: str | None = None  # COLLATE clause on the column def
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
            # Don't set ``not_null = True`` here: ``primary_key=True``
            # is already enough — ``ColumnDef.effective_not_null()``
            # treats PK as implicit NOT NULL for constraint-validation
            # purposes.  Leaving the raw ``not_null`` field False lets
            # ``PRAGMA table_info`` distinguish PK-implied NULL-ness
            # from explicit NOT NULL declarations (matching SQLite,
            # which reports ``notnull = 0`` for ``id INTEGER PRIMARY
            # KEY`` and ``notnull = 1`` only when the user wrote both
            # ``PRIMARY KEY NOT NULL``).
            primary_key = True
        elif kw_seq == ("PRIMARY", "KEY", "AUTOINCREMENT"):
            # SQLite: AUTOINCREMENT is only valid after PRIMARY KEY on
            # an INTEGER column.  Same NOT NULL story as the
            # PRIMARY-KEY-only branch: leave ``not_null`` False so the
            # pragma surfaces the explicit-vs-implicit distinction.
            primary_key = True
            autoincrement = True
        elif kw_seq == ("UNIQUE",):
            unique = True
        elif kw_seq[0:1] == ("CHECK",):
            expr_node = _maybe_child(c, "expr")
            if expr_node is not None:
                check_expression = _expr(expr_node, _state)
                # Capture the source-ish text of the CHECK predicate so
                # the VM can surface it in constraint-violation errors —
                # matches SQLite's ``CHECK constraint failed: a > 0``.
                check_expr_text = _render_expr_text(expr_node)
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
        elif kw_seq[0:1] == ("COLLATE",):
            # col_constraint grammar: "COLLATE" NAME
            # The NAME is the collation name (BINARY / NOCASE / RTRIM,
            # or any user-defined name).  Stored upper-cased on the
            # column definition; the planner consults it as the default
            # collation when an ORDER BY references the column without
            # an explicit COLLATE override.
            name_token = next(
                (t for t in c.children if isinstance(t, Token) and _token_type(t) == "NAME"),
                None,
            )
            if name_token is not None:
                collation = name_token.value.upper()
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
        autoincrement=autoincrement,
        unique=unique,
        default=col_default,
        check_expr=check_expression,
        check_expr_text=check_expr_text,
        foreign_key=foreign_key,
        collation=collation,
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
            "ON" NAME "(" index_col { "," index_col } ")" ;
        index_col = NAME [ "ASC" | "DESC" ] ;

    index_name and table_name are direct NAME tokens on the statement node.
    Column names are extracted from ``index_col`` child nodes; ASC/DESC hints
    are accepted for SQLite compatibility but ignored by the backend.
    """
    unique = _has_keyword_child(node, "UNIQUE")
    if_not_exists = _has_keyword_sequence(node, ("IF", "NOT", "EXISTS"))

    # Direct NAME tokens on the statement are: index_name and table_name.
    # (Column names are inside index_col child nodes since the grammar change.)
    direct_names = [
        c.value
        for c in node.children
        if isinstance(c, Token) and _token_type(c) == "NAME"
    ]
    if len(direct_names) < 2:
        raise ProgrammingError(
            "create_index_stmt: expected index_name and table_name"
        )

    index_name = direct_names[0]
    table_name = direct_names[1]

    # Column names come from index_col child nodes (new grammar).
    # Each ``index_col`` may be a NAME, a function_call, or a parenthesised
    # expression — the latter two are SQLite "indexed expressions" (e.g.
    # ``CREATE INDEX idx ON t(LOWER(name))``).  Mini-sqlite parses the
    # expression but indexes only when the index_col is a bare NAME; expression
    # indices are accepted-and-ignored (still listed in PRAGMA index_list, but
    # the underlying index is empty / not consulted by the optimizer).  This
    # unlocks ORM/migration code that issues such indexes without affecting
    # correctness (only performance).
    #
    # Strategy: synthesise a synthetic column name from the expression so the
    # IndexDef.columns tuple has the right arity.  Real lookups against bare
    # NAME columns still work; lookups against the expression do not match the
    # index (but the planner doesn't try, so it transparently falls back to a
    # table scan).
    #
    # COLLATE clause is silently discarded.  Only the BINARY collation is
    # implemented; the index behaves as if no COLLATE was specified.
    index_col_nodes = _child_nodes(node, "index_col")
    if index_col_nodes:
        col_names: list[str] = []
        for i, ic in enumerate(index_col_nodes):
            # The index_col body is `expr [COLLATE NAME] [ASC|DESC]`.  Detect
            # the simple "bare column" case where the expression is a single
            # column_ref (with no operator / function / parens around it).
            # Anything more complex (LOWER(col), col+1, etc.) is a SQLite
            # "indexed expression" — accept-and-ignore by assigning a synthetic
            # column name so the index registers but no lookups match.
            bare_name = _extract_bare_column_name(ic)
            if bare_name is not None:
                col_names.append(bare_name)
            else:
                col_names.append(f"__expr_{i}")
        columns: tuple[str, ...] = tuple(col_names)
    else:
        columns = tuple(direct_names[2:])

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
            [ "FOR" "EACH" "ROW" ]
            "BEGIN" trigger_body_stmt ";" { trigger_body_stmt ";" } "END" ;

    SQLite makes ``FOR EACH ROW`` optional (it has been the only granularity
    since SQLite has no statement-level triggers, so the clause is redundant).
    The grammar now accepts it as an optional clause; the adapter ignores it
    either way because it uses keyword scanning — FOR/EACH/ROW are not
    surfaced as KEYWORD tokens by the lexer.

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
    window_defs: dict = field(default_factory=dict)  # name → window_spec ASTNode

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


def _collation_transform(expr: Expr, collation: str) -> Expr:
    """Wrap *expr* in the scalar-function call that applies *collation*.

    SQLite's three built-in collations correspond exactly to mini-sqlite's
    existing scalar functions:

    * ``BINARY``  — no transform (identity)
    * ``NOCASE``  — ``lower(expr)`` (ASCII case-insensitive)
    * ``RTRIM``   — ``rtrim(expr)`` (strip trailing spaces)

    Unknown collation names fall through to identity, matching SQLite's
    "validate lazily" approach (the user may have registered a custom
    collation we don't know about; the comparison just runs un-collated).
    """
    coll = collation.upper()
    if coll == "NOCASE":
        return FunctionCall(name="lower", args=(FuncArg(value=expr),))
    if coll == "RTRIM":
        return FunctionCall(name="rtrim", args=(FuncArg(value=expr),))
    # BINARY (and any unknown name) → identity.
    return expr


def _collated(node: ASTNode, state: _PlaceholderCounter) -> tuple[Expr, str | None]:
    """Translate ``collated = bitwise [ "COLLATE" NAME ]``.

    Returns ``(expr, collation_name_or_None)``.  The collation name is
    stripped from the result and returned separately so the caller (the
    comparison rule) can propagate it across both operands — SQLite's
    semantics is that a collation attached to either side of a
    comparison applies to the *comparison*, not just to that operand.

    For a bare ``bitwise`` (no COLLATE), this is equivalent to calling
    ``_bitwise`` directly and returning ``None`` for the collation.
    """
    bw = _child_node(node, "bitwise")
    expr = _bitwise(bw, state)
    if not _has_keyword_child(node, "COLLATE"):
        return expr, None
    # Find the NAME token that follows COLLATE.
    seen_collate = False
    for c in node.children:
        if _is_keyword(c, "COLLATE"):
            seen_collate = True
            continue
        if seen_collate and isinstance(c, Token) and _token_type(c) == "NAME":
            return expr, c.value.upper()
    return expr, None


def _lex_cmp(lhs: list[Expr], rhs: list[Expr], strict_op: BinaryOp, final_op: BinaryOp) -> Expr:
    """Build a lexicographic comparison for row values (iterative, right-to-left).

    Truth table for ``(a, b) < (x, y)`` (strict_op=LT, final_op=LT):

    +-------+-------+-------+--------+
    | a < x | a = x | b < y | result |
    +-------+-------+-------+--------+
    | TRUE  |   -   |   -   | TRUE   |
    | FALSE | TRUE  | TRUE  | TRUE   |
    | FALSE | TRUE  | FALSE | FALSE  |
    | FALSE | FALSE |   -   | FALSE  |
    +-------+-------+-------+--------+

    Expands to ``a < x OR (a = x AND b < y)``.

    Built right-to-left to avoid Python recursion limits on wide row values.
    """
    result: Expr = BinaryExpr(op=final_op, left=lhs[-1], right=rhs[-1])
    for lv, rv in zip(reversed(lhs[:-1]), reversed(rhs[:-1]), strict=True):
        result = BinaryExpr(
            op=BinaryOp.OR,
            left=BinaryExpr(op=strict_op, left=lv, right=rv),
            right=BinaryExpr(
                op=BinaryOp.AND,
                left=BinaryExpr(op=BinaryOp.EQ, left=lv, right=rv),
                right=result,
            ),
        )
    return result


def _expand_row_value_cmp(lhs: list[Expr], op: BinaryOp, rhs: list[Expr]) -> Expr:
    """Expand a row-value comparison ``(a, b, …) op (x, y, …)`` to scalars.

    Semantics mirror SQLite's row-value specification:

    * ``=``  → ``a=x AND b=y AND …``
    * ``!=`` → ``a!=x OR b!=y OR …``  (any differing column → unequal)
    * ``<``  → lexicographic: ``a<x OR (a=x AND b<y) OR …``
    * ``<=`` → ``a<x OR (a=x AND b<=y) OR …``
    * ``>``  → symmetric to ``<``
    * ``>=`` → symmetric to ``<=``
    """
    n = len(lhs)
    if n == 0:
        return Literal(value=1 if op == BinaryOp.EQ else 0)
    if n != len(rhs):
        raise ProgrammingError(
            f"row value misuse: left side has {n} column(s), right side has {len(rhs)}"
        )
    if op == BinaryOp.EQ:
        result: Expr = BinaryExpr(op=BinaryOp.EQ, left=lhs[0], right=rhs[0])
        for lv, rv in zip(lhs[1:], rhs[1:], strict=True):
            result = BinaryExpr(
                op=BinaryOp.AND, left=result,
                right=BinaryExpr(op=BinaryOp.EQ, left=lv, right=rv),
            )
        return result
    if op == BinaryOp.NOT_EQ:
        result = BinaryExpr(op=BinaryOp.NOT_EQ, left=lhs[0], right=rhs[0])
        for lv, rv in zip(lhs[1:], rhs[1:], strict=True):
            result = BinaryExpr(
                op=BinaryOp.OR, left=result,
                right=BinaryExpr(op=BinaryOp.NOT_EQ, left=lv, right=rv),
            )
        return result
    if op in (BinaryOp.LT, BinaryOp.LTE):
        return _lex_cmp(lhs, rhs, BinaryOp.LT, op)
    if op in (BinaryOp.GT, BinaryOp.GTE):
        return _lex_cmp(lhs, rhs, BinaryOp.GT, op)
    raise ProgrammingError(f"unsupported row-value comparison operator: {op}")


def _expand_row_value_in(
    lhs: list[Expr],
    candidates: list[list[Expr]],
    negated: bool,
) -> Expr:
    """Expand ``(a, b) IN ((x, y), (p, q))`` to ``(a=x AND b=y) OR (a=p AND b=q)``.

    The empty-list case ``(a, b) IN ()`` expands to the constant FALSE (0),
    and ``(a, b) NOT IN ()`` expands to TRUE (1), matching SQLite's semantics.
    """
    if not candidates:
        always_false: Expr = Literal(value=0)
        return UnaryExpr(op=UnaryOp.NOT, operand=always_false) if negated else always_false
    clauses: list[Expr] = [_expand_row_value_cmp(lhs, BinaryOp.EQ, cand) for cand in candidates]
    result: Expr = clauses[0]
    for clause in clauses[1:]:
        result = BinaryExpr(op=BinaryOp.OR, left=result, right=clause)
    return UnaryExpr(op=UnaryOp.NOT, operand=result) if negated else result


def _comparison(node: ASTNode, state: _PlaceholderCounter) -> Expr:
    """Comparison covers: bare collated, cmp_op, BETWEEN, IN, LIKE, IS NULL.

    Grammar (after row-value extension):
    ``comparison = row_value cmp_op row_value
                 | row_value [NOT] IN ( row_value_list )
                 | collated [cmp_op collated | "BETWEEN" ... | ...]``.

    Row-value forms are expanded to equivalent scalar BinaryExpr trees so
    the planner and VM require no changes.  Scalar forms are unchanged.
    """
    # Row-value comparison: first ASTNode child is a row_value, not a collated.
    first_rv = next(
        (c for c in node.children if isinstance(c, ASTNode) and c.rule_name == "row_value"),
        None,
    )
    if first_rv is not None:
        row_value_nodes = [
            c for c in node.children
            if isinstance(c, ASTNode) and c.rule_name == "row_value"
        ]
        lhs_exprs = list(_row_value(row_value_nodes[0], state))
        cmp = _maybe_child(node, "cmp_op")
        if cmp is not None:
            op = _cmp_op_to_binop(cmp)
            rhs_exprs = list(_row_value(row_value_nodes[1], state))
            return _expand_row_value_cmp(lhs_exprs, op, rhs_exprs)
        negated = _has_keyword_child(node, "NOT")
        rv_list_node = _maybe_child(node, "row_value_list")
        if rv_list_node is not None:
            cand_nodes = [
                c for c in rv_list_node.children
                if isinstance(c, ASTNode) and c.rule_name == "row_value"
            ]
            candidates = [list(_row_value(rv, state)) for rv in cand_nodes]
            return _expand_row_value_in(lhs_exprs, candidates, negated)
        raise ProgrammingError("malformed row-value comparison")

    collateds = [c for c in node.children if isinstance(c, ASTNode) and c.rule_name == "collated"]
    left, left_coll = _collated(collateds[0], state)

    # Bare collated → pass through.  A trailing COLLATE on a non-
    # comparison expression is a no-op for value semantics (the value
    # itself is unchanged); the collation only matters when used as a
    # sort key or as a comparison operand.  ORDER BY captures the
    # collation via its own ``[ "COLLATE" NAME ]`` slot in
    # ``order_item`` (see ``_order_item``); the comparison forms below
    # handle the collation as part of building the BinaryExpr.
    #
    # For the bare case, we drop the collation rather than wrapping in
    # a scalar function call, because doing so would change the
    # column's display name from ``column1`` to ``lower(column1)``,
    # which then breaks any caller that looks up the column by its
    # original name (notably the VM's sort key resolver).
    if len(collateds) == 1 and not any(
        isinstance(c, ASTNode) and c.rule_name == "cmp_op" for c in node.children
    ) and not _has_keyword_child(node, "BETWEEN") and not _has_keyword_child(node, "IN") \
       and not _has_keyword_child(node, "LIKE") and not _has_keyword_child(node, "GLOB") \
       and not _has_keyword_child(node, "IS"):
        return left

    # cmp_op form.  SQLite says: a COLLATE attached to either side
    # propagates to the comparison.  If neither side has an explicit
    # collation, the comparison is BINARY.  If both sides have an
    # explicit collation, the LEFT one wins (SQLite docs: "If both
    # operands carry a COLLATE clause, the left one is used").
    cmp = _maybe_child(node, "cmp_op")
    if cmp is not None:
        op = _cmp_op_to_binop(cmp)
        right, right_coll = _collated(collateds[1], state)
        coll = left_coll if left_coll is not None else right_coll
        if coll is not None:
            left = _collation_transform(left, coll)
            right = _collation_transform(right, coll)
        return BinaryExpr(op=op, left=left, right=right)

    # BETWEEN / NOT BETWEEN.  The collation propagates to *both* range
    # bounds when attached to the operand; the bounds carry their own
    # COLLATE separately if specified.  We use the leftmost non-None
    # collation across the three operands.
    if _has_keyword_child(node, "BETWEEN"):
        negated = _has_keyword_child(node, "NOT")
        low, low_coll = _collated(collateds[1], state)
        high, high_coll = _collated(collateds[2], state)
        coll = left_coll if left_coll is not None else (
            low_coll if low_coll is not None else high_coll
        )
        if coll is not None:
            left = _collation_transform(left, coll)
            low = _collation_transform(low, coll)
            high = _collation_transform(high, coll)
        expr: Expr = Between(operand=left, low=low, high=high)
        return UnaryExpr(op=UnaryOp.NOT, operand=expr) if negated else expr

    # IN / NOT IN.
    if _has_keyword_child(node, "IN"):
        negated = _has_keyword_child(node, "NOT")
        # Apply any COLLATE attached to the LHS (the test operand).  We
        # do *not* propagate it into the value list — that would change
        # value semantics; if a user wants case-insensitive IN, the
        # SQLite idiom is ``lower(x) IN ('a','b')`` directly.
        if left_coll is not None:
            left = _collation_transform(left, left_coll)
        # The grammar wraps the list in an optional in_expr node.
        # When in_expr is absent the parentheses are empty — `IN ()` — which
        # SQLite defines as always-false (IN) / always-true (NOT IN).
        in_expr_node = _maybe_child(node, "in_expr")
        if in_expr_node is None:
            # Empty IN list: `x IN ()` is always FALSE, `x NOT IN ()` always TRUE.
            # Model as In/NotIn with an empty values tuple.
            if negated:
                return NotIn(operand=left, values=())
            return In(operand=left, values=())
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
        values = tuple(
            _expr(c, state) for c in vl.children if isinstance(c, ASTNode) and c.rule_name == "expr"
        )
        if negated:
            return NotIn(operand=left, values=values)
        return In(operand=left, values=values)

    # LIKE / NOT LIKE — pattern is typically a string literal, but SQLite also
    # accepts a NULL pattern (which makes the predicate always NULL → no rows
    # match, matching three-valued logic).
    #
    # An optional ESCAPE clause supplies a single-character literal that
    # disables wildcard meaning for the following character in the pattern.
    if _has_keyword_child(node, "LIKE"):
        negated = _has_keyword_child(node, "NOT")
        pat_expr, _ = _collated(collateds[1], state)
        # NULL pattern: LIKE NULL always yields NULL (no rows satisfy WHERE).
        if isinstance(pat_expr, Literal) and pat_expr.value is None:
            return Literal(value=None)
        if not isinstance(pat_expr, Literal) or not isinstance(pat_expr.value, str):
            raise ProgrammingError("LIKE pattern must be a string literal")
        # ESCAPE 'c' — third collated is the escape character.  It must be a
        # single-character string literal; SQLite raises "ESCAPE expression
        # must be a single character" otherwise.
        escape_char: str | None = None
        if _has_keyword_child(node, "ESCAPE") and len(collateds) >= 3:
            esc_expr, _ = _collated(collateds[2], state)
            if not isinstance(esc_expr, Literal) or not isinstance(esc_expr.value, str):
                raise ProgrammingError("ESCAPE expression must be a string literal")
            if len(esc_expr.value) != 1:
                raise ProgrammingError("ESCAPE expression must be a single character")
            escape_char = esc_expr.value
        if negated:
            return NotLike(operand=left, pattern=pat_expr.value, escape=escape_char)
        return Like(operand=left, pattern=pat_expr.value, escape=escape_char)

    # GLOB / NOT GLOB — case-sensitive pattern match using Unix glob syntax.
    #
    # SQL:  string GLOB pattern
    # Internal: glob(pattern, string)  — same argument order as SQLite's C API.
    if _has_keyword_child(node, "GLOB"):
        negated = _has_keyword_child(node, "NOT")
        pat_expr, _ = _collated(collateds[1], state)
        glob_call: Expr = FunctionCall(
            name="glob",
            args=(FuncArg(value=pat_expr), FuncArg(value=left)),
        )
        if negated:
            return UnaryExpr(op=UnaryOp.NOT, operand=glob_call)
        return glob_call

    # IS NULL / IS NOT NULL / IS [NOT] DISTINCT FROM / IS [NOT] <expr>.
    if _has_keyword_child(node, "IS"):
        if _has_keyword_child(node, "DISTINCT"):
            # "IS [NOT] DISTINCT FROM collated"
            right, right_coll = _collated(collateds[1], state)
            coll = left_coll if left_coll is not None else right_coll
            if coll is not None:
                left = _collation_transform(left, coll)
                right = _collation_transform(right, coll)
            if _has_keyword_child(node, "NOT"):
                return BinaryExpr(op=BinaryOp.IS_NOT_DISTINCT_FROM, left=left, right=right)
            return BinaryExpr(op=BinaryOp.IS_DISTINCT_FROM, left=left, right=right)
        # ``IS NULL`` / ``IS NOT NULL`` — detect by the presence of NULL in
        # the keyword tail.  The bare-RHS form (``IS <expr>``) has exactly
        # two ``collated`` children; the NULL form has only one.
        if len(collateds) == 1:
            if _has_keyword_child(node, "NOT"):
                return IsNotNull(operand=left)
            return IsNull(operand=left)
        # ``x IS y`` / ``x IS NOT y`` for arbitrary RHS — SQLite's NULL-safe
        # equality.  ``IS`` ≡ ``IS NOT DISTINCT FROM`` and ``IS NOT`` ≡
        # ``IS DISTINCT FROM``.  Routed through the existing IS_[NOT_]DISTINCT_FROM
        # planner/codegen/VM paths so no downstream changes are needed.
        right, right_coll = _collated(collateds[1], state)
        coll = left_coll if left_coll is not None else right_coll
        if coll is not None:
            left = _collation_transform(left, coll)
            right = _collation_transform(right, coll)
        if _has_keyword_child(node, "NOT"):
            return BinaryExpr(op=BinaryOp.IS_DISTINCT_FROM, left=left, right=right)
        return BinaryExpr(op=BinaryOp.IS_NOT_DISTINCT_FROM, left=left, right=right)

    return left


def _bitwise(node: ASTNode, state: _PlaceholderCounter) -> Expr:
    """Handle the ``bitwise`` grammar rule.

    Grammar::

        bitwise = additive { ( "&" | "|" | "<<" | ">>" ) additive }

    SQLite defines four binary bitwise operators (``&``, ``|``, ``<<``,
    ``>>``) at a single precedence level sitting *between* additive and
    comparison.  All four are left-associative.  Operands are coerced to
    64-bit signed integers (per SQLite's CAST-to-INTEGER rules) before the
    bitwise operation is applied; the actual coercion lives in the VM —
    here we just build the typed expression tree.
    """
    return _left_assoc_punct(
        node,
        "additive",
        _additive,
        {
            "BIT_AND_OP": BinaryOp.BIT_AND,
            "BIT_OR_OP": BinaryOp.BIT_OR,
            "SHIFT_LEFT": BinaryOp.BIT_SHL,
            "SHIFT_RIGHT": BinaryOp.BIT_SHR,
        },
        state,
    )


def _additive(node: ASTNode, state: _PlaceholderCounter) -> Expr:
    """Handle the ``additive`` grammar rule.

    Grammar::

        additive = multiplicative
                   { ( "+" | "-" | "||" | JSON_ARROW | JSON_ARROW_TEXT )
                     multiplicative }

    ``+``, ``-``, and ``||`` are ordinary binary operators (BinaryExpr).
    ``->`` and ``->>`` are SQLite 3.38+ JSON path-shortcut operators; they
    translate at this layer into function calls to the built-in scalar
    helpers ``__json_arrow`` and ``__json_arrow_text`` respectively.  The
    helpers internally normalise their second argument to a JSON path:

    * an integer ``N`` becomes ``$[N]`` (array index)
    * a string ``"a"`` becomes ``$.a`` (object key)
    * a string already starting with ``$`` is used as-is

    ``->`` returns the result re-encoded as JSON text (matching SQLite's
    JSON-typed return); ``->>`` returns the unwrapped SQL scalar.
    """
    children = node.children
    subs = [c for c in children if isinstance(c, ASTNode) and c.rule_name == "multiplicative"]
    if len(subs) == 1:
        return _multiplicative(subs[0], state)

    result = _multiplicative(subs[0], state)
    sub_idx = 1
    arrow_token_map = {
        "PLUS": BinaryOp.ADD,
        "MINUS": BinaryOp.SUB,
        "CONCAT_OP": BinaryOp.CONCAT,
    }
    for c in children:
        if not isinstance(c, Token):
            continue
        ttype = _token_type(c)
        if ttype in arrow_token_map and sub_idx < len(subs):
            op = arrow_token_map[ttype]
            result = BinaryExpr(op=op, left=result, right=_multiplicative(subs[sub_idx], state))
            sub_idx += 1
        elif ttype in ("JSON_ARROW", "JSON_ARROW_TEXT") and sub_idx < len(subs):
            # `j -> p` and `j ->> p` desugar to function calls so the codegen
            # can route them through the existing scalar-function dispatcher.
            fn_name = "__json_arrow" if ttype == "JSON_ARROW" else "__json_arrow_text"
            rhs = _multiplicative(subs[sub_idx], state)
            result = FunctionCall(
                name=fn_name,
                args=(FuncArg(value=result), FuncArg(value=rhs)),
            )
            sub_idx += 1
    return result


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
    # unary = ( "-" | "~" | "+" ) unary | primary
    #
    # SQLite supports three unary prefix operators at the same precedence level:
    #   -x   arithmetic negation
    #   ~x   bitwise NOT (coerces x to a 64-bit signed integer, then flips
    #        every bit — equivalent to ``-(x + 1)`` for integers).
    #   +x   no-op identity — SQLite documents it as a valid prefix that
    #        evaluates to its operand unchanged.  Useful for symmetry
    #        when writing signed numeric literals (``+5`` ≡ ``5``) and
    #        for normalising user-supplied expressions in tools.
    if any(_is_token(c, type_="MINUS") for c in node.children):
        inner = _child_node(node, "unary")
        return UnaryExpr(op=UnaryOp.NEG, operand=_unary(inner, state))
    if any(_is_token(c, type_="BIT_NOT_OP") for c in node.children):
        inner = _child_node(node, "unary")
        return UnaryExpr(op=UnaryOp.BIT_NOT, operand=_unary(inner, state))
    if any(_is_token(c, type_="PLUS") for c in node.children):
        # No-op: just return the inner expression unchanged.  We don't
        # introduce a UnaryOp.POS because the operand is bit-for-bit
        # identical — wrapping it would only add a useless layer for
        # the planner / codegen to peel.
        inner = _child_node(node, "unary")
        return _unary(inner, state)
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
    # function_call = (NAME | "REPLACE") "(" (STAR | "DISTINCT" value_list | [value_list]) ")"
    #
    # The function name is either a NAME token or the REPLACE keyword (which the
    # lexer tokenises as KEYWORD, not NAME, because REPLACE is also used for DML).
    name_tok = next(
        c
        for c in node.children
        if isinstance(c, Token)
        and (
            _token_type(c) == "NAME"
            or (_token_type(c) == "KEYWORD" and c.value.upper() == "REPLACE")
        )
    )
    name = name_tok.value
    # DISTINCT modifier: COUNT(DISTINCT col), SUM(DISTINCT col), …
    distinct = any(
        isinstance(c, Token) and _token_type(c) == "KEYWORD" and c.value.upper() == "DISTINCT"
        for c in node.children
    )
    star = any(_is_token(c, type_="STAR") for c in node.children)
    vl = _maybe_child(node, "value_list")
    args: list[FuncArg] = []
    if star:
        args.append(FuncArg(star=True))
    elif vl is not None:
        for c in vl.children:
            if isinstance(c, ASTNode) and c.rule_name == "expr":
                args.append(FuncArg(value=_expr(c, state)))

    # Optional FILTER (WHERE expr) clause — applies only to aggregate functions.
    # The grammar rule is:  filter_clause = "FILTER" "(" "WHERE" expr ")"
    # We extract the expr child of the filter_clause child of this node.
    filter_node = _maybe_child(node, "filter_clause")
    filter_expr_val = None
    if filter_node is not None:
        filter_expr_child = next(
            (c for c in filter_node.children if isinstance(c, ASTNode) and c.rule_name == "expr"),
            None,
        )
        if filter_expr_child is not None:
            filter_expr_val = _expr(filter_expr_child, state)

    # Aggregate functions fold into AggregateExpr; everything else stays generic.
    upper = name.upper()
    agg_map = {
        "COUNT": AggFunc.COUNT,
        "SUM": AggFunc.SUM,
        "AVG": AggFunc.AVG,
        "MIN": AggFunc.MIN,
        "MAX": AggFunc.MAX,
        # SQLite-specific: TOTAL() is like SUM() but returns 0.0 for empty
        # sets or all-NULL input, never returning NULL.
        "TOTAL": AggFunc.TOTAL,
        # JSON aggregates: accumulate values across the group into a JSON document.
        "JSON_GROUP_ARRAY": AggFunc.JSON_GROUP_ARRAY,
    }
    if upper in agg_map:
        # SQLite's MIN/MAX are overloaded: one-argument form is an aggregate
        # (MIN(col) over a GROUP BY), while two-or-more-argument form is a
        # scalar function that picks the smallest/largest among its arguments.
        # COUNT/SUM/AVG/TOTAL are always aggregates (no scalar overload).
        if len(args) > 1 and upper in ("MIN", "MAX"):
            # Scalar form: MIN(a, b, ...) / MAX(a, b, ...) — route to the
            # scalar function registry, not the aggregate path.
            return FunctionCall(name=name.lower(), args=tuple(args))
        if len(args) != 1:
            raise ProgrammingError(f"{upper}: expected 1 argument, got {len(args)}")
        return AggregateExpr(
            func=agg_map[upper], arg=args[0], distinct=distinct,
            filter_expr=filter_expr_val,
        )

    if upper in ("GROUP_CONCAT", "STRING_AGG"):
        # GROUP_CONCAT(col)          — SQLite default separator ','
        # GROUP_CONCAT(col, sep)     — explicit string literal separator
        # STRING_AGG(col, sep)       — SQLite 3.44+ synonym for GROUP_CONCAT;
        #                              separator is mandatory in standard SQL
        #                              but SQLite is permissive and accepts a
        #                              single-arg form too (defaults to ',').
        #
        # SQL:2003 §10.9 requires the separator to be a character-string
        # literal; we enforce that at parse time so the codegen can bake the
        # separator into the instruction stream rather than evaluating it
        # dynamically each time.
        if len(args) == 0 or len(args) > 2:
            raise ProgrammingError(
                f"{upper}: expected 1 or 2 arguments "
                "(column [, separator_literal])"
            )
        # SQLite forbids ``DISTINCT`` on the two-argument form: a DISTINCT
        # aggregate must take exactly one argument.  The reference engine
        # raises ``OperationalError: DISTINCT aggregates must have
        # exactly one argument``; we surface the same message so callers
        # can rely on text-matching tests against either implementation.
        if distinct and len(args) > 1:
            raise ProgrammingError(
                "DISTINCT aggregates must have exactly one argument"
            )
        separator: str | None = None
        if len(args) == 2:
            sep_expr = args[1].value
            if not isinstance(sep_expr, Literal) or not isinstance(sep_expr.value, str):
                raise ProgrammingError(
                    f"{upper}: separator must be a string literal, "
                    f"got {type(sep_expr).__name__}"
                )
            separator = sep_expr.value
        return AggregateExpr(
            func=AggFunc.GROUP_CONCAT,
            arg=args[0],
            distinct=distinct,
            separator=separator,
            filter_expr=filter_expr_val,
        )

    if upper == "JSON_GROUP_OBJECT":
        # JSON_GROUP_OBJECT(key_expr, val_expr)
        # Builds a JSON object by accumulating key-value pairs across the group.
        # Both key and value may be arbitrary expressions — unlike GROUP_CONCAT's
        # separator, the key is evaluated per row, not baked in at compile time.
        # The key expression is stored in key_arg; val_expr goes in the normal arg.
        if len(args) != 2:
            raise ProgrammingError(
                "JSON_GROUP_OBJECT: expected exactly 2 arguments (key, value), "
                f"got {len(args)}"
            )
        # As with GROUP_CONCAT(DISTINCT col, sep), SQLite forbids DISTINCT
        # on multi-argument aggregates: there is no well-defined notion of
        # "distinct (key, value) pair" in the engine, so the parser rejects
        # the combination outright with the same diagnostic.
        if distinct:
            raise ProgrammingError(
                "DISTINCT aggregates must have exactly one argument"
            )
        return AggregateExpr(
            func=AggFunc.JSON_GROUP_OBJECT,
            arg=args[1],       # value expression → the main arg
            key_arg=args[0],   # key expression → stored separately
            filter_expr=filter_expr_val,
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


def _frame_clause(node: ASTNode) -> WinFrame | None:
    """Parse a ``frame_clause`` node into a :class:`WinFrame`, or return None.

    Grammar::

        frame_clause  = frame_unit "BETWEEN" frame_bound "AND" frame_bound
                      | frame_unit frame_bound ;
        frame_unit    = "ROWS" | "RANGE" | "GROUPS" ;
        frame_bound   = "UNBOUNDED" "PRECEDING"
                      | "UNBOUNDED" "FOLLOWING"
                      | "CURRENT" "ROW"
                      | expr "PRECEDING"
                      | expr "FOLLOWING" ;

    Returns None if the node is not a frame_clause (defensive).
    """
    if node.rule_name != "frame_clause":
        return None

    # Determine frame unit from the frame_unit child.
    fu = _maybe_child(node, "frame_unit")
    unit = "ROWS"
    if fu is not None:
        unit_tok = next(
            (c for c in fu.children if isinstance(c, Token) and _token_type(c) == "KEYWORD"),
            None,
        )
        if unit_tok is not None:
            unit = unit_tok.value.upper()   # ROWS | RANGE | GROUPS

    def _parse_bound(bound_node: ASTNode) -> FrameBound:
        """Convert a frame_bound AST node to a FrameBound dataclass."""
        toks = [c for c in bound_node.children if isinstance(c, Token)]
        kw_vals = [t.value.upper() for t in toks if _token_type(t) == "KEYWORD"]

        if "UNBOUNDED" in kw_vals and "PRECEDING" in kw_vals:
            return FrameBound(kind="UNBOUNDED_PRECEDING")
        if "UNBOUNDED" in kw_vals and "FOLLOWING" in kw_vals:
            return FrameBound(kind="UNBOUNDED_FOLLOWING")
        if "CURRENT" in kw_vals and "ROW" in kw_vals:
            return FrameBound(kind="CURRENT_ROW")

        # expr PRECEDING / expr FOLLOWING — the offset is a literal integer
        # constant but it is deeply nested inside the generic expr grammar
        # (expr → or_expr → and_expr → … → primary → NUMBER).  Walk the
        # entire subtree to find the first NUMBER token rather than relying on
        # a shallow child scan.
        expr_node = _maybe_child(bound_node, "expr")
        if expr_node is not None:
            def _find_number(n: object) -> Token | None:
                if isinstance(n, Token) and _token_type(n) == "NUMBER":
                    return n
                if isinstance(n, ASTNode):
                    for child in n.children:
                        result = _find_number(child)
                        if result is not None:
                            return result
                return None

            num_tok = _find_number(expr_node)
            # _parse_number handles decimal, scientific, and 0x hex forms.
            offset = int(_parse_number(num_tok.value)) if num_tok else 0
            if "FOLLOWING" in kw_vals:
                return FrameBound(kind="FOLLOWING", offset=offset)
            return FrameBound(kind="PRECEDING", offset=offset)

        # Fallback: treat as CURRENT ROW.
        return FrameBound(kind="CURRENT_ROW")

    # Collect frame_bound children.
    bounds = [c for c in node.children if isinstance(c, ASTNode) and c.rule_name == "frame_bound"]

    if len(bounds) == 2:
        # BETWEEN start AND end form.
        start = _parse_bound(bounds[0])
        end = _parse_bound(bounds[1])
    elif len(bounds) == 1:
        # Shorthand: frame_unit frame_bound → end = CURRENT ROW.
        start = _parse_bound(bounds[0])
        end = FrameBound(kind="CURRENT_ROW")
    else:
        # Grammar mismatch — shouldn't happen; return full-partition frame.
        start = FrameBound(kind="UNBOUNDED_PRECEDING")
        end = FrameBound(kind="UNBOUNDED_FOLLOWING")

    return WinFrame(unit=unit, start=start, end=end)


def _window_func_call(node: ASTNode, state: _PlaceholderCounter) -> WindowFuncExpr:
    """Translate a ``window_func_call`` node into a :class:`WindowFuncExpr`.

    Grammar::

        window_func_call = NAME "(" ( STAR | [ value_list ] ) ")" "OVER" "(" window_spec ")" ;
        window_spec      = [ partition_clause ] [ order_clause ] [ frame_clause ] ;
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

    # Extract the window_spec node — either inline (OVER (...)) or a named
    # reference (OVER name) that resolves via state.window_defs.
    ws = _maybe_child(node, "window_spec")
    if ws is None:
        win_name_ref = _maybe_child(node, "window_name_ref")
        if win_name_ref is not None:
            tok = next(
                (c for c in win_name_ref.children if isinstance(c, Token)),
                None,
            )
            if tok is not None:
                ws = state.window_defs.get(tok.value.upper())
                if ws is None:
                    raise OperationalError(
                        f"no such window definition: {tok.value!r}"
                    )

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

    # ROWS / RANGE / GROUPS BETWEEN … AND … frame clause (optional).
    frame: WinFrame | None = None
    if ws is not None:
        fc = _maybe_child(ws, "frame_clause")
        if fc is not None:
            frame = _frame_clause(fc)

    return WindowFuncExpr(
        func=func_name,
        arg=arg,
        partition_by=tuple(partition_exprs),
        order_by=tuple(order_keys),
        extra_args=extra_args_tuple,
        frame=frame,
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


_CURRENT_TIME_KEYWORDS: dict[str, str] = {
    "CURRENT_DATE": "date",
    "CURRENT_TIME": "time",
    "CURRENT_TIMESTAMP": "datetime",
}


def _column_ref_to_expr(node: ASTNode) -> Expr:
    # column_ref = NAME [ "." NAME ]
    names = [c for c in node.children if isinstance(c, Token) and _token_type(c) == "NAME"]
    if len(names) == 1:
        # ``CURRENT_DATE``, ``CURRENT_TIME``, ``CURRENT_TIMESTAMP`` are SQL
        # *expressions* (no parentheses), not column references, yet the
        # lexer tokenises them as plain NAME tokens because the SQL token
        # grammar doesn't list them as keywords.  Intercept the bare-name
        # case here and rewrite to the equivalent scalar-function call —
        # ``date('now')`` / ``time('now')`` / ``datetime('now')`` — which
        # the sql-vm backend already implements.
        #
        # Matching is case-insensitive (SQLite accepts ``current_date``,
        # ``Current_Date``, …).  Once rewritten, the planner / codegen /
        # VM see a perfectly normal ``FunctionCall`` and don't need to
        # know anything special about these keywords.
        upper = names[0].value.upper()
        fn = _CURRENT_TIME_KEYWORDS.get(upper)
        if fn is not None:
            return FunctionCall(
                name=fn,
                args=(FuncArg(value=Literal(value="now")),),
            )
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


def _extract_bare_column_name(node: ASTNode) -> str | None:
    """Return the column name if *node* (an ``index_col``) wraps a single
    bare column reference; otherwise None.

    The ``index_col`` body is ``expr [COLLATE NAME] [ASC|DESC]``.  When the
    expression is just a column reference (no operator, no function call),
    the chain of grammar rules
    ``expr → or_expr → and_expr → not_expr → comparison → additive →
    multiplicative → unary → primary → column_ref → NAME``
    contains exactly one ASTNode child at each level.  We walk down the
    chain; if at any level there's more than one significant child (i.e. an
    operator was used), or the leaf isn't a NAME token, we return None.

    This conservatively detects the bare-column case so the index registers
    under the real column name in PRAGMA index_list.  Compound expressions
    fall back to a synthetic ``__expr_N`` placeholder name.
    """
    cur: ASTNode | Token | None = node
    # Walk through nested wrapper rules.  At each level the node must have
    # exactly one ASTNode child (other than ignorable tokens).
    while isinstance(cur, ASTNode):
        # Collect just the meaningful children: ASTNodes are the recursion
        # path; Tokens here would indicate an operator or punctuation that
        # makes this expression non-bare.
        child_nodes = [c for c in cur.children if isinstance(c, ASTNode)]
        child_tokens = [c for c in cur.children if isinstance(c, Token)]
        # The presence of an ASC/DESC/COLLATE token at the index_col level
        # is fine — those are recorded separately and the bare column is
        # still bare.  At lower levels (additive, multiplicative, etc.)
        # operator tokens mean the expression is compound.
        if cur.rule_name == "index_col":
            # First ASTNode child is the expr; subsequent tokens are
            # COLLATE / ASC / DESC.  Recurse into the expr only.
            if len(child_nodes) != 1:
                return None
            cur = child_nodes[0]
            continue
        # column_ref is the leaf: NAME [. NAME].  Bare column = exactly one
        # NAME token; "t.col" form gets the trailing NAME (the column name).
        if cur.rule_name == "column_ref":
            name_tokens = [
                c.value for c in cur.children
                if isinstance(c, Token) and _token_type(c) == "NAME"
            ]
            if not name_tokens:
                return None
            # For "table.col" the last NAME is the column.
            return name_tokens[-1]
        # function_call, paren-expr, etc.: not a bare column.
        if cur.rule_name in (
            "function_call", "window_func_call", "case_expr",
            "cast_expr",
        ):
            return None
        # Any operator token at this level → compound expression.
        # Punctuation like commas can appear; skip those.
        operator_tokens = [
            t for t in child_tokens
            if _token_type(t) in (
                "PLUS", "MINUS", "STAR", "SLASH", "PERCENT", "CONCAT_OP",
                "JSON_ARROW", "JSON_ARROW_TEXT",
            )
            or (_token_type(t) == "KEYWORD" and t.value.upper() in (
                "AND", "OR", "NOT", "BETWEEN", "IN", "LIKE", "GLOB",
                "ESCAPE", "IS",
            ))
        ]
        if operator_tokens:
            return None
        # Recurse into the single ASTNode child if there is exactly one.
        if len(child_nodes) == 1:
            cur = child_nodes[0]
            continue
        return None
    return None


def _cte_col_aliases(cte_node: ASTNode) -> list[str]:
    """Extract the optional column-alias list from a ``cte_def`` AST node.

    Grammar::

        cte_def = NAME [ "(" NAME { "," NAME } ")" ] "AS" "(" query_stmt ")" ;

    Returns an empty list if no column list was written, or a list of alias
    strings (in order) if one was.

    Example — ``WITH RECURSIVE cnt(n, m) AS (...)``::

        _cte_col_aliases(cte_node)  →  ['n', 'm']

    The implementation is a tiny state machine that iterates the children of
    the ``cte_def`` node:

    1. Consume the first NAME token (the CTE name).
    2. If the next non-trivial child is ``(`` → enter *in_col_list* mode and
       collect all NAME tokens until the closing ``)``.
    3. If the next non-trivial child is ``AS`` → no column list, stop.
    """
    aliases: list[str] = []
    state = "cte_name"
    for c in cte_node.children:
        if state == "cte_name":
            if isinstance(c, Token) and _token_type(c) == "NAME":
                state = "after_name"
        elif state == "after_name":
            if isinstance(c, Token) and c.value == "(":
                state = "in_col_list"
            elif _is_keyword(c, "AS"):
                break   # no column list
        elif state == "in_col_list":
            if isinstance(c, Token) and _token_type(c) == "NAME":
                aliases.append(c.value)
            elif isinstance(c, Token) and c.value == ")":
                break   # end of column list
    return aliases


def _apply_cte_col_aliases(
    stmt: SelectStmt | UnionStmt | IntersectStmt | ExceptStmt,
    aliases: list[str],
) -> SelectStmt | UnionStmt | IntersectStmt | ExceptStmt:
    """Apply column aliases declared in a CTE definition to the CTE's body.

    When a CTE is declared with an explicit column list::

        WITH RECURSIVE cnt(n) AS (SELECT 1 UNION ALL SELECT n+1 FROM cnt WHERE n<5)

    …the anchor query ``SELECT 1`` produces an output column whose default
    name is ``"1"`` (the literal).  The column alias list ``(n)`` says the
    output column should be named ``n``.

    For a plain ``SelectStmt`` body, we add ``alias=<declared_name>`` to
    each :class:`SelectItem`.  For a set-op tree (``a UNION b``,
    ``a INTERSECT b``, ``a EXCEPT b``) we walk down the *left* spine
    until we reach the leftmost :class:`SelectStmt` and rewrite that
    one's items.  SQLite (and the SQL standard) derives the output
    column names of a set-op chain entirely from the leftmost operand,
    so renaming the leftmost SELECT is sufficient to make the planner
    see the right schema.

    If ``aliases`` is shorter than the SELECT list the trailing items
    keep their current aliases (matches SQLite).  If ``aliases`` is
    empty, ``stmt`` is returned unchanged.
    """
    if not aliases:
        return stmt
    if isinstance(stmt, (UnionStmt, IntersectStmt, ExceptStmt)):
        # Recursively rewrite the left side; right side is left alone.
        new_left = _apply_cte_col_aliases(stmt.left, aliases)  # type: ignore[arg-type]
        if isinstance(stmt, UnionStmt):
            return UnionStmt(left=new_left, right=stmt.right, all=stmt.all)  # type: ignore[arg-type]
        if isinstance(stmt, IntersectStmt):
            return IntersectStmt(left=new_left, right=stmt.right, all=stmt.all)  # type: ignore[arg-type]
        return ExceptStmt(left=new_left, right=stmt.right, all=stmt.all)  # type: ignore[arg-type]
    if not stmt.items:
        return stmt
    new_items_list: list[SelectItem] = []
    for i, item in enumerate(stmt.items):
        alias = aliases[i] if i < len(aliases) else item.alias
        new_items_list.append(SelectItem(expr=item.expr, alias=alias))
    return SelectStmt(
        items=tuple(new_items_list),
        from_=stmt.from_,
        joins=stmt.joins,
        where=stmt.where,
        group_by=stmt.group_by,
        having=stmt.having,
        order_by=stmt.order_by,
        limit=stmt.limit,
        distinct=stmt.distinct,
    )


def _has_keyword_child(node: ASTNode, kw: str) -> bool:
    return any(_is_keyword(c, kw) for c in node.children)


def _render_expr_text(node: ASTNode) -> str:
    """Best-effort: render an ``expr`` AST node back to SQL source text.

    Used by CHECK-constraint error messages so mini-sqlite's
    ``ConstraintViolation`` quotes the original predicate
    (``CHECK constraint failed: a > 0``) instead of the older
    ``<table>.<col>`` form that SQLite never emits.

    The renderer walks all leaf tokens depth-first, joins them with
    single spaces, then suppresses spaces around punctuation that
    would otherwise look odd: no space after ``(``, no space before
    ``)``, no space before ``,``.  STRING tokens have already had
    their surrounding single-quotes stripped by the lexer, so we
    re-quote them (and double any embedded ``'``) to round-trip the
    literal.

    The output is normalised whitespace, not byte-identical to the
    original source — but it matches SQLite's ``CHECK constraint
    failed: …`` text for the common comparison / AND / OR / function
    patterns that account for nearly all real CHECK constraints.
    """
    tokens: list[Token] = []

    def _collect(n: object) -> None:
        if isinstance(n, Token):
            t = _token_type(n)
            if t not in ("WHITESPACE", "COMMENT", "EOF"):
                tokens.append(n)
        elif isinstance(n, ASTNode):
            for c in n.children:
                _collect(c)

    _collect(node)

    parts: list[str] = []
    prev_kind: str | None = None
    for tok in tokens:
        kind = _token_type(tok)
        val = tok.value
        if kind == "STRING":
            # Lexer stripped the surrounding single-quotes; restore them
            # and re-escape any internal apostrophes by doubling.
            val = "'" + val.replace("'", "''") + "'"
        # Decide whether a separator space is needed before this token.
        if parts:
            prev = parts[-1]
            need_space = True
            # No space before ``)`` or ``,``.
            if val == ")" or val == "," or prev == "(" or val == "(" and prev_kind == "NAME":
                need_space = False
            if need_space:
                parts.append(" ")
        parts.append(val)
        prev_kind = kind

    return "".join(parts).strip()


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
    """Parse a NUMBER token value into an int or float.

    SQLite recognises three numeric literal forms:

    * Decimal integer   ``123``       → ``int``
    * Decimal float     ``1.5``, ``1e3``, ``1.5E-2``  → ``float``
    * Hex integer       ``0x1F``      → ``int`` (always)

    The lexer maps ``HEX_INT`` to ``NUMBER`` so this single helper handles
    both spellings transparently.  Hex literals are always integers in
    SQLite — there's no ``0x1.8p3`` IEEE-754 hex-float syntax to worry
    about — and they cannot be negative (the ``-`` becomes a unary
    operator at parse time).

    Two SQLite-faithful quirks for hex literals:

    1. **Length cap of 16 hex digits.**  SQLite stores integers in a
       64-bit signed slot, so a literal with more than 16 hex digits
       (i.e. > 64 bits) is rejected at parse time with the same
       ``hex literal too big`` error sqlite3 raises.  This also caps
       the cost of ``int(s, 16)`` (which is O(N²) in the digit count
       and is *not* covered by Python's PYTHONINTMAXSTRDIGITS guard,
       since that guard only applies to base-10 conversions) so a
       megabyte-sized hex literal can't pin a parser thread.
    2. **64-bit two's-complement reinterpretation.**  ``0xFFFFFFFFFFFFFFFF``
       (16 set bits) evaluates to ``-1``, not ``+18446744073709551615``,
       matching SQLite's INTEGER affinity.
    """
    if s.startswith(("0x", "0X")):
        digits = s[2:]
        # 16 hex digits = 64 bits, the upper bound of SQLite INTEGER.
        if len(digits) > 16:
            raise OperationalError(f"hex literal too big: {s}")
        value = int(digits, 16) if digits else 0
        # Top bit set → reinterpret as signed (two's-complement wrap).
        if value & (1 << 63):
            value -= 1 << 64
        return value
    if "." in s or "e" in s or "E" in s:
        return float(s)
    return int(s)


def _unquote_string(s: str) -> str:
    """Unescape the body of a SQL string literal received from the sql-lexer.

    The sql-lexer already strips the surrounding single quotes before handing the
    token value to the adapter, so ``s`` here is the *body* of the literal — e.g.
    the SQL text ``'O''Brien'`` arrives here as ``O''Brien``.

    SQLite recognises **only one** escape convention inside a single-quoted
    string: doubled apostrophes.  ``'O''Brien'`` represents ``O'Brien``.
    Backslashes are *literal* characters — ``'a\\_b'`` is the four-character
    string ``a\\_b``, not ``a_b``.  This matches the behaviour of the real
    ``sqlite3`` module and is essential for ``LIKE … ESCAPE '\\'`` to work
    correctly (the pattern must retain the backslash so the LIKE matcher can
    see it as an escape character).
    """
    out: list[str] = []
    i = 0
    while i < len(s):
        if s[i] == "'" and i + 1 < len(s) and s[i + 1] == "'":
            # ANSI SQL doubled-quote escape: '' → '
            out.append("'")
            i += 2
        else:
            out.append(s[i])
            i += 1
    return "".join(out)
