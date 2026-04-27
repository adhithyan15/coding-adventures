"""
Unit tests for InMemoryBackend.

These tests exercise the backend directly without going through the
conformance suite. The conformance suite (see test_conformance.py) runs
the same backend through a parallel set of *black-box* assertions;
together the two provide defense in depth.
"""

from __future__ import annotations

import pytest

from sql_backend.backend import TransactionHandle
from sql_backend.errors import (
    ColumnNotFound,
    ConstraintViolation,
    TableAlreadyExists,
    TableNotFound,
    Unsupported,
)
from sql_backend.in_memory import InMemoryBackend
from sql_backend.schema import ColumnDef


# Fixture: a tiny table we can mutate freely in each test.
def make_backend() -> InMemoryBackend:
    b = InMemoryBackend()
    b.create_table(
        "t",
        [
            ColumnDef(name="id", type_name="INTEGER", primary_key=True),
            ColumnDef(name="val", type_name="TEXT"),
        ],
        if_not_exists=False,
    )
    b.insert("t", {"id": 1, "val": "one"})
    b.insert("t", {"id": 2, "val": "two"})
    return b


class TestConstruction:
    def test_empty(self) -> None:
        b = InMemoryBackend()
        assert b.tables() == []

    def test_from_tables_preloads(self) -> None:
        b = InMemoryBackend.from_tables(
            {"t": ([ColumnDef(name="id", type_name="INTEGER")], [{"id": 1}, {"id": 2}])},
        )
        assert b.tables() == ["t"]
        assert len(b.columns("t")) == 1

    def test_from_tables_bypasses_constraint_check(self) -> None:
        # from_tables() is a fixture helper — it can load data that would
        # fail on insert(). Verify that behavior is preserved (duplicate
        # PK values in the preload do not raise).
        b = InMemoryBackend.from_tables(
            {
                "t": (
                    [ColumnDef(name="id", type_name="INTEGER", primary_key=True)],
                    [{"id": 1}, {"id": 1}],  # Would fail if gone through insert().
                ),
            },
        )
        assert b.tables() == ["t"]


class TestSchema:
    def test_tables_lists_created(self) -> None:
        b = make_backend()
        assert "t" in b.tables()

    def test_columns_raises_on_missing(self) -> None:
        b = make_backend()
        with pytest.raises(TableNotFound):
            b.columns("missing")


class TestInsert:
    def test_basic_insert(self) -> None:
        b = make_backend()
        b.insert("t", {"id": 3, "val": "three"})
        it = b.scan("t")
        ids = []
        while True:
            r = it.next()
            if r is None:
                break
            ids.append(r["id"])
        assert 3 in ids

    def test_rejects_unknown_column(self) -> None:
        b = make_backend()
        with pytest.raises(ColumnNotFound):
            b.insert("t", {"id": 99, "val": "x", "ghost": "z"})

    def test_rejects_duplicate_pk(self) -> None:
        b = make_backend()
        with pytest.raises(ConstraintViolation):
            b.insert("t", {"id": 1, "val": "dup"})

    def test_missing_column_becomes_null(self) -> None:
        b = make_backend()
        b.insert("t", {"id": 42})  # val omitted, no default → NULL
        it = b.scan("t")
        found = None
        while True:
            r = it.next()
            if r is None:
                break
            if r["id"] == 42:
                found = r
                break
        assert found is not None
        assert found["val"] is None

    def test_default_applied_when_missing(self) -> None:
        b = InMemoryBackend()
        b.create_table(
            "d",
            [
                ColumnDef(name="id", type_name="INTEGER", primary_key=True),
                ColumnDef(name="status", type_name="TEXT", default="active"),
            ],
            if_not_exists=False,
        )
        b.insert("d", {"id": 1})
        it = b.scan("d")
        r = it.next()
        assert r == {"id": 1, "status": "active"}

    def test_explicit_null_passes_when_not_not_null(self) -> None:
        b = make_backend()
        b.insert("t", {"id": 3, "val": None})

    def test_not_null_from_primary_key(self) -> None:
        b = make_backend()
        with pytest.raises(ConstraintViolation):
            b.insert("t", {"id": None, "val": "x"})

    def test_unique_violation(self) -> None:
        b = InMemoryBackend()
        b.create_table(
            "u",
            [
                ColumnDef(name="id", type_name="INTEGER", primary_key=True),
                ColumnDef(name="email", type_name="TEXT", unique=True),
            ],
            if_not_exists=False,
        )
        b.insert("u", {"id": 1, "email": "a@b"})
        with pytest.raises(ConstraintViolation):
            b.insert("u", {"id": 2, "email": "a@b"})

    def test_unique_allows_multiple_nulls(self) -> None:
        # SQL semantics: NULL never equals anything, including NULL. A
        # UNIQUE column may hold any number of NULLs.
        b = InMemoryBackend()
        b.create_table(
            "u",
            [
                ColumnDef(name="id", type_name="INTEGER", primary_key=True),
                ColumnDef(name="email", type_name="TEXT", unique=True),
            ],
            if_not_exists=False,
        )
        b.insert("u", {"id": 1, "email": None})
        b.insert("u", {"id": 2, "email": None})


class TestUpdate:
    def test_positioned_update(self) -> None:
        b = make_backend()
        cur = b._open_cursor("t")
        cur.next()  # id=1
        b.update("t", cur, {"val": "ONE"})
        it = b.scan("t")
        r = it.next()
        assert r["val"] == "ONE"

    def test_update_rejects_unknown_column(self) -> None:
        b = make_backend()
        cur = b._open_cursor("t")
        cur.next()
        with pytest.raises(ColumnNotFound):
            b.update("t", cur, {"ghost": "nope"})

    def test_update_not_null_violation(self) -> None:
        b = InMemoryBackend()
        b.create_table(
            "nn",
            [
                ColumnDef(name="id", type_name="INTEGER", primary_key=True),
                ColumnDef(name="v", type_name="TEXT", not_null=True),
            ],
            if_not_exists=False,
        )
        b.insert("nn", {"id": 1, "v": "x"})
        cur = b._open_cursor("nn")
        cur.next()
        with pytest.raises(ConstraintViolation):
            b.update("nn", cur, {"v": None})

    def test_update_unique_ignores_self(self) -> None:
        # Updating a row should NOT flag the row's own existing value as a
        # UNIQUE conflict — we're replacing it.
        b = InMemoryBackend()
        b.create_table(
            "u",
            [
                ColumnDef(name="id", type_name="INTEGER", primary_key=True),
                ColumnDef(name="e", type_name="TEXT", unique=True),
            ],
            if_not_exists=False,
        )
        b.insert("u", {"id": 1, "e": "x@y"})
        cur = b._open_cursor("u")
        cur.next()
        # Re-set the same value — should not raise.
        b.update("u", cur, {"e": "x@y"})

    def test_update_unique_conflict_with_other_row(self) -> None:
        b = InMemoryBackend()
        b.create_table(
            "u",
            [
                ColumnDef(name="id", type_name="INTEGER", primary_key=True),
                ColumnDef(name="e", type_name="TEXT", unique=True),
            ],
            if_not_exists=False,
        )
        b.insert("u", {"id": 1, "e": "a@b"})
        b.insert("u", {"id": 2, "e": "c@d"})
        cur = b._open_cursor("u")
        cur.next()  # id=1
        with pytest.raises(ConstraintViolation):
            b.update("u", cur, {"e": "c@d"})

    def test_update_without_current_row_raises(self) -> None:
        b = make_backend()
        cur = b._open_cursor("t")
        # No next() call — cursor has no current row.
        with pytest.raises(Unsupported):
            b.update("t", cur, {"val": "x"})

    def test_update_on_missing_table(self) -> None:
        b = make_backend()
        cur = b._open_cursor("t")
        cur.next()
        with pytest.raises(TableNotFound):
            b.update("missing", cur, {"val": "x"})

    def test_rejects_foreign_cursor(self) -> None:
        # Any non-ListCursor is rejected.
        b = make_backend()

        class FakeCursor:
            def next(self) -> None:
                return None

            def close(self) -> None:
                pass

            def current_row(self) -> None:
                return None

        with pytest.raises(Unsupported):
            b.update("t", FakeCursor(), {"val": "x"})  # type: ignore[arg-type]


class TestDelete:
    def test_positioned_delete(self) -> None:
        b = make_backend()
        cur = b._open_cursor("t")
        cur.next()  # id=1
        b.delete("t", cur)
        cur.close()
        it = b.scan("t")
        ids = []
        while True:
            r = it.next()
            if r is None:
                break
            ids.append(r["id"])
        assert ids == [2]

    def test_delete_adjusts_cursor(self) -> None:
        # After deleting the current row, the cursor must not skip the
        # subsequent row on the next next() call.
        b = make_backend()
        cur = b._open_cursor("t")
        cur.next()  # id=1
        b.delete("t", cur)
        # id=2 was at index 1, now at index 0. Cursor index was decremented
        # to -1 + 1 = 0 on next call.
        next_row = cur.next()
        assert next_row is not None
        assert next_row["id"] == 2

    def test_delete_without_current_raises(self) -> None:
        b = make_backend()
        cur = b._open_cursor("t")
        with pytest.raises(Unsupported):
            b.delete("t", cur)

    def test_delete_on_missing_table(self) -> None:
        b = make_backend()
        cur = b._open_cursor("t")
        cur.next()
        with pytest.raises(TableNotFound):
            b.delete("missing", cur)

    def test_rejects_foreign_cursor(self) -> None:
        b = make_backend()

        class FakeCursor:
            def next(self) -> None:
                return None

            def close(self) -> None:
                pass

            def current_row(self) -> None:
                return None

        with pytest.raises(Unsupported):
            b.delete("t", FakeCursor())  # type: ignore[arg-type]


class TestDDL:
    def test_create_duplicate_raises(self) -> None:
        b = make_backend()
        with pytest.raises(TableAlreadyExists):
            b.create_table("t", [], if_not_exists=False)

    def test_create_duplicate_if_not_exists(self) -> None:
        b = make_backend()
        b.create_table("t", [], if_not_exists=True)  # no-op
        assert "t" in b.tables()

    def test_drop(self) -> None:
        b = make_backend()
        b.drop_table("t", if_exists=False)
        assert "t" not in b.tables()

    def test_drop_missing_raises(self) -> None:
        b = InMemoryBackend()
        with pytest.raises(TableNotFound):
            b.drop_table("ghost", if_exists=False)

    def test_drop_missing_if_exists_noop(self) -> None:
        b = InMemoryBackend()
        b.drop_table("ghost", if_exists=True)
        assert b.tables() == []


class TestTransactions:
    def test_commit_persists(self) -> None:
        b = make_backend()
        h = b.begin_transaction()
        b.insert("t", {"id": 3, "val": "three"})
        b.commit(h)
        it = b.scan("t")
        ids = []
        while True:
            r = it.next()
            if r is None:
                break
            ids.append(r["id"])
        assert 3 in ids

    def test_rollback_restores(self) -> None:
        b = make_backend()
        h = b.begin_transaction()
        b.insert("t", {"id": 3, "val": "three"})
        b.rollback(h)
        it = b.scan("t")
        ids = []
        while True:
            r = it.next()
            if r is None:
                break
            ids.append(r["id"])
        assert 3 not in ids

    def test_rollback_restores_after_delete(self) -> None:
        b = make_backend()
        cur = b._open_cursor("t")
        cur.next()
        h = b.begin_transaction()
        b.delete("t", cur)
        cur.close()
        b.rollback(h)
        it = b.scan("t")
        ids = []
        while True:
            r = it.next()
            if r is None:
                break
            ids.append(r["id"])
        assert ids == [1, 2]

    def test_nested_transactions_rejected(self) -> None:
        b = make_backend()
        b.begin_transaction()
        with pytest.raises(Unsupported):
            b.begin_transaction()

    def test_commit_without_active_raises(self) -> None:
        b = make_backend()
        with pytest.raises(Unsupported):
            b.commit(TransactionHandle(1))

    def test_rollback_without_active_raises(self) -> None:
        b = make_backend()
        with pytest.raises(Unsupported):
            b.rollback(TransactionHandle(1))

    def test_stale_handle_rejected(self) -> None:
        b = make_backend()
        h1 = b.begin_transaction()
        b.commit(h1)
        # Starting a new transaction, the old handle must not be accepted.
        h2 = b.begin_transaction()
        assert h1 != h2
        with pytest.raises(Unsupported):
            b.commit(h1)
        b.commit(h2)
