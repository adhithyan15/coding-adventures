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
from sql_backend.schema import ColumnDef as BackendColumnDef
from sql_planner import (
    AggFunc,
    AggregateExpr,
    Assignment,
    Between,
    BinaryExpr,
    BinaryOp,
    Column,
    CreateTableStmt,
    DeleteStmt,
    DropTableStmt,
    FuncArg,
    FunctionCall,
    In,
    InsertValuesStmt,
    IsNotNull,
    IsNull,
    JoinClause,
    JoinKind,
    Like,
    Limit,
    Literal,
    NotIn,
    NotLike,
    SelectItem,
    SelectStmt,
    SortKey,
    Statement,
    TableRef,
    UnaryExpr,
    UnaryOp,
    UpdateStmt,
    Wildcard,
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


def to_statement(ast: ASTNode) -> Statement:
    """Convert a parsed ``program`` ASTNode to a planner ``Statement``.

    The grammar's top rule is ``program = statement { ";" statement } [";"]``.
    We currently require exactly one statement per execute() call — matching
    both sqlite3's semantics and our own spec — so the driver slices on ``;``
    before calling us. Here we just walk down past ``program`` and
    ``statement`` to the actual statement node.
    """
    prog = _child_node(ast, "program") if ast.rule_name != "program" else ast
    statement = _only_child_node(prog, "statement")
    return _stmt_dispatch(statement)


# --------------------------------------------------------------------------
# Statement dispatch.
# --------------------------------------------------------------------------


def _stmt_dispatch(stmt: ASTNode) -> Statement:
    # ``statement`` has exactly one child, which is the real statement node.
    inner = _single_child(stmt)
    if not isinstance(inner, ASTNode):
        raise ProgrammingError(f"unexpected statement shape: {inner}")
    match inner.rule_name:
        case "select_stmt":
            return _select(inner)
        case "insert_stmt":
            return _insert(inner)
        case "update_stmt":
            return _update(inner)
        case "delete_stmt":
            return _delete(inner)
        case "create_table_stmt":
            return _create_table(inner)
        case "drop_table_stmt":
            return _drop_table(inner)
    raise ProgrammingError(f"unsupported statement: {inner.rule_name}")


# --------------------------------------------------------------------------
# SELECT.
# --------------------------------------------------------------------------


def _select(node: ASTNode) -> SelectStmt:
    state = _PlaceholderCounter()

    distinct = _has_keyword_child(node, "DISTINCT")
    items = _select_list(_child_node(node, "select_list"), state)

    # FROM + JOINs.
    from_node = _child_node(node, "table_ref")
    from_ref = _table_ref(from_node)
    joins = tuple(_join_clause(c, state) for c in _child_nodes(node, "join_clause"))

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


def _table_ref(node: ASTNode) -> TableRef:
    # table_ref = table_name [ "AS" NAME ]
    # table_name = NAME [ "." NAME ]
    tn = _child_node(node, "table_name")
    parts = [c.value for c in tn.children if isinstance(c, Token) and _token_type(c) == "NAME"]
    table = parts[-1]  # schema.table → we ignore the schema qualifier
    alias: str | None = None
    for i, c in enumerate(node.children):
        if _is_keyword(c, "AS") and i + 1 < len(node.children):
            nxt = node.children[i + 1]
            if isinstance(nxt, Token):
                alias = nxt.value
    return TableRef(table=table, alias=alias)


def _join_clause(node: ASTNode, state: _PlaceholderCounter) -> JoinClause:
    # join_clause = join_type "JOIN" table_ref "ON" expr
    jt = _child_node(node, "join_type")
    kind = _join_kind(jt)
    right = _table_ref(_child_node(node, "table_ref"))
    # The grammar requires ON for every join kind (including CROSS). We
    # translate the predicate through so INNER/LEFT can use it; CROSS
    # joins ignore it semantically.
    expr_node = _maybe_child(node, "expr")
    on = _expr(expr_node, state) if expr_node is not None else None
    return JoinClause(kind=kind, right=right, on=on)


def _join_kind(node: ASTNode) -> str:
    # join_type = "CROSS" | "INNER" | ... — look at the first keyword token.
    for c in node.children:
        if isinstance(c, Token) and _token_type(c) == "KEYWORD":
            kw = c.value.upper()
            if kw == "CROSS":
                return JoinKind.CROSS
            if kw == "INNER":
                return JoinKind.INNER
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


def _insert(node: ASTNode) -> InsertValuesStmt:
    state = _PlaceholderCounter()
    # insert_stmt =
    #   "INSERT" "INTO" NAME [ "(" NAME { "," NAME } ")" ] "VALUES"
    #   row_value { "," row_value }
    table_tok = _first_token(node, kind="NAME")
    assert table_tok is not None
    table = table_tok.value

    # Explicit column list: everything between LPAREN and RPAREN before "VALUES".
    columns: tuple[str, ...] | None = None
    i = 0
    # skip INSERT INTO NAME
    while i < len(node.children) and not _is_token(node.children[i], type_="LPAREN"):
        if _is_keyword(node.children[i], "VALUES"):
            break
        i += 1
    if i < len(node.children) and _is_token(node.children[i], type_="LPAREN"):
        cols: list[str] = []
        j = i + 1
        while j < len(node.children) and not _is_token(node.children[j], type_="RPAREN"):
            c = node.children[j]
            if isinstance(c, Token) and _token_type(c) == "NAME":
                cols.append(c.value)
            j += 1
        columns = tuple(cols)

    rows = tuple(_row_value(rv, state) for rv in _child_nodes(node, "row_value"))
    return InsertValuesStmt(table=table, columns=columns, rows=rows)


def _row_value(node: ASTNode, state: _PlaceholderCounter) -> tuple[Expr, ...]:
    return tuple(
        _expr(c, state) for c in node.children if isinstance(c, ASTNode) and c.rule_name == "expr"
    )


def _update(node: ASTNode) -> UpdateStmt:
    state = _PlaceholderCounter()
    # update_stmt = "UPDATE" NAME "SET" assignment { "," assignment } [where]
    table_tok = _first_token(node, kind="NAME")
    assert table_tok is not None
    table = table_tok.value

    assignments = tuple(
        _assignment(c, state)
        for c in node.children
        if isinstance(c, ASTNode) and c.rule_name == "assignment"
    )
    where = _maybe_expr(node, "where_clause", state, skip=1)
    return UpdateStmt(table=table, assignments=assignments, where=where)


def _assignment(node: ASTNode, state: _PlaceholderCounter) -> Assignment:
    # assignment = NAME "=" expr
    col_tok = next(c for c in node.children if isinstance(c, Token) and _token_type(c) == "NAME")
    value = _expr(_child_node(node, "expr"), state)
    return Assignment(column=col_tok.value, value=value)


def _delete(node: ASTNode) -> DeleteStmt:
    state = _PlaceholderCounter()
    # delete_stmt = "DELETE" "FROM" NAME [where]
    table_tok = _first_token(node, kind="NAME")
    assert table_tok is not None
    where = _maybe_expr(node, "where_clause", state, skip=1)
    return DeleteStmt(table=table_tok.value, where=where)


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
    cols = tuple(_col_def(c) for c in _child_nodes(node, "col_def"))
    return CreateTableStmt(table=table_tok.value, columns=cols, if_not_exists=if_not_exists)


def _col_def(node: ASTNode) -> BackendColumnDef:
    # col_def = NAME NAME { col_constraint }
    names = [c for c in node.children if isinstance(c, Token) and _token_type(c) == "NAME"]
    col_name = names[0].value
    type_name = names[1].value.upper() if len(names) > 1 else "TEXT"

    not_null = False
    primary_key = False
    unique = False
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
        # DEFAULT and NULL left alone; the backend's default is already NULL-OK.
    return BackendColumnDef(
        name=col_name,
        type_name=type_name,
        not_null=not_null,
        primary_key=primary_key,
        unique=unique,
    )


def _drop_table(node: ASTNode) -> DropTableStmt:
    if_exists = _has_keyword_sequence(node, ("IF", "EXISTS"))
    table_tok = _first_token(node, kind="NAME")
    assert table_tok is not None
    return DropTableStmt(table=table_tok.value, if_exists=if_exists)


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
       and not _has_keyword_child(node, "LIKE") and not _has_keyword_child(node, "IS"):
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

    # IS NULL / IS NOT NULL.
    if _has_keyword_child(node, "IS"):
        if _has_keyword_child(node, "NOT"):
            return IsNotNull(operand=left)
        return IsNull(operand=left)

    return left


def _additive(node: ASTNode, state: _PlaceholderCounter) -> Expr:
    # additive = multiplicative { ("+"|"-") multiplicative }
    return _left_assoc_punct(
        node,
        "multiplicative",
        _multiplicative,
        {"PLUS": BinaryOp.ADD, "MINUS": BinaryOp.SUB},
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
        elif isinstance(c, ASTNode):
            if c.rule_name == "function_call":
                return _function_call(c, state)
            if c.rule_name == "column_ref":
                return _column_ref_to_expr(c)
            if c.rule_name == "expr":
                return _expr(c, state)
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
    return FunctionCall(name=name, args=tuple(args))


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
