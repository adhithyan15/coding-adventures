"""
Tier-1 SQL feature tests: UNION/INTERSECT/EXCEPT, INSERT…SELECT, transactions.

These tests exercise the complete pipeline end-to-end from LogicalPlan through
codegen to VM execution. They are integration tests, not unit tests.

Coverage targets
----------------
- vm.py: _do_insert_from_result, _do_capture_left, _do_intersect,
  _do_except, _do_begin_transaction, _do_commit_transaction,
  _do_rollback_transaction
- ir.py: InsertFromResult, CaptureLeftResult, IntersectResult,
  ExceptResult, BeginTransaction, CommitTransaction, RollbackTransaction
- errors.py: TransactionError

Structure
---------
- TestUnion          — UNION and UNION ALL
- TestIntersect      — INTERSECT and INTERSECT ALL
- TestExcept         — EXCEPT and EXCEPT ALL
- TestInsertSelect   — INSERT INTO … SELECT
- TestTransactions   — BEGIN / COMMIT / ROLLBACK happy paths
- TestTransactionErrors — nested BEGIN, stray COMMIT/ROLLBACK
"""

from __future__ import annotations

import pytest
from sql_backend.in_memory import InMemoryBackend
from sql_backend.schema import ColumnDef
from sql_codegen import compile
from sql_planner import (
    Begin,
    BinaryExpr,
    BinaryOp,
    Column,
    Commit,
    Except,
    Filter,
    Insert,
    InsertSource,
    Intersect,
    Literal,
    Project,
    ProjectionItem,
    Rollback,
    Scan,
    Union,
)

from sql_vm import TransactionError, execute

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _make_backend(*tables: tuple[str, list[ColumnDef], list[dict]]) -> InMemoryBackend:
    """Create an in-memory backend with one or more pre-populated tables."""
    be = InMemoryBackend()
    for table_name, cols, rows in tables:
        be.create_table(table_name, cols, False)
        for row in rows:
            be.insert(table_name, row)
    return be


def _int_col() -> ColumnDef:
    return ColumnDef(name="v", type_name="INTEGER")


def _scan_table(be: InMemoryBackend, table: str) -> list[dict]:
    """Helper to read all rows back from a table."""
    cur = be.scan(table)
    rows: list[dict] = []
    while True:
        r = cur.next()
        if r is None:
            break
        rows.append(dict(r))
    cur.close()
    return rows


# ---------------------------------------------------------------------------
# UNION / UNION ALL
# ---------------------------------------------------------------------------


class TestUnion:
    """UNION deduplicates; UNION ALL keeps all rows."""

    def _plan_union(self, left_rows: list[int], right_rows: list[int], all_flag: bool) -> object:
        """Build a Union plan from two literal-value scans."""
        from sql_planner import Insert, InsertSource  # noqa: F401 (just to check imports work)
        # Build each side as: Project(Scan(t))
        left_be = _make_backend(
            ("left_t", [_int_col()], [{"v": v} for v in left_rows])
        )
        right_be = _make_backend(
            ("right_t", [_int_col()], [{"v": v} for v in right_rows])
        )
        return left_be, right_be

    def test_union_all_appends_rows(self) -> None:
        """UNION ALL concatenates the two result sets without dedup."""
        # Build two single-table plans and combine with Union(all=True).
        be = _make_backend(
            ("a", [_int_col()], [{"v": 1}, {"v": 2}]),
            ("b", [_int_col()], [{"v": 3}, {"v": 4}]),
        )
        left = Project(
            input=Scan(table="a"),
            items=(ProjectionItem(expr=Column(table="a", col="v"), alias="v"),),
        )
        right = Project(
            input=Scan(table="b"),
            items=(ProjectionItem(expr=Column(table="b", col="v"), alias="v"),),
        )
        plan = Union(left=left, right=right, all=True)
        result = execute(compile(plan), be)
        assert set(result.rows) == {(1,), (2,), (3,), (4,)}
        assert len(result.rows) == 4

    def test_union_deduplicates(self) -> None:
        """UNION removes duplicates present in both sides."""
        be = _make_backend(
            ("a", [_int_col()], [{"v": 1}, {"v": 2}]),
            ("b", [_int_col()], [{"v": 2}, {"v": 3}]),
        )
        left = Project(
            input=Scan(table="a"),
            items=(ProjectionItem(expr=Column(table="a", col="v"), alias="v"),),
        )
        right = Project(
            input=Scan(table="b"),
            items=(ProjectionItem(expr=Column(table="b", col="v"), alias="v"),),
        )
        plan = Union(left=left, right=right, all=False)
        result = execute(compile(plan), be)
        # 1, 2, 3 — the duplicate 2 appears only once.
        assert set(result.rows) == {(1,), (2,), (3,)}
        assert len(result.rows) == 3

    def test_union_all_preserves_duplicates_within_sides(self) -> None:
        """UNION ALL does NOT remove duplicates even from the same side."""
        be = _make_backend(
            ("a", [_int_col()], [{"v": 1}, {"v": 1}]),
            ("b", [_int_col()], [{"v": 1}]),
        )
        left = Project(
            input=Scan(table="a"),
            items=(ProjectionItem(expr=Column(table="a", col="v"), alias="v"),),
        )
        right = Project(
            input=Scan(table="b"),
            items=(ProjectionItem(expr=Column(table="b", col="v"), alias="v"),),
        )
        plan = Union(left=left, right=right, all=True)
        result = execute(compile(plan), be)
        assert len(result.rows) == 3
        assert all(r == (1,) for r in result.rows)

    def test_union_empty_right(self) -> None:
        """UNION with an empty right side returns exactly the left rows."""
        be = _make_backend(
            ("a", [_int_col()], [{"v": 7}, {"v": 8}]),
            ("b", [_int_col()], []),
        )
        left = Project(
            input=Scan(table="a"),
            items=(ProjectionItem(expr=Column(table="a", col="v"), alias="v"),),
        )
        right = Project(
            input=Scan(table="b"),
            items=(ProjectionItem(expr=Column(table="b", col="v"), alias="v"),),
        )
        plan = Union(left=left, right=right, all=True)
        result = execute(compile(plan), be)
        assert set(result.rows) == {(7,), (8,)}

    def test_union_empty_left(self) -> None:
        """UNION with an empty left side returns exactly the right rows."""
        be = _make_backend(
            ("a", [_int_col()], []),
            ("b", [_int_col()], [{"v": 5}]),
        )
        left = Project(
            input=Scan(table="a"),
            items=(ProjectionItem(expr=Column(table="a", col="v"), alias="v"),),
        )
        right = Project(
            input=Scan(table="b"),
            items=(ProjectionItem(expr=Column(table="b", col="v"), alias="v"),),
        )
        plan = Union(left=left, right=right, all=True)
        result = execute(compile(plan), be)
        assert result.rows == ((5,),)

    def test_union_result_schema_from_left(self) -> None:
        """The output column name is determined by the left-side schema."""
        be = _make_backend(
            ("a", [ColumnDef(name="x", type_name="INTEGER")], [{"x": 1}]),
            ("b", [ColumnDef(name="y", type_name="INTEGER")], [{"y": 2}]),
        )
        left = Project(
            input=Scan(table="a"),
            items=(ProjectionItem(expr=Column(table="a", col="x"), alias="x"),),
        )
        right = Project(
            input=Scan(table="b"),
            items=(ProjectionItem(expr=Column(table="b", col="y"), alias="x"),),
        )
        plan = Union(left=left, right=right, all=True)
        result = execute(compile(plan), be)
        assert result.columns == ("x",)

    def test_union_with_filter_on_left(self) -> None:
        """Filters on individual sides work before the UNION."""
        be = _make_backend(
            ("a", [_int_col()], [{"v": 1}, {"v": 2}, {"v": 3}]),
            ("b", [_int_col()], [{"v": 4}]),
        )
        # Only rows with v > 1 from table 'a'.
        filtered = Filter(
            input=Scan(table="a"),
            predicate=BinaryExpr(
                op=BinaryOp.GT,
                left=Column(table="a", col="v"),
                right=Literal(1),
            ),
        )
        left = Project(
            input=filtered,
            items=(ProjectionItem(expr=Column(table="a", col="v"), alias="v"),),
        )
        right = Project(
            input=Scan(table="b"),
            items=(ProjectionItem(expr=Column(table="b", col="v"), alias="v"),),
        )
        plan = Union(left=left, right=right, all=True)
        result = execute(compile(plan), be)
        assert set(result.rows) == {(2,), (3,), (4,)}


# ---------------------------------------------------------------------------
# INTERSECT / INTERSECT ALL
# ---------------------------------------------------------------------------


class TestIntersect:
    """INTERSECT returns rows present in both sets."""

    def _simple_plan(
        self,
        be: InMemoryBackend,
        left_table: str,
        right_table: str,
        all_flag: bool,
    ) -> object:
        left = Project(
            input=Scan(table=left_table),
            items=(ProjectionItem(expr=Column(table=left_table, col="v"), alias="v"),),
        )
        right = Project(
            input=Scan(table=right_table),
            items=(ProjectionItem(expr=Column(table=right_table, col="v"), alias="v"),),
        )
        return Intersect(left=left, right=right, all=all_flag)

    def test_intersect_basic(self) -> None:
        """Rows present in both sides appear in the result exactly once."""
        be = _make_backend(
            ("a", [_int_col()], [{"v": 1}, {"v": 2}, {"v": 3}]),
            ("b", [_int_col()], [{"v": 2}, {"v": 3}, {"v": 4}]),
        )
        plan = self._simple_plan(be, "a", "b", all_flag=False)
        result = execute(compile(plan), be)
        assert set(result.rows) == {(2,), (3,)}

    def test_intersect_no_common_rows(self) -> None:
        """Disjoint sets produce an empty intersection."""
        be = _make_backend(
            ("a", [_int_col()], [{"v": 1}, {"v": 2}]),
            ("b", [_int_col()], [{"v": 3}, {"v": 4}]),
        )
        plan = self._simple_plan(be, "a", "b", all_flag=False)
        result = execute(compile(plan), be)
        assert result.rows == ()

    def test_intersect_empty_left(self) -> None:
        """Empty left side → empty intersection."""
        be = _make_backend(
            ("a", [_int_col()], []),
            ("b", [_int_col()], [{"v": 1}]),
        )
        plan = self._simple_plan(be, "a", "b", all_flag=False)
        result = execute(compile(plan), be)
        assert result.rows == ()

    def test_intersect_empty_right(self) -> None:
        """Empty right side → empty intersection."""
        be = _make_backend(
            ("a", [_int_col()], [{"v": 1}]),
            ("b", [_int_col()], []),
        )
        plan = self._simple_plan(be, "a", "b", all_flag=False)
        result = execute(compile(plan), be)
        assert result.rows == ()

    def test_intersect_deduplicates_duplicates_in_left(self) -> None:
        """INTERSECT (not ALL) returns each row once even if left has duplicates."""
        be = _make_backend(
            ("a", [_int_col()], [{"v": 1}, {"v": 1}, {"v": 2}]),
            ("b", [_int_col()], [{"v": 1}, {"v": 2}]),
        )
        plan = self._simple_plan(be, "a", "b", all_flag=False)
        result = execute(compile(plan), be)
        # Both 1 and 2 appear, but each only once.
        assert set(result.rows) == {(1,), (2,)}
        assert len(result.rows) == 2

    def test_intersect_all_respects_min_multiplicity(self) -> None:
        """INTERSECT ALL: row appears min(left_count, right_count) times."""
        be = _make_backend(
            # v=1 appears 3 times on left, 2 on right → min=2 in output
            ("a", [_int_col()], [{"v": 1}, {"v": 1}, {"v": 1}]),
            ("b", [_int_col()], [{"v": 1}, {"v": 1}]),
        )
        plan = self._simple_plan(be, "a", "b", all_flag=True)
        result = execute(compile(plan), be)
        assert list(result.rows).count((1,)) == 2

    def test_intersect_all_with_disjoint_sets(self) -> None:
        """INTERSECT ALL on disjoint sets yields no rows."""
        be = _make_backend(
            ("a", [_int_col()], [{"v": 1}]),
            ("b", [_int_col()], [{"v": 2}]),
        )
        plan = self._simple_plan(be, "a", "b", all_flag=True)
        result = execute(compile(plan), be)
        assert result.rows == ()

    def test_intersect_result_schema(self) -> None:
        """Schema comes from the left side."""
        be = _make_backend(
            ("a", [_int_col()], [{"v": 1}]),
            ("b", [_int_col()], [{"v": 1}]),
        )
        plan = self._simple_plan(be, "a", "b", all_flag=False)
        result = execute(compile(plan), be)
        assert "v" in result.columns


# ---------------------------------------------------------------------------
# EXCEPT / EXCEPT ALL
# ---------------------------------------------------------------------------


class TestExcept:
    """EXCEPT returns rows in the left set not in the right set."""

    def _simple_plan(
        self,
        be: InMemoryBackend,
        left_table: str,
        right_table: str,
        all_flag: bool,
    ) -> object:
        left = Project(
            input=Scan(table=left_table),
            items=(ProjectionItem(expr=Column(table=left_table, col="v"), alias="v"),),
        )
        right = Project(
            input=Scan(table=right_table),
            items=(ProjectionItem(expr=Column(table=right_table, col="v"), alias="v"),),
        )
        return Except(left=left, right=right, all=all_flag)

    def test_except_basic(self) -> None:
        """Rows only in left survive; rows also in right are excluded."""
        be = _make_backend(
            ("a", [_int_col()], [{"v": 1}, {"v": 2}, {"v": 3}]),
            ("b", [_int_col()], [{"v": 2}, {"v": 3}]),
        )
        plan = self._simple_plan(be, "a", "b", all_flag=False)
        result = execute(compile(plan), be)
        assert result.rows == ((1,),)

    def test_except_right_disjoint(self) -> None:
        """If right has no overlap with left, all left rows survive."""
        be = _make_backend(
            ("a", [_int_col()], [{"v": 1}, {"v": 2}]),
            ("b", [_int_col()], [{"v": 3}, {"v": 4}]),
        )
        plan = self._simple_plan(be, "a", "b", all_flag=False)
        result = execute(compile(plan), be)
        assert set(result.rows) == {(1,), (2,)}

    def test_except_empty_left(self) -> None:
        """Empty left → empty result."""
        be = _make_backend(
            ("a", [_int_col()], []),
            ("b", [_int_col()], [{"v": 1}]),
        )
        plan = self._simple_plan(be, "a", "b", all_flag=False)
        result = execute(compile(plan), be)
        assert result.rows == ()

    def test_except_empty_right(self) -> None:
        """Empty right → all left rows survive."""
        be = _make_backend(
            ("a", [_int_col()], [{"v": 5}, {"v": 6}]),
            ("b", [_int_col()], []),
        )
        plan = self._simple_plan(be, "a", "b", all_flag=False)
        result = execute(compile(plan), be)
        assert set(result.rows) == {(5,), (6,)}

    def test_except_deduplicates(self) -> None:
        """EXCEPT (not ALL) returns each surviving row exactly once."""
        be = _make_backend(
            # v=1 twice in left, but right has v=2 only → v=1 survives, deduped
            ("a", [_int_col()], [{"v": 1}, {"v": 1}, {"v": 2}]),
            ("b", [_int_col()], [{"v": 2}]),
        )
        plan = self._simple_plan(be, "a", "b", all_flag=False)
        result = execute(compile(plan), be)
        assert result.rows == ((1,),)

    def test_except_all_bag_semantics(self) -> None:
        """EXCEPT ALL subtracts right occurrences from left occurrences."""
        be = _make_backend(
            # v=1 appears 3× on left, 1× on right → 2 copies survive
            ("a", [_int_col()], [{"v": 1}, {"v": 1}, {"v": 1}]),
            ("b", [_int_col()], [{"v": 1}]),
        )
        plan = self._simple_plan(be, "a", "b", all_flag=True)
        result = execute(compile(plan), be)
        assert list(result.rows).count((1,)) == 2

    def test_except_all_right_exceeds_left(self) -> None:
        """EXCEPT ALL: if right count >= left count, row disappears entirely."""
        be = _make_backend(
            ("a", [_int_col()], [{"v": 1}]),
            ("b", [_int_col()], [{"v": 1}, {"v": 1}]),
        )
        plan = self._simple_plan(be, "a", "b", all_flag=True)
        result = execute(compile(plan), be)
        assert result.rows == ()

    def test_except_all_mixed_values(self) -> None:
        """EXCEPT ALL works correctly when multiple distinct values are present."""
        be = _make_backend(
            ("a", [_int_col()], [{"v": 1}, {"v": 1}, {"v": 2}, {"v": 3}]),
            ("b", [_int_col()], [{"v": 1}, {"v": 3}]),
        )
        plan = self._simple_plan(be, "a", "b", all_flag=True)
        result = execute(compile(plan), be)
        # v=1: 2-1=1 copy survives; v=2: 1-0=1 copy; v=3: 1-1=0 copies
        assert list(result.rows).count((1,)) == 1
        assert list(result.rows).count((2,)) == 1
        assert list(result.rows).count((3,)) == 0

    def test_except_schema_from_left(self) -> None:
        """Output schema follows the left side."""
        be = _make_backend(
            ("a", [_int_col()], [{"v": 1}]),
            ("b", [_int_col()], []),
        )
        plan = self._simple_plan(be, "a", "b", all_flag=False)
        result = execute(compile(plan), be)
        assert "v" in result.columns


# ---------------------------------------------------------------------------
# INSERT … SELECT
# ---------------------------------------------------------------------------


class TestInsertSelect:
    """INSERT INTO t SELECT … bulk-inserts rows from a query result."""

    def _target(self) -> tuple[InMemoryBackend, str]:
        be = InMemoryBackend()
        be.create_table("dst", [_int_col()], False)
        return be, "dst"

    def test_insert_select_basic(self) -> None:
        """All rows from the SELECT are inserted into the target table."""
        be, dst = self._target()
        be.create_table("src", [_int_col()], False)
        for v in (10, 20, 30):
            be.insert("src", {"v": v})

        plan = Insert(
            table=dst,
            columns=("v",),
            source=InsertSource(query=Project(
                input=Scan(table="src"),
                items=(ProjectionItem(expr=Column(table="src", col="v"), alias="v"),),
            )),
        )
        result = execute(compile(plan), be)
        assert result.rows_affected == 3
        rows = _scan_table(be, dst)
        assert {r["v"] for r in rows} == {10, 20, 30}

    def test_insert_select_with_filter(self) -> None:
        """Only rows passing the WHERE clause are inserted."""
        be, dst = self._target()
        be.create_table("src", [_int_col()], False)
        for v in (1, 2, 3, 4, 5):
            be.insert("src", {"v": v})

        filtered = Filter(
            input=Scan(table="src"),
            predicate=BinaryExpr(
                op=BinaryOp.GT,
                left=Column(table="src", col="v"),
                right=Literal(3),
            ),
        )
        plan = Insert(
            table=dst,
            columns=("v",),
            source=InsertSource(query=Project(
                input=filtered,
                items=(ProjectionItem(expr=Column(table="src", col="v"), alias="v"),),
            )),
        )
        result = execute(compile(plan), be)
        assert result.rows_affected == 2
        rows = _scan_table(be, dst)
        assert {r["v"] for r in rows} == {4, 5}

    def test_insert_select_no_rows_source_empty(self) -> None:
        """If the SELECT returns no rows, zero rows are inserted."""
        be, dst = self._target()
        be.create_table("src", [_int_col()], False)

        plan = Insert(
            table=dst,
            columns=("v",),
            source=InsertSource(query=Project(
                input=Scan(table="src"),
                items=(ProjectionItem(expr=Column(table="src", col="v"), alias="v"),),
            )),
        )
        result = execute(compile(plan), be)
        assert result.rows_affected == 0
        assert _scan_table(be, dst) == []

    def test_insert_select_does_not_leave_rows_in_result(self) -> None:
        """After INSERT … SELECT, the result.rows list is empty."""
        be, dst = self._target()
        be.create_table("src", [_int_col()], False)
        be.insert("src", {"v": 42})

        plan = Insert(
            table=dst,
            columns=("v",),
            source=InsertSource(query=Project(
                input=Scan(table="src"),
                items=(ProjectionItem(expr=Column(table="src", col="v"), alias="v"),),
            )),
        )
        result = execute(compile(plan), be)
        # The SELECT rows should have been consumed by the insert, not returned.
        assert result.rows == ()

    def test_insert_select_self_copy(self) -> None:
        """INSERT INTO t SELECT * FROM t doubles the rows in the table."""
        be = InMemoryBackend()
        be.create_table("t", [_int_col()], False)
        be.insert("t", {"v": 1})
        be.insert("t", {"v": 2})

        plan = Insert(
            table="t",
            columns=("v",),
            source=InsertSource(query=Project(
                input=Scan(table="t"),
                items=(ProjectionItem(expr=Column(table="t", col="v"), alias="v"),),
            )),
        )
        execute(compile(plan), be)
        rows = _scan_table(be, "t")
        assert len(rows) == 4
        assert {r["v"] for r in rows} == {1, 2}


# ---------------------------------------------------------------------------
# Transactions — happy paths
# ---------------------------------------------------------------------------


class TestTransactions:
    """BEGIN / COMMIT / ROLLBACK coordinate with the InMemoryBackend."""

    def _fresh_backend(self) -> InMemoryBackend:
        be = InMemoryBackend()
        be.create_table("t", [_int_col()], False)
        return be

    def test_begin_commit_visible(self) -> None:
        """Rows inserted inside a committed transaction persist."""
        be = self._fresh_backend()

        # BEGIN
        execute(compile(Begin()), be)
        # INSERT a row
        execute(
            compile(Insert(
                table="t",
                columns=("v",),
                source=InsertSource(values=((Literal(99),),)),
            )),
            be,
        )
        # COMMIT
        execute(compile(Commit()), be)

        rows = _scan_table(be, "t")
        assert any(r["v"] == 99 for r in rows)

    def test_begin_rollback_reverts(self) -> None:
        """Rows inserted inside a rolled-back transaction disappear."""
        be = self._fresh_backend()
        be.insert("t", {"v": 1})  # baseline row, committed

        execute(compile(Begin()), be)
        execute(
            compile(Insert(
                table="t",
                columns=("v",),
                source=InsertSource(values=((Literal(999),),)),
            )),
            be,
        )
        execute(compile(Rollback()), be)

        rows = _scan_table(be, "t")
        assert all(r["v"] != 999 for r in rows), f"rolled-back row leaked: {rows}"

    def test_commit_clears_handle(self) -> None:
        """After COMMIT the handle is cleared — a second COMMIT raises."""
        be = self._fresh_backend()
        execute(compile(Begin()), be)
        execute(compile(Commit()), be)
        # A second COMMIT with no active transaction should raise.
        with pytest.raises(TransactionError):
            execute(compile(Commit()), be)

    def test_rollback_clears_handle(self) -> None:
        """After ROLLBACK the handle is cleared — a second ROLLBACK raises."""
        be = self._fresh_backend()
        execute(compile(Begin()), be)
        execute(compile(Rollback()), be)
        with pytest.raises(TransactionError):
            execute(compile(Rollback()), be)

    def test_multiple_transactions_sequential(self) -> None:
        """Multiple sequential transactions each commit successfully."""
        be = self._fresh_backend()
        for v in (1, 2, 3):
            execute(compile(Begin()), be)
            execute(
                compile(Insert(
                    table="t",
                    columns=("v",),
                    source=InsertSource(values=((Literal(v),),)),
                )),
                be,
            )
            execute(compile(Commit()), be)

        rows = _scan_table(be, "t")
        assert {r["v"] for r in rows} == {1, 2, 3}


# ---------------------------------------------------------------------------
# Transaction error cases
# ---------------------------------------------------------------------------


class TestTransactionErrors:
    """Misuse of transaction control instructions."""

    def _fresh_backend(self) -> InMemoryBackend:
        be = InMemoryBackend()
        be.create_table("t", [_int_col()], False)
        return be

    def test_begin_while_active_raises(self) -> None:
        """Nested BEGIN raises TransactionError."""
        be = self._fresh_backend()
        execute(compile(Begin()), be)
        with pytest.raises(TransactionError) as exc:
            execute(compile(Begin()), be)
        assert "already active" in str(exc.value).lower()
        # Clean up the first transaction.
        execute(compile(Rollback()), be)

    def test_commit_without_begin_raises(self) -> None:
        """COMMIT with no active transaction raises TransactionError."""
        be = self._fresh_backend()
        with pytest.raises(TransactionError) as exc:
            execute(compile(Commit()), be)
        assert "no active" in str(exc.value).lower()

    def test_rollback_without_begin_raises(self) -> None:
        """ROLLBACK with no active transaction raises TransactionError."""
        be = self._fresh_backend()
        with pytest.raises(TransactionError) as exc:
            execute(compile(Rollback()), be)
        assert "no active" in str(exc.value).lower()

    def test_transaction_error_is_vm_error_subclass(self) -> None:
        """TransactionError is a VmError so callers can catch a single root."""
        from sql_vm import VmError
        assert issubclass(TransactionError, VmError)


# ---------------------------------------------------------------------------
# Edge cases and misc coverage
# ---------------------------------------------------------------------------


class TestSetOpEdgeCases:
    """Miscellaneous edge cases that exercise less-travelled code paths."""

    def test_union_with_literals_only(self) -> None:
        """Union of two empty-table scans with a WHERE=FALSE produces no rows."""
        be = InMemoryBackend()
        be.create_table("t", [_int_col()], False)
        left = Project(
            input=Filter(
                input=Scan(table="t"),
                predicate=Literal(False),
            ),
            items=(ProjectionItem(expr=Column(table="t", col="v"), alias="v"),),
        )
        right = Project(
            input=Filter(
                input=Scan(table="t"),
                predicate=Literal(False),
            ),
            items=(ProjectionItem(expr=Column(table="t", col="v"), alias="v"),),
        )
        plan = Union(left=left, right=right, all=True)
        result = execute(compile(plan), be)
        assert result.rows == ()

    def test_intersect_full_overlap(self) -> None:
        """When both sides are identical, INTERSECT returns those rows."""
        be = _make_backend(
            ("a", [_int_col()], [{"v": 1}, {"v": 2}]),
            ("b", [_int_col()], [{"v": 1}, {"v": 2}]),
        )
        left = Project(
            input=Scan(table="a"),
            items=(ProjectionItem(expr=Column(table="a", col="v"), alias="v"),),
        )
        right = Project(
            input=Scan(table="b"),
            items=(ProjectionItem(expr=Column(table="b", col="v"), alias="v"),),
        )
        plan = Intersect(left=left, right=right, all=False)
        result = execute(compile(plan), be)
        assert set(result.rows) == {(1,), (2,)}

    def test_except_full_overlap_yields_empty(self) -> None:
        """When both sides are identical, EXCEPT returns no rows."""
        be = _make_backend(
            ("a", [_int_col()], [{"v": 5}]),
            ("b", [_int_col()], [{"v": 5}]),
        )
        left = Project(
            input=Scan(table="a"),
            items=(ProjectionItem(expr=Column(table="a", col="v"), alias="v"),),
        )
        right = Project(
            input=Scan(table="b"),
            items=(ProjectionItem(expr=Column(table="b", col="v"), alias="v"),),
        )
        plan = Except(left=left, right=right, all=False)
        result = execute(compile(plan), be)
        assert result.rows == ()
