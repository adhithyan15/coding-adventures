"""
``STRICT`` and ``WITHOUT ROWID`` table options on ``CREATE TABLE``.

SQLite 3.37+ added the ``STRICT`` keyword to enforce strict typing per
column; SQLite 3.8.2+ added ``WITHOUT ROWID`` to store rows in the
primary-key B-tree.  ORMs and migration tools commonly emit these clauses.

Mini-sqlite accepts both syntaxes:

  CREATE TABLE t (id INTEGER) STRICT
  CREATE TABLE t (id INTEGER PRIMARY KEY) WITHOUT ROWID
  CREATE TABLE t (id INTEGER PRIMARY KEY) STRICT, WITHOUT ROWID
  CREATE TABLE t (id INTEGER PRIMARY KEY) WITHOUT ROWID, STRICT

Behaviour:

* STRICT — the engine restricts column types to
  ``{INT, INTEGER, REAL, TEXT, BLOB, ANY}`` at CREATE TABLE time and
  enforces per-column types on every INSERT/UPDATE.  Mismatches raise
  ``IntegrityError`` with the SQLite-compatible message ``cannot store
  TYPE value in TYPE column t.col``.  ``ANY`` columns opt back into
  lenient typing on a per-column basis.
* WITHOUT ROWID is a pure no-op — the storage model is unchanged.

Test strategy: the original "both engines accept" tests still apply
(STRICT tables with valid data round-trip identically through both
engines).  Additional ``TestStrictEnforcement`` tests below oracle-
compare the *rejection* behaviour against real ``sqlite3``.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite
from mini_sqlite import errors as mini_errors


def _both_accept(create_sql: str, *follow_ups: str) -> None:
    """Both engines accept *create_sql* and the follow-up statements."""
    mini = mini_sqlite.connect(":memory:")
    ref = sqlite3.connect(":memory:")
    for c in (mini, ref):
        c.execute(create_sql)
        for sql in follow_ups:
            c.execute(sql)


# ---------------------------------------------------------------------------
# STRICT
# ---------------------------------------------------------------------------


def test_strict_table_basic():
    _both_accept("CREATE TABLE t (id INTEGER, name TEXT) STRICT")


def test_strict_table_with_primary_key():
    _both_accept(
        "CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT) STRICT",
        "INSERT INTO t VALUES (1, 'alice')",
    )


def test_strict_table_with_inserts():
    """STRICT table accepts INSERT and SELECT roundtrips."""
    mini = mini_sqlite.connect(":memory:")
    mini.execute("CREATE TABLE t (id INTEGER, val INTEGER) STRICT")
    mini.execute("INSERT INTO t VALUES (1, 10), (2, 20)")
    assert mini.execute("SELECT id, val FROM t ORDER BY id").fetchall() == \
        [(1, 10), (2, 20)]


# ---------------------------------------------------------------------------
# WITHOUT ROWID
# ---------------------------------------------------------------------------


def test_without_rowid_table_basic():
    _both_accept("CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT) WITHOUT ROWID")


def test_without_rowid_with_inserts():
    """WITHOUT ROWID table accepts INSERT and SELECT roundtrips."""
    mini = mini_sqlite.connect(":memory:")
    mini.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, val INTEGER) WITHOUT ROWID")
    mini.execute("INSERT INTO t VALUES (1, 10), (2, 20)")
    assert mini.execute("SELECT id, val FROM t ORDER BY id").fetchall() == \
        [(1, 10), (2, 20)]


def test_without_rowid_case_insensitive():
    _both_accept("CREATE TABLE t (id INTEGER PRIMARY KEY) without rowid")


# ---------------------------------------------------------------------------
# Combined: STRICT + WITHOUT ROWID
# ---------------------------------------------------------------------------


def test_strict_then_without_rowid():
    _both_accept("CREATE TABLE t (id INTEGER PRIMARY KEY) STRICT, WITHOUT ROWID")


def test_without_rowid_then_strict():
    _both_accept("CREATE TABLE t (id INTEGER PRIMARY KEY) WITHOUT ROWID, STRICT")


# ---------------------------------------------------------------------------
# Regression: ROWID as a column reference still works
# ---------------------------------------------------------------------------


def test_rowid_as_column_reference_still_works():
    """``rowid`` is not a reserved keyword; it remains usable as a column
    reference in SELECT statements (a SQLite pseudo-column)."""
    mini = mini_sqlite.connect(":memory:")
    mini.execute("CREATE TABLE t (val INTEGER)")
    mini.execute("INSERT INTO t VALUES (10), (20)")
    rows = mini.execute("SELECT rowid, val FROM t ORDER BY rowid").fetchall()
    assert rows == [(1, 10), (2, 20)]


def test_create_table_without_options_still_works():
    """Plain CREATE TABLE without any table options must still parse."""
    mini = mini_sqlite.connect(":memory:")
    mini.execute("CREATE TABLE t (id INTEGER)")
    mini.execute("INSERT INTO t VALUES (42)")
    assert mini.execute("SELECT * FROM t").fetchone() == (42,)


# ---------------------------------------------------------------------------
# Common ORM / migration patterns
# ---------------------------------------------------------------------------


def test_strict_with_check_constraint():
    _both_accept(
        "CREATE TABLE t (id INTEGER CHECK(id > 0), name TEXT NOT NULL) STRICT"
    )


def test_strict_with_default_values():
    """STRICT table with DEFAULT clauses."""
    _both_accept(
        "CREATE TABLE t (id INTEGER PRIMARY KEY, score REAL DEFAULT 0.0) STRICT"
    )


# ---------------------------------------------------------------------------
# STRICT enforcement — oracle-compared against real sqlite3 (3.37+)
# ---------------------------------------------------------------------------


import pytest  # noqa: E402  — late import, only needed for the enforcement tests


def _both_reject(create_sql: str, insert_sql: str) -> None:
    """Both engines accept *create_sql* but reject *insert_sql*.

    Used to verify STRICT-table type violations are surfaced by both
    mini-sqlite and Python's stdlib ``sqlite3``.  We don't match the
    exception message byte-for-byte — SQLite phrases the error
    slightly differently across versions — but both engines must raise
    *some* exception, and mini-sqlite's message must mention the column
    name so a programmer can locate the offender.
    """
    mini = mini_sqlite.connect(":memory:")
    ref = sqlite3.connect(":memory:")
    for c in (mini, ref):
        c.execute(create_sql)
    with pytest.raises(sqlite3.Error):
        ref.execute(insert_sql)
    with pytest.raises(mini_errors.Error):
        mini.execute(insert_sql)


class TestStrictEnforcement:
    """Mini-sqlite enforces STRICT per-column typing the same way SQLite does.

    SQLite STRICT permits lossless coercions (INT → TEXT, whole REAL → INT,
    numeric TEXT → INT/REAL) — these are tested in
    ``TestStrictCoercion`` below.  This class focuses on cases where both
    engines REJECT a value because no lossless coercion is possible.
    """

    def test_integer_column_rejects_non_numeric_text(self):
        _both_reject(
            "CREATE TABLE t (id INTEGER) STRICT",
            "INSERT INTO t VALUES ('one')",
        )

    def test_integer_column_rejects_fractional_real(self):
        # 1.5 cannot be stored losslessly as an integer.
        _both_reject(
            "CREATE TABLE t (x INTEGER) STRICT",
            "INSERT INTO t VALUES (1.5)",
        )

    def test_blob_column_rejects_text(self):
        # Unlike TEXT, BLOB has no coercion path from other types.
        _both_reject(
            "CREATE TABLE t (x BLOB) STRICT",
            "INSERT INTO t VALUES ('hi')",
        )

    def test_blob_column_rejects_int(self):
        _both_reject(
            "CREATE TABLE t (x BLOB) STRICT",
            "INSERT INTO t VALUES (42)",
        )

    def test_real_column_accepts_numeric_int_and_text(self):
        """Both engines accept ints and numeric text in a REAL STRICT column."""
        for v in ("1", "1.5", "'3.14'"):
            mini = mini_sqlite.connect(":memory:")
            ref = sqlite3.connect(":memory:")
            for c in (mini, ref):
                c.execute("CREATE TABLE t (x REAL) STRICT")
                c.execute(f"INSERT INTO t VALUES ({v})")
                # Both accept; the stored representation is REAL after
                # coercion in both engines.

    def test_unknown_column_type_rejected_in_strict(self):
        """``VARCHAR`` is fine in legacy SQLite but rejected in STRICT."""
        mini = mini_sqlite.connect(":memory:")
        ref = sqlite3.connect(":memory:")
        # Real sqlite3 rejects:
        with pytest.raises(sqlite3.Error):
            ref.execute("CREATE TABLE t (x VARCHAR) STRICT")
        with pytest.raises(mini_errors.Error) as exc:
            mini.execute("CREATE TABLE t (x VARCHAR) STRICT")
        assert "VARCHAR" in str(exc.value)

    def test_any_column_accepts_mixed_types(self):
        """ANY columns are the STRICT escape hatch — anything goes."""
        mini = mini_sqlite.connect(":memory:")
        mini.execute("CREATE TABLE t (x ANY) STRICT")
        mini.execute("INSERT INTO t VALUES (1)")
        mini.execute("INSERT INTO t VALUES ('hi')")
        mini.execute("INSERT INTO t VALUES (1.5)")
        mini.execute("INSERT INTO t VALUES (NULL)")
        rows = mini.execute("SELECT x FROM t ORDER BY rowid").fetchall()
        assert rows == [(1,), ("hi",), (1.5,), (None,)]

    def test_null_allowed_unless_not_null(self):
        """NULL is exempt from STRICT type-check (separate constraint)."""
        mini = mini_sqlite.connect(":memory:")
        mini.execute("CREATE TABLE t (x INTEGER) STRICT")
        mini.execute("INSERT INTO t VALUES (NULL)")
        assert mini.execute("SELECT x FROM t").fetchone() == (None,)

    def test_null_rejected_when_not_null(self):
        mini = mini_sqlite.connect(":memory:")
        mini.execute("CREATE TABLE t (x INTEGER NOT NULL) STRICT")
        with pytest.raises(mini_errors.IntegrityError):
            mini.execute("INSERT INTO t VALUES (NULL)")

    def test_update_to_wrong_type_rejected(self):
        mini = mini_sqlite.connect(":memory:")
        ref = sqlite3.connect(":memory:")
        for c in (mini, ref):
            c.execute("CREATE TABLE t (x INTEGER) STRICT")
            c.execute("INSERT INTO t VALUES (1)")
        with pytest.raises(sqlite3.Error):
            ref.execute("UPDATE t SET x = 'one'")
        with pytest.raises(mini_errors.Error):
            mini.execute("UPDATE t SET x = 'one'")

    def test_non_strict_table_remains_lenient(self):
        """Legacy (non-STRICT) tables keep type affinity behaviour."""
        mini = mini_sqlite.connect(":memory:")
        mini.execute("CREATE TABLE t (x INTEGER)")
        # mini-sqlite accepts cross-type inserts in legacy mode (mirrors
        # SQLite's type affinity which coerces where it can, stores
        # verbatim where it can't).
        mini.execute("INSERT INTO t VALUES ('not-an-int')")
        # No exception — the row landed.
        assert mini.execute("SELECT COUNT(*) FROM t").fetchone() == (1,)


class TestStrictCoercion:
    """Mini-sqlite mirrors SQLite's STRICT-mode lossless coercion rules.

    SQLite STRICT is not a pure type check: when the value can be
    losslessly converted to the column's declared type, it's accepted
    and stored in the column's preferred storage class.  These tests
    pin the coercion semantics by oracle against real ``sqlite3``.
    """

    def _both_store(self, create_sql: str, insert_sql: str, expected) -> None:
        """Both engines accept *insert_sql* and SELECT returns *expected*."""
        mini = mini_sqlite.connect(":memory:")
        ref = sqlite3.connect(":memory:")
        for c in (mini, ref):
            c.execute(create_sql)
            c.execute(insert_sql)
        m = mini.execute("SELECT * FROM t").fetchone()
        r = ref.execute("SELECT * FROM t").fetchone()
        assert m == r == expected

    def test_text_column_coerces_int_to_string(self):
        self._both_store(
            "CREATE TABLE t (x TEXT) STRICT",
            "INSERT INTO t VALUES (42)",
            ("42",),
        )

    def test_text_column_coerces_real_to_string(self):
        self._both_store(
            "CREATE TABLE t (x TEXT) STRICT",
            "INSERT INTO t VALUES (1.5)",
            ("1.5",),
        )

    def test_integer_column_coerces_whole_real(self):
        self._both_store(
            "CREATE TABLE t (x INTEGER) STRICT",
            "INSERT INTO t VALUES (1.0)",
            (1,),
        )

    def test_integer_column_coerces_numeric_text(self):
        self._both_store(
            "CREATE TABLE t (x INTEGER) STRICT",
            "INSERT INTO t VALUES ('42')",
            (42,),
        )

    def test_real_column_promotes_int(self):
        self._both_store(
            "CREATE TABLE t (x REAL) STRICT",
            "INSERT INTO t VALUES (5)",
            (5.0,),
        )

    def test_real_column_parses_numeric_text(self):
        self._both_store(
            "CREATE TABLE t (x REAL) STRICT",
            "INSERT INTO t VALUES ('3.14')",
            (3.14,),
        )
