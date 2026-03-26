"""Executor — the relational pipeline that runs a SELECT statement.

This module is the heart of the execution engine.  It takes the AST node
for a ``select_stmt`` and a ``DataSource``, then walks the node tree to
execute each clause in the correct order.

Execution Order
---------------

SQL has a *logical* evaluation order that differs from the *syntactic*
order in which clauses are written.  We follow the standard logical order:

.. code-block:: text

    1. FROM        — identify source table(s)
    2. JOIN        — combine tables
    3. WHERE       — filter individual rows
    4. GROUP BY    — group rows for aggregation
    5. HAVING      — filter groups
    6. SELECT      — project and rename columns
    7. DISTINCT    — deduplicate rows
    8. ORDER BY    — sort rows
    9. LIMIT       — paginate

This is also the order in which the clauses are processed here.

AST Navigation
--------------

The grammar produces nodes like this for ``SELECT id, name FROM employees``::

    ASTNode("program", [
      ASTNode("statement", [
        ASTNode("select_stmt", [
          Token(KEYWORD, "SELECT"),
          ASTNode("select_list", [...]),
          Token(KEYWORD, "FROM"),
          ASTNode("table_ref", [
            ASTNode("table_name", [Token(NAME, "employees")])
          ]),
          ...optional clauses...
        ])
      ])
    ])

The executor locates child nodes by their ``rule_name``.  A helper
``_find_child`` scans the children list linearly.
"""

from __future__ import annotations

from typing import Any

from lang_parser import ASTNode
from lexer import Token

from sql_execution_engine.aggregate import compute_aggregates
from sql_execution_engine.data_source import DataSource
from sql_execution_engine.errors import ColumnNotFoundError
from sql_execution_engine.expression import (
    _keyword_value,
    _node_text,
    eval_expr,
)
from sql_execution_engine.join import perform_join
from sql_execution_engine.result import QueryResult


# ---------------------------------------------------------------------------
# Public entry point
# ---------------------------------------------------------------------------


def execute_select(stmt: ASTNode, source: DataSource) -> QueryResult:
    """Execute a single ``select_stmt`` AST node against *source*.

    This is the main entry point called by ``engine.py``.  It orchestrates
    the full relational pipeline described in the module docstring.

    Args:
        stmt: The ``select_stmt`` AST node.
        source: The data provider.

    Returns:
        A ``QueryResult`` with the output column names and rows.
    """
    # --- Phase 1: FROM -------------------------------------------------------
    # Identify the primary table and scan it.
    table_ref = _find_child(stmt, "table_ref")
    primary_table, primary_alias = _extract_table_ref(table_ref)
    rows = source.scan(primary_table)

    # Add qualified column names (alias.col) to every row.
    effective_alias = primary_alias or primary_table
    rows = _qualify_rows(rows, effective_alias)

    # --- Phase 2: JOIN -------------------------------------------------------
    join_clauses = _find_children(stmt, "join_clause")
    for join_clause in join_clauses:
        rows = _process_join(rows, join_clause, source)

    # --- Phase 3: WHERE ------------------------------------------------------
    where_clause = _find_child(stmt, "where_clause")
    if where_clause:
        rows = _apply_where(rows, where_clause)

    # --- Phase 4 & 5: GROUP BY / HAVING / Aggregates -------------------------
    group_clause = _find_child(stmt, "group_clause")
    having_clause = _find_child(stmt, "having_clause")
    select_list = _find_child(stmt, "select_list")

    # Detect whether the query uses any aggregate functions.
    agg_specs = _extract_agg_specs(select_list)
    having_agg_specs = _extract_agg_specs_from_having(having_clause)
    all_agg_specs = list({(f, a) for f, a in agg_specs + having_agg_specs})

    if group_clause or all_agg_specs:
        rows = _apply_group_by_and_aggregate(
            rows, group_clause, having_clause, all_agg_specs
        )

    # --- Phase 6: ORDER BY (before projection so non-selected cols accessible)
    order_clause = _find_child(stmt, "order_clause")
    if order_clause:
        rows = _apply_order_by(rows, order_clause)

    # --- Phase 7: SELECT projection ------------------------------------------
    # Detect DISTINCT qualifier
    has_distinct = _has_distinct_qualifier(stmt)

    columns, rows = _apply_select(select_list, rows, primary_table, effective_alias, source)

    # --- Phase 8: DISTINCT ---------------------------------------------------
    if has_distinct:
        rows = _apply_distinct(rows)

    # --- Phase 9: LIMIT / OFFSET ---------------------------------------------
    limit_clause = _find_child(stmt, "limit_clause")
    if limit_clause:
        rows = _apply_limit(rows, limit_clause)

    return QueryResult(columns=columns, rows=rows)


# ---------------------------------------------------------------------------
# Phase 1: FROM
# ---------------------------------------------------------------------------


def _extract_table_ref(table_ref: ASTNode | None) -> tuple[str, str]:
    """Extract the table name and optional alias from a ``table_ref`` node.

    Grammar: ``table_ref = table_name [ "AS" NAME ]``
             ``table_name = NAME``
    """
    if table_ref is None:
        return "", ""

    table_name_node = _find_child(table_ref, "table_name")
    if table_name_node is None:
        # Flat token
        name_token = _first_token(table_ref)
        return (name_token.value if name_token else "", "")

    name_token = _first_token(table_name_node)
    table_name = name_token.value if name_token else ""

    # Look for "AS" alias
    alias = ""
    children = table_ref.children
    for i, child in enumerate(children):
        if isinstance(child, Token) and child.value.upper() == "AS":
            if i + 1 < len(children):
                next_child = children[i + 1]
                if isinstance(next_child, Token):
                    alias = next_child.value

    return table_name, alias


def _qualify_rows(rows: list[dict[str, Any]], alias: str) -> list[dict[str, Any]]:
    """Add qualified column names (``alias.col``) to every row."""
    qualified: list[dict[str, Any]] = []
    for row in rows:
        qrow: dict[str, Any] = dict(row)
        for key, val in row.items():
            if "." not in key:
                qrow[f"{alias}.{key}"] = val
        qualified.append(qrow)
    return qualified


# ---------------------------------------------------------------------------
# Phase 2: JOIN
# ---------------------------------------------------------------------------


def _process_join(
    left_rows: list[dict[str, Any]],
    join_clause: ASTNode,
    source: DataSource,
) -> list[dict[str, Any]]:
    """Process a single JOIN clause.

    Grammar: ``join_clause = join_type "JOIN" table_ref "ON" expr``
             ``join_type   = "INNER" | "LEFT" [ "OUTER" ] | "RIGHT" [ "OUTER" ]
                           | "FULL" [ "OUTER" ] | "CROSS"``
    """
    # Extract join type
    join_type_node = _find_child(join_clause, "join_type")
    join_type = _extract_join_type(join_type_node)

    # Extract table ref
    table_ref = _find_child(join_clause, "table_ref")
    right_table, right_alias = _extract_table_ref(table_ref)
    effective_alias = right_alias or right_table

    right_rows = source.scan(right_table)
    right_rows = _qualify_rows(right_rows, effective_alias)

    # Extract ON condition
    on_condition = _find_on_condition(join_clause)

    return perform_join(
        left_rows=left_rows,
        left_alias="",
        right_rows=right_rows,
        right_alias=effective_alias,
        join_type=join_type,
        on_condition=on_condition,
    )


def _extract_join_type(join_type_node: ASTNode | None) -> str:
    """Extract the join type string from a ``join_type`` node."""
    if join_type_node is None:
        return "INNER"
    keywords: list[str] = []
    for child in join_type_node.children:
        if isinstance(child, Token):
            keywords.append(child.value.upper())
    return " ".join(keywords) if keywords else "INNER"


def _find_on_condition(join_clause: ASTNode) -> ASTNode | Token | None:
    """Find the expression after the ON keyword in a join_clause."""
    children = join_clause.children
    for i, child in enumerate(children):
        if isinstance(child, Token) and child.value.upper() == "ON":
            if i + 1 < len(children):
                return children[i + 1]
    return None


# ---------------------------------------------------------------------------
# Phase 3: WHERE
# ---------------------------------------------------------------------------


def _apply_where(
    rows: list[dict[str, Any]],
    where_clause: ASTNode,
) -> list[dict[str, Any]]:
    """Filter rows using the WHERE clause expression.

    Grammar: ``where_clause = "WHERE" expr``
    The expression is the second child (after the WHERE keyword token).
    """
    # Find the expression child (the one after "WHERE")
    expr_node = _find_expr_in_clause(where_clause)
    if expr_node is None:
        return rows
    return [row for row in rows if _is_truthy(eval_expr(expr_node, row))]


# ---------------------------------------------------------------------------
# Phase 4 & 5: GROUP BY and HAVING
# ---------------------------------------------------------------------------


def _apply_group_by_and_aggregate(
    rows: list[dict[str, Any]],
    group_clause: ASTNode | None,
    having_clause: ASTNode | None,
    agg_specs: list[tuple[str, str]],
) -> list[dict[str, Any]]:
    """Group rows by GROUP BY keys and compute aggregates.

    When there is no GROUP BY clause but aggregate functions are used,
    all rows form a single group.
    """
    # Extract GROUP BY column names.
    group_keys: list[str] = []
    if group_clause:
        group_keys = _extract_group_keys(group_clause)

    # Partition rows into groups.
    groups: dict[tuple[Any, ...], list[dict[str, Any]]] = {}
    for row in rows:
        key = tuple(_safe_get(row, k) for k in group_keys)
        groups.setdefault(key, []).append(row)

    # Compute aggregates for each group.
    result: list[dict[str, Any]] = []
    for key_values, group_rows in groups.items():
        # Build a representative row (first row in group, augmented with
        # group key values and aggregate results).
        rep_row: dict[str, Any] = dict(group_rows[0])
        for col, val in zip(group_keys, key_values):
            rep_row[col] = val
        if agg_specs:
            rep_row.update(compute_aggregates(group_rows, agg_specs))
        # Store the group rows for reference (some HAVING expressions may need them).
        rep_row["_group_rows"] = group_rows

        # Apply HAVING filter.
        if having_clause:
            having_expr = _find_expr_in_clause(having_clause)
            if having_expr and not _is_truthy(eval_expr(having_expr, rep_row)):
                continue

        result.append(rep_row)

    return result


def _extract_group_keys(group_clause: ASTNode) -> list[str]:
    """Extract column name strings from a ``group_clause`` node.

    Grammar: ``group_clause = "GROUP" "BY" expr { "," expr }``
    """
    keys: list[str] = []
    for child in group_clause.children:
        if isinstance(child, Token):
            continue
        # Each non-token child is an expr for a GROUP BY column.
        keys.append(_node_text(child).strip())
    return keys


def _extract_agg_specs(select_list: ASTNode | None) -> list[tuple[str, str]]:
    """Scan the select_list for aggregate function calls.

    Returns a list of ``(func_name, arg)`` tuples, e.g.
    ``[("COUNT", "*"), ("SUM", "salary")]``.
    """
    if select_list is None:
        return []
    specs: list[tuple[str, str]] = []
    _collect_agg_specs(select_list, specs)
    return specs


def _extract_agg_specs_from_having(having_clause: ASTNode | None) -> list[tuple[str, str]]:
    """Scan the HAVING clause for aggregate function calls."""
    if having_clause is None:
        return []
    specs: list[tuple[str, str]] = []
    _collect_agg_specs(having_clause, specs)
    return specs


def _collect_agg_specs(node: ASTNode | Token, specs: list[tuple[str, str]]) -> None:
    """Recursively collect aggregate function specs from a subtree."""
    if isinstance(node, Token):
        return
    if node.rule_name == "function_call":
        name_token = node.children[0]
        if isinstance(name_token, Token):
            func_name = name_token.value.upper()
            if func_name in ("COUNT", "SUM", "AVG", "MIN", "MAX"):
                # Extract argument text
                # children: NAME "(" arg... ")"
                arg_parts: list[str] = []
                for child in node.children[2:-1]:
                    if isinstance(child, Token) and child.value not in (",",):
                        arg_parts.append(child.value)
                    elif isinstance(child, ASTNode):
                        arg_parts.append(_node_text(child))
                arg = "".join(arg_parts)
                pair = (func_name, arg)
                if pair not in specs:
                    specs.append(pair)
    for child in node.children:
        _collect_agg_specs(child, specs)


# ---------------------------------------------------------------------------
# Phase 6: SELECT projection
# ---------------------------------------------------------------------------


def _apply_select(
    select_list: ASTNode | None,
    rows: list[dict[str, Any]],
    primary_table: str,
    primary_alias: str,
    source: DataSource,
) -> tuple[list[str], list[dict[str, Any]]]:
    """Project and rename columns according to the SELECT list.

    Handles:
    - ``SELECT *``       — expand to all columns from the primary table.
    - ``SELECT col``     — project named column.
    - ``SELECT expr AS alias`` — evaluate expression and rename.

    Returns:
        (column_names, projected_rows)
    """
    if select_list is None:
        return [], rows

    # Check for STAR token
    if _is_star_select(select_list):
        if rows:
            # Determine columns: all bare (unqualified) keys in row order.
            cols = _all_bare_columns(rows, primary_alias)
            projected = [{c: row.get(c) for c in cols} for row in rows]
            return cols, projected
        # No rows — get columns from schema
        schema_cols = source.schema(primary_alias or primary_table)
        return schema_cols, []

    # Walk select_items
    items = _find_children(select_list, "select_item")
    if not items:
        # Possibly directly contains expr nodes (single-item select_list)
        items = [select_list]

    columns: list[str] = []
    projected_rows: list[dict[str, Any]] = []

    # Determine output columns from the first pass (or schema).
    for item in items:
        col_name, _ = _extract_select_item_name(item, rows)
        columns.append(col_name)

    for row in rows:
        projected_row: dict[str, Any] = {}
        for item in items:
            col_name, expr_node = _extract_select_item_name(item, rows)
            if expr_node is not None:
                try:
                    val = eval_expr(expr_node, row)
                except ColumnNotFoundError:
                    val = None
            else:
                val = None
            projected_row[col_name] = val
        projected_rows.append(projected_row)

    return columns, projected_rows


def _is_star_select(select_list: ASTNode) -> bool:
    """Return True if the select_list is just ``*``."""
    for child in select_list.children:
        if isinstance(child, Token) and child.value == "*":
            return True
        if isinstance(child, ASTNode) and child.rule_name == "select_item":
            # Check if the item is a star
            for grandchild in child.children:
                if isinstance(grandchild, Token) and grandchild.value == "*":
                    return True
    return False


def _extract_select_item_name(
    item: ASTNode,
    rows: list[dict[str, Any]],
) -> tuple[str, ASTNode | Token | None]:
    """Extract the output column name and expression from a select_item.

    Grammar: ``select_item = expr [ "AS" NAME ]``
    """
    children = item.children

    # Find AS alias
    alias: str | None = None
    expr_node: ASTNode | Token | None = None

    for i, child in enumerate(children):
        if isinstance(child, Token) and child.value.upper() == "AS":
            if i + 1 < len(children):
                alias_token = children[i + 1]
                if isinstance(alias_token, Token):
                    alias = alias_token.value
        elif not (isinstance(child, Token) and child.value.upper() == "AS"):
            if expr_node is None and not (isinstance(child, Token) and child.value.upper() == "AS"):
                expr_node = child

    if alias:
        return alias, expr_node

    # Derive name from expression
    if expr_node is not None:
        name = _infer_column_name(expr_node)
        return name, expr_node

    return "?", None


def _infer_column_name(node: ASTNode | Token) -> str:
    """Infer an output column name from an expression node."""
    if isinstance(node, Token):
        return node.value
    if node.rule_name == "column_ref":
        # Use the last NAME token (the column part, not the table prefix)
        tokens = [c for c in node.children if isinstance(c, Token)]
        if tokens:
            return tokens[-1].value
    if node.rule_name == "function_call":
        # e.g. COUNT(*) → "COUNT(*)"
        return _node_text(node)
    # For expressions, return the full text
    return _node_text(node)


def _all_bare_columns(rows: list[dict[str, Any]], alias: str) -> list[str]:
    """Return all unqualified column names from the row set.

    Qualified names like ``employees.id`` are excluded; bare names like
    ``id`` and internal keys like ``_agg_COUNT(*)`` are excluded too.
    """
    cols: list[str] = []
    seen: set[str] = set()
    if not rows:
        return cols
    for key in rows[0]:
        if "." not in key and not key.startswith("_"):
            if key not in seen:
                cols.append(key)
                seen.add(key)
    return cols


# ---------------------------------------------------------------------------
# Phase 7: DISTINCT
# ---------------------------------------------------------------------------


def _apply_distinct(rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Remove duplicate rows.

    Two rows are considered identical if all their values are equal.
    Order is preserved (first occurrence wins).
    """
    seen: list[tuple[Any, ...]] = []
    result: list[dict[str, Any]] = []
    for row in rows:
        key = tuple(row.values())
        if key not in seen:
            seen.append(key)
            result.append(row)
    return result


# ---------------------------------------------------------------------------
# Phase 8: ORDER BY
# ---------------------------------------------------------------------------


def _apply_order_by(
    rows: list[dict[str, Any]],
    order_clause: ASTNode,
) -> list[dict[str, Any]]:
    """Sort rows according to the ORDER BY clause.

    Grammar: ``order_clause = "ORDER" "BY" order_item { "," order_item }``
             ``order_item   = expr [ "ASC" | "DESC" ]``
    """
    order_items = _find_children(order_clause, "order_item")
    if not order_items:
        return rows

    # Build a list of (expr_node, ascending) from back to front
    # and apply sorts from least significant to most significant.
    sort_specs: list[tuple[ASTNode | Token, bool]] = []
    for item in order_items:
        expr_node, ascending = _extract_order_item(item)
        sort_specs.append((expr_node, ascending))

    # Apply sorts from last to first (stable sort guarantees correctness).
    result = list(rows)
    for expr_node, ascending in reversed(sort_specs):
        def sort_key(row: dict[str, Any], en: ASTNode | Token = expr_node) -> Any:
            try:
                val = eval_expr(en, row)
            except ColumnNotFoundError:
                val = None
            # None sorts last in both ASC and DESC
            return (val is None, val)

        result.sort(key=sort_key, reverse=not ascending)

    return result


def _extract_order_item(item: ASTNode) -> tuple[ASTNode | Token, bool]:
    """Extract (expression, is_ascending) from an ``order_item`` node."""
    children = item.children
    ascending = True
    expr_node: ASTNode | Token = children[0]

    for child in children[1:]:
        if isinstance(child, Token) and child.value.upper() == "DESC":
            ascending = False
        elif isinstance(child, Token) and child.value.upper() == "ASC":
            ascending = True

    return expr_node, ascending


# ---------------------------------------------------------------------------
# Phase 9: LIMIT / OFFSET
# ---------------------------------------------------------------------------


def _apply_limit(
    rows: list[dict[str, Any]],
    limit_clause: ASTNode,
) -> list[dict[str, Any]]:
    """Apply LIMIT and optional OFFSET to the row list.

    Grammar: ``limit_clause = "LIMIT" NUMBER [ "OFFSET" NUMBER ]``
    """
    limit: int | None = None
    offset: int = 0

    children = limit_clause.children
    i = 0
    while i < len(children):
        child = children[i]
        if isinstance(child, Token):
            if child.value.upper() == "LIMIT":
                i += 1
                if i < len(children):
                    limit = int(children[i].value)  # type: ignore[union-attr]
            elif child.value.upper() == "OFFSET":
                i += 1
                if i < len(children):
                    offset = int(children[i].value)  # type: ignore[union-attr]
        i += 1

    if limit is None:
        return rows[offset:]
    return rows[offset: offset + limit]


# ---------------------------------------------------------------------------
# DISTINCT qualifier detection
# ---------------------------------------------------------------------------


def _has_distinct_qualifier(stmt: ASTNode) -> bool:
    """Return True if the SELECT statement has a DISTINCT qualifier."""
    for child in stmt.children:
        if isinstance(child, Token) and child.value.upper() == "DISTINCT":
            return True
    return False


# ---------------------------------------------------------------------------
# Generic AST helpers
# ---------------------------------------------------------------------------


def _find_child(node: ASTNode, rule_name: str) -> ASTNode | None:
    """Return the first direct child with the given rule_name, or None."""
    for child in node.children:
        if isinstance(child, ASTNode) and child.rule_name == rule_name:
            return child
    return None


def _find_children(node: ASTNode, rule_name: str) -> list[ASTNode]:
    """Return all direct children with the given rule_name."""
    return [
        child
        for child in node.children
        if isinstance(child, ASTNode) and child.rule_name == rule_name
    ]


def _find_expr_in_clause(clause: ASTNode) -> ASTNode | Token | None:
    """Find the expression child in a WHERE, HAVING, etc. clause node.

    Skips keyword tokens at the start of the clause.
    """
    for child in clause.children:
        if isinstance(child, Token):
            kw = child.value.upper()
            if kw in ("WHERE", "HAVING", "ON"):
                continue
        # First non-keyword child is the expression.
        return child
    return None


def _first_token(node: ASTNode) -> Token | None:
    """Return the first Token in a node's children."""
    for child in node.children:
        if isinstance(child, Token):
            return child
    return None


def _safe_get(row: dict[str, Any], key: str) -> Any:
    """Get a value from a row, returning None if not found."""
    if key in row:
        return row[key]
    # Try case-insensitive
    key_lower = key.lower()
    for k, v in row.items():
        if k.lower() == key_lower:
            return v
    return None


def _is_truthy(val: Any) -> bool:
    """Return True if val is a truthy SQL value (not NULL, not False)."""
    if val is None:
        return False
    return bool(val)
