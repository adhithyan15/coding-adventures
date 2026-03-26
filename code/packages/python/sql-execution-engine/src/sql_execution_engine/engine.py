"""Engine — public entry points for SQL execution.

This module provides the top-level API for the SQL execution engine.  It
glues together the ``sql_parser`` and the ``executor`` module.

The two public functions are:

- ``execute(sql, source)``     — execute a single SQL statement.
- ``execute_all(sql, source)`` — execute multiple semicolon-separated statements.

Design Philosophy
-----------------

The engine layer is intentionally thin.  All it does is:

1. Parse the SQL text into an AST using the ``sql-parser`` package.
2. Walk the program node to find SELECT statements.
3. Delegate each SELECT to the ``executor.execute_select()`` function.

This separation of concerns means you can unit-test the executor with
pre-built ASTs (no parsing required) and the expression evaluator with
hand-crafted row dicts (no joining required).

Only SELECT
-----------

This engine only executes ``select_stmt`` nodes.  INSERT, UPDATE, DELETE,
CREATE TABLE, and DROP TABLE are parsed but intentionally ignored — the
engine is read-only.  Attempting to execute a non-SELECT statement does
not raise an error; the statement is silently skipped.

Example Usage
-------------

.. code-block:: python

    from sql_execution_engine import execute, DataSource

    class MySource(DataSource):
        def schema(self, name): ...
        def scan(self, name): ...

    result = execute("SELECT id, name FROM users WHERE id = 1", MySource())
    print(result.columns)  # ["id", "name"]
    print(result.rows)     # [{"id": 1, "name": "Alice"}]
"""

from __future__ import annotations

from lang_parser import ASTNode
from lexer import Token
from sql_parser import parse_sql

from sql_execution_engine.data_source import DataSource
from sql_execution_engine.errors import ExecutionError
from sql_execution_engine.executor import execute_select
from sql_execution_engine.result import QueryResult


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def execute(sql: str, source: DataSource) -> QueryResult:
    """Parse and execute a single SQL SELECT statement.

    This is the primary entry point.  It:

    1. Parses *sql* into an AST using the ``sql-parser`` package.
    2. Finds the first ``select_stmt`` in the AST.
    3. Executes it against *source* using the relational pipeline.

    Args:
        sql: A SQL string containing exactly one SELECT statement.
             (If the string contains multiple statements separated by
             semicolons, only the **first** SELECT is executed.  Use
             ``execute_all()`` to run all statements.)
        source: The data provider implementing the ``DataSource`` ABC.

    Returns:
        A ``QueryResult`` with column names and result rows.

    Raises:
        ExecutionError: If something goes wrong during execution
                        (table not found, column not found, etc.).
        GrammarParseError: If the SQL text has syntax errors.

    Example::

        result = execute("SELECT * FROM employees WHERE salary > 80000", src)
    """
    ast = parse_sql(sql)
    select_node = _find_first_select(ast)
    if select_node is None:
        # No SELECT found — return empty result.
        return QueryResult()
    return execute_select(select_node, source)


def execute_all(sql: str, source: DataSource) -> list[QueryResult]:
    """Parse and execute all SELECT statements in a SQL string.

    Useful when a single string contains multiple semicolon-separated
    SELECT statements (common in SQL scripts).

    Non-SELECT statements (INSERT, UPDATE, etc.) are silently skipped.

    Args:
        sql: A SQL string containing one or more statements separated by
             semicolons.
        source: The data provider.

    Returns:
        A list of ``QueryResult`` objects, one per SELECT statement found.
        If no SELECT statements exist, returns an empty list.

    Raises:
        ExecutionError: If any execution step fails.
        GrammarParseError: If the SQL has syntax errors.

    Example::

        results = execute_all(
            "SELECT id FROM employees; SELECT name FROM departments",
            src,
        )
        # results[0] has employee ids, results[1] has department names
    """
    ast = parse_sql(sql)
    results: list[QueryResult] = []
    for select_node in _find_all_selects(ast):
        results.append(execute_select(select_node, source))
    return results


# ---------------------------------------------------------------------------
# AST navigation helpers
# ---------------------------------------------------------------------------


def _find_first_select(ast: ASTNode) -> ASTNode | None:
    """Return the first ``select_stmt`` node anywhere in the AST."""
    if ast.rule_name == "select_stmt":
        return ast
    for child in ast.children:
        if isinstance(child, ASTNode):
            result = _find_first_select(child)
            if result is not None:
                return result
    return None


def _find_all_selects(ast: ASTNode) -> list[ASTNode]:
    """Return all ``select_stmt`` nodes anywhere in the AST."""
    results: list[ASTNode] = []
    _collect_selects(ast, results)
    return results


def _collect_selects(node: ASTNode | Token, results: list[ASTNode]) -> None:
    """Recursively collect all select_stmt nodes."""
    if isinstance(node, Token):
        return
    if node.rule_name == "select_stmt":
        results.append(node)
        return  # Don't recurse into nested selects (not supported)
    for child in node.children:
        _collect_selects(child, results)
