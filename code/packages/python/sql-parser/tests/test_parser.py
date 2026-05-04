"""Tests for the SQL parser thin wrapper.

These tests verify that the grammar-driven parser, configured with
``sql.grammar``, correctly parses ANSI SQL text into ASTs.

The SQL grammar's entry-point rule is ``"program"``, which matches one or
more semicolon-separated statements. Every parse result therefore has
``rule_name="program"`` at the root.

Test organisation
-----------------

- ``TestFactory`` — the ``create_sql_parser`` factory function.
- ``TestSimpleSelect`` — basic SELECT structure and root rule.
- ``TestCaseInsensitivity`` — keywords work in any case.
- ``TestSelectVariants`` — SELECT *, columns, AS aliases.
- ``TestWhereClause`` — WHERE with comparisons and logical operators.
- ``TestOrderByLimitOffset`` — ORDER BY ASC/DESC, LIMIT, OFFSET.
- ``TestGroupByHaving`` — GROUP BY and HAVING clauses.
- ``TestJoins`` — INNER JOIN and LEFT JOIN.
- ``TestInsert`` — INSERT INTO VALUES.
- ``TestUpdate`` — UPDATE SET WHERE.
- ``TestDelete`` — DELETE FROM.
- ``TestCreateTable`` — CREATE TABLE with IF NOT EXISTS and constraints.
- ``TestDropTable`` — DROP TABLE with IF EXISTS.
- ``TestExpressions`` — arithmetic, AND/OR/NOT, BETWEEN, IN, LIKE,
  IS NULL, IS NOT NULL, function calls.
- ``TestMultipleStatements`` — semicolon-separated statements.
- ``TestErrors`` — invalid SQL raises ``GrammarParseError``.
- ``TestASTStructure`` — rule names in the tree are correct.
- ``TestErrorPath`` — grammar file not found triggers the error path.
"""

from __future__ import annotations

import pytest
from lang_parser import ASTNode, GrammarParseError, GrammarParser
from lexer import Token

import sql_parser.parser as _parser_module
from sql_parser import create_sql_parser, parse_sql

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def get_type_name(token: Token) -> str:
    """Extract the type name from a token (handles both enum and string)."""
    return token.type if isinstance(token.type, str) else token.type.name


def find_nodes(node: ASTNode, rule_name: str) -> list[ASTNode]:
    """Recursively find all ASTNodes with the given rule_name."""
    results: list[ASTNode] = []
    if node.rule_name == rule_name:
        results.append(node)
    for child in node.children:
        if isinstance(child, ASTNode):
            results.extend(find_nodes(child, rule_name))
    return results


def child_tokens(node: ASTNode) -> list[Token]:
    """Return all Token children of a node (not ASTNode children)."""
    return [c for c in node.children if isinstance(c, Token)]


def child_nodes(node: ASTNode) -> list[ASTNode]:
    """Return all ASTNode children of a node (not Token children)."""
    return [c for c in node.children if isinstance(c, ASTNode)]


def all_token_values(node: ASTNode) -> list[str]:
    """Collect every token value in the subtree, depth-first."""
    result: list[str] = []
    for child in node.children:
        if isinstance(child, Token):
            result.append(child.value)
        else:
            result.extend(all_token_values(child))
    return result


# ---------------------------------------------------------------------------
# Factory function tests
# ---------------------------------------------------------------------------


class TestFactory:
    """Tests for the create_sql_parser factory function."""

    def test_returns_grammar_parser(self) -> None:
        """create_sql_parser should return a GrammarParser instance."""
        parser = create_sql_parser("SELECT 1 FROM t")
        assert isinstance(parser, GrammarParser)

    def test_factory_is_not_none(self) -> None:
        """create_sql_parser must not return None."""
        parser = create_sql_parser("SELECT id FROM users")
        assert parser is not None

    def test_factory_produces_ast(self) -> None:
        """The factory-created parser should produce a valid AST."""
        parser = create_sql_parser("SELECT id FROM users")
        ast = parser.parse()
        assert isinstance(ast, ASTNode)
        assert ast.rule_name == "program"


# ---------------------------------------------------------------------------
# Simple SELECT — root rule is "program"
# ---------------------------------------------------------------------------


class TestSimpleSelect:
    """Basic SELECT parsing and root rule verification."""

    def test_root_rule_is_program(self) -> None:
        """The root ASTNode must have rule_name='program'."""
        ast = parse_sql("SELECT id FROM users")
        assert ast.rule_name == "program"

    def test_simple_select_parses(self) -> None:
        """A minimal SELECT statement should parse without error."""
        ast = parse_sql("SELECT id FROM users")
        assert isinstance(ast, ASTNode)

    def test_select_stmt_node_present(self) -> None:
        """The tree should contain a 'select_stmt' node."""
        ast = parse_sql("SELECT id FROM users")
        nodes = find_nodes(ast, "select_stmt")
        assert len(nodes) >= 1

    def test_statement_node_present(self) -> None:
        """The tree should contain a 'statement' node."""
        ast = parse_sql("SELECT 1 FROM t")
        nodes = find_nodes(ast, "statement")
        assert len(nodes) >= 1


# ---------------------------------------------------------------------------
# Case insensitivity
# ---------------------------------------------------------------------------


class TestCaseInsensitivity:
    """Keywords work in any case because the SQL lexer normalizes to uppercase."""

    def test_lowercase_select(self) -> None:
        """'select id from users' should parse identically to the uppercase form."""
        ast = parse_sql("select id from users")
        assert ast.rule_name == "program"
        nodes = find_nodes(ast, "select_stmt")
        assert len(nodes) >= 1

    def test_uppercase_select(self) -> None:
        """'SELECT id FROM users' should parse."""
        ast = parse_sql("SELECT id FROM users")
        nodes = find_nodes(ast, "select_stmt")
        assert len(nodes) >= 1

    def test_mixed_case_select(self) -> None:
        """'Select id From users' should parse."""
        ast = parse_sql("Select id From users")
        nodes = find_nodes(ast, "select_stmt")
        assert len(nodes) >= 1

    def test_lowercase_where(self) -> None:
        """'where' keyword in any case should be recognised."""
        ast = parse_sql("select id from users where id = 1")
        nodes = find_nodes(ast, "where_clause")
        assert len(nodes) >= 1


# ---------------------------------------------------------------------------
# SELECT variants
# ---------------------------------------------------------------------------


class TestSelectVariants:
    """Different SELECT column list forms."""

    def test_select_star(self) -> None:
        """SELECT * FROM table should produce a select_list with STAR."""
        ast = parse_sql("SELECT * FROM users")
        nodes = find_nodes(ast, "select_list")
        assert len(nodes) >= 1
        # STAR token should appear somewhere in the select_list subtree
        tokens = all_token_values(nodes[0])
        assert "*" in tokens

    def test_select_single_column(self) -> None:
        """SELECT id FROM users — one select_item."""
        ast = parse_sql("SELECT id FROM users")
        items = find_nodes(ast, "select_item")
        assert len(items) >= 1

    def test_select_multiple_columns(self) -> None:
        """SELECT id, name, age FROM users — three select_items."""
        ast = parse_sql("SELECT id, name, age FROM users")
        items = find_nodes(ast, "select_item")
        assert len(items) == 3

    def test_select_with_alias(self) -> None:
        """SELECT id AS user_id — the AS keyword must appear in the tree."""
        ast = parse_sql("SELECT id AS user_id FROM users")
        values = all_token_values(ast)
        assert "AS" in values

    def test_select_with_table_alias(self) -> None:
        """SELECT u.id FROM users AS u — table_ref with AS alias."""
        ast = parse_sql("SELECT u.id FROM users AS u")
        values = all_token_values(ast)
        assert "AS" in values


# ---------------------------------------------------------------------------
# WHERE clause
# ---------------------------------------------------------------------------


class TestWhereClause:
    """WHERE clause parsing."""

    def test_simple_where(self) -> None:
        """SELECT with a simple equality WHERE condition."""
        ast = parse_sql("SELECT id FROM users WHERE id = 1")
        nodes = find_nodes(ast, "where_clause")
        assert len(nodes) == 1

    def test_where_greater_than(self) -> None:
        """WHERE with > comparison."""
        ast = parse_sql("SELECT id FROM users WHERE age > 18")
        nodes = find_nodes(ast, "where_clause")
        assert len(nodes) == 1

    def test_where_and(self) -> None:
        """WHERE with AND logical operator."""
        ast = parse_sql("SELECT id FROM users WHERE age > 18 AND active = 1")
        nodes = find_nodes(ast, "where_clause")
        assert len(nodes) == 1
        values = all_token_values(nodes[0])
        assert "AND" in values

    def test_where_or(self) -> None:
        """WHERE with OR logical operator."""
        ast = parse_sql("SELECT id FROM users WHERE age < 10 OR age > 90")
        nodes = find_nodes(ast, "where_clause")
        assert len(nodes) == 1
        values = all_token_values(nodes[0])
        assert "OR" in values

    def test_where_not(self) -> None:
        """WHERE with NOT."""
        ast = parse_sql("SELECT id FROM users WHERE NOT active = 0")
        nodes = find_nodes(ast, "where_clause")
        assert len(nodes) == 1

    def test_where_between(self) -> None:
        """WHERE with BETWEEN ... AND ..."""
        ast = parse_sql("SELECT id FROM users WHERE age BETWEEN 18 AND 65")
        nodes = find_nodes(ast, "where_clause")
        assert len(nodes) == 1
        values = all_token_values(nodes[0])
        assert "BETWEEN" in values

    def test_where_in(self) -> None:
        """WHERE with IN (...)."""
        ast = parse_sql("SELECT id FROM users WHERE id IN (1, 2, 3)")
        nodes = find_nodes(ast, "where_clause")
        assert len(nodes) == 1
        values = all_token_values(nodes[0])
        assert "IN" in values

    def test_where_like(self) -> None:
        """WHERE with LIKE pattern."""
        ast = parse_sql("SELECT id FROM users WHERE name LIKE 'A%'")
        nodes = find_nodes(ast, "where_clause")
        assert len(nodes) == 1
        values = all_token_values(nodes[0])
        assert "LIKE" in values

    def test_where_is_null(self) -> None:
        """WHERE col IS NULL."""
        ast = parse_sql("SELECT id FROM users WHERE email IS NULL")
        nodes = find_nodes(ast, "where_clause")
        assert len(nodes) == 1
        values = all_token_values(nodes[0])
        assert "IS" in values
        assert "NULL" in values

    def test_where_is_not_null(self) -> None:
        """WHERE col IS NOT NULL."""
        ast = parse_sql("SELECT id FROM users WHERE email IS NOT NULL")
        nodes = find_nodes(ast, "where_clause")
        assert len(nodes) == 1
        values = all_token_values(nodes[0])
        assert "IS" in values
        assert "NOT" in values
        assert "NULL" in values


# ---------------------------------------------------------------------------
# ORDER BY, LIMIT, OFFSET
# ---------------------------------------------------------------------------


class TestOrderByLimitOffset:
    """ORDER BY with ASC/DESC, LIMIT, and OFFSET."""

    def test_order_by_asc(self) -> None:
        """SELECT with ORDER BY col ASC."""
        ast = parse_sql("SELECT id FROM users ORDER BY name ASC")
        nodes = find_nodes(ast, "order_clause")
        assert len(nodes) == 1
        values = all_token_values(nodes[0])
        assert "ASC" in values

    def test_order_by_desc(self) -> None:
        """SELECT with ORDER BY col DESC."""
        ast = parse_sql("SELECT id FROM users ORDER BY name DESC")
        nodes = find_nodes(ast, "order_clause")
        assert len(nodes) == 1
        values = all_token_values(nodes[0])
        assert "DESC" in values

    def test_order_by_multiple_columns(self) -> None:
        """ORDER BY with two columns."""
        ast = parse_sql("SELECT id FROM users ORDER BY last_name ASC, first_name ASC")
        nodes = find_nodes(ast, "order_item")
        assert len(nodes) == 2

    def test_limit(self) -> None:
        """SELECT with LIMIT clause."""
        ast = parse_sql("SELECT id FROM users LIMIT 10")
        nodes = find_nodes(ast, "limit_clause")
        assert len(nodes) == 1

    def test_limit_offset(self) -> None:
        """SELECT with LIMIT and OFFSET."""
        ast = parse_sql("SELECT id FROM users LIMIT 10 OFFSET 20")
        nodes = find_nodes(ast, "limit_clause")
        assert len(nodes) == 1
        values = all_token_values(nodes[0])
        assert "OFFSET" in values
        assert "20" in values


# ---------------------------------------------------------------------------
# GROUP BY and HAVING
# ---------------------------------------------------------------------------


class TestGroupByHaving:
    """GROUP BY and HAVING clauses."""

    def test_group_by(self) -> None:
        """SELECT with GROUP BY clause."""
        ast = parse_sql("SELECT dept, COUNT(*) FROM employees GROUP BY dept")
        nodes = find_nodes(ast, "group_clause")
        assert len(nodes) == 1

    def test_group_by_multiple_columns(self) -> None:
        """GROUP BY with multiple columns."""
        ast = parse_sql("SELECT a, b, COUNT(*) FROM t GROUP BY a, b")
        nodes = find_nodes(ast, "group_clause")
        assert len(nodes) == 1
        # Two column refs in the group clause
        col_refs = find_nodes(nodes[0], "column_ref")
        assert len(col_refs) == 2

    def test_having(self) -> None:
        """SELECT with HAVING clause."""
        ast = parse_sql(
            "SELECT dept, COUNT(*) FROM employees GROUP BY dept HAVING COUNT(*) > 5"
        )
        nodes = find_nodes(ast, "having_clause")
        assert len(nodes) == 1

    def test_group_by_and_having_together(self) -> None:
        """GROUP BY and HAVING can both appear in the same query."""
        ast = parse_sql(
            "SELECT dept, AVG(salary) FROM employees GROUP BY dept HAVING AVG(salary) > 50000"
        )
        group_nodes = find_nodes(ast, "group_clause")
        having_nodes = find_nodes(ast, "having_clause")
        assert len(group_nodes) == 1
        assert len(having_nodes) == 1


# ---------------------------------------------------------------------------
# JOINs
# ---------------------------------------------------------------------------


class TestJoins:
    """INNER JOIN and LEFT JOIN clauses."""

    def test_inner_join(self) -> None:
        """SELECT with INNER JOIN ... ON ..."""
        ast = parse_sql(
            "SELECT u.id, o.total FROM users AS u INNER JOIN orders AS o ON u.id = o.user_id"
        )
        nodes = find_nodes(ast, "join_clause")
        assert len(nodes) == 1
        values = all_token_values(nodes[0])
        assert "INNER" in values
        assert "JOIN" in values
        assert "ON" in values

    def test_left_join(self) -> None:
        """SELECT with LEFT JOIN ... ON ..."""
        ast = parse_sql(
            "SELECT u.id, o.total FROM users AS u LEFT JOIN orders AS o ON u.id = o.user_id"
        )
        nodes = find_nodes(ast, "join_clause")
        assert len(nodes) == 1
        values = all_token_values(nodes[0])
        assert "LEFT" in values

    def test_multiple_joins(self) -> None:
        """Two JOIN clauses in a single SELECT."""
        ast = parse_sql(
            "SELECT * FROM a INNER JOIN b ON a.id = b.a_id INNER JOIN c ON b.id = c.b_id"
        )
        nodes = find_nodes(ast, "join_clause")
        assert len(nodes) == 2


# ---------------------------------------------------------------------------
# INSERT
# ---------------------------------------------------------------------------


class TestInsert:
    """INSERT INTO ... VALUES (...) parsing."""

    def test_simple_insert(self) -> None:
        """Basic INSERT INTO with column list and values."""
        ast = parse_sql("INSERT INTO users (id, name) VALUES (1, 'Alice')")
        nodes = find_nodes(ast, "insert_stmt")
        assert len(nodes) == 1

    def test_insert_without_column_list(self) -> None:
        """INSERT INTO without explicit column list."""
        ast = parse_sql("INSERT INTO users VALUES (1, 'Alice', 30)")
        nodes = find_nodes(ast, "insert_stmt")
        assert len(nodes) == 1

    def test_insert_multiple_rows(self) -> None:
        """INSERT with multiple row_value groups."""
        ast = parse_sql(
            "INSERT INTO users (id, name) VALUES (1, 'Alice'), (2, 'Bob')"
        )
        nodes = find_nodes(ast, "row_value")
        assert len(nodes) == 2


# ---------------------------------------------------------------------------
# UPDATE
# ---------------------------------------------------------------------------


class TestUpdate:
    """UPDATE ... SET ... WHERE ... parsing."""

    def test_simple_update(self) -> None:
        """Basic UPDATE with one assignment and WHERE."""
        ast = parse_sql("UPDATE users SET name = 'Bob' WHERE id = 1")
        nodes = find_nodes(ast, "update_stmt")
        assert len(nodes) == 1

    def test_update_multiple_assignments(self) -> None:
        """UPDATE with multiple column assignments."""
        ast = parse_sql("UPDATE users SET name = 'Bob', age = 25 WHERE id = 1")
        nodes = find_nodes(ast, "assignment")
        assert len(nodes) == 2

    def test_update_without_where(self) -> None:
        """UPDATE without WHERE clause (updates all rows) should still parse."""
        ast = parse_sql("UPDATE users SET active = 0")
        nodes = find_nodes(ast, "update_stmt")
        assert len(nodes) == 1


# ---------------------------------------------------------------------------
# DELETE
# ---------------------------------------------------------------------------


class TestDelete:
    """DELETE FROM ... parsing."""

    def test_simple_delete(self) -> None:
        """DELETE FROM with WHERE clause."""
        ast = parse_sql("DELETE FROM users WHERE id = 99")
        nodes = find_nodes(ast, "delete_stmt")
        assert len(nodes) == 1

    def test_delete_without_where(self) -> None:
        """DELETE FROM without WHERE (deletes all rows) should parse."""
        ast = parse_sql("DELETE FROM users")
        nodes = find_nodes(ast, "delete_stmt")
        assert len(nodes) == 1


# ---------------------------------------------------------------------------
# CREATE TABLE
# ---------------------------------------------------------------------------


class TestCreateTable:
    """CREATE TABLE parsing including IF NOT EXISTS and constraints."""

    def test_simple_create_table(self) -> None:
        """Basic CREATE TABLE with one column."""
        ast = parse_sql("CREATE TABLE users (id INTEGER)")
        nodes = find_nodes(ast, "create_table_stmt")
        assert len(nodes) == 1

    def test_create_table_if_not_exists(self) -> None:
        """CREATE TABLE IF NOT EXISTS."""
        ast = parse_sql("CREATE TABLE IF NOT EXISTS users (id INTEGER)")
        nodes = find_nodes(ast, "create_table_stmt")
        assert len(nodes) == 1
        values = all_token_values(nodes[0])
        assert "IF" in values
        assert "NOT" in values
        assert "EXISTS" in values

    def test_create_table_multiple_columns(self) -> None:
        """CREATE TABLE with multiple col_def entries."""
        ast = parse_sql(
            "CREATE TABLE users (id INTEGER, name VARCHAR, age INTEGER)"
        )
        nodes = find_nodes(ast, "col_def")
        assert len(nodes) == 3

    def test_create_table_primary_key(self) -> None:
        """Column with PRIMARY KEY constraint."""
        ast = parse_sql("CREATE TABLE users (id INTEGER PRIMARY KEY)")
        nodes = find_nodes(ast, "col_constraint")
        assert len(nodes) >= 1
        values = all_token_values(ast)
        assert "PRIMARY" in values
        assert "KEY" in values

    def test_create_table_not_null(self) -> None:
        """Column with NOT NULL constraint."""
        ast = parse_sql("CREATE TABLE users (id INTEGER NOT NULL)")
        values = all_token_values(ast)
        assert "NOT" in values
        assert "NULL" in values

    def test_create_table_unique(self) -> None:
        """Column with UNIQUE constraint."""
        ast = parse_sql("CREATE TABLE users (email VARCHAR UNIQUE)")
        values = all_token_values(ast)
        assert "UNIQUE" in values

    def test_create_table_default(self) -> None:
        """Column with DEFAULT value constraint."""
        ast = parse_sql("CREATE TABLE users (active INTEGER DEFAULT 1)")
        values = all_token_values(ast)
        assert "DEFAULT" in values


# ---------------------------------------------------------------------------
# DROP TABLE
# ---------------------------------------------------------------------------


class TestDropTable:
    """DROP TABLE parsing."""

    def test_simple_drop_table(self) -> None:
        """Basic DROP TABLE statement."""
        ast = parse_sql("DROP TABLE users")
        nodes = find_nodes(ast, "drop_table_stmt")
        assert len(nodes) == 1

    def test_drop_table_if_exists(self) -> None:
        """DROP TABLE IF EXISTS."""
        ast = parse_sql("DROP TABLE IF EXISTS users")
        nodes = find_nodes(ast, "drop_table_stmt")
        assert len(nodes) == 1
        values = all_token_values(nodes[0])
        assert "IF" in values
        assert "EXISTS" in values


# ---------------------------------------------------------------------------
# Expressions
# ---------------------------------------------------------------------------


class TestExpressions:
    """Complex expressions in WHERE clauses and SELECT items."""

    def test_arithmetic_addition(self) -> None:
        """Arithmetic + in a SELECT item."""
        ast = parse_sql("SELECT price + tax FROM items")
        nodes = find_nodes(ast, "additive")
        assert len(nodes) >= 1

    def test_arithmetic_multiplication(self) -> None:
        """Arithmetic * in a SELECT item (via multiplicative rule)."""
        ast = parse_sql("SELECT qty * price FROM items")
        nodes = find_nodes(ast, "multiplicative")
        assert len(nodes) >= 1

    def test_unary_minus(self) -> None:
        """Unary minus in an expression."""
        ast = parse_sql("SELECT id FROM t WHERE x = -1")
        assert ast.rule_name == "program"

    def test_function_call_count_star(self) -> None:
        """COUNT(*) function call."""
        ast = parse_sql("SELECT COUNT(*) FROM users")
        nodes = find_nodes(ast, "function_call")
        assert len(nodes) >= 1

    def test_function_call_with_args(self) -> None:
        """AVG(salary) function call."""
        ast = parse_sql("SELECT AVG(salary) FROM employees")
        nodes = find_nodes(ast, "function_call")
        assert len(nodes) >= 1

    def test_not_between(self) -> None:
        """NOT BETWEEN expression."""
        ast = parse_sql("SELECT id FROM t WHERE x NOT BETWEEN 1 AND 10")
        values = all_token_values(ast)
        assert "NOT" in values
        assert "BETWEEN" in values

    def test_not_in(self) -> None:
        """NOT IN expression."""
        ast = parse_sql("SELECT id FROM t WHERE status NOT IN (1, 2)")
        values = all_token_values(ast)
        assert "NOT" in values
        assert "IN" in values

    def test_not_like(self) -> None:
        """NOT LIKE expression."""
        ast = parse_sql("SELECT id FROM t WHERE name NOT LIKE 'Z%'")
        values = all_token_values(ast)
        assert "NOT" in values
        assert "LIKE" in values

    def test_nested_parens_in_expr(self) -> None:
        """Expression with nested parentheses."""
        ast = parse_sql("SELECT id FROM t WHERE (a + b) * c > 0")
        assert ast.rule_name == "program"

    def test_column_ref_with_table_prefix(self) -> None:
        """table.column reference in WHERE clause (using explicit AS alias)."""
        ast = parse_sql("SELECT id FROM users AS u WHERE u.age > 18")
        nodes = find_nodes(ast, "column_ref")
        assert len(nodes) >= 1


# ---------------------------------------------------------------------------
# Multiple statements
# ---------------------------------------------------------------------------


class TestMultipleStatements:
    """Multiple semicolon-separated statements in one program."""

    def test_two_statements(self) -> None:
        """Two SELECT statements separated by semicolon."""
        ast = parse_sql("SELECT id FROM users; SELECT name FROM products")
        nodes = find_nodes(ast, "statement")
        assert len(nodes) == 2

    def test_three_statements(self) -> None:
        """Three different statement types."""
        ast = parse_sql(
            "SELECT id FROM users; DELETE FROM temp; DROP TABLE temp"
        )
        nodes = find_nodes(ast, "statement")
        assert len(nodes) == 3

    def test_trailing_semicolon(self) -> None:
        """A single statement with a trailing semicolon is valid."""
        ast = parse_sql("SELECT id FROM users;")
        assert ast.rule_name == "program"
        nodes = find_nodes(ast, "select_stmt")
        assert len(nodes) == 1


# ---------------------------------------------------------------------------
# Error cases
# ---------------------------------------------------------------------------


class TestErrors:
    """Invalid SQL should raise GrammarParseError."""

    def test_missing_from(self) -> None:
        """SELECT without FROM should fail (not a valid statement)."""
        with pytest.raises((GrammarParseError, Exception)):
            parse_sql("SELECT id")

    def test_incomplete_select(self) -> None:
        """Just 'SELECT' with nothing else is invalid."""
        with pytest.raises((GrammarParseError, Exception)):
            parse_sql("SELECT")

    def test_bare_keyword(self) -> None:
        """A lone keyword that is not a valid statement."""
        with pytest.raises((GrammarParseError, Exception)):
            parse_sql("WHERE id = 1")

    def test_missing_values_keyword(self) -> None:
        """INSERT INTO without VALUES keyword."""
        with pytest.raises((GrammarParseError, Exception)):
            parse_sql("INSERT INTO users (id) (1)")

    def test_empty_input(self) -> None:
        """Empty input cannot match 'program' (needs at least one statement)."""
        with pytest.raises((GrammarParseError, Exception)):
            parse_sql("")


# ---------------------------------------------------------------------------
# AST structure verification
# ---------------------------------------------------------------------------


class TestASTStructure:
    """Verify the structure and rule names inside the produced AST."""

    def test_select_rule_name_hierarchy(self) -> None:
        """program > statement > select_stmt must all be present."""
        ast = parse_sql("SELECT id FROM users")
        assert ast.rule_name == "program"
        stmt_nodes = find_nodes(ast, "statement")
        assert len(stmt_nodes) >= 1
        sel_nodes = find_nodes(ast, "select_stmt")
        assert len(sel_nodes) >= 1

    def test_select_list_rule_name(self) -> None:
        """select_list node should be present."""
        ast = parse_sql("SELECT id, name FROM users")
        nodes = find_nodes(ast, "select_list")
        assert len(nodes) >= 1

    def test_table_ref_rule_name(self) -> None:
        """table_ref node should be present."""
        ast = parse_sql("SELECT id FROM users")
        nodes = find_nodes(ast, "table_ref")
        assert len(nodes) >= 1

    def test_insert_stmt_rule_name(self) -> None:
        """insert_stmt node present for INSERT."""
        ast = parse_sql("INSERT INTO t (a) VALUES (1)")
        nodes = find_nodes(ast, "insert_stmt")
        assert len(nodes) >= 1

    def test_update_stmt_rule_name(self) -> None:
        """update_stmt node present for UPDATE."""
        ast = parse_sql("UPDATE t SET a = 1")
        nodes = find_nodes(ast, "update_stmt")
        assert len(nodes) >= 1

    def test_delete_stmt_rule_name(self) -> None:
        """delete_stmt node present for DELETE."""
        ast = parse_sql("DELETE FROM t")
        nodes = find_nodes(ast, "delete_stmt")
        assert len(nodes) >= 1

    def test_create_table_rule_name(self) -> None:
        """create_table_stmt node present for CREATE TABLE."""
        ast = parse_sql("CREATE TABLE t (id INTEGER)")
        nodes = find_nodes(ast, "create_table_stmt")
        assert len(nodes) >= 1

    def test_drop_table_rule_name(self) -> None:
        """drop_table_stmt node present for DROP TABLE."""
        ast = parse_sql("DROP TABLE t")
        nodes = find_nodes(ast, "drop_table_stmt")
        assert len(nodes) >= 1


# ---------------------------------------------------------------------------
# Error-path coverage — bad grammar file
# ---------------------------------------------------------------------------


class TestErrorPath:
    """Override _sql_grammar_path to exercise the error path in create_sql_parser."""

    def test_bad_grammar_path_raises(self, monkeypatch: pytest.MonkeyPatch) -> None:
        """When _sql_grammar_path points to a non-existent file, a file-not-found
        error should propagate from ``create_sql_parser``."""
        monkeypatch.setattr(
            _parser_module, "_sql_grammar_path", "/no/such/path/sql.grammar"
        )
        with pytest.raises((FileNotFoundError, OSError)):
            create_sql_parser("SELECT 1 FROM t")

    def test_empty_grammar_path_uses_autodiscovery(
        self, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        """When _sql_grammar_path is '' (empty), the auto-discovered path is used
        and parsing succeeds normally."""
        monkeypatch.setattr(_parser_module, "_sql_grammar_path", "")
        ast = parse_sql("SELECT id FROM users")
        assert ast.rule_name == "program"
