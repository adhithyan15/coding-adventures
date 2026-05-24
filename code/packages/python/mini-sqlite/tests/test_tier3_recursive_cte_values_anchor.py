"""Tests for ``WITH RECURSIVE … AS (VALUES(…) UNION ALL SELECT …)``.

Previously mini-sqlite raised
``ProgrammingError: expected child rule 'select_stmt' under query_stmt``
when the anchor of a recursive CTE was a ``VALUES`` expression rather
than a ``SELECT`` statement.  The canonical "count from N" pattern is::

    WITH RECURSIVE c(n) AS (
        VALUES(1)
        UNION ALL
        SELECT n + 1 FROM c WHERE n < 5
    )
    SELECT n FROM c

The adapter now accepts a ``values_stmt`` child as the anchor for the
recursive branch.  Single-row VALUES translates directly to a
``SelectStmt`` (which is what ``RecursiveCTERef.anchor`` expects).
Multi-row VALUES anchors are rejected with a clear error pointing
users to the ``SELECT … UNION ALL SELECT …`` rewrite — the planner's
recursive-CTE anchor path requires a single SELECT.
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


class TestBasic:
    def test_count_one_to_five(self) -> None:
        _both_match(
            query=(
                "WITH RECURSIVE c(n) AS ("
                "  VALUES(1)"
                "  UNION ALL"
                "  SELECT n + 1 FROM c WHERE n < 5"
                ") SELECT n FROM c"
            ),
        )

    def test_count_zero_to_three(self) -> None:
        _both_match(
            query=(
                "WITH RECURSIVE c(n) AS ("
                "  VALUES(0)"
                "  UNION ALL"
                "  SELECT n + 1 FROM c WHERE n < 3"
                ") SELECT n FROM c ORDER BY n"
            ),
        )


class TestMultiColumnAnchor:
    def test_two_column_anchor(self) -> None:
        _both_match(
            query=(
                "WITH RECURSIVE pair(a, b) AS ("
                "  VALUES(1, 10)"
                "  UNION ALL"
                "  SELECT a + 1, b * 2 FROM pair WHERE a < 4"
                ") SELECT a, b FROM pair"
            ),
        )

    def test_three_column_anchor(self) -> None:
        _both_match(
            query=(
                "WITH RECURSIVE t(a, b, c) AS ("
                "  VALUES(1, 2, 3)"
                "  UNION ALL"
                "  SELECT a + 1, b + 1, c + 1 FROM t WHERE a < 3"
                ") SELECT a, b, c FROM t"
            ),
        )


class TestUnionVariants:
    def test_union_all(self) -> None:
        _both_match(
            query=(
                "WITH RECURSIVE c(n) AS ("
                "  VALUES(1)"
                "  UNION ALL"
                "  SELECT n + 1 FROM c WHERE n < 4"
                ") SELECT n FROM c"
            ),
        )

    def test_union_distinct(self) -> None:
        _both_match(
            query=(
                "WITH RECURSIVE c(n) AS ("
                "  VALUES(1)"
                "  UNION"
                "  SELECT n + 1 FROM c WHERE n < 3"
                ") SELECT n FROM c"
            ),
        )


class TestSelectAnchorStillWorks:
    """Regression: pre-existing SELECT-anchor recursion must keep working."""

    def test_select_anchor(self) -> None:
        _both_match(
            query=(
                "WITH RECURSIVE c(n) AS ("
                "  SELECT 1"
                "  UNION ALL"
                "  SELECT n + 1 FROM c WHERE n < 4"
                ") SELECT n FROM c"
            ),
        )


class TestMultiRowAnchorRejection:
    """Multi-row VALUES anchor should fail with a clear, actionable error."""

    def test_multi_row_values_anchor_rejected(self) -> None:
        import pytest

        from mini_sqlite import errors as mini_errors

        mini = mini_sqlite.connect(":memory:")
        with pytest.raises(mini_errors.ProgrammingError, match="multi-row VALUES"):
            mini.execute(
                "WITH RECURSIVE c(n) AS ("
                "  VALUES(1), (2)"
                "  UNION ALL"
                "  SELECT n + 1 FROM c WHERE n < 5"
                ") SELECT n FROM c"
            )
