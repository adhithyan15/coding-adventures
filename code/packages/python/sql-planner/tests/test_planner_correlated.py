"""Planner tests for correlated subquery resolution.

These tests verify that column references in subqueries that resolve only
against the *outer* query's scope produce :class:`~sql_planner.expr.CorrelatedRef`
nodes rather than raising :class:`UnknownColumn`.

Background
----------
When the planner encounters an ``InSubquery``, ``ExistsSubquery``,
``ScalarSubquery``, or ``NotInSubquery`` expression inside a WHERE clause,
it plans the inner SELECT while passing the *outer* query's column scope
as ``outer_scope``.  Inside that inner plan, any column that cannot be
resolved against the inner table but *can* be found in the outer scope
is returned as a :class:`CorrelatedRef` (qualified) or resolved via
bare-name lookup in the outer scope.

Codegen later compiles each :class:`CorrelatedRef` to a
:class:`~sql_codegen.ir.LoadOuterColumn` instruction.
"""

from __future__ import annotations

import pytest

from sql_planner import (
    BinaryExpr,
    BinaryOp,
    Column,
    CorrelatedRef,
    ExistsSubquery,
    Filter,
    InMemorySchemaProvider,
    InSubquery,
    Literal,
    NotInSubquery,
    Project,
    ScalarSubquery,
    Scan,
    UnknownColumn,
    plan,
)
from sql_planner.ast import SelectItem, SelectStmt, TableRef

# ---------------------------------------------------------------------------
# Schema helpers
# ---------------------------------------------------------------------------


def _two_table_schema() -> InMemorySchemaProvider:
    """employees(id, name, dept_id) + departments(id, dept_name)."""
    return InMemorySchemaProvider({
        "employees": ["id", "name", "dept_id"],
        "departments": ["id", "dept_name"],
    })


# ---------------------------------------------------------------------------
# Helper: build a correlated IN-subquery plan via the planner
# ---------------------------------------------------------------------------


def _plan_correlated_in_query(schema: InMemorySchemaProvider) -> Project:
    """Plan:  SELECT name FROM employees AS e
              WHERE e.dept_id IN (SELECT id FROM departments WHERE id = e.dept_id)

    The inner SELECT references ``e.dept_id`` which is NOT a column of
    ``departments`` — it must resolve via outer_scope to CorrelatedRef.
    """
    # Inner SELECT: SELECT id FROM departments WHERE id = e.dept_id
    inner_where = BinaryExpr(
        op=BinaryOp.EQ,
        left=Column("departments", "id"),
        right=Column("e", "dept_id"),  # "e" is outer alias → CorrelatedRef after planning
    )
    inner_stmt = SelectStmt(
        from_=TableRef(table="departments"),
        items=(SelectItem(expr=Column("departments", "id")),),
        where=inner_where,
    )

    # Outer SELECT: SELECT name FROM employees AS e WHERE ...IN...
    outer_stmt = SelectStmt(
        from_=TableRef(table="employees", alias="e"),
        items=(SelectItem(expr=Column("e", "name")),),
        where=InSubquery(
            operand=Column("e", "dept_id"),
            query=inner_stmt,
        ),
    )
    result = plan(outer_stmt, schema)
    assert isinstance(result, Project)
    return result


# ---------------------------------------------------------------------------
# Correlated IN subquery
# ---------------------------------------------------------------------------


class TestCorrelatedInSubquery:
    def test_inner_filter_contains_correlated_ref(self) -> None:
        """CorrelatedRef appears in the inner filter predicate."""
        schema = _two_table_schema()
        outer = _plan_correlated_in_query(schema)

        # Outer shape: Project(Filter(Scan(employees), InSubquery(...)))
        outer_filter = outer.input
        assert isinstance(outer_filter, Filter)
        assert isinstance(outer_filter.input, Scan)

        in_sub = outer_filter.predicate
        assert isinstance(in_sub, InSubquery)

        # Inner plan: Project(Filter(Scan(departments), BinaryExpr))
        inner_project = in_sub.query
        assert isinstance(inner_project, Project)
        inner_filter = inner_project.input
        assert isinstance(inner_filter, Filter)

        # The right-hand side of the equality should be a CorrelatedRef
        pred = inner_filter.predicate
        assert isinstance(pred, BinaryExpr)
        assert pred.op == BinaryOp.EQ
        assert pred.right == CorrelatedRef(outer_alias="e", col="dept_id")

    def test_outer_operand_resolved_as_column(self) -> None:
        """The IN-operand (e.dept_id) resolves against the outer scope."""
        schema = _two_table_schema()
        outer = _plan_correlated_in_query(schema)

        in_sub = outer.input.predicate  # type: ignore[union-attr]
        assert isinstance(in_sub, InSubquery)
        assert in_sub.operand == Column("e", "dept_id")


# ---------------------------------------------------------------------------
# Correlated EXISTS subquery
# ---------------------------------------------------------------------------


class TestCorrelatedExistsSubquery:
    def test_exists_inner_ref_becomes_correlated(self) -> None:
        """EXISTS subquery inner reference to outer alias → CorrelatedRef."""
        schema = _two_table_schema()

        # SELECT id FROM employees AS e
        # WHERE EXISTS (SELECT 1 FROM departments WHERE id = e.dept_id)
        inner_stmt = SelectStmt(
            from_=TableRef(table="departments"),
            items=(SelectItem(expr=Literal(value=1)),),
            where=BinaryExpr(
                op=BinaryOp.EQ,
                left=Column("departments", "id"),
                right=Column("e", "dept_id"),  # outer ref
            ),
        )
        outer_stmt = SelectStmt(
            from_=TableRef(table="employees", alias="e"),
            items=(SelectItem(expr=Column("e", "id")),),
            where=ExistsSubquery(query=inner_stmt),
        )
        result = plan(outer_stmt, schema)
        assert isinstance(result, Project)

        outer_filter = result.input
        assert isinstance(outer_filter, Filter)
        exists = outer_filter.predicate
        assert isinstance(exists, ExistsSubquery)

        inner_project = exists.query
        assert isinstance(inner_project, Project)
        inner_filter = inner_project.input
        assert isinstance(inner_filter, Filter)
        pred = inner_filter.predicate
        assert isinstance(pred, BinaryExpr)
        assert pred.right == CorrelatedRef(outer_alias="e", col="dept_id")


# ---------------------------------------------------------------------------
# Correlated scalar subquery
# ---------------------------------------------------------------------------


class TestCorrelatedScalarSubquery:
    def test_scalar_subquery_ref_becomes_correlated(self) -> None:
        """Scalar subquery inner reference to outer alias → CorrelatedRef."""
        schema = _two_table_schema()

        # SELECT e.name, (SELECT dept_name FROM departments WHERE id = e.dept_id)
        # FROM employees AS e
        inner_stmt = SelectStmt(
            from_=TableRef(table="departments"),
            items=(SelectItem(expr=Column("departments", "dept_name")),),
            where=BinaryExpr(
                op=BinaryOp.EQ,
                left=Column("departments", "id"),
                right=Column("e", "dept_id"),  # outer ref
            ),
        )
        outer_stmt = SelectStmt(
            from_=TableRef(table="employees", alias="e"),
            items=(
                SelectItem(expr=Column("e", "name")),
                SelectItem(expr=ScalarSubquery(query=inner_stmt)),
            ),
        )
        result = plan(outer_stmt, schema)
        assert isinstance(result, Project)

        # Find the scalar subquery in the projection items
        scalar_item = result.items[1]
        assert isinstance(scalar_item.expr, ScalarSubquery)
        inner_project = scalar_item.expr.query
        assert isinstance(inner_project, Project)
        inner_filter = inner_project.input
        assert isinstance(inner_filter, Filter)
        pred = inner_filter.predicate
        assert isinstance(pred, BinaryExpr)
        assert pred.right == CorrelatedRef(outer_alias="e", col="dept_id")


# ---------------------------------------------------------------------------
# Correlated NOT IN subquery
# ---------------------------------------------------------------------------


class TestCorrelatedNotInSubquery:
    def test_not_in_inner_ref_becomes_correlated(self) -> None:
        """NOT IN subquery inner reference to outer alias → CorrelatedRef."""
        schema = _two_table_schema()

        inner_stmt = SelectStmt(
            from_=TableRef(table="departments"),
            items=(SelectItem(expr=Column("departments", "id")),),
            where=BinaryExpr(
                op=BinaryOp.EQ,
                left=Column("departments", "id"),
                right=Column("e", "dept_id"),  # outer ref
            ),
        )
        outer_stmt = SelectStmt(
            from_=TableRef(table="employees", alias="e"),
            items=(SelectItem(expr=Column("e", "name")),),
            where=NotInSubquery(
                operand=Column("e", "dept_id"),
                query=inner_stmt,
            ),
        )
        result = plan(outer_stmt, schema)
        assert isinstance(result, Project)

        not_in_sub = result.input.predicate  # type: ignore[union-attr]
        assert isinstance(not_in_sub, NotInSubquery)
        inner_project = not_in_sub.query
        assert isinstance(inner_project, Project)
        inner_filter = inner_project.input
        assert isinstance(inner_filter, Filter)
        pred = inner_filter.predicate
        assert isinstance(pred, BinaryExpr)
        assert pred.right == CorrelatedRef(outer_alias="e", col="dept_id")


# ---------------------------------------------------------------------------
# Bare-name outer scope resolution (unqualified reference)
# ---------------------------------------------------------------------------


class TestBareNameOuterScope:
    def test_bare_col_resolves_to_correlated_ref(self) -> None:
        """Bare column reference that only exists in outer scope → CorrelatedRef.

        If the inner WHERE has ``WHERE id = dept_id`` and the inner table
        (departments) does not have a column called ``dept_id``, the planner
        should try the outer scope and find it there, producing a CorrelatedRef
        with the outer table alias.
        """
        schema = _two_table_schema()

        # The inner WHERE uses bare ``dept_id`` — not a column of departments
        # (departments has "id" and "dept_name"). It should resolve via the
        # outer scope (employees via alias "e").
        inner_stmt = SelectStmt(
            from_=TableRef(table="departments"),
            items=(SelectItem(expr=Column("departments", "id")),),
            where=BinaryExpr(
                op=BinaryOp.EQ,
                left=Column("departments", "id"),
                right=Column(None, "dept_id"),  # bare — only in outer scope
            ),
        )
        outer_stmt = SelectStmt(
            from_=TableRef(table="employees", alias="e"),
            items=(SelectItem(expr=Column("e", "name")),),
            where=InSubquery(
                operand=Column("e", "dept_id"),
                query=inner_stmt,
            ),
        )
        result = plan(outer_stmt, schema)
        assert isinstance(result, Project)

        in_sub = result.input.predicate  # type: ignore[union-attr]
        assert isinstance(in_sub, InSubquery)
        inner_project = in_sub.query
        inner_filter = inner_project.input  # type: ignore[union-attr]
        assert isinstance(inner_filter, Filter)
        pred = inner_filter.predicate
        assert isinstance(pred, BinaryExpr)
        # The bare "dept_id" should resolve to CorrelatedRef pointing at "e"
        assert pred.right == CorrelatedRef(outer_alias="e", col="dept_id")


# ---------------------------------------------------------------------------
# Error cases — still raises UnknownColumn when outer scope doesn't help
# ---------------------------------------------------------------------------


class TestCorrelatedErrors:
    def test_unknown_outer_alias_raises(self) -> None:
        """Qualified ref to an alias not in inner or outer scope → UnknownColumn."""
        schema = _two_table_schema()

        # "x.dept_id" — "x" is not an alias in either scope
        inner_stmt = SelectStmt(
            from_=TableRef(table="departments"),
            items=(SelectItem(expr=Column("departments", "id")),),
            where=BinaryExpr(
                op=BinaryOp.EQ,
                left=Column("departments", "id"),
                right=Column("x", "dept_id"),  # "x" doesn't exist anywhere
            ),
        )
        outer_stmt = SelectStmt(
            from_=TableRef(table="employees", alias="e"),
            items=(SelectItem(expr=Column("e", "name")),),
            where=InSubquery(
                operand=Column("e", "dept_id"),
                query=inner_stmt,
            ),
        )
        with pytest.raises(UnknownColumn):
            plan(outer_stmt, schema)

    def test_bare_col_not_in_any_scope_raises(self) -> None:
        """Bare column reference absent from both inner and outer scope → UnknownColumn."""
        schema = _two_table_schema()

        inner_stmt = SelectStmt(
            from_=TableRef(table="departments"),
            items=(SelectItem(expr=Column("departments", "id")),),
            where=BinaryExpr(
                op=BinaryOp.EQ,
                left=Column("departments", "id"),
                right=Column(None, "nonexistent_col"),  # not in any scope
            ),
        )
        outer_stmt = SelectStmt(
            from_=TableRef(table="employees", alias="e"),
            items=(SelectItem(expr=Column("e", "name")),),
            where=InSubquery(
                operand=Column("e", "dept_id"),
                query=inner_stmt,
            ),
        )
        with pytest.raises(UnknownColumn):
            plan(outer_stmt, schema)
