"""tests/test_tier3_check_constraints.py — Phase 4a: CHECK constraints.

Tests are organised into three classes:

TestCheckConstraintPipeline
    Unit-level tests: grammar parses, adapter builds ColumnDef with check_expr,
    planner produces CreateTable plan, codegen emits check_instrs in ColumnDef.

TestCheckConstraintIntegration
    End-to-end tests through mini_sqlite.connect().
    Covers: basic check, multiple checks, boundary values, NULL semantics,
    UPDATE enforcement, multi-column checks, and table survival after DDL.

TestCheckConstraintErrors
    Error cases: insert violation, update violation, compound expression.
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


# ---------------------------------------------------------------------------
# TestCheckConstraintPipeline — unit tests
# ---------------------------------------------------------------------------


class TestCheckConstraintPipeline:
    """Verify each pipeline stage handles CHECK correctly."""

    def test_grammar_parses_check_constraint(self) -> None:
        """Grammar accepts CREATE TABLE with CHECK (expr) without a parse error."""
        conn = _conn()
        conn.execute("CREATE TABLE t (id INTEGER, score INTEGER CHECK (score > 0))")

    def test_grammar_parses_compound_check(self) -> None:
        """Grammar accepts CHECK with AND-compound expression."""
        conn = _conn()
        conn.execute(
            "CREATE TABLE t (x INTEGER CHECK (x >= 0 AND x <= 100))"
        )

    def test_adapter_populates_check_expr(self) -> None:
        """Adapter builds ColumnDef.check_expr from the parse tree."""
        from sql_parser import parse_sql
        from sql_planner import BinaryExpr, BinaryOp

        from mini_sqlite.adapter import to_statement

        ast = parse_sql("CREATE TABLE t (id INTEGER, score INTEGER CHECK (score > 0))")
        stmt = to_statement(ast)
        score_col = next(c for c in stmt.columns if c.name == "score")
        assert score_col.check_expr is not None
        assert isinstance(score_col.check_expr, BinaryExpr)
        assert score_col.check_expr.op == BinaryOp.GT

    def test_codegen_compiles_check_instrs(self) -> None:
        """Codegen stores compiled check instructions in IR ColumnDef."""
        from sql_codegen import CHECK_CURSOR_ID, IrColumnDef, LoadColumn, compile
        from sql_codegen import CreateTable as IrCreateTable
        from sql_parser import parse_sql
        from sql_planner import plan
        from sql_planner.schema_provider import InMemorySchemaProvider

        from mini_sqlite.adapter import to_statement

        ast = parse_sql("CREATE TABLE t (id INTEGER, score INTEGER CHECK (score > 0))")
        stmt = to_statement(ast)
        p = plan(stmt, InMemorySchemaProvider({}))
        prog = compile(p)

        create_instrs = [i for i in prog.instructions if isinstance(i, IrCreateTable)]
        assert len(create_instrs) == 1
        ir_ct = create_instrs[0]
        score_ir = next(c for c in ir_ct.columns if c.name == "score")
        assert isinstance(score_ir, IrColumnDef)
        assert len(score_ir.check_instrs) > 0
        # The first instruction loads the column from the sentinel cursor.
        first = score_ir.check_instrs[0]
        assert isinstance(first, LoadColumn)
        assert first.cursor_id == CHECK_CURSOR_ID
        assert first.column == "score"


# ---------------------------------------------------------------------------
# TestCheckConstraintIntegration — end-to-end SQL tests
# ---------------------------------------------------------------------------


class TestCheckConstraintIntegration:
    """Full pipeline: SQL text → backend mutation with CHECK enforcement."""

    def test_insert_valid_row_passes(self) -> None:
        """INSERT satisfying CHECK (score > 0) succeeds."""
        conn = _conn()
        conn.execute("CREATE TABLE t (id INTEGER, score INTEGER CHECK (score > 0))")
        conn.execute("INSERT INTO t VALUES (1, 5)")
        row = conn.execute("SELECT score FROM t WHERE id = 1").fetchone()
        assert row == (5,)

    def test_insert_boundary_value_passes(self) -> None:
        """CHECK (score > 0): score = 1 is the minimum valid value."""
        conn = _conn()
        conn.execute("CREATE TABLE t (id INTEGER, score INTEGER CHECK (score > 0))")
        conn.execute("INSERT INTO t VALUES (1, 1)")
        row = conn.execute("SELECT score FROM t WHERE id = 1").fetchone()
        assert row == (1,)

    def test_insert_null_passes(self) -> None:
        """NULL bypasses CHECK (SQL three-valued-logic: NULL result is not FALSE)."""
        conn = _conn()
        conn.execute("CREATE TABLE t (id INTEGER, score INTEGER CHECK (score > 0))")
        conn.execute("INSERT INTO t (id) VALUES (1)")
        row = conn.execute("SELECT score FROM t WHERE id = 1").fetchone()
        assert row == (None,)

    def test_update_valid_passes(self) -> None:
        """UPDATE setting a valid value passes CHECK."""
        conn = _conn()
        conn.execute("CREATE TABLE t (id INTEGER, score INTEGER CHECK (score > 0))")
        conn.execute("INSERT INTO t VALUES (1, 5)")
        conn.execute("UPDATE t SET score = 99 WHERE id = 1")
        row = conn.execute("SELECT score FROM t WHERE id = 1").fetchone()
        assert row == (99,)

    def test_multiple_check_columns(self) -> None:
        """Multiple columns with separate CHECK constraints are all enforced."""
        conn = _conn()
        conn.execute(
            "CREATE TABLE t (id INTEGER, a INTEGER CHECK (a > 0), b INTEGER CHECK (b < 100))"
        )
        conn.execute("INSERT INTO t VALUES (1, 1, 99)")
        row = conn.execute("SELECT a, b FROM t WHERE id = 1").fetchone()
        assert row == (1, 99)

    def test_compound_check_expression(self) -> None:
        """CHECK (x >= 0 AND x <= 100) — range constraint works end-to-end."""
        conn = _conn()
        conn.execute("CREATE TABLE t (id INTEGER, x INTEGER CHECK (x >= 0 AND x <= 100))")
        conn.execute("INSERT INTO t VALUES (1, 0)")
        conn.execute("INSERT INTO t VALUES (2, 50)")
        conn.execute("INSERT INTO t VALUES (3, 100)")
        rows = conn.execute("SELECT id, x FROM t ORDER BY id").fetchall()
        assert rows == [(1, 0), (2, 50), (3, 100)]

    def test_no_check_table_unaffected(self) -> None:
        """Tables without CHECK constraints are unaffected by the registry."""
        conn = _conn()
        conn.execute("CREATE TABLE t (id INTEGER, score INTEGER)")
        conn.execute("INSERT INTO t VALUES (1, -999)")
        row = conn.execute("SELECT score FROM t WHERE id = 1").fetchone()
        assert row == (-999,)

    def test_check_survives_other_inserts(self) -> None:
        """CHECK registry persists across many execute() calls."""
        conn = _conn()
        conn.execute("CREATE TABLE t (id INTEGER, score INTEGER CHECK (score > 0))")
        for i in range(1, 6):
            conn.execute(f"INSERT INTO t VALUES ({i}, {i * 10})")
        rows = conn.execute("SELECT id FROM t ORDER BY id").fetchall()
        assert rows == [(1,), (2,), (3,), (4,), (5,)]


# ---------------------------------------------------------------------------
# TestCheckConstraintErrors — error cases
# ---------------------------------------------------------------------------


class TestCheckConstraintErrors:
    """Error cases: violations on INSERT and UPDATE."""

    def test_insert_violates_check(self) -> None:
        """INSERT with score = -1 violates CHECK (score > 0)."""
        conn = _conn()
        conn.execute("CREATE TABLE t (id INTEGER, score INTEGER CHECK (score > 0))")
        with pytest.raises(IntegrityError):
            conn.execute("INSERT INTO t VALUES (1, -1)")

    def test_insert_zero_violates_check(self) -> None:
        """INSERT with score = 0 violates CHECK (score > 0): 0 is not > 0."""
        conn = _conn()
        conn.execute("CREATE TABLE t (id INTEGER, score INTEGER CHECK (score > 0))")
        with pytest.raises(IntegrityError):
            conn.execute("INSERT INTO t VALUES (1, 0)")

    def test_update_violates_check(self) -> None:
        """UPDATE setting score = -5 violates CHECK (score > 0)."""
        conn = _conn()
        conn.execute("CREATE TABLE t (id INTEGER, score INTEGER CHECK (score > 0))")
        conn.execute("INSERT INTO t VALUES (1, 10)")
        with pytest.raises(IntegrityError):
            conn.execute("UPDATE t SET score = -5 WHERE id = 1")
        # Row should be unchanged after the failed update.
        row = conn.execute("SELECT score FROM t WHERE id = 1").fetchone()
        assert row == (10,)

    def test_second_column_check_violated(self) -> None:
        """Second column's CHECK is also enforced."""
        conn = _conn()
        conn.execute(
            "CREATE TABLE t (id INTEGER, a INTEGER CHECK (a > 0), b INTEGER CHECK (b < 100))"
        )
        conn.execute("INSERT INTO t VALUES (1, 1, 99)")
        # Valid a, invalid b
        with pytest.raises(IntegrityError):
            conn.execute("INSERT INTO t VALUES (2, 1, 100)")
        # Invalid a, valid b
        with pytest.raises(IntegrityError):
            conn.execute("INSERT INTO t VALUES (3, 0, 50)")

    def test_violation_message_mentions_column(self) -> None:
        """IntegrityError message identifies which column's CHECK failed."""
        conn = _conn()
        conn.execute("CREATE TABLE t (id INTEGER, score INTEGER CHECK (score > 0))")
        with pytest.raises(IntegrityError, match="score"):
            conn.execute("INSERT INTO t VALUES (1, -1)")

    def test_compound_check_lower_bound_violated(self) -> None:
        """CHECK (x >= 0 AND x <= 100): negative value violates lower bound."""
        conn = _conn()
        conn.execute("CREATE TABLE t (id INTEGER, x INTEGER CHECK (x >= 0 AND x <= 100))")
        with pytest.raises(IntegrityError):
            conn.execute("INSERT INTO t VALUES (1, -1)")

    def test_compound_check_upper_bound_violated(self) -> None:
        """CHECK (x >= 0 AND x <= 100): 101 violates upper bound."""
        conn = _conn()
        conn.execute("CREATE TABLE t (id INTEGER, x INTEGER CHECK (x >= 0 AND x <= 100))")
        with pytest.raises(IntegrityError):
            conn.execute("INSERT INTO t VALUES (1, 101)")

    def test_table_not_found_still_raises_operational_error(self) -> None:
        """Unrelated operational errors are unaffected by CHECK support."""
        conn = _conn()
        with pytest.raises(OperationalError):
            conn.execute("INSERT INTO no_such_table VALUES (1, 2)")
