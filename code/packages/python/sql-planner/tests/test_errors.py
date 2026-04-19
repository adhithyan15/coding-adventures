"""PlanError hierarchy — equality, string rendering, subclassing."""

from __future__ import annotations

from sql_planner.errors import (
    AmbiguousColumn,
    InternalError,
    InvalidAggregate,
    PlanError,
    UnknownColumn,
    UnknownTable,
    UnsupportedStatement,
)


class TestSubclassing:
    def test_all_inherit_from_plan_error(self) -> None:
        for cls in (
            AmbiguousColumn,
            UnknownTable,
            UnknownColumn,
            InvalidAggregate,
            UnsupportedStatement,
            InternalError,
        ):
            assert issubclass(cls, PlanError)
            assert issubclass(cls, Exception)


class TestEquality:
    def test_same_fields_equal(self) -> None:
        assert AmbiguousColumn(column="x", tables=["a", "b"]) == AmbiguousColumn(
            column="x", tables=["a", "b"]
        )

    def test_different_fields_unequal(self) -> None:
        assert UnknownTable(table="a") != UnknownTable(table="b")


class TestStringRendering:
    def test_ambiguous_column(self) -> None:
        e = AmbiguousColumn(column="id", tables=["a", "b"])
        assert str(e) == "ambiguous column 'id': present in a, b"

    def test_unknown_table(self) -> None:
        assert str(UnknownTable(table="x")) == "unknown table: 'x'"

    def test_unknown_column_qualified(self) -> None:
        assert str(UnknownColumn(table="t", column="c")) == "unknown column: t.c"

    def test_unknown_column_bare(self) -> None:
        assert str(UnknownColumn(table=None, column="c")) == "unknown column: 'c'"

    def test_invalid_aggregate(self) -> None:
        assert str(InvalidAggregate(message="no agg in WHERE")) == "no agg in WHERE"

    def test_unsupported_statement(self) -> None:
        assert str(UnsupportedStatement(kind="UNION")) == "unsupported statement: UNION"

    def test_internal_error(self) -> None:
        assert str(InternalError(message="oops")) == "oops"


class TestRaising:
    def test_raises_and_catches_as_plan_error(self) -> None:
        try:
            raise UnknownColumn(table=None, column="foo")
        except PlanError as e:
            assert isinstance(e, UnknownColumn)
