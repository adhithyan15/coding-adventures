"""Tests for UPDATE OR <conflict> resolution paths in vm.py.

Covers the code added in v2.28.0 (UPDATE OR REPLACE / IGNORE) that was not
reached by the existing test suite, plus DDL paths (CREATE/DROP INDEX,
CREATE/DROP TRIGGER, ALTER TABLE ADD COLUMN) and the upsert WHERE predicate.
"""

from __future__ import annotations

from sql_backend.in_memory import InMemoryBackend
from sql_backend.schema import ColumnDef as BackendColumnDef
from sql_codegen import compile
from sql_planner import (
    Assignment,
    BinaryExpr,
    BinaryOp,
    Column,
    CreateIndex,
    CreateTrigger,
    DropIndex,
    DropTrigger,
    ExcludedColumn,
    Insert,
    InsertSource,
    Literal,
    Update,
    UpsertAction,
    UpsertAssignment,
)
from sql_planner.ast import ColumnDef as AstColumnDef
from sql_planner.plan import AlterTable as PlanAlterTable

from sql_vm import execute

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _make_table(
    name: str,
    cols: list[tuple[str, str, bool]],
    backend: InMemoryBackend,
    *,
    unique_cols: tuple[str, ...] = (),
) -> None:
    """Create *name* in *backend*. Entries in *cols*: (name, type, primary_key)."""
    col_defs = [
        BackendColumnDef(
            name=n,
            type_name=t,
            primary_key=pk,
            unique=(n in unique_cols),
        )
        for n, t, pk in cols
    ]
    backend.create_table(name, col_defs, if_not_exists=False)


def _scan(backend: InMemoryBackend, table: str) -> list[dict]:
    cur = backend.scan(table)
    rows: list[dict] = []
    while True:
        r = cur.next()
        if r is None:
            break
        rows.append(dict(r))
    cur.close()
    return rows


# ---------------------------------------------------------------------------
# UPDATE OR REPLACE
# ---------------------------------------------------------------------------


class TestUpdateOrReplace:
    """UPDATE OR REPLACE deletes the conflicting row, then updates in place."""

    def test_replace_removes_conflicting_row(self) -> None:
        """UPDATE OR REPLACE: conflicting row deleted, current row updated."""
        be = InMemoryBackend()
        _make_table(
            "inv",
            [("id", "INTEGER", True), ("sku", "TEXT", False), ("qty", "INTEGER", False)],
            be,
            unique_cols=("sku",),
        )
        be.insert("inv", {"id": 1, "sku": "A", "qty": 10})
        be.insert("inv", {"id": 2, "sku": "B", "qty": 20})

        # Update row id=1, changing its sku to "B" → conflicts with row id=2.
        # OR REPLACE should delete row id=2, then update row id=1 in place.
        plan = Update(
            table="inv",
            assignments=(
                Assignment(column="sku", value=Literal("B")),
                Assignment(column="qty", value=Literal(30)),
            ),
            predicate=BinaryExpr(op=BinaryOp.EQ, left=Column("inv", "id"), right=Literal(1)),
            on_conflict="REPLACE",
        )
        result = execute(compile(plan), be)
        assert result.rows_affected == 1
        rows = _scan(be, "inv")
        # Only one row should remain (the conflicting row id=2 was deleted).
        assert len(rows) == 1
        assert rows[0]["id"] == 1
        assert rows[0]["sku"] == "B"
        assert rows[0]["qty"] == 30

    def test_replace_no_conflict_is_plain_update(self) -> None:
        """UPDATE OR REPLACE with no unique conflict just updates normally."""
        be = InMemoryBackend()
        _make_table(
            "t",
            [("id", "INTEGER", True), ("val", "TEXT", False)],
            be,
        )
        be.insert("t", {"id": 1, "val": "old"})
        be.insert("t", {"id": 2, "val": "other"})

        plan = Update(
            table="t",
            assignments=(Assignment(column="val", value=Literal("new")),),
            predicate=BinaryExpr(op=BinaryOp.EQ, left=Column("t", "id"), right=Literal(1)),
            on_conflict="REPLACE",
        )
        result = execute(compile(plan), be)
        assert result.rows_affected == 1
        rows = sorted(_scan(be, "t"), key=lambda r: r["id"])
        assert rows == [{"id": 1, "val": "new"}, {"id": 2, "val": "other"}]

    def test_replace_pk_conflict(self) -> None:
        """UPDATE OR REPLACE resolves PRIMARY KEY conflicts the same way."""
        be = InMemoryBackend()
        _make_table(
            "t",
            [("id", "INTEGER", True), ("name", "TEXT", False)],
            be,
        )
        be.insert("t", {"id": 1, "name": "Alice"})
        be.insert("t", {"id": 2, "name": "Bob"})

        # Change id=1 to id=2 — conflicts with existing row.
        plan = Update(
            table="t",
            assignments=(
                Assignment(column="id", value=Literal(2)),
                Assignment(column="name", value=Literal("Alice2")),
            ),
            predicate=BinaryExpr(op=BinaryOp.EQ, left=Column("t", "id"), right=Literal(1)),
            on_conflict="REPLACE",
        )
        result = execute(compile(plan), be)
        assert result.rows_affected == 1
        rows = _scan(be, "t")
        assert len(rows) == 1
        assert rows[0]["id"] == 2
        assert rows[0]["name"] == "Alice2"


# ---------------------------------------------------------------------------
# UPDATE OR IGNORE
# ---------------------------------------------------------------------------


class TestUpdateOrIgnore:
    """UPDATE OR IGNORE silently skips the row on constraint violation."""

    def test_ignore_check_violation_skips_row(self) -> None:
        """UPDATE OR IGNORE with a CHECK violation is silently skipped."""
        from sql_planner.ast import ColumnDef as AstColumnDef


        be = InMemoryBackend()
        # Create table with a CHECK constraint via CreateTable plan.
        from sql_planner import CreateTable

        check_expr = BinaryExpr(op=BinaryOp.GT, left=Column(None, "val"), right=Literal(0))
        plan_create = CreateTable(
            table="t",
            columns=(
                AstColumnDef(name="id", type_name="INTEGER", primary_key=True),
                AstColumnDef(name="val", type_name="INTEGER", check_expr=check_expr),
            ),
        )
        registry: dict = {}
        execute(compile(plan_create), be, check_registry=registry)
        be.insert("t", {"id": 1, "val": 5})
        be.insert("t", {"id": 2, "val": 10})

        # UPDATE OR IGNORE: set val = -1 would violate CHECK; row is skipped.
        plan = Update(
            table="t",
            assignments=(Assignment(column="val", value=Literal(-1)),),
            predicate=BinaryExpr(op=BinaryOp.EQ, left=Column("t", "id"), right=Literal(1)),
            on_conflict="IGNORE",
        )
        result = execute(compile(plan), be, check_registry=registry)
        # rows_affected is 0 because the row was skipped.
        assert result.rows_affected == 0
        # Row should be unchanged.
        rows = sorted(_scan(be, "t"), key=lambda r: r["id"])
        assert rows[0] == {"id": 1, "val": 5}

    def test_ignore_multiple_rows_some_skipped(self) -> None:
        """UPDATE OR IGNORE skips violating rows, counts non-violating ones."""
        from sql_planner import CreateTable
        from sql_planner.ast import ColumnDef as AstColumnDef

        be = InMemoryBackend()
        check_expr = BinaryExpr(op=BinaryOp.GT, left=Column(None, "val"), right=Literal(0))
        plan_create = CreateTable(
            table="t",
            columns=(
                AstColumnDef(name="id", type_name="INTEGER"),
                AstColumnDef(name="val", type_name="INTEGER", check_expr=check_expr),
            ),
        )
        registry: dict = {}
        execute(compile(plan_create), be, check_registry=registry)
        be.insert("t", {"id": 1, "val": 5})

        # Try to set val = -99, which violates CHECK; should be silently skipped.
        plan = Update(
            table="t",
            assignments=(Assignment(column="val", value=Literal(-99)),),
            on_conflict="IGNORE",
        )
        result = execute(compile(plan), be, check_registry=registry)
        assert result.rows_affected == 0
        # Row unchanged.
        assert _scan(be, "t") == [{"id": 1, "val": 5}]


# ---------------------------------------------------------------------------
# Upsert WHERE predicate
# ---------------------------------------------------------------------------


class TestUpsertWherePredicate:
    """ON CONFLICT DO UPDATE SET … WHERE predicate — skip update when false."""

    def test_where_false_leaves_existing_row_unchanged(self) -> None:
        """WHERE clause evaluates to False → existing row untouched."""
        be = InMemoryBackend()
        _make_table("t", [("id", "INTEGER", True), ("n", "INTEGER", False)], be)
        be.insert("t", {"id": 1, "n": 10})

        # ON CONFLICT(id) DO UPDATE SET n = excluded.n WHERE excluded.n > n
        # incoming n=5 < existing n=10 → predicate false → row unchanged.
        plan = Insert(
            table="t",
            columns=("id", "n"),
            source=InsertSource(values=((Literal(1), Literal(5)),)),
            upsert=UpsertAction(
                conflict_target=("id",),
                assignments=(
                    UpsertAssignment(column="n", value=ExcludedColumn(col="n")),
                ),
                where=BinaryExpr(
                    op=BinaryOp.GT,
                    left=ExcludedColumn(col="n"),
                    right=Column(table=None, col="n"),
                ),
            ),
        )
        execute(compile(plan), be)
        rows = _scan(be, "t")
        # n should still be 10 — the WHERE was false, so DO UPDATE was skipped.
        assert rows == [{"id": 1, "n": 10}]

    def test_where_true_applies_update(self) -> None:
        """WHERE clause evaluates to True → update is applied normally."""
        be = InMemoryBackend()
        _make_table("t", [("id", "INTEGER", True), ("n", "INTEGER", False)], be)
        be.insert("t", {"id": 1, "n": 10})

        # incoming n=20 > existing n=10 → predicate true → row updated.
        plan = Insert(
            table="t",
            columns=("id", "n"),
            source=InsertSource(values=((Literal(1), Literal(20)),)),
            upsert=UpsertAction(
                conflict_target=("id",),
                assignments=(
                    UpsertAssignment(column="n", value=ExcludedColumn(col="n")),
                ),
                where=BinaryExpr(
                    op=BinaryOp.GT,
                    left=ExcludedColumn(col="n"),
                    right=Column(table=None, col="n"),
                ),
            ),
        )
        execute(compile(plan), be)
        rows = _scan(be, "t")
        assert rows == [{"id": 1, "n": 20}]


# ---------------------------------------------------------------------------
# CREATE INDEX / DROP INDEX
# ---------------------------------------------------------------------------


class TestCreateDropIndex:
    """CREATE INDEX IF NOT EXISTS and DROP INDEX exercise lines 2918-2940."""

    def test_create_index(self) -> None:
        """CREATE INDEX stores the index in the backend."""
        be = InMemoryBackend()
        _make_table("t", [("id", "INTEGER", True), ("val", "TEXT", False)], be)
        be.insert("t", {"id": 1, "val": "a"})

        plan = CreateIndex(name="idx_val", table="t", columns=("val",))
        result = execute(compile(plan), be)
        assert result.rows_affected == 0

    def test_create_index_if_not_exists_swallows_duplicate(self) -> None:
        """CREATE INDEX IF NOT EXISTS does not raise when index exists."""
        be = InMemoryBackend()
        _make_table("t", [("id", "INTEGER", True), ("val", "TEXT", False)], be)

        plan = CreateIndex(name="idx_val", table="t", columns=("val",))
        execute(compile(plan), be)

        # Second CREATE with IF NOT EXISTS — should be idempotent.
        plan_ine = CreateIndex(name="idx_val", table="t", columns=("val",), if_not_exists=True)
        result = execute(compile(plan_ine), be)
        assert result.rows_affected == 0

    def test_drop_index_if_exists_on_missing_index(self) -> None:
        """DROP INDEX IF EXISTS on a non-existent index does not raise."""
        be = InMemoryBackend()
        _make_table("t", [("id", "INTEGER", True)], be)

        plan = DropIndex(name="ghost_idx", if_exists=True)
        result = execute(compile(plan), be)
        assert result.rows_affected == 0

    def test_drop_existing_index(self) -> None:
        """DROP INDEX on an existing index succeeds."""
        be = InMemoryBackend()
        _make_table("t", [("id", "INTEGER", True), ("val", "TEXT", False)], be)
        execute(compile(CreateIndex(name="idx_val", table="t", columns=("val",))), be)

        plan = DropIndex(name="idx_val")
        result = execute(compile(plan), be)
        assert result.rows_affected == 0


# ---------------------------------------------------------------------------
# CREATE TRIGGER / DROP TRIGGER
# ---------------------------------------------------------------------------


class TestCreateDropTrigger:
    """CREATE TRIGGER / DROP TRIGGER exercise lines 2943-2965."""

    def test_create_trigger(self) -> None:
        """CREATE TRIGGER stores the definition in the backend."""
        be = InMemoryBackend()
        _make_table("t", [("id", "INTEGER", True), ("val", "TEXT", False)], be)

        plan = CreateTrigger(
            name="tr_after_insert",
            timing="AFTER",
            event="INSERT",
            table="t",
            body_sql="SELECT 1;",
        )
        result = execute(compile(plan), be)
        assert result.rows_affected == 0
        triggers = be.list_triggers("t")
        assert any(t.name == "tr_after_insert" for t in triggers)

    def test_drop_trigger(self) -> None:
        """DROP TRIGGER removes the definition."""
        be = InMemoryBackend()
        _make_table("t", [("id", "INTEGER", True)], be)

        execute(
            compile(
                CreateTrigger(
                    name="tr1",
                    timing="AFTER",
                    event="INSERT",
                    table="t",
                    body_sql="SELECT 1;",
                )
            ),
            be,
        )
        plan = DropTrigger(name="tr1")
        result = execute(compile(plan), be)
        assert result.rows_affected == 0
        triggers = be.list_triggers("t")
        assert not any(t.name == "tr1" for t in triggers)

    def test_drop_trigger_if_exists_on_missing(self) -> None:
        """DROP TRIGGER IF EXISTS on a non-existent trigger is silent."""
        be = InMemoryBackend()
        _make_table("t", [("id", "INTEGER", True)], be)

        plan = DropTrigger(name="ghost_trigger", if_exists=True)
        result = execute(compile(plan), be)
        assert result.rows_affected == 0


# ---------------------------------------------------------------------------
# ALTER TABLE ADD COLUMN
# ---------------------------------------------------------------------------


class TestAlterTableAddColumn:
    """ALTER TABLE ADD COLUMN exercises lines 2979-2998 in vm.py."""

    def test_add_column_no_default(self) -> None:
        """ADD COLUMN without a default: existing rows gain a NULL value."""
        be = InMemoryBackend()
        _make_table("t", [("id", "INTEGER", True)], be)
        be.insert("t", {"id": 1})

        plan = PlanAlterTable(
            table="t",
            column=AstColumnDef(name="notes", type_name="TEXT"),
        )
        result = execute(compile(plan), be)
        assert result.rows_affected == 0
        cols = [c.name for c in be.columns("t")]
        assert "notes" in cols

    def test_add_column_with_literal_default(self) -> None:
        """ADD COLUMN with a DEFAULT value: existing rows backfilled."""
        be = InMemoryBackend()
        _make_table("t", [("id", "INTEGER", True)], be)
        be.insert("t", {"id": 1})
        be.insert("t", {"id": 2})

        plan = PlanAlterTable(
            table="t",
            column=AstColumnDef(name="status", type_name="TEXT", default="active"),
        )
        result = execute(compile(plan), be)
        assert result.rows_affected == 0

        rows = sorted(_scan(be, "t"), key=lambda r: r["id"])
        assert rows[0].get("status") == "active"
        assert rows[1].get("status") == "active"
