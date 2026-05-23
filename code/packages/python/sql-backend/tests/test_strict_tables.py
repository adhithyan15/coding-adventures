"""Tests for SQLite STRICT-table type enforcement on InMemoryBackend.

These tests target the backend layer in isolation — they construct
``ColumnDef`` lists and call ``create_table(strict=True)`` directly so
the parser/planner/codegen plumbing is irrelevant.  End-to-end SQL tests
live in ``mini-sqlite/tests/test_tier3_strict_tables.py`` and verify the
same behaviour through the public ``Connection.execute`` API.
"""

from __future__ import annotations

import pytest

from sql_backend.errors import ConstraintViolation
from sql_backend.in_memory import InMemoryBackend
from sql_backend.schema import ColumnDef


def _make(strict: bool, *cols: ColumnDef) -> InMemoryBackend:
    """Construct a backend with one ``t`` table containing *cols*."""
    b = InMemoryBackend()
    b.create_table("t", list(cols), if_not_exists=False, strict=strict)
    return b


class TestCreateTableTypeWhitelist:
    """STRICT tables only allow a small set of column types."""

    def test_int_integer_real_text_blob_any_accepted(self) -> None:
        # The full SQLite-allowed set.  Each in its own table so we don't
        # have to worry about cross-column uniqueness.
        for type_name in ("INT", "INTEGER", "REAL", "TEXT", "BLOB", "ANY"):
            b = InMemoryBackend()
            b.create_table(
                f"t_{type_name.lower()}",
                [ColumnDef(name="x", type_name=type_name)],
                if_not_exists=False,
                strict=True,
            )

    def test_case_insensitive_type_names(self) -> None:
        # SQLite type names are case-insensitive — "integer", "Integer",
        # and "INTEGER" must all be accepted.
        for spelling in ("integer", "Integer", "INTEGER"):
            b = InMemoryBackend()
            b.create_table(
                "t",
                [ColumnDef(name="x", type_name=spelling)],
                if_not_exists=False,
                strict=True,
            )

    def test_rejects_unknown_type_in_strict_table(self) -> None:
        # VARCHAR is fine in legacy SQLite but not in STRICT.
        b = InMemoryBackend()
        with pytest.raises(ConstraintViolation) as exc:
            b.create_table(
                "t",
                [ColumnDef(name="name", type_name="VARCHAR")],
                if_not_exists=False,
                strict=True,
            )
        assert "VARCHAR" in str(exc.value)
        assert "t.name" in str(exc.value)

    def test_non_strict_table_accepts_any_type(self) -> None:
        # Sanity check: without strict=True, the wide-open SQLite rules
        # apply and VARCHAR/BANANA/anything is fine.
        b = InMemoryBackend()
        b.create_table(
            "t",
            [
                ColumnDef(name="a", type_name="VARCHAR"),
                ColumnDef(name="b", type_name="BANANA"),
            ],
            if_not_exists=False,
            strict=False,
        )


class TestInsertTypeEnforcement:
    """Once strict, INSERT validates per-column types using SQLite-style coercion.

    SQLite STRICT *permits* lossless coercions: INT 42 → TEXT '42', whole
    REAL 1.0 → INT 1, numeric-looking TEXT '42' → INT 42, etc.  These
    tests pin the coercion rules to match oracle behaviour against
    sqlite3 3.37+.
    """

    def test_integer_accepts_int(self) -> None:
        b = _make(True, ColumnDef(name="x", type_name="INTEGER"))
        b.insert("t", {"x": 7})

    def test_integer_rejects_non_whole_float(self) -> None:
        # 1.5 is not losslessly representable as an int — rejected.
        b = _make(True, ColumnDef(name="x", type_name="INTEGER"))
        with pytest.raises(ConstraintViolation) as exc:
            b.insert("t", {"x": 1.5})
        assert "cannot store REAL value in INTEGER column" in str(exc.value)

    def test_integer_accepts_whole_float_and_promotes(self) -> None:
        # 1.0 → 1 (whole-number REAL coerces to INT in STRICT mode).
        b = _make(True, ColumnDef(name="x", type_name="INTEGER"))
        b.insert("t", {"x": 1.0})
        row = b.scan("t").next()
        assert row is not None
        assert row["x"] == 1
        assert isinstance(row["x"], int) and not isinstance(row["x"], float)

    def test_integer_rejects_non_numeric_text(self) -> None:
        b = _make(True, ColumnDef(name="x", type_name="INTEGER"))
        with pytest.raises(ConstraintViolation) as exc:
            b.insert("t", {"x": "seven"})
        assert "cannot store TEXT value in INTEGER column" in str(exc.value)

    def test_integer_accepts_numeric_text(self) -> None:
        # SQLite STRICT parses TEXT that looks like an integer.
        b = _make(True, ColumnDef(name="x", type_name="INTEGER"))
        b.insert("t", {"x": "42"})
        row = b.scan("t").next()
        assert row is not None
        assert row["x"] == 42
        assert isinstance(row["x"], int)

    def test_real_accepts_int_and_float(self) -> None:
        b = _make(True, ColumnDef(name="x", type_name="REAL"))
        b.insert("t", {"x": 3})
        b.insert("t", {"x": 2.5})

    def test_real_accepts_numeric_text(self) -> None:
        b = _make(True, ColumnDef(name="x", type_name="REAL"))
        b.insert("t", {"x": "3.14"})
        row = b.scan("t").next()
        assert row is not None
        assert row["x"] == 3.14

    def test_real_rejects_non_numeric_text(self) -> None:
        b = _make(True, ColumnDef(name="x", type_name="REAL"))
        with pytest.raises(ConstraintViolation):
            b.insert("t", {"x": "pi"})

    def test_text_accepts_int_via_coercion(self) -> None:
        # SQLite STRICT *does* accept INT in a TEXT column, coercing it
        # to its decimal string representation.
        b = _make(True, ColumnDef(name="x", type_name="TEXT"))
        b.insert("t", {"x": 42})
        row = b.scan("t").next()
        assert row is not None
        assert row["x"] == "42"

    def test_text_accepts_str(self) -> None:
        b = _make(True, ColumnDef(name="x", type_name="TEXT"))
        b.insert("t", {"x": "hello"})

    def test_blob_rejects_text(self) -> None:
        b = _make(True, ColumnDef(name="x", type_name="BLOB"))
        b.insert("t", {"x": b"\x00\x01"})
        with pytest.raises(ConstraintViolation) as exc:
            b.insert("t", {"x": "not a blob"})
        assert "cannot store TEXT value in BLOB column" in str(exc.value)

    def test_blob_rejects_int(self) -> None:
        # Unlike TEXT, BLOB does NOT accept INT in SQLite STRICT.
        b = _make(True, ColumnDef(name="x", type_name="BLOB"))
        with pytest.raises(ConstraintViolation) as exc:
            b.insert("t", {"x": 42})
        assert "cannot store INT value in BLOB column" in str(exc.value)

    def test_any_column_accepts_everything(self) -> None:
        # The ANY column type is the STRICT escape hatch: a per-column
        # opt-out of strict typing while keeping STRICT on the rest of
        # the table.
        b = _make(True, ColumnDef(name="x", type_name="ANY"))
        b.insert("t", {"x": 1})
        b.insert("t", {"x": "hello"})
        b.insert("t", {"x": 3.14})
        b.insert("t", {"x": b"bytes"})
        b.insert("t", {"x": None})

    def test_null_always_allowed_for_nullable_column(self) -> None:
        # NULL is exempt from the type check (it has no storage class).
        b = _make(True, ColumnDef(name="x", type_name="INTEGER"))
        b.insert("t", {"x": None})

    def test_null_rejected_when_column_is_not_null(self) -> None:
        # NOT NULL is checked *before* the type-check helper.  This is a
        # NOT NULL violation, not a type violation — and we want both
        # constraints layered cleanly, not collapsed.
        b = _make(True, ColumnDef(name="x", type_name="INTEGER", not_null=True))
        with pytest.raises(ConstraintViolation) as exc:
            b.insert("t", {"x": None})
        assert "NOT NULL" in str(exc.value)


class TestUpdateTypeEnforcement:
    """STRICT type validation must also fire on UPDATE."""

    def test_update_to_wrong_type_rejected(self) -> None:
        b = _make(True, ColumnDef(name="x", type_name="INTEGER"))
        b.insert("t", {"x": 1})
        # Open a positioned cursor on the row, then attempt UPDATE.
        cur = b._open_cursor("t")  # noqa: SLF001 — test exercises internals
        cur.next()  # advance to row 0
        # "two" doesn't parse as int — STRICT rejects it.
        with pytest.raises(ConstraintViolation) as exc:
            b.update("t", cur, {"x": "two"})
        assert "cannot store TEXT value in INTEGER column t.x" in str(exc.value)

    def test_update_to_valid_type_succeeds(self) -> None:
        b = _make(True, ColumnDef(name="x", type_name="INTEGER"))
        b.insert("t", {"x": 1})
        cur = b._open_cursor("t")  # noqa: SLF001
        cur.next()
        b.update("t", cur, {"x": 2})
        # Re-scan to verify the row mutated.
        it = b.scan("t")
        row = it.next()
        assert row is not None
        assert row["x"] == 2


class TestBackwardCompatibility:
    """Non-strict tables (the default) keep legacy lenient behaviour."""

    def test_default_create_table_is_not_strict(self) -> None:
        # Existing callers that don't pass strict= must still work.
        b = InMemoryBackend()
        b.create_table(
            "t",
            [ColumnDef(name="x", type_name="VARCHAR")],
            if_not_exists=False,
        )
        # And INSERT accepts whatever Python value we throw at it.
        b.insert("t", {"x": 42})
        b.insert("t", {"x": "string"})
        b.insert("t", {"x": 3.14})

    def test_columns_method_unchanged_for_strict_table(self) -> None:
        # The strict flag lives on _Table, not on individual ColumnDef
        # objects — so backend.columns() returns the original defs
        # untouched.
        b = _make(
            True,
            ColumnDef(name="x", type_name="INTEGER"),
            ColumnDef(name="y", type_name="TEXT"),
        )
        cols = b.columns("t")
        assert [c.name for c in cols] == ["x", "y"]
        assert [c.type_name for c in cols] == ["INTEGER", "TEXT"]
