"""Expression evaluator — evaluate AST expression nodes against a row.

The SQL expression grammar produces a recursive tree of nodes. This module
walks that tree and computes a Python value for each node.

Expression Grammar (abbreviated)
----------------------------------

.. code-block:: text

    expr            = or_expr
    or_expr         = and_expr { "OR" and_expr }
    and_expr        = not_expr { "AND" not_expr }
    not_expr        = [ "NOT" ] comparison
    comparison      = additive { cmp_op additive }
                    | additive "BETWEEN" additive "AND" additive
                    | additive "IN" "(" value_list ")"
                    | additive "LIKE" additive
                    | additive "IS" "NULL"
                    | additive "IS" "NOT" "NULL"
    additive        = multiplicative { ("+" | "-") multiplicative }
    multiplicative  = unary { ("*" | "/" | "%") unary }
    unary           = [ "-" ] primary
    primary         = NUMBER | STRING | "NULL" | "TRUE" | "FALSE"
                    | column_ref | function_call
                    | "(" expr ")"
    column_ref      = NAME [ "." NAME ]
    function_call   = NAME "(" ( "*" | expr { "," expr } ) ")"

Row Context
-----------

The ``row_ctx`` parameter is a ``dict[str, Any]`` mapping column names
(and optionally ``"alias.column"`` qualified names) to their values.
For example::

    {"id": 1, "name": "Alice", "employees.id": 1, "employees.name": "Alice"}

SQL NULL
--------

Python ``None`` represents SQL NULL. NULL propagates through arithmetic
and comparisons (any operation involving NULL yields NULL). The IS NULL
and IS NOT NULL tests are the only way to test for NULL.

LIKE patterns
-------------

We support the SQL LIKE ``%`` wildcard at the start and/or end of the
pattern (e.g., ``'A%'``, ``'%son'``, ``'%lic%'``).  Internal ``%``
wildcards and ``_`` single-character wildcards are not supported in this
educational implementation.
"""

from __future__ import annotations

import re
from typing import Any

from lang_parser import ASTNode
from lexer import Token

from sql_execution_engine.errors import ColumnNotFoundError


# ---------------------------------------------------------------------------
# Public entry point
# ---------------------------------------------------------------------------


def eval_expr(node: ASTNode | Token, row_ctx: dict[str, Any]) -> Any:
    """Recursively evaluate an AST expression node.

    Args:
        node: An ``ASTNode`` from the SQL parser (e.g. ``rule_name="expr"``)
              or a raw ``Token`` (matched directly by the grammar).
        row_ctx: The current row's values, keyed by column name and optionally
                 by ``"table.column"`` qualified names.

    Returns:
        The computed Python value — ``None``, ``int``, ``float``,
        ``str``, or ``bool``.

    Raises:
        ColumnNotFoundError: If a column reference cannot be resolved.
    """
    if isinstance(node, Token):
        return _eval_token(node, row_ctx)

    rule = node.rule_name

    # Pass-through rules — just recurse into the single meaningful child.
    if rule in ("expr", "statement", "program"):
        return eval_expr(node.children[0], row_ctx)

    if rule == "or_expr":
        return _eval_or(node, row_ctx)
    if rule == "and_expr":
        return _eval_and(node, row_ctx)
    if rule == "not_expr":
        return _eval_not(node, row_ctx)
    if rule == "comparison":
        return _eval_comparison(node, row_ctx)
    if rule == "additive":
        return _eval_additive(node, row_ctx)
    if rule == "multiplicative":
        return _eval_multiplicative(node, row_ctx)
    if rule == "unary":
        return _eval_unary(node, row_ctx)
    if rule == "primary":
        return _eval_primary(node, row_ctx)
    if rule == "column_ref":
        return _eval_column_ref(node, row_ctx)
    if rule == "function_call":
        return _eval_function_call_in_expr(node, row_ctx)

    # For any other intermediate rule (e.g. cmp_op), recurse into first child.
    if node.children:
        return eval_expr(node.children[0], row_ctx)

    return None


# ---------------------------------------------------------------------------
# Token evaluation (leaf nodes)
# ---------------------------------------------------------------------------


def _eval_token(token: Token, row_ctx: dict[str, Any]) -> Any:  # noqa: ANN001
    """Evaluate a raw token to its Python value.

    The grammar produces raw tokens when matching literals like ``"SELECT"``
    or token references like ``NUMBER``.  We convert them to Python values.
    """
    type_name = token.type_name if hasattr(token, "type_name") else str(token.token_type)
    value = token.value

    if type_name == "NUMBER":
        # Try integer first, then float.
        try:
            return int(value)
        except ValueError:
            return float(value)

    if type_name == "STRING":
        # The lexer already strips surrounding quotes.
        return value

    if type_name == "KEYWORD":
        if value == "NULL":
            return None
        if value == "TRUE":
            return True
        if value == "FALSE":
            return False
        if value == "AND":
            return "AND"
        if value == "OR":
            return "OR"

    # NAME token used directly (e.g., as a column reference leaf)
    if type_name == "NAME":
        return _resolve_column(value, row_ctx)

    # Operator tokens — return their string value for the caller to handle.
    return value


# ---------------------------------------------------------------------------
# Boolean operators
# ---------------------------------------------------------------------------


def _eval_or(node: ASTNode, row_ctx: dict[str, Any]) -> Any:
    """Evaluate ``or_expr = and_expr { "OR" and_expr }``."""
    # Grammar: first child is and_expr, then pairs of ("OR", and_expr)
    result = eval_expr(node.children[0], row_ctx)
    i = 1
    while i < len(node.children):
        # Skip "OR" keyword token
        i += 1
        if i >= len(node.children):
            break
        right = eval_expr(node.children[i], row_ctx)
        i += 1
        # SQL three-valued logic: NULL OR TRUE = TRUE, NULL OR FALSE = NULL
        if result is True or right is True:
            result = True
        elif result is None or right is None:
            result = None
        else:
            result = False
    return result


def _eval_and(node: ASTNode, row_ctx: dict[str, Any]) -> Any:
    """Evaluate ``and_expr = not_expr { "AND" not_expr }``."""
    result = eval_expr(node.children[0], row_ctx)
    i = 1
    while i < len(node.children):
        i += 1  # skip "AND"
        if i >= len(node.children):
            break
        right = eval_expr(node.children[i], row_ctx)
        i += 1
        # SQL three-valued logic: NULL AND FALSE = FALSE, NULL AND TRUE = NULL
        if result is False or right is False:
            result = False
        elif result is None or right is None:
            result = None
        else:
            result = bool(result) and bool(right)
    return result


def _eval_not(node: ASTNode, row_ctx: dict[str, Any]) -> Any:
    """Evaluate ``not_expr = [ "NOT" ] comparison``."""
    children = node.children
    if _is_keyword(children[0], "NOT"):
        val = eval_expr(children[1], row_ctx)
        if val is None:
            return None
        return not bool(val)
    return eval_expr(children[0], row_ctx)


# ---------------------------------------------------------------------------
# Comparison
# ---------------------------------------------------------------------------


def _eval_comparison(node: ASTNode, row_ctx: dict[str, Any]) -> Any:
    """Evaluate the comparison rule.

    Handles: ``=``, ``!=``/``<>``, ``<``, ``>``, ``<=``, ``>=``,
    ``BETWEEN``, ``IN``, ``LIKE``, ``IS NULL``, ``IS NOT NULL``.
    """
    children = node.children

    if len(children) == 1:
        # No comparison operator — just an additive expression.
        return eval_expr(children[0], row_ctx)

    left = eval_expr(children[0], row_ctx)

    # Detect the form by looking at the second child.
    second = children[1]
    second_kw = _keyword_value(second)

    # --- IS NULL / IS NOT NULL ---
    if second_kw == "IS":
        third_kw = _keyword_value(children[2])
        if third_kw == "NOT":
            return left is not None
        return left is None

    # --- BETWEEN low AND high ---
    if second_kw == "BETWEEN":
        low = eval_expr(children[2], row_ctx)
        # children[3] is "AND" keyword
        high = eval_expr(children[4], row_ctx)
        if left is None or low is None or high is None:
            return None
        return low <= left <= high

    # --- IN (value_list) ---
    if second_kw == "IN":
        # children[2] is "(" token, children[3] is in_expr or value_list, children[4] is ")"
        # The grammar wraps the list in an intermediate ``in_expr`` node:
        #   in_expr = query_stmt | value_list
        # We only handle the value_list branch here; unwrap if necessary.
        value_list_node = children[3]
        if isinstance(value_list_node, ASTNode) and value_list_node.rule_name == "in_expr":
            value_list_node = value_list_node.children[0]
        values = _eval_value_list(value_list_node, row_ctx)
        if left is None:
            return None
        # NULL in list causes NULL result only if left is NULL (handled above)
        return left in values

    # --- LIKE ---
    if second_kw == "LIKE":
        pattern_val = eval_expr(children[2], row_ctx)
        if left is None or pattern_val is None:
            return None
        return _like_match(str(left), str(pattern_val))

    # --- NOT IN / NOT LIKE / NOT BETWEEN ---
    if second_kw == "NOT":
        third_kw = _keyword_value(children[2])
        if third_kw == "BETWEEN":
            low = eval_expr(children[3], row_ctx)
            high = eval_expr(children[5], row_ctx)
            if left is None or low is None or high is None:
                return None
            return not (low <= left <= high)
        if third_kw == "IN":
            # children[4] is in_expr or value_list; unwrap if necessary.
            value_list_node = children[4]
            if isinstance(value_list_node, ASTNode) and value_list_node.rule_name == "in_expr":
                value_list_node = value_list_node.children[0]
            values = _eval_value_list(value_list_node, row_ctx)
            if left is None:
                return None
            return left not in values
        if third_kw == "LIKE":
            pattern_val = eval_expr(children[3], row_ctx)
            if left is None or pattern_val is None:
                return None
            return not _like_match(str(left), str(pattern_val))

    # --- Standard binary operator ---
    # children[1] is cmp_op node, children[2] is the right additive
    op_node = children[1]
    op = _get_op_string(op_node)
    right = eval_expr(children[2], row_ctx)

    if left is None or right is None:
        return None  # NULL comparison always yields NULL

    return _apply_cmp_op(op, left, right)


def _apply_cmp_op(op: str, left: Any, right: Any) -> bool:
    """Apply a comparison operator and return the boolean result."""
    if op in ("=", "=="):
        return left == right
    if op in ("!=", "<>"):
        return left != right
    if op == "<":
        return left < right  # type: ignore[operator]
    if op == ">":
        return left > right  # type: ignore[operator]
    if op == "<=":
        return left <= right  # type: ignore[operator]
    if op == ">=":
        return left >= right  # type: ignore[operator]
    return False


def _eval_value_list(node: ASTNode | Token, row_ctx: dict[str, Any]) -> list[Any]:
    """Evaluate a ``value_list`` node to a Python list of values.

    Grammar: ``value_list = expr { "," expr }``
    """
    if isinstance(node, Token):
        return [eval_expr(node, row_ctx)]

    values: list[Any] = []
    for child in node.children:
        if isinstance(child, Token) and child.value == ",":
            continue
        values.append(eval_expr(child, row_ctx))
    return values


def _like_match(value: str, pattern: str) -> bool:
    """Implement SQL LIKE matching with ``%`` wildcards.

    Converts the SQL LIKE pattern to a Python regex by splitting on ``%``
    and joining with ``.*``, escaping any regex metacharacters in each part.
    Supported: ``%`` (any sequence of characters, including empty).
    The ``_`` single-character wildcard is not supported in this
    educational implementation.

    Examples::

        _like_match("Alice", "A%")    → True
        _like_match("Alice", "%ce")   → True
        _like_match("Alice", "%lic%") → True
        _like_match("Alice", "B%")    → False
    """
    # Split on % wildcard, escape each literal part, then join with .*
    parts = pattern.split("%")
    regex = ".*".join(re.escape(p) for p in parts)
    return bool(re.fullmatch(regex, value, re.IGNORECASE))


# ---------------------------------------------------------------------------
# Arithmetic operators
# ---------------------------------------------------------------------------


def _eval_additive(node: ASTNode, row_ctx: dict[str, Any]) -> Any:
    """Evaluate ``additive = multiplicative { ("+" | "-") multiplicative }``."""
    result = eval_expr(node.children[0], row_ctx)
    i = 1
    while i < len(node.children):
        op_token = node.children[i]
        i += 1
        right = eval_expr(node.children[i], row_ctx)
        i += 1
        if result is None or right is None:
            result = None
            continue
        op = op_token.value if isinstance(op_token, Token) else str(op_token)
        if op == "+":
            result = result + right
        elif op == "-":
            result = result - right
    return result


def _eval_multiplicative(node: ASTNode, row_ctx: dict[str, Any]) -> Any:
    """Evaluate ``multiplicative = unary { ("*" | "/" | "%") unary }``."""
    result = eval_expr(node.children[0], row_ctx)
    i = 1
    while i < len(node.children):
        op_token = node.children[i]
        i += 1
        right = eval_expr(node.children[i], row_ctx)
        i += 1
        if result is None or right is None:
            result = None
            continue
        op = op_token.value if isinstance(op_token, Token) else str(op_token)
        if op == "*":
            result = result * right
        elif op == "/":
            result = result / right if right != 0 else None
        elif op == "%":
            result = result % right if right != 0 else None
    return result


def _eval_unary(node: ASTNode, row_ctx: dict[str, Any]) -> Any:
    """Evaluate ``unary = [ "-" ] primary``."""
    children = node.children
    if isinstance(children[0], Token) and children[0].value == "-":
        val = eval_expr(children[1], row_ctx)
        return -val if val is not None else None
    return eval_expr(children[0], row_ctx)


# ---------------------------------------------------------------------------
# Primary values
# ---------------------------------------------------------------------------


def _eval_primary(node: ASTNode, row_ctx: dict[str, Any]) -> Any:
    """Evaluate a ``primary`` node.

    primary = NUMBER | STRING | "NULL" | "TRUE" | "FALSE"
            | column_ref | function_call | "(" expr ")"
    """
    children = node.children

    if not children:
        return None

    first = children[0]

    # Parenthesized expression: "(" expr ")"
    if isinstance(first, Token) and first.value == "(":
        return eval_expr(children[1], row_ctx)

    # Delegate to sub-rules
    if isinstance(first, ASTNode):
        return eval_expr(first, row_ctx)

    # Raw token (NUMBER, STRING, NULL, TRUE, FALSE)
    return _eval_token(first, row_ctx)


# ---------------------------------------------------------------------------
# Column reference
# ---------------------------------------------------------------------------


def _eval_column_ref(node: ASTNode, row_ctx: dict[str, Any]) -> Any:
    """Evaluate a ``column_ref = NAME [ "." NAME ]`` node.

    Supports:
    - ``name`` — looks up the bare column name.
    - ``table.column`` — looks up the qualified name (as stored in row_ctx).
    """
    children = node.children

    if len(children) == 1:
        # Simple name: just a NAME token.
        name = children[0].value  # type: ignore[union-attr]
        return _resolve_column(name, row_ctx)

    if len(children) >= 3:
        # Qualified: NAME "." NAME
        table = children[0].value  # type: ignore[union-attr]
        col = children[2].value  # type: ignore[union-attr]
        qualified = f"{table}.{col}"
        if qualified in row_ctx:
            return row_ctx[qualified]
        # Fall back to bare column name
        return _resolve_column(col, row_ctx)

    return None


def _resolve_column(name: str, row_ctx: dict[str, Any]) -> Any:
    """Look up a column name in the row context.

    The row context may contain both qualified (``table.col``) and
    unqualified (``col``) keys. We try the unqualified name first.

    Raises:
        ColumnNotFoundError: If the name cannot be found.
    """
    if name in row_ctx:
        return row_ctx[name]
    # Try case-insensitive match
    name_lower = name.lower()
    for key in row_ctx:
        if key.lower() == name_lower:
            return row_ctx[key]
    raise ColumnNotFoundError(name)


# ---------------------------------------------------------------------------
# Function calls (in expression context — aggregate functions handled separately)
# ---------------------------------------------------------------------------


def _eval_function_call_in_expr(node: ASTNode, row_ctx: dict[str, Any]) -> Any:
    """Evaluate a scalar function call.

    In an expression context (not GROUP BY), we evaluate aggregate function
    references against the current row context if pre-computed values are
    present (the executor stores them under ``"_agg_<FUNC>(<col>)"`` keys).

    Grammar: ``function_call = NAME "(" ( "*" | expr { "," expr } ) ")"``
    """
    children = node.children
    func_name = children[0].value.upper()  # type: ignore[union-attr]
    # children[1] is "("

    # Look for a pre-computed aggregate value in the row context.
    # The executor stores these as "_agg_COUNT(*)", "_agg_SUM(salary)", etc.
    agg_key = _make_agg_key(func_name, children[2:-1], row_ctx)
    if agg_key and agg_key in row_ctx:
        return row_ctx[agg_key]

    # Scalar functions — evaluate argument and apply
    if func_name == "COALESCE":
        for arg_child in children[2:-1]:
            if isinstance(arg_child, Token) and arg_child.value == ",":
                continue
            val = eval_expr(arg_child, row_ctx)
            if val is not None:
                return val
        return None

    if func_name == "UPPER":
        val = eval_expr(children[2], row_ctx)
        return val.upper() if isinstance(val, str) else val

    if func_name == "LOWER":
        val = eval_expr(children[2], row_ctx)
        return val.lower() if isinstance(val, str) else val

    if func_name == "LENGTH":
        val = eval_expr(children[2], row_ctx)
        return len(val) if isinstance(val, str) else None

    if func_name == "ABS":
        val = eval_expr(children[2], row_ctx)
        return abs(val) if val is not None else None

    return None


def _make_agg_key(
    func_name: str,
    arg_children: list[Any],
    row_ctx: dict[str, Any],
) -> str | None:
    """Build the aggregate key as stored by the executor."""
    # Flatten to get the argument string
    parts: list[str] = []
    for child in arg_children:
        if isinstance(child, Token):
            if child.value not in (",",):
                parts.append(child.value)
        elif isinstance(child, ASTNode):
            parts.append(_node_text(child))
    arg_str = "".join(parts)
    return f"_agg_{func_name}({arg_str})"


def _node_text(node: ASTNode | Token) -> str:
    """Extract the raw text from an AST node or token."""
    if isinstance(node, Token):
        return node.value
    return "".join(_node_text(c) for c in node.children)


# ---------------------------------------------------------------------------
# Helper utilities
# ---------------------------------------------------------------------------


def _is_keyword(node: ASTNode | Token, value: str) -> bool:
    """Return True if *node* is a KEYWORD token with the given value."""
    if isinstance(node, Token):
        return node.value.upper() == value.upper()
    return False


def _keyword_value(node: ASTNode | Token) -> str:
    """Return the uppercase value of a keyword token, or empty string."""
    if isinstance(node, Token):
        return node.value.upper()
    # Could be a single-child node wrapping a token
    if node.children and isinstance(node.children[0], Token):
        return node.children[0].value.upper()
    return ""


def _get_op_string(node: ASTNode | Token) -> str:
    """Extract the operator string from a cmp_op node or token."""
    if isinstance(node, Token):
        return node.value
    if node.children:
        # cmp_op may be a node with one or two token children (e.g. "<>", "!=")
        return "".join(
            c.value for c in node.children if isinstance(c, Token)
        )
    return ""
