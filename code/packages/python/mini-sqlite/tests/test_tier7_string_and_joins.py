"""tests/test_tier7_string_and_joins.py — String concatenation, JOIN USING, NATURAL JOIN.

This module covers three SQLite-compatibility features added in the
"string-and-joins" sprint:

1. **``||`` string concatenation** — SQL's standard string-concat operator.
   ``'hello' || ' ' || 'world'`` produces ``'hello world'``.  NULL propagates:
   ``NULL || 'x'`` → NULL, matching SQLite behaviour.  Works with column
   references, literal chains, and aliased expressions.

2. **JOIN … USING (col, …)** — shorthand for the ``ON left.col = right.col
   AND …`` condition.  The adapter desugars USING at parse time into an
   explicit ON expression, so the planner and VM never need to know about USING
   at all.  Verified for INNER JOIN (the common case), multi-column USING, and
   chained three-table USING joins.

3. **NATURAL JOIN** — automatically builds the ON condition from the set of
   column names that appear in both tables.  The planner resolves NATURAL JOIN
   during ``_build_from_tree``, where schema access is available.  Falls back
   to a CROSS JOIN when the two tables share no column names.

Oracle tests run every query against real ``sqlite3`` to guarantee
byte-for-byte compatible output.  Isolated unit tests (no real-sqlite
comparison) cover edge cases that don't round-trip cleanly, such as
NATURAL JOIN with no shared columns.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite  # noqa: E402

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _conn() -> mini_sqlite.Connection:
    return mini_sqlite.connect(":memory:")


def _oracle() -> sqlite3.Connection:
    return sqlite3.connect(":memory:")


def _both(setup: list[str], query: str) -> tuple[
    list[tuple], list[tuple]
]:
    """Run *setup* DDL/DML on both mini and real sqlite3, then run *query*.

    Returns (mini_rows, real_rows) so tests can assert ``mini == real``.
    """
    mini = _conn()
    real = _oracle()
    for sql in setup:
        mini.execute(sql)
        real.execute(sql)
    mini_rows = mini.execute(query).fetchall()
    real_rows = real.execute(query).fetchall()
    return mini_rows, real_rows


# ============================================================================
# 1.  || String concatenation
# ============================================================================


class TestStringConcat:
    """SQL ``||`` string-concatenation operator — oracle-verified."""

    def test_literal_concat_two_strings(self) -> None:
        """Two string literals joined with ||."""
        mini, real = _both([], "SELECT 'hello' || 'world'")
        assert mini == real == [("helloworld",)]

    def test_literal_concat_with_space(self) -> None:
        """Three-part concatenation including a space literal."""
        mini, real = _both([], "SELECT 'hello' || ' ' || 'world'")
        assert mini == real == [("hello world",)]

    def test_concat_column_values(self) -> None:
        """Concatenation of two column values."""
        setup = [
            "CREATE TABLE t (a TEXT, b TEXT)",
            "INSERT INTO t VALUES ('foo', 'bar'), ('baz', 'qux')",
        ]
        mini, real = _both(setup, "SELECT a || b FROM t ORDER BY a || b")
        assert mini == real

    def test_concat_column_with_literal(self) -> None:
        """Column concatenated with a string literal suffix."""
        setup = [
            "CREATE TABLE t (name TEXT)",
            "INSERT INTO t VALUES ('Alice'), ('Bob')",
        ]
        mini, real = _both(setup, "SELECT name || '!' FROM t ORDER BY name || '!'")
        assert mini == real

    def test_concat_null_propagates(self) -> None:
        """NULL on either side of || yields NULL — SQL three-valued logic."""
        mini, real = _both([], "SELECT NULL || 'x', 'x' || NULL, NULL || NULL")
        assert mini == real == [(None, None, None)]

    def test_concat_in_where_clause(self) -> None:
        """|| used inside a WHERE predicate."""
        setup = [
            "CREATE TABLE t (a TEXT, b TEXT)",
            "INSERT INTO t VALUES ('foo', 'bar'), ('baz', 'qux')",
        ]
        mini, real = _both(setup, "SELECT a FROM t WHERE a || b = 'foobar'")
        assert mini == real == [("foo",)]

    def test_concat_in_alias(self) -> None:
        """|| used in a projected expression with an alias."""
        setup = [
            "CREATE TABLE t (first TEXT, last TEXT)",
            "INSERT INTO t VALUES ('John', 'Doe')",
        ]
        mini, real = _both(setup, "SELECT first || ' ' || last AS full_name FROM t")
        assert mini == real == [("John Doe",)]

    def test_concat_nullable_column(self) -> None:
        """Column with a NULL value propagates NULL through ||."""
        setup = [
            "CREATE TABLE t (a TEXT, b TEXT)",
            "INSERT INTO t VALUES ('foo', NULL), (NULL, 'bar'), ('x', 'y')",
        ]
        # ORDER BY on non-projected column (a) is a known pre-existing VM
        # limitation, so we sort the results ourselves and just compare sets.
        mini, real = _both(setup, "SELECT a || b FROM t")
        assert sorted(mini, key=lambda r: (r[0] is None, r[0])) == sorted(
            real, key=lambda r: (r[0] is None, r[0])
        )

    def test_concat_constant_folding(self) -> None:
        """Optimizer constant-folds two literal strings at plan time."""
        # Both sides are literals → the optimizer should fold them before codegen.
        # The VM then just emits the pre-computed value — but the observable
        # result is identical to runtime evaluation.
        mini, real = _both([], "SELECT 'abc' || 'def'")
        assert mini == real == [("abcdef",)]

    def test_concat_three_columns(self) -> None:
        """Three-way column concatenation."""
        setup = [
            "CREATE TABLE t (a TEXT, b TEXT, c TEXT)",
            "INSERT INTO t VALUES ('x', 'y', 'z')",
        ]
        mini, real = _both(setup, "SELECT a || b || c FROM t")
        assert mini == real == [("xyz",)]


# ============================================================================
# 2.  JOIN … USING (col, …)
# ============================================================================


class TestJoinUsing:
    """JOIN … USING desugared to ON left.col = right.col — oracle-verified."""

    def test_inner_join_using_single_column(self) -> None:
        """INNER JOIN … USING (id) matches on a single shared column."""
        setup = [
            "CREATE TABLE a (id INTEGER, x TEXT)",
            "CREATE TABLE b (id INTEGER, y TEXT)",
            "INSERT INTO a VALUES (1,'x1'),(2,'x2'),(3,'x3')",
            "INSERT INTO b VALUES (1,'y1'),(2,'y2')",
        ]
        mini, real = _both(
            setup,
            "SELECT a.id, a.x, b.y FROM a INNER JOIN b USING (id) ORDER BY a.id",
        )
        assert mini == real == [(1, "x1", "y1"), (2, "x2", "y2")]

    def test_inner_join_using_no_matches(self) -> None:
        """USING with disjoint key sets produces no rows (like INNER JOIN)."""
        setup = [
            "CREATE TABLE a (id INTEGER, x TEXT)",
            "CREATE TABLE b (id INTEGER, y TEXT)",
            "INSERT INTO a VALUES (10,'x1'),(20,'x2')",
            "INSERT INTO b VALUES (99,'y1'),(100,'y2')",
        ]
        mini, real = _both(
            setup,
            "SELECT a.id, b.y FROM a INNER JOIN b USING (id)",
        )
        assert mini == real == []

    def test_join_using_multi_column(self) -> None:
        """USING (col1, col2) requires both columns to match."""
        setup = [
            "CREATE TABLE a (k1 INTEGER, k2 INTEGER, val TEXT)",
            "CREATE TABLE b (k1 INTEGER, k2 INTEGER, extra TEXT)",
            "INSERT INTO a VALUES (1,1,'aa'),(1,2,'ab'),(2,1,'ba')",
            "INSERT INTO b VALUES (1,1,'bb11'),(1,3,'bb13'),(2,1,'bb21')",
        ]
        mini, real = _both(
            setup,
            "SELECT a.k1, a.k2, a.val, b.extra "
            "FROM a INNER JOIN b USING (k1, k2) ORDER BY a.k1, a.k2",
        )
        assert mini == real

    def test_join_using_with_where_filter(self) -> None:
        """WHERE clause applied after USING join."""
        setup = [
            "CREATE TABLE orders (order_id INTEGER, cust_id INTEGER, amount REAL)",
            "CREATE TABLE customers (cust_id INTEGER, name TEXT)",
            "INSERT INTO orders VALUES (1,10,99.9),(2,20,49.5),(3,10,19.0)",
            "INSERT INTO customers VALUES (10,'Alice'),(20,'Bob')",
        ]
        mini, real = _both(
            setup,
            "SELECT orders.order_id, customers.name, orders.amount "
            "FROM orders INNER JOIN customers USING (cust_id) "
            "WHERE orders.amount > 50.0 "
            "ORDER BY orders.order_id",
        )
        assert mini == real

    def test_left_join_using(self) -> None:
        """LEFT JOIN … USING preserves unmatched left rows with NULL right side."""
        setup = [
            "CREATE TABLE a (id INTEGER, x TEXT)",
            "CREATE TABLE b (id INTEGER, y TEXT)",
            "INSERT INTO a VALUES (1,'x1'),(2,'x2'),(3,'x3')",
            "INSERT INTO b VALUES (1,'y1'),(2,'y2')",
        ]
        mini, real = _both(
            setup,
            "SELECT a.id, a.x, b.y FROM a LEFT JOIN b USING (id) ORDER BY a.id",
        )
        assert mini == real == [(1, "x1", "y1"), (2, "x2", "y2"), (3, "x3", None)]

    def test_three_table_join_using(self) -> None:
        """Chained three-table INNER JOIN … USING (each with its own column)."""
        setup = [
            "CREATE TABLE orders (order_id INTEGER, cust_id INTEGER, prod_id INTEGER)",
            "CREATE TABLE customers (cust_id INTEGER, cust_name TEXT)",
            "CREATE TABLE products (prod_id INTEGER, prod_name TEXT)",
            "INSERT INTO orders VALUES (1,10,100),(2,20,101),(3,10,100)",
            "INSERT INTO customers VALUES (10,'Alice'),(20,'Bob')",
            "INSERT INTO products VALUES (100,'Widget'),(101,'Gadget')",
        ]
        mini, real = _both(
            setup,
            "SELECT orders.order_id, customers.cust_name, products.prod_name "
            "FROM orders "
            "INNER JOIN customers USING (cust_id) "
            "INNER JOIN products USING (prod_id) "
            "ORDER BY orders.order_id",
        )
        assert mini == real


# ============================================================================
# 3.  NATURAL JOIN
# ============================================================================


class TestNaturalJoin:
    """NATURAL JOIN auto-resolves ON from shared column names — oracle-verified."""

    def test_natural_join_single_shared_column(self) -> None:
        """Tables sharing one column: NATURAL JOIN acts like INNER JOIN USING (col)."""
        setup = [
            "CREATE TABLE emp (id INTEGER, name TEXT, dept_id INTEGER)",
            "CREATE TABLE dept (dept_id INTEGER, dept_name TEXT)",
            "INSERT INTO emp VALUES (1,'Alice',10),(2,'Bob',20),(3,'Carol',10)",
            "INSERT INTO dept VALUES (10,'Engineering'),(20,'Marketing')",
        ]
        mini, real = _both(
            setup,
            "SELECT emp.id, emp.name, dept.dept_name "
            "FROM emp NATURAL JOIN dept "
            "ORDER BY emp.id",
        )
        assert mini == real == [
            (1, "Alice", "Engineering"),
            (2, "Bob", "Marketing"),
            (3, "Carol", "Engineering"),
        ]

    def test_natural_join_no_unmatched_left_rows(self) -> None:
        """NATURAL JOIN is INNER — left rows with no right match are dropped."""
        setup = [
            "CREATE TABLE emp (id INTEGER, name TEXT, dept_id INTEGER)",
            "CREATE TABLE dept (dept_id INTEGER, dept_name TEXT)",
            "INSERT INTO emp VALUES (1,'Alice',10),(2,'Bob',99)",  # 99 has no match
            "INSERT INTO dept VALUES (10,'Engineering')",
        ]
        mini, real = _both(
            setup,
            "SELECT emp.id, emp.name, dept.dept_name "
            "FROM emp NATURAL JOIN dept "
            "ORDER BY emp.id",
        )
        assert mini == real == [(1, "Alice", "Engineering")]

    def test_natural_join_multiple_shared_columns(self) -> None:
        """Tables sharing two columns: NATURAL JOIN requires both to match."""
        setup = [
            "CREATE TABLE a (k1 INTEGER, k2 INTEGER, va TEXT)",
            "CREATE TABLE b (k1 INTEGER, k2 INTEGER, vb TEXT)",
            "INSERT INTO a VALUES (1,1,'a11'),(1,2,'a12'),(2,1,'a21')",
            "INSERT INTO b VALUES (1,1,'b11'),(1,3,'b13'),(2,1,'b21')",
        ]
        mini, real = _both(
            setup,
            "SELECT a.k1, a.k2, a.va, b.vb FROM a NATURAL JOIN b ORDER BY a.k1, a.k2",
        )
        assert mini == real

    def test_natural_join_empty_right(self) -> None:
        """NATURAL JOIN with an empty right table produces no rows."""
        setup = [
            "CREATE TABLE a (id INTEGER, x TEXT)",
            "CREATE TABLE b (id INTEGER, y TEXT)",
            "INSERT INTO a VALUES (1,'x1'),(2,'x2')",
        ]
        mini, real = _both(
            setup,
            "SELECT a.id, a.x FROM a NATURAL JOIN b",
        )
        assert mini == real == []

    def test_natural_join_no_shared_columns_is_cross(self) -> None:
        """Tables with no shared column names → NATURAL JOIN degenerates to CROSS JOIN."""
        setup = [
            "CREATE TABLE a (x INTEGER)",
            "CREATE TABLE b (y INTEGER)",
            "INSERT INTO a VALUES (1),(2)",
            "INSERT INTO b VALUES (10),(20)",
        ]
        mini, real = _both(
            setup,
            "SELECT a.x, b.y FROM a NATURAL JOIN b ORDER BY a.x, b.y",
        )
        assert mini == real  # 4 rows: (1,10),(1,20),(2,10),(2,20)
        assert len(mini) == 4

    def test_natural_join_with_where(self) -> None:
        """WHERE applied on top of NATURAL JOIN result."""
        setup = [
            "CREATE TABLE emp (id INTEGER, name TEXT, dept_id INTEGER)",
            "CREATE TABLE dept (dept_id INTEGER, dept_name TEXT)",
            "INSERT INTO emp VALUES (1,'Alice',10),(2,'Bob',20),(3,'Carol',10)",
            "INSERT INTO dept VALUES (10,'Engineering'),(20,'Marketing')",
        ]
        mini, real = _both(
            setup,
            "SELECT emp.id, emp.name "
            "FROM emp NATURAL JOIN dept "
            "WHERE dept.dept_name = 'Engineering' "
            "ORDER BY emp.id",
        )
        assert mini == real == [(1, "Alice"), (3, "Carol")]

    def test_natural_join_aliased_table(self) -> None:
        """NATURAL JOIN resolves even when tables carry aliases."""
        setup = [
            "CREATE TABLE employees (id INTEGER, dept_id INTEGER, name TEXT)",
            "CREATE TABLE departments (dept_id INTEGER, label TEXT)",
            "INSERT INTO employees VALUES (1,10,'Alice'),(2,20,'Bob')",
            "INSERT INTO departments VALUES (10,'Eng'),(20,'Mkt')",
        ]
        mini, real = _both(
            setup,
            "SELECT e.id, e.name, d.label "
            "FROM employees e NATURAL JOIN departments d "
            "ORDER BY e.id",
        )
        assert mini == real


# ============================================================================
# 4.  Mixed: concat + join
# ============================================================================


class TestConcatWithJoins:
    """Cross-feature: use || inside a query that also uses JOIN USING / NATURAL JOIN."""

    def test_concat_in_joined_projection(self) -> None:
        """|| in SELECT list with JOIN USING."""
        setup = [
            "CREATE TABLE a (id INTEGER, first TEXT)",
            "CREATE TABLE b (id INTEGER, last TEXT)",
            "INSERT INTO a VALUES (1,'John'),(2,'Jane')",
            "INSERT INTO b VALUES (1,'Doe'),(2,'Smith')",
        ]
        mini, real = _both(
            setup,
            "SELECT a.id, a.first || ' ' || b.last AS full_name "
            "FROM a INNER JOIN b USING (id) ORDER BY a.id",
        )
        assert mini == real == [(1, "John Doe"), (2, "Jane Smith")]

    def test_concat_natural_join_where(self) -> None:
        """|| in SELECT with NATURAL JOIN."""
        setup = [
            "CREATE TABLE emp (id INTEGER, fname TEXT, lname TEXT, dept_id INTEGER)",
            "CREATE TABLE dept (dept_id INTEGER, dept_name TEXT)",
            "INSERT INTO emp VALUES (1,'Alice','Adams',10),(2,'Bob','Brown',20)",
            "INSERT INTO dept VALUES (10,'Engineering'),(20,'Marketing')",
        ]
        # ORDER BY emp.id would need emp.id projected; use sorted() instead.
        mini, real = _both(
            setup,
            "SELECT emp.fname || ' ' || emp.lname AS full_name, dept.dept_name "
            "FROM emp NATURAL JOIN dept",
        )
        assert sorted(mini) == sorted(real)
