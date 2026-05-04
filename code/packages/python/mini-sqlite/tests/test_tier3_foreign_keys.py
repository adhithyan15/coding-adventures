"""tests/test_tier3_foreign_keys.py — Phase 4b: FOREIGN KEY constraints.

Tests are organised into three classes:

TestForeignKeyPipeline
    Unit-level tests: grammar parses, adapter builds ColumnDef with foreign_key,
    codegen stores foreign_key in IR ColumnDef.

TestForeignKeyIntegration
    End-to-end tests through mini_sqlite.connect().
    Covers: valid insert, null FK, multi-child rows, delete after child deleted.

TestForeignKeyErrors
    Error cases: insert child with missing parent, delete parent with child,
    update child FK to missing parent, error message content.
"""

from __future__ import annotations

import pytest

import mini_sqlite
from mini_sqlite.errors import IntegrityError, OperationalError

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _conn() -> mini_sqlite.Connection:
    return mini_sqlite.connect(":memory:", auto_index=False)


def _setup(conn: mini_sqlite.Connection) -> None:
    """Create customers(id PK) and orders(id, customer_id → customers.id)."""
    conn.execute("CREATE TABLE customers (id INTEGER PRIMARY KEY, name TEXT)")
    conn.execute(
        "CREATE TABLE orders ("
        "id INTEGER PRIMARY KEY, "
        "customer_id INTEGER REFERENCES customers(id)"
        ")"
    )


# ---------------------------------------------------------------------------
# TestForeignKeyPipeline — unit tests
# ---------------------------------------------------------------------------


class TestForeignKeyPipeline:
    """Verify each pipeline stage handles REFERENCES correctly."""

    def test_grammar_parses_references(self) -> None:
        """Grammar accepts REFERENCES table(col) without a parse error."""
        conn = _conn()
        conn.execute("CREATE TABLE p (id INTEGER PRIMARY KEY)")
        conn.execute("CREATE TABLE c (id INTEGER, p_id INTEGER REFERENCES p(id))")

    def test_grammar_parses_references_no_column(self) -> None:
        """Grammar accepts REFERENCES table without explicit column."""
        conn = _conn()
        conn.execute("CREATE TABLE p (id INTEGER PRIMARY KEY)")
        conn.execute("CREATE TABLE c (id INTEGER, p_id INTEGER REFERENCES p)")

    def test_adapter_populates_foreign_key(self) -> None:
        """Adapter builds ColumnDef.foreign_key from the parse tree."""
        from sql_parser import parse_sql

        from mini_sqlite.adapter import to_statement

        ast = parse_sql(
            "CREATE TABLE orders (id INTEGER, customer_id INTEGER REFERENCES customers(id))"
        )
        stmt = to_statement(ast)
        fk_col = next(c for c in stmt.columns if c.name == "customer_id")
        assert fk_col.foreign_key is not None
        ref_table, ref_col = fk_col.foreign_key
        assert ref_table == "customers"
        assert ref_col == "id"

    def test_adapter_populates_foreign_key_no_column(self) -> None:
        """Adapter stores None ref_col when no column is specified."""
        from sql_parser import parse_sql

        from mini_sqlite.adapter import to_statement

        ast = parse_sql(
            "CREATE TABLE orders (id INTEGER, customer_id INTEGER REFERENCES customers)"
        )
        stmt = to_statement(ast)
        fk_col = next(c for c in stmt.columns if c.name == "customer_id")
        assert fk_col.foreign_key is not None
        ref_table, ref_col = fk_col.foreign_key
        assert ref_table == "customers"
        assert ref_col is None

    def test_codegen_stores_foreign_key_in_ir(self) -> None:
        """Codegen propagates foreign_key into the IR ColumnDef."""
        from sql_codegen import CreateTable as IrCreateTable
        from sql_codegen import IrColumnDef, compile
        from sql_parser import parse_sql
        from sql_planner import plan
        from sql_planner.schema_provider import InMemorySchemaProvider

        from mini_sqlite.adapter import to_statement

        ast = parse_sql(
            "CREATE TABLE orders (id INTEGER, customer_id INTEGER REFERENCES customers(id))"
        )
        stmt = to_statement(ast)
        p = plan(stmt, InMemorySchemaProvider({}))
        prog = compile(p)

        ct = next(i for i in prog.instructions if isinstance(i, IrCreateTable))
        fk_ir = next(c for c in ct.columns if c.name == "customer_id")
        assert isinstance(fk_ir, IrColumnDef)
        assert fk_ir.foreign_key == ("customers", "id")


# ---------------------------------------------------------------------------
# TestForeignKeyIntegration — end-to-end SQL tests
# ---------------------------------------------------------------------------


class TestForeignKeyIntegration:
    """Full pipeline: SQL text → backend mutation with FK enforcement."""

    def test_insert_child_valid(self) -> None:
        """INSERT child with existing parent row succeeds."""
        conn = _conn()
        _setup(conn)
        conn.execute("INSERT INTO customers VALUES (1, 'Alice')")
        conn.execute("INSERT INTO orders VALUES (10, 1)")
        row = conn.execute("SELECT customer_id FROM orders WHERE id = 10").fetchone()
        assert row == (1,)

    def test_insert_multiple_children_same_parent(self) -> None:
        """Multiple children referencing the same parent all succeed."""
        conn = _conn()
        _setup(conn)
        conn.execute("INSERT INTO customers VALUES (1, 'Alice')")
        conn.execute("INSERT INTO orders VALUES (10, 1)")
        conn.execute("INSERT INTO orders VALUES (11, 1)")
        conn.execute("INSERT INTO orders VALUES (12, 1)")
        rows = conn.execute("SELECT id FROM orders ORDER BY id").fetchall()
        assert rows == [(10,), (11,), (12,)]

    def test_insert_null_fk_passes(self) -> None:
        """NULL FK value is allowed (order not assigned to a customer)."""
        conn = _conn()
        _setup(conn)
        conn.execute("INSERT INTO orders (id) VALUES (10)")
        row = conn.execute("SELECT customer_id FROM orders WHERE id = 10").fetchone()
        assert row == (None,)

    def test_delete_child_then_delete_parent(self) -> None:
        """Parent can be deleted once all referencing children are removed."""
        conn = _conn()
        _setup(conn)
        conn.execute("INSERT INTO customers VALUES (1, 'Alice')")
        conn.execute("INSERT INTO orders VALUES (10, 1)")
        conn.execute("DELETE FROM orders WHERE id = 10")
        conn.execute("DELETE FROM customers WHERE id = 1")
        assert conn.execute("SELECT id FROM customers").fetchall() == []

    def test_no_fk_table_unaffected(self) -> None:
        """Tables without FK constraints are completely unaffected."""
        conn = _conn()
        conn.execute("CREATE TABLE t (id INTEGER, val INTEGER)")
        conn.execute("INSERT INTO t VALUES (1, 9999)")
        row = conn.execute("SELECT val FROM t WHERE id = 1").fetchone()
        assert row == (9999,)

    def test_fk_survives_many_inserts(self) -> None:
        """FK registry persists across many execute() calls."""
        conn = _conn()
        _setup(conn)
        for i in range(1, 6):
            conn.execute(f"INSERT INTO customers VALUES ({i}, 'Customer{i}')")
        for i in range(1, 6):
            conn.execute(f"INSERT INTO orders VALUES ({i * 10}, {i})")
        rows = conn.execute("SELECT id FROM orders ORDER BY id").fetchall()
        assert rows == [(10,), (20,), (30,), (40,), (50,)]


# ---------------------------------------------------------------------------
# TestForeignKeyErrors — error cases
# ---------------------------------------------------------------------------


class TestForeignKeyErrors:
    """Error cases: FK violations on INSERT, UPDATE, and DELETE."""

    def test_insert_child_missing_parent(self) -> None:
        """INSERT child with non-existent parent raises IntegrityError."""
        conn = _conn()
        _setup(conn)
        with pytest.raises(IntegrityError):
            conn.execute("INSERT INTO orders VALUES (10, 999)")

    def test_insert_child_missing_parent_no_customers_at_all(self) -> None:
        """INSERT child when parent table is empty raises IntegrityError."""
        conn = _conn()
        _setup(conn)
        with pytest.raises(IntegrityError):
            conn.execute("INSERT INTO orders VALUES (1, 1)")

    def test_update_child_to_missing_parent(self) -> None:
        """UPDATE child FK to non-existent parent raises IntegrityError."""
        conn = _conn()
        _setup(conn)
        conn.execute("INSERT INTO customers VALUES (1, 'Alice')")
        conn.execute("INSERT INTO orders VALUES (10, 1)")
        with pytest.raises(IntegrityError):
            conn.execute("UPDATE orders SET customer_id = 999 WHERE id = 10")
        row = conn.execute("SELECT customer_id FROM orders WHERE id = 10").fetchone()
        assert row == (1,)

    def test_delete_parent_with_child_raises(self) -> None:
        """DELETE parent row referenced by a child raises IntegrityError."""
        conn = _conn()
        _setup(conn)
        conn.execute("INSERT INTO customers VALUES (1, 'Alice')")
        conn.execute("INSERT INTO orders VALUES (10, 1)")
        with pytest.raises(IntegrityError):
            conn.execute("DELETE FROM customers WHERE id = 1")
        row = conn.execute("SELECT id FROM customers WHERE id = 1").fetchone()
        assert row == (1,)

    def test_violation_message_mentions_fk(self) -> None:
        """IntegrityError message mentions FOREIGN KEY."""
        conn = _conn()
        _setup(conn)
        with pytest.raises(IntegrityError, match="FOREIGN KEY"):
            conn.execute("INSERT INTO orders VALUES (10, 42)")

    def test_second_fk_column_enforced(self) -> None:
        """All FK columns on a table are individually enforced."""
        conn = _conn()
        conn.execute("CREATE TABLE a (id INTEGER PRIMARY KEY)")
        conn.execute("CREATE TABLE b (id INTEGER PRIMARY KEY)")
        conn.execute(
            "CREATE TABLE c ("
            "id INTEGER, "
            "a_id INTEGER REFERENCES a(id), "
            "b_id INTEGER REFERENCES b(id)"
            ")"
        )
        conn.execute("INSERT INTO a VALUES (1)")
        conn.execute("INSERT INTO b VALUES (2)")
        # Valid: both parents exist
        conn.execute("INSERT INTO c VALUES (1, 1, 2)")
        # Invalid: b_id=99 missing
        with pytest.raises(IntegrityError):
            conn.execute("INSERT INTO c VALUES (2, 1, 99)")
        # Invalid: a_id=99 missing
        with pytest.raises(IntegrityError):
            conn.execute("INSERT INTO c VALUES (3, 99, 2)")

    def test_table_not_found_still_raises_operational_error(self) -> None:
        """Unrelated operational errors are unaffected by FK support."""
        conn = _conn()
        with pytest.raises(OperationalError):
            conn.execute("INSERT INTO no_such_table VALUES (1, 2)")
