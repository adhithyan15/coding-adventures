"""Tests for SQLite's ``LIMIT`` extensions: signed counts + MySQL-style ``offset, count``.

SQLite supports two non-standard ``LIMIT`` shapes beyond the basic
``LIMIT N`` / ``LIMIT N OFFSET M`` forms:

* **Negative count means "no limit".**  ``LIMIT -1`` is a documented
  idiom for "unbounded".  Combined with ``OFFSET N`` (``LIMIT -1
  OFFSET 10``) it gives "skip 10, return the rest" — useful in
  paged queries when you want everything from a given row onwards.

* **Comma syntax: ``LIMIT m, n``.**  MySQL-compatible shorthand for
  ``LIMIT n OFFSET m``.  Note the reversed argument order — the
  FIRST number is the offset, the SECOND is the count.  This is the
  only place in SQL where the order swaps.

* **Negative offset is treated as zero.**  ``LIMIT 5 OFFSET -3``
  returns the first 5 rows (no skip).

Mini-sqlite previously parse-errored on every one of these because
the grammar only accepted ``LIMIT NUMBER [ OFFSET NUMBER ]`` (no
sign, no comma form).
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _both_match(*stmts: str, query: str) -> None:
    mini = mini_sqlite.connect(":memory:")
    ref = sqlite3.connect(":memory:")
    for c in (mini, ref):
        for s in stmts:
            c.execute(s)
    assert mini.execute(query).fetchall() == ref.execute(query).fetchall()


_SOURCE = (
    "CREATE TABLE t (v INT)",
    "INSERT INTO t VALUES (1), (2), (3), (4), (5)",
)


class TestNegativeLimit:
    def test_limit_neg_one_returns_all(self) -> None:
        _both_match(*_SOURCE, query="SELECT v FROM t LIMIT -1")

    def test_limit_neg_one_with_offset(self) -> None:
        _both_match(*_SOURCE, query="SELECT v FROM t LIMIT -1 OFFSET 2")

    def test_limit_minus_large(self) -> None:
        _both_match(*_SOURCE, query="SELECT v FROM t LIMIT -999")


class TestMySQLCommaSyntax:
    def test_limit_offset_count(self) -> None:
        # LIMIT 1, 2  ≡  LIMIT 2 OFFSET 1  → rows 2 and 3
        _both_match(*_SOURCE, query="SELECT v FROM t LIMIT 1, 2")

    def test_limit_zero_count(self) -> None:
        # LIMIT 0, 2  ≡  LIMIT 2 OFFSET 0  → rows 1 and 2
        _both_match(*_SOURCE, query="SELECT v FROM t LIMIT 0, 2")

    def test_limit_large_offset(self) -> None:
        # LIMIT 3, 10 ≡  LIMIT 10 OFFSET 3 → rows 4 and 5
        _both_match(*_SOURCE, query="SELECT v FROM t LIMIT 3, 10")

    def test_limit_offset_beyond_end(self) -> None:
        _both_match(*_SOURCE, query="SELECT v FROM t LIMIT 99, 5")


class TestNegativeOffset:
    def test_negative_offset_treated_as_zero(self) -> None:
        _both_match(*_SOURCE, query="SELECT v FROM t LIMIT 3 OFFSET -2")


class TestUnchangedBehaviour:
    """Regression: the existing LIMIT / LIMIT N OFFSET M paths must still work."""

    def test_plain_limit(self) -> None:
        _both_match(*_SOURCE, query="SELECT v FROM t LIMIT 2")

    def test_limit_zero(self) -> None:
        _both_match(*_SOURCE, query="SELECT v FROM t LIMIT 0")

    def test_limit_with_offset(self) -> None:
        _both_match(*_SOURCE, query="SELECT v FROM t LIMIT 2 OFFSET 1")

    def test_limit_exceeds_rows(self) -> None:
        _both_match(*_SOURCE, query="SELECT v FROM t LIMIT 100")


class TestOrderByLimitInteraction:
    def test_neg_limit_with_order_by(self) -> None:
        _both_match(*_SOURCE, query="SELECT v FROM t ORDER BY v DESC LIMIT -1 OFFSET 1")

    def test_comma_limit_with_order_by(self) -> None:
        _both_match(*_SOURCE, query="SELECT v FROM t ORDER BY v DESC LIMIT 1, 2")
