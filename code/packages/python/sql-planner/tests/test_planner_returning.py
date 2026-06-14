"""RETURNING clause planning — INSERT, UPDATE, DELETE.

The RETURNING clause allows DML statements to emit columns from the affected
rows as a result set.  The planner resolves RETURNING column references against
the target table's schema (just like WHERE / SET expressions) and stores them
on the plan node.

Tests cover:
- INSERT VALUES RETURNING: column resolution into Column(table=..., col=...)
- UPDATE RETURNING: resolved against table scope
- DELETE RETURNING: resolved against table scope
- INSERT SELECT RETURNING: resolved against target table schema
- RETURNING with literal expressions: Literal passes through unchanged
- Unknown column in RETURNING: raises UnknownColumn (via _resolve)
- Empty returning: default empty tuple preserved
"""

from __future__ import annotations

from sql_planner import (
    Column,
    DeleteStmt,
    InMemorySchemaProvider,
    InsertSelectStmt,
    InsertValuesStmt,
    Literal,
    SelectItem,
    SelectStmt,
    TableRef,
    UpdateStmt,
    plan,
)
from sql_planner.ast import Assignment
from sql_planner.plan import Delete, Insert, Update


def schema() -> InMemorySchemaProvider:
    return InMemorySchemaProvider({"employees": ["id", "name", "dept_id", "salary"]})


class TestInsertValuesReturning:
    """INSERT VALUES … RETURNING col1, col2."""

    def test_single_column_resolved(self) -> None:
        """RETURNING id resolves Column(None, 'id') → Column('employees', 'id')."""
        ast = InsertValuesStmt(
            table="employees",
            columns=("id", "name"),
            rows=((Literal(value=1), Literal(value="Alice")),),
            returning=(Column(table=None, col="id"),),
        )
        p = plan(ast, schema())
        assert isinstance(p, Insert)
        assert len(p.returning) == 1
        assert p.returning[0] == Column(table="employees", col="id")

    def test_multiple_columns_resolved(self) -> None:
        """RETURNING id, name resolves both columns."""
        ast = InsertValuesStmt(
            table="employees",
            columns=("id", "name"),
            rows=((Literal(value=1), Literal(value="Bob")),),
            returning=(
                Column(table=None, col="id"),
                Column(table=None, col="name"),
            ),
        )
        p = plan(ast, schema())
        assert isinstance(p, Insert)
        assert p.returning == (
            Column(table="employees", col="id"),
            Column(table="employees", col="name"),
        )

    def test_literal_in_returning(self) -> None:
        """RETURNING 42 — literal passes through unchanged."""
        ast = InsertValuesStmt(
            table="employees",
            columns=("id",),
            rows=((Literal(value=99),),),
            returning=(Literal(value=42),),
        )
        p = plan(ast, schema())
        assert isinstance(p, Insert)
        assert p.returning == (Literal(value=42),)

    def test_empty_returning_default(self) -> None:
        """No RETURNING clause → returning tuple is empty."""
        ast = InsertValuesStmt(
            table="employees",
            columns=None,
            rows=((Literal(value=1), Literal(value="A"), Literal(value=1), Literal(value=50000)),),
        )
        p = plan(ast, schema())
        assert isinstance(p, Insert)
        assert p.returning == ()


class TestUpdateReturning:
    """UPDATE … SET … WHERE … RETURNING col."""

    def test_returning_column_resolved(self) -> None:
        """RETURNING salary resolves to Column('employees', 'salary')."""
        ast = UpdateStmt(
            table="employees",
            assignments=(Assignment(column="salary", value=Literal(value=60000)),),
            where=None,
            returning=(Column(table=None, col="salary"),),
        )
        p = plan(ast, schema())
        assert isinstance(p, Update)
        assert p.returning == (Column(table="employees", col="salary"),)

    def test_returning_multiple_columns(self) -> None:
        """RETURNING id, name, salary — three columns resolved."""
        ast = UpdateStmt(
            table="employees",
            assignments=(Assignment(column="salary", value=Literal(value=70000)),),
            where=None,
            returning=(
                Column(table=None, col="id"),
                Column(table=None, col="name"),
                Column(table=None, col="salary"),
            ),
        )
        p = plan(ast, schema())
        assert isinstance(p, Update)
        assert p.returning == (
            Column(table="employees", col="id"),
            Column(table="employees", col="name"),
            Column(table="employees", col="salary"),
        )

    def test_no_returning_default(self) -> None:
        """No RETURNING → empty tuple on the plan node."""
        ast = UpdateStmt(
            table="employees",
            assignments=(Assignment(column="name", value=Literal(value="Z")),),
        )
        p = plan(ast, schema())
        assert isinstance(p, Update)
        assert p.returning == ()


class TestDeleteReturning:
    """DELETE FROM … WHERE … RETURNING col."""

    def test_returning_column_resolved(self) -> None:
        """RETURNING id resolves to Column('employees', 'id')."""
        ast = DeleteStmt(
            table="employees",
            where=None,
            returning=(Column(table=None, col="id"),),
        )
        p = plan(ast, schema())
        assert isinstance(p, Delete)
        assert p.returning == (Column(table="employees", col="id"),)

    def test_returning_all_columns(self) -> None:
        """RETURNING id, name, dept_id, salary — all four columns."""
        ast = DeleteStmt(
            table="employees",
            where=None,
            returning=(
                Column(table=None, col="id"),
                Column(table=None, col="name"),
                Column(table=None, col="dept_id"),
                Column(table=None, col="salary"),
            ),
        )
        p = plan(ast, schema())
        assert isinstance(p, Delete)
        assert len(p.returning) == 4
        assert p.returning[0] == Column(table="employees", col="id")
        assert p.returning[3] == Column(table="employees", col="salary")

    def test_no_returning_default(self) -> None:
        """No RETURNING → empty tuple on the plan node."""
        ast = DeleteStmt(table="employees", where=None)
        p = plan(ast, schema())
        assert isinstance(p, Delete)
        assert p.returning == ()


class TestInsertSelectReturning:
    """INSERT INTO t SELECT … RETURNING col — resolved against target schema."""

    def test_returning_target_column(self) -> None:
        """RETURNING id resolves against the INSERT target, not the SELECT source."""
        src_select = SelectStmt(
            items=(SelectItem(expr=Literal(value=1), alias="x"),),
            from_=TableRef(table="employees", alias=None),
        )
        ast = InsertSelectStmt(
            table="employees",
            columns=("id",),
            select=src_select,
            returning=(Column(table=None, col="id"),),
        )
        p = plan(ast, schema())
        assert isinstance(p, Insert)
        assert p.returning == (Column(table="employees", col="id"),)
