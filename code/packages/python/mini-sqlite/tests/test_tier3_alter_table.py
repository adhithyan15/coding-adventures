"""tests/test_tier3_alter_table.py — Phase 3: ALTER TABLE ADD COLUMN.

Tests are organised into three classes mirroring the implementation layers:

TestAlterTablePipeline
    Unit-level tests: grammar parses, adapter builds AlterTableStmt, planner
    produces AlterTable plan node, codegen emits AlterTable IR instruction.

TestAlterTableIntegration
    End-to-end tests through mini_sqlite.connect() (SQL text → backend).
    Covers: basic add, nullable/not-null columns, default values, querying
    the new column, and interactions with existing data.

TestAlterTableErrors
    Error cases: table not found, duplicate column, type errors.
"""

from __future__ import annotations

import pytest

import mini_sqlite
from mini_sqlite.errors import OperationalError

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _conn() -> mini_sqlite.Connection:
    """Return a fresh in-memory connection with auto_index disabled."""
    return mini_sqlite.connect(":memory:", auto_index=False)


def _setup_users(conn: mini_sqlite.Connection) -> None:
    """Create a users table with two rows for round-trip tests."""
    conn.execute("CREATE TABLE users (id INTEGER, name TEXT)")
    conn.execute("INSERT INTO users VALUES (1, 'Alice')")
    conn.execute("INSERT INTO users VALUES (2, 'Bob')")


# ---------------------------------------------------------------------------
# TestAlterTablePipeline — pipeline unit tests
# ---------------------------------------------------------------------------


class TestAlterTablePipeline:
    """Verify each pipeline stage handles ALTER TABLE correctly."""

    def test_grammar_parses_alter_table(self) -> None:
        """Grammar accepts ALTER TABLE … ADD COLUMN without a parse error."""
        conn = _conn()
        conn.execute("CREATE TABLE t (id INTEGER)")
        conn.execute("ALTER TABLE t ADD COLUMN extra TEXT")

    def test_grammar_parses_without_column_keyword(self) -> None:
        """Grammar accepts ALTER TABLE … ADD col_def (no COLUMN keyword)."""
        conn = _conn()
        conn.execute("CREATE TABLE t (id INTEGER)")
        conn.execute("ALTER TABLE t ADD extra TEXT")

    def test_adapter_produces_alter_table_stmt(self) -> None:
        """Adapter builds AlterTableStmt from the parse tree."""
        from sql_parser import parse_sql
        from sql_planner import AlterTableStmt

        from mini_sqlite.adapter import to_statement

        ast = parse_sql("ALTER TABLE users ADD score INTEGER")
        stmt = to_statement(ast)
        assert isinstance(stmt, AlterTableStmt)
        assert stmt.table == "users"
        assert stmt.column.name == "score"
        assert stmt.column.type_name == "INTEGER"

    def test_planner_produces_alter_table_plan(self) -> None:
        """Planner produces an AlterTable plan node from AlterTableStmt."""
        from sql_backend import InMemoryBackend
        from sql_backend.schema import ColumnDef
        from sql_planner import AlterTableStmt, plan
        from sql_planner.plan import AlterTable

        stmt = AlterTableStmt(
            table="users",
            column=ColumnDef(name="score", type_name="INTEGER"),
        )
        backend = InMemoryBackend()
        backend.create_table(
            "users", [ColumnDef(name="id", type_name="INTEGER")], if_not_exists=False
        )
        from sql_planner.schema_provider import InMemorySchemaProvider
        provider = InMemorySchemaProvider({"users": ["id"]})
        p = plan(stmt, provider)
        assert isinstance(p, AlterTable)
        assert p.table == "users"
        assert p.column.name == "score"

    def test_codegen_emits_alter_table_ir(self) -> None:
        """Codegen emits an AlterTable IR instruction."""
        from sql_codegen import AlterTable as IrAlterTable
        from sql_codegen import compile
        from sql_planner.plan import AlterTable

        plan_node = AlterTable(
            table="users",
            column=__import__("sql_backend.schema", fromlist=["ColumnDef"]).ColumnDef(
                name="score", type_name="INTEGER"
            ),
        )
        prog = compile(plan_node)
        alter_instrs = [i for i in prog.instructions if isinstance(i, IrAlterTable)]
        assert len(alter_instrs) == 1
        assert alter_instrs[0].table == "users"
        assert alter_instrs[0].column.name == "score"
        assert alter_instrs[0].column.type == "INTEGER"


# ---------------------------------------------------------------------------
# TestAlterTableIntegration — end-to-end SQL tests
# ---------------------------------------------------------------------------


class TestAlterTableIntegration:
    """Full pipeline: SQL text → backend mutation."""

    def test_add_nullable_column(self) -> None:
        """New nullable column appears in SELECT * with NULL values for old rows."""
        conn = _conn()
        _setup_users(conn)
        conn.execute("ALTER TABLE users ADD COLUMN score INTEGER")
        rows = conn.execute("SELECT id, name, score FROM users ORDER BY id").fetchall()
        assert rows == [(1, "Alice", None), (2, "Bob", None)]

    def test_add_column_without_column_keyword(self) -> None:
        """ALTER TABLE t ADD col_name type (no COLUMN keyword) also works."""
        conn = _conn()
        _setup_users(conn)
        conn.execute("ALTER TABLE users ADD email TEXT")
        rows = conn.execute("SELECT id, email FROM users ORDER BY id").fetchall()
        assert rows == [(1, None), (2, None)]

    def test_new_column_is_writable(self) -> None:
        """After ALTER TABLE, INSERT and UPDATE can target the new column."""
        conn = _conn()
        _setup_users(conn)
        conn.execute("ALTER TABLE users ADD COLUMN score INTEGER")
        conn.execute("UPDATE users SET score = 10 WHERE id = 1")
        conn.execute("UPDATE users SET score = 20 WHERE id = 2")
        rows = conn.execute("SELECT id, score FROM users ORDER BY id").fetchall()
        assert rows == [(1, 10), (2, 20)]

    def test_insert_into_new_column(self) -> None:
        """New rows can specify the added column value."""
        conn = _conn()
        _setup_users(conn)
        conn.execute("ALTER TABLE users ADD COLUMN score INTEGER")
        conn.execute("INSERT INTO users VALUES (3, 'Carol', 99)")
        row = conn.execute("SELECT score FROM users WHERE id = 3").fetchone()
        assert row == (99,)

    def test_add_multiple_columns(self) -> None:
        """Calling ALTER TABLE multiple times adds multiple columns."""
        conn = _conn()
        _setup_users(conn)
        conn.execute("ALTER TABLE users ADD COLUMN score INTEGER")
        conn.execute("ALTER TABLE users ADD COLUMN email TEXT")
        rows = conn.execute("SELECT id, score, email FROM users ORDER BY id").fetchall()
        assert rows == [(1, None, None), (2, None, None)]

    def test_add_column_to_empty_table(self) -> None:
        """ALTER TABLE on an empty table adds the column with no rows to backfill."""
        conn = _conn()
        conn.execute("CREATE TABLE t (id INTEGER)")
        conn.execute("ALTER TABLE t ADD COLUMN data TEXT")
        conn.execute("INSERT INTO t VALUES (1, 'hello')")
        row = conn.execute("SELECT data FROM t WHERE id = 1").fetchone()
        assert row == ("hello",)

    def test_add_not_null_column(self) -> None:
        """NOT NULL column is added with NULL backfill for existing rows."""
        conn = _conn()
        _setup_users(conn)
        conn.execute("ALTER TABLE users ADD COLUMN active INTEGER NOT NULL")
        rows = conn.execute("SELECT id, active FROM users ORDER BY id").fetchall()
        assert rows == [(1, None), (2, None)]

    def test_where_filter_on_new_column(self) -> None:
        """WHERE clause can reference the new column after ALTER TABLE."""
        conn = _conn()
        _setup_users(conn)
        conn.execute("ALTER TABLE users ADD COLUMN score INTEGER")
        conn.execute("UPDATE users SET score = 42 WHERE id = 1")
        rows = conn.execute("SELECT name FROM users WHERE score = 42").fetchall()
        assert rows == [("Alice",)]

    def test_alter_table_commit(self) -> None:
        """ALTER TABLE changes are reflected in the connection after commit."""
        conn = _conn()
        _setup_users(conn)
        conn.execute("ALTER TABLE users ADD COLUMN tag TEXT")
        conn.commit()
        rows = conn.execute("SELECT tag FROM users WHERE id = 1").fetchall()
        assert rows == [(None,)]


# ---------------------------------------------------------------------------
# TestAlterTableErrors — error handling
# ---------------------------------------------------------------------------


class TestAlterTableErrors:
    """Error cases: nonexistent table, duplicate column."""

    def test_table_not_found(self) -> None:
        """ALTER TABLE on a nonexistent table raises OperationalError."""
        conn = _conn()
        with pytest.raises(OperationalError):
            conn.execute("ALTER TABLE no_such_table ADD COLUMN x INTEGER")

    def test_duplicate_column(self) -> None:
        """Adding a column that already exists raises OperationalError."""
        conn = _conn()
        conn.execute("CREATE TABLE t (id INTEGER, name TEXT)")
        with pytest.raises(OperationalError):
            conn.execute("ALTER TABLE t ADD COLUMN name TEXT")
