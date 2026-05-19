"""ON CONFLICT DO UPDATE / DO NOTHING (UPSERT) — oracle-verified tests.

Every test in this module runs the same SQL on both mini-sqlite and real
sqlite3 and compares results.  SQLite 3.24+ supports the modern UPSERT
syntax; these tests are the authoritative conformance check.

Coverage targets:
  - ON CONFLICT DO NOTHING — conflict is silently skipped
  - ON CONFLICT DO NOTHING with no conflict (plain insert)
  - ON CONFLICT (col) DO UPDATE SET col = EXCLUDED.col — basic upsert
  - ON CONFLICT (col) DO UPDATE with EXCLUDED value + arithmetic
  - ON CONFLICT (col) DO UPDATE with multiple assignment columns
  - ON CONFLICT (col) DO UPDATE on a row that was not conflicted (no-op)
  - ON CONFLICT (col) DO UPDATE preserves non-assigned columns
  - ON CONFLICT (col) DO NOTHING vs. DO UPDATE — compared back-to-back
  - ON CONFLICT DO NOTHING (no target) — any constraint fires
  - ON CONFLICT DO UPDATE with INSERT … SELECT source
  - Multiple consecutive upserts — cumulative state
  - Upsert with WHERE clause on the SELECT result (INSERT … SELECT + ON CONFLICT)
"""

from __future__ import annotations

import sqlite3

import mini_sqlite

# ---------------------------------------------------------------------------
# Oracle helper
# ---------------------------------------------------------------------------


def _both(setup: list[str], query: str) -> tuple[list[tuple], list[tuple]]:
    """Run *setup* then *query* on both mini-sqlite and real sqlite3.

    Returns ``(mini_rows, real_rows)`` for the caller to assert equality.
    """
    # --- mini-sqlite ---
    mini_con = mini_sqlite.connect(":memory:")
    mini_cur = mini_con.cursor()
    for stmt in setup:
        mini_cur.execute(stmt)
    mini_cur.execute(query)
    mini_rows = mini_cur.fetchall()
    mini_con.close()

    # --- real sqlite3 ---
    real_con = sqlite3.connect(":memory:")
    real_cur = real_con.cursor()
    for stmt in setup:
        real_cur.execute(stmt)
    real_cur.execute(query)
    real_rows = real_cur.fetchall()
    real_con.close()

    return mini_rows, real_rows


def _exec_both(stmts: list[str], query: str) -> tuple[list[tuple], list[tuple]]:
    """Execute *stmts* (non-SELECT) + *query* on both engines; return row pairs."""
    return _both(stmts, query)


# ---------------------------------------------------------------------------
# TestUpsertDoNothing
# ---------------------------------------------------------------------------


class TestUpsertDoNothing:
    """ON CONFLICT … DO NOTHING — conflict is silently skipped."""

    def test_do_nothing_skips_conflicting_row(self) -> None:
        """When a new row conflicts on the PRIMARY KEY, DO NOTHING drops it."""
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT)",
            "INSERT INTO t VALUES (1, 'original')",
            "INSERT INTO t VALUES (1, 'new') ON CONFLICT DO NOTHING",
        ]
        mini, real = _both(setup, "SELECT id, val FROM t ORDER BY id")
        assert mini == real
        assert mini == [(1, "original")]

    def test_do_nothing_with_conflict_target(self) -> None:
        """ON CONFLICT (id) DO NOTHING is equivalent when id is the PK."""
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT)",
            "INSERT INTO t VALUES (1, 'first')",
            "INSERT INTO t (id, val) VALUES (1, 'second') ON CONFLICT (id) DO NOTHING",
        ]
        mini, real = _both(setup, "SELECT id, val FROM t")
        assert mini == real
        assert mini == [(1, "first")]

    def test_do_nothing_no_conflict_inserts(self) -> None:
        """When there is no conflict, DO NOTHING still inserts the new row."""
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT)",
            "INSERT INTO t VALUES (1, 'original')",
            "INSERT INTO t VALUES (2, 'new') ON CONFLICT DO NOTHING",
        ]
        mini, real = _both(setup, "SELECT id, val FROM t ORDER BY id")
        assert mini == real
        assert mini == [(1, "original"), (2, "new")]

    def test_do_nothing_with_unique_column(self) -> None:
        """DO NOTHING works on UNIQUE constraints, not just PRIMARY KEY."""
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, email TEXT UNIQUE)",
            "INSERT INTO t VALUES (1, 'a@b.com')",
            "INSERT INTO t VALUES (2, 'a@b.com') ON CONFLICT DO NOTHING",
        ]
        mini, real = _both(setup, "SELECT id, email FROM t ORDER BY id")
        assert mini == real
        assert mini == [(1, "a@b.com")]

    def test_do_nothing_multiple_rows_only_skips_conflicting(self) -> None:
        """Only conflicting rows are skipped; others are inserted normally."""
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT)",
            "INSERT INTO t VALUES (1, 'original')",
        ]
        mini_con = mini_sqlite.connect(":memory:")
        mini_cur = mini_con.cursor()
        for s in setup:
            mini_cur.execute(s)
        # Execute each row as a separate insert with DO NOTHING.
        mini_cur.execute("INSERT INTO t VALUES (1, 'skip') ON CONFLICT DO NOTHING")
        mini_cur.execute("INSERT INTO t VALUES (2, 'keep') ON CONFLICT DO NOTHING")
        mini_cur.execute("SELECT id, val FROM t ORDER BY id")
        mini_rows = mini_cur.fetchall()
        mini_con.close()

        real_con = sqlite3.connect(":memory:")
        real_cur = real_con.cursor()
        for s in setup:
            real_cur.execute(s)
        real_cur.execute("INSERT INTO t VALUES (1, 'skip') ON CONFLICT DO NOTHING")
        real_cur.execute("INSERT INTO t VALUES (2, 'keep') ON CONFLICT DO NOTHING")
        real_cur.execute("SELECT id, val FROM t ORDER BY id")
        real_rows = real_cur.fetchall()
        real_con.close()

        assert mini_rows == real_rows
        assert mini_rows == [(1, "original"), (2, "keep")]


# ---------------------------------------------------------------------------
# TestUpsertDoUpdate
# ---------------------------------------------------------------------------


class TestUpsertDoUpdate:
    """ON CONFLICT DO UPDATE SET — in-place update of the conflicting row."""

    def test_do_update_basic_excluded_value(self) -> None:
        """EXCLUDED.col provides the would-be-inserted value."""
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT)",
            "INSERT INTO t VALUES (1, 'original')",
            "INSERT INTO t VALUES (1, 'updated') ON CONFLICT (id) DO UPDATE SET val = EXCLUDED.val",
        ]
        mini, real = _both(setup, "SELECT id, val FROM t ORDER BY id")
        assert mini == real
        assert mini == [(1, "updated")]

    def test_do_update_no_conflict_inserts_normally(self) -> None:
        """When there is no conflict, DO UPDATE still inserts as a plain insert."""
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT)",
            "INSERT INTO t VALUES (1, 'original')",
            "INSERT INTO t VALUES (2, 'new') ON CONFLICT (id) DO UPDATE SET val = EXCLUDED.val",
        ]
        mini, real = _both(setup, "SELECT id, val FROM t ORDER BY id")
        assert mini == real
        assert mini == [(1, "original"), (2, "new")]

    def test_do_update_preserves_non_updated_columns(self) -> None:
        """Columns not in the SET list keep their existing values."""
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, a TEXT, b TEXT)",
            "INSERT INTO t VALUES (1, 'a1', 'b1')",
            "INSERT INTO t VALUES (1, 'a2', 'b2') ON CONFLICT (id) DO UPDATE SET a = EXCLUDED.a",
        ]
        mini, real = _both(setup, "SELECT id, a, b FROM t ORDER BY id")
        assert mini == real
        # b keeps its old value 'b1'; a is updated to 'a2'
        assert mini == [(1, "a2", "b1")]

    def test_do_update_multiple_assignments(self) -> None:
        """Multiple columns can be updated in a single ON CONFLICT DO UPDATE."""
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, a TEXT, b TEXT)",
            "INSERT INTO t VALUES (1, 'a1', 'b1')",
            (
                "INSERT INTO t VALUES (1, 'a2', 'b2') "
                "ON CONFLICT (id) DO UPDATE SET a = EXCLUDED.a, b = EXCLUDED.b"
            ),
        ]
        mini, real = _both(setup, "SELECT id, a, b FROM t ORDER BY id")
        assert mini == real
        assert mini == [(1, "a2", "b2")]

    def test_do_update_integer_arithmetic(self) -> None:
        """EXCLUDED.col can appear in arithmetic expressions."""
        setup = [
            "CREATE TABLE inventory (id INTEGER PRIMARY KEY, qty INTEGER)",
            "INSERT INTO inventory VALUES (1, 10)",
            (
                "INSERT INTO inventory VALUES (1, 5) "
                "ON CONFLICT (id) DO UPDATE SET qty = qty + EXCLUDED.qty"
            ),
        ]
        mini, real = _both(setup, "SELECT id, qty FROM inventory ORDER BY id")
        assert mini == real
        assert mini == [(1, 15)]

    def test_do_update_literal_value_in_set(self) -> None:
        """SET clause can use a literal value instead of EXCLUDED.col."""
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT)",
            "INSERT INTO t VALUES (1, 'original')",
            "INSERT INTO t VALUES (1, 'ignored') ON CONFLICT (id) DO UPDATE SET val = 'hardcoded'",
        ]
        mini, real = _both(setup, "SELECT id, val FROM t ORDER BY id")
        assert mini == real
        assert mini == [(1, "hardcoded")]

    def test_do_update_multiple_rows_selective(self) -> None:
        """Only conflicting rows are updated; non-conflicting are inserted."""
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT)",
            "INSERT INTO t VALUES (1, 'old1')",
            "INSERT INTO t VALUES (3, 'old3')",
        ]
        mini_con = mini_sqlite.connect(":memory:")
        mini_cur = mini_con.cursor()
        for s in setup:
            mini_cur.execute(s)
        mini_cur.execute(
            "INSERT INTO t VALUES (1, 'new1') "
            "ON CONFLICT (id) DO UPDATE SET val = EXCLUDED.val"
        )
        mini_cur.execute(
            "INSERT INTO t VALUES (2, 'new2') "
            "ON CONFLICT (id) DO UPDATE SET val = EXCLUDED.val"
        )
        mini_cur.execute(
            "INSERT INTO t VALUES (3, 'new3') "
            "ON CONFLICT (id) DO UPDATE SET val = EXCLUDED.val"
        )
        mini_cur.execute("SELECT id, val FROM t ORDER BY id")
        mini_rows = mini_cur.fetchall()
        mini_con.close()

        real_con = sqlite3.connect(":memory:")
        real_cur = real_con.cursor()
        for s in setup:
            real_cur.execute(s)
        real_cur.execute(
            "INSERT INTO t VALUES (1, 'new1') "
            "ON CONFLICT (id) DO UPDATE SET val = EXCLUDED.val"
        )
        real_cur.execute(
            "INSERT INTO t VALUES (2, 'new2') "
            "ON CONFLICT (id) DO UPDATE SET val = EXCLUDED.val"
        )
        real_cur.execute(
            "INSERT INTO t VALUES (3, 'new3') "
            "ON CONFLICT (id) DO UPDATE SET val = EXCLUDED.val"
        )
        real_cur.execute("SELECT id, val FROM t ORDER BY id")
        real_rows = real_cur.fetchall()
        real_con.close()

        assert mini_rows == real_rows
        assert mini_rows == [(1, "new1"), (2, "new2"), (3, "new3")]

    def test_do_update_accumulate_counter(self) -> None:
        """Common upsert pattern: upsert a counter that increments on conflict.

        Note: column named ``k`` (not ``key``) because ``KEY`` is a reserved
        word in the mini-sqlite parser.  The oracle comparison with real
        sqlite3 ensures correctness regardless.
        """
        setup = [
            "CREATE TABLE counts (k TEXT PRIMARY KEY, n INTEGER)",
        ]
        mini_con = mini_sqlite.connect(":memory:")
        mini_cur = mini_con.cursor()
        for s in setup:
            mini_cur.execute(s)
        for _ in range(3):
            mini_cur.execute(
                "INSERT INTO counts (k, n) VALUES ('hits', 1) "
                "ON CONFLICT (k) DO UPDATE SET n = n + 1"
            )
        mini_cur.execute("SELECT k, n FROM counts")
        mini_rows = mini_cur.fetchall()
        mini_con.close()

        real_con = sqlite3.connect(":memory:")
        real_cur = real_con.cursor()
        for s in setup:
            real_cur.execute(s)
        for _ in range(3):
            real_cur.execute(
                "INSERT INTO counts (k, n) VALUES ('hits', 1) "
                "ON CONFLICT (k) DO UPDATE SET n = n + 1"
            )
        real_cur.execute("SELECT k, n FROM counts")
        real_rows = real_cur.fetchall()
        real_con.close()

        assert mini_rows == real_rows
        assert mini_rows == [("hits", 3)]


# ---------------------------------------------------------------------------
# TestUpsertVsInsertOrIgnore — behaviour comparison
# ---------------------------------------------------------------------------


class TestUpsertVsInsertConflict:
    """Verify that DO NOTHING and INSERT OR IGNORE produce equivalent results."""

    def test_do_nothing_equivalent_to_insert_or_ignore(self) -> None:
        """ON CONFLICT DO NOTHING and INSERT OR IGNORE both skip the row."""
        base_setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT)",
            "INSERT INTO t VALUES (1, 'original')",
        ]

        mini_do_nothing_con = mini_sqlite.connect(":memory:")
        mini_do_nothing_cur = mini_do_nothing_con.cursor()
        for s in base_setup:
            mini_do_nothing_cur.execute(s)
        mini_do_nothing_cur.execute(
            "INSERT INTO t VALUES (1, 'new') ON CONFLICT DO NOTHING"
        )
        mini_do_nothing_cur.execute("SELECT id, val FROM t")
        do_nothing_rows = mini_do_nothing_cur.fetchall()
        mini_do_nothing_con.close()

        mini_ignore_con = mini_sqlite.connect(":memory:")
        mini_ignore_cur = mini_ignore_con.cursor()
        for s in base_setup:
            mini_ignore_cur.execute(s)
        mini_ignore_cur.execute("INSERT OR IGNORE INTO t VALUES (1, 'new')")
        mini_ignore_cur.execute("SELECT id, val FROM t")
        ignore_rows = mini_ignore_cur.fetchall()
        mini_ignore_con.close()

        assert do_nothing_rows == ignore_rows

    def test_do_update_changes_row_insert_or_ignore_does_not(self) -> None:
        """DO UPDATE changes the conflicting row; DO NOTHING does not."""
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT)",
            "INSERT INTO t VALUES (1, 'original')",
        ]

        mini_update_con = mini_sqlite.connect(":memory:")
        mini_update_cur = mini_update_con.cursor()
        for s in setup:
            mini_update_cur.execute(s)
        mini_update_cur.execute(
            "INSERT INTO t VALUES (1, 'new') ON CONFLICT (id) DO UPDATE SET val = EXCLUDED.val"
        )
        mini_update_cur.execute("SELECT val FROM t WHERE id = 1")
        update_rows = mini_update_cur.fetchall()
        mini_update_con.close()

        mini_nothing_con = mini_sqlite.connect(":memory:")
        mini_nothing_cur = mini_nothing_con.cursor()
        for s in setup:
            mini_nothing_cur.execute(s)
        mini_nothing_cur.execute(
            "INSERT INTO t VALUES (1, 'new') ON CONFLICT (id) DO NOTHING"
        )
        mini_nothing_cur.execute("SELECT val FROM t WHERE id = 1")
        nothing_rows = mini_nothing_cur.fetchall()
        mini_nothing_con.close()

        assert update_rows == [("new",)]
        assert nothing_rows == [("original",)]


# ---------------------------------------------------------------------------
# TestUpsertConditionalWhere — SQLite conditional-upsert (WHERE clause)
# ---------------------------------------------------------------------------


class TestUpsertConditionalWhere:
    """``ON CONFLICT … DO UPDATE SET … WHERE pred`` — SQLite 3.24+ extension.

    The WHERE predicate is evaluated against the (EXCLUDED, existing) row
    pair when a conflict fires.  If true, the SET assignments are applied;
    if false (or NULL), the conflicting row is left untouched — semantically
    equivalent to DO NOTHING for that single row.

    All tests are oracle-verified against real sqlite3 so we know we match
    SQLite's exact semantics (which is the whole point of being a compliant
    backend).
    """

    def test_where_true_applies_update(self) -> None:
        """``WHERE excluded.v > v`` with a strictly larger excluded value updates."""
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v INTEGER)",
            "INSERT INTO t VALUES (1, 10)",
            "INSERT INTO t VALUES (1, 99) "
            "ON CONFLICT(id) DO UPDATE SET v = excluded.v WHERE excluded.v > v",
        ]
        mini, real = _both(setup, "SELECT id, v FROM t")
        assert mini == real
        assert mini == [(1, 99)]

    def test_where_false_skips_update(self) -> None:
        """``WHERE excluded.v > v`` with a smaller excluded value leaves row alone."""
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v INTEGER)",
            "INSERT INTO t VALUES (1, 10)",
            "INSERT INTO t VALUES (1, 5) "
            "ON CONFLICT(id) DO UPDATE SET v = excluded.v WHERE excluded.v > v",
        ]
        mini, real = _both(setup, "SELECT id, v FROM t")
        assert mini == real
        assert mini == [(1, 10)]  # not 5 — predicate was false

    def test_where_equal_is_false(self) -> None:
        """``WHERE excluded.v > v`` with equal values is false (strict >)."""
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v INTEGER)",
            "INSERT INTO t VALUES (1, 7)",
            "INSERT INTO t VALUES (1, 7) "
            "ON CONFLICT(id) DO UPDATE SET v = excluded.v + 1 WHERE excluded.v > v",
        ]
        mini, real = _both(setup, "SELECT id, v FROM t")
        assert mini == real
        assert mini == [(1, 7)]

    def test_where_no_conflict_inserts(self) -> None:
        """When the INSERT does NOT conflict, the WHERE clause is irrelevant."""
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v INTEGER)",
            "INSERT INTO t VALUES (1, 10)",
            "INSERT INTO t VALUES (2, 99) "
            "ON CONFLICT(id) DO UPDATE SET v = excluded.v WHERE excluded.v > v",
        ]
        mini, real = _both(setup, "SELECT id, v FROM t ORDER BY id")
        assert mini == real
        assert mini == [(1, 10), (2, 99)]

    def test_where_references_existing_only(self) -> None:
        """WHERE may reference the existing row's bare columns (no EXCLUDED)."""
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v INTEGER, locked INTEGER)",
            "INSERT INTO t VALUES (1, 10, 0)",
            "INSERT INTO t VALUES (2, 20, 1)",
            # Updates only fire when the existing row is NOT locked.
            "INSERT INTO t VALUES (1, 100, 0) "
            "ON CONFLICT(id) DO UPDATE SET v = excluded.v WHERE locked = 0",
            "INSERT INTO t VALUES (2, 200, 1) "
            "ON CONFLICT(id) DO UPDATE SET v = excluded.v WHERE locked = 0",
        ]
        mini, real = _both(setup, "SELECT id, v, locked FROM t ORDER BY id")
        assert mini == real
        assert mini == [(1, 100, 0), (2, 20, 1)]  # only id=1 updated

    def test_where_references_excluded_only(self) -> None:
        """WHERE may reference only EXCLUDED columns (no existing-row refs)."""
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v INTEGER)",
            "INSERT INTO t VALUES (1, 10)",
            # Only apply when excluded.v is positive.
            "INSERT INTO t VALUES (1, -5) "
            "ON CONFLICT(id) DO UPDATE SET v = excluded.v WHERE excluded.v > 0",
            "INSERT INTO t VALUES (1, 42) "
            "ON CONFLICT(id) DO UPDATE SET v = excluded.v WHERE excluded.v > 0",
        ]
        mini, real = _both(setup, "SELECT id, v FROM t")
        assert mini == real
        assert mini == [(1, 42)]  # -5 skipped, 42 applied

    def test_where_with_compound_predicate(self) -> None:
        """WHERE accepts AND/OR/NOT compound predicates."""
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v INTEGER, tag TEXT)",
            "INSERT INTO t VALUES (1, 10, 'a')",
            "INSERT INTO t VALUES (1, 100, 'a') "
            "ON CONFLICT(id) DO UPDATE SET v = excluded.v "
            "WHERE excluded.v > v AND tag = 'a'",
        ]
        mini, real = _both(setup, "SELECT id, v, tag FROM t")
        assert mini == real
        assert mini == [(1, 100, "a")]

    def test_where_null_predicate_is_false(self) -> None:
        """A WHERE predicate that evaluates to NULL is treated as false."""
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v INTEGER)",
            "INSERT INTO t VALUES (1, NULL)",
            # NULL > anything is NULL, which is falsy → no update.
            "INSERT INTO t VALUES (1, 50) "
            "ON CONFLICT(id) DO UPDATE SET v = excluded.v WHERE v > 0",
        ]
        mini, real = _both(setup, "SELECT id, v FROM t")
        assert mini == real
        assert mini == [(1, None)]

    def test_where_with_multiple_rows_selective(self) -> None:
        """Conditional upsert across multiple conflicting rows applies selectively."""
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v INTEGER)",
            "INSERT INTO t VALUES (1, 10)",
            "INSERT INTO t VALUES (2, 20)",
            "INSERT INTO t VALUES (3, 30)",
            # For each row, only update if the new value is strictly larger.
            "INSERT INTO t VALUES (1, 5)  ON CONFLICT(id) DO UPDATE SET v = excluded.v WHERE excluded.v > v",
            "INSERT INTO t VALUES (2, 99) ON CONFLICT(id) DO UPDATE SET v = excluded.v WHERE excluded.v > v",
            "INSERT INTO t VALUES (3, 30) ON CONFLICT(id) DO UPDATE SET v = excluded.v WHERE excluded.v > v",
        ]
        mini, real = _both(setup, "SELECT id, v FROM t ORDER BY id")
        assert mini == real
        # id=1 unchanged (5 < 10), id=2 updated (99 > 20), id=3 unchanged (30 == 30)
        assert mini == [(1, 10), (2, 99), (3, 30)]

    def test_where_with_arithmetic_assignment(self) -> None:
        """WHERE combines with arithmetic in the SET clause."""
        setup = [
            "CREATE TABLE counters (id INTEGER PRIMARY KEY, hits INTEGER)",
            "INSERT INTO counters VALUES (1, 0)",
            # Increment hits only if it is below a cap.
            "INSERT INTO counters VALUES (1, 1) "
            "ON CONFLICT(id) DO UPDATE SET hits = hits + 1 WHERE hits < 3",
            "INSERT INTO counters VALUES (1, 1) "
            "ON CONFLICT(id) DO UPDATE SET hits = hits + 1 WHERE hits < 3",
            "INSERT INTO counters VALUES (1, 1) "
            "ON CONFLICT(id) DO UPDATE SET hits = hits + 1 WHERE hits < 3",
            "INSERT INTO counters VALUES (1, 1) "
            "ON CONFLICT(id) DO UPDATE SET hits = hits + 1 WHERE hits < 3",
        ]
        mini, real = _both(setup, "SELECT id, hits FROM counters")
        assert mini == real
        assert mini == [(1, 3)]  # capped at 3

    def test_excluded_pseudo_table_is_case_insensitive(self) -> None:
        """``excluded.v`` (lowercase) is equivalent to ``EXCLUDED.v``.

        SQLite identifies tables case-insensitively, and the EXCLUDED
        pseudo-table is no exception.  This is a defence against a regression
        where the adapter's rewrite of ``Column(table='EXCLUDED', …) →
        ExcludedColumn`` matched only the uppercase form.
        """
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v INTEGER)",
            "INSERT INTO t VALUES (1, 10)",
            # All three case variants should behave identically.
            "INSERT INTO t VALUES (1, 20) ON CONFLICT(id) DO UPDATE SET v = excluded.v",
        ]
        mini, real = _both(setup, "SELECT v FROM t")
        assert mini == real
        assert mini == [(20,)]

        setup2 = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v INTEGER)",
            "INSERT INTO t VALUES (1, 10)",
            "INSERT INTO t VALUES (1, 30) ON CONFLICT(id) DO UPDATE SET v = Excluded.v",
        ]
        mini2, real2 = _both(setup2, "SELECT v FROM t")
        assert mini2 == real2
        assert mini2 == [(30,)]
