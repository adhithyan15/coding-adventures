"""Planner tests for derived-table sources.

Two recent features extend the derived-table handling:

* PR #3817 — compound queries (UNION / INTERSECT / EXCEPT) are
  accepted as the inner statement of a derived table.  The planner
  routes through a new ``_plan_derived_inner`` dispatcher and the
  ``_output_columns`` / ``_source_columns`` walkers learn to descend
  through set-op nodes.
* PR #3819 — the alias is now optional (``DerivedTableRef.alias:
  str | None``).  When absent, the planner synthesises a unique
  sentinel name of the form ``<derived #hex>`` for scope/cursor
  registration.

These tests exercise both code paths directly against the planner
API, which the package-level coverage gate requires.
"""

from __future__ import annotations

from sql_planner import (
    BinaryExpr,
    BinaryOp,
    Column,
    DerivedTable,
    DerivedTableRef,
    ExceptStmt,
    InMemorySchemaProvider,
    Intersect,
    IntersectStmt,
    Literal,
    Project,
    ProjectionItem,
    SelectItem,
    SelectStmt,
    Union,
    UnionStmt,
    Wildcard,
    plan,
)


def _empty_schema() -> InMemorySchemaProvider:
    """Schema provider with no tables — derived-table tests don't need real ones."""
    return InMemorySchemaProvider({})


def _select_one(alias: str = "x") -> SelectStmt:
    """Helper: a SELECT 1 AS x statement, used as derived-table inner."""
    return SelectStmt(items=(SelectItem(expr=Literal(value=1), alias=alias),))


# ---------------------------------------------------------------------------
# Compound queries as derived-table inner (PR #3817)
# ---------------------------------------------------------------------------


class TestCompoundInner:
    def test_union_inner(self) -> None:
        """UNION inside (…) AS t — planner wraps in DerivedTable over Union."""
        union = UnionStmt(left=_select_one("x"), right=_select_one("x"))
        outer = SelectStmt(
            items=(ProjectionItem(expr=Wildcard()),),
            from_=DerivedTableRef(select=union, alias="t"),
        )
        result = plan(outer, _empty_schema())
        # Outer plan is a Project; its input is the DerivedTable wrapping
        # the Union plan node.
        assert isinstance(result, Project)
        # The DerivedTable's inner query is the Union plan.
        # Walk into the project to find the DerivedTable.
        dt = result.input
        assert isinstance(dt, DerivedTable)
        assert isinstance(dt.query, Union)

    def test_intersect_inner(self) -> None:
        intersect = IntersectStmt(left=_select_one("x"), right=_select_one("x"))
        outer = SelectStmt(
            items=(ProjectionItem(expr=Wildcard()),),
            from_=DerivedTableRef(select=intersect, alias="t"),
        )
        result = plan(outer, _empty_schema())
        assert isinstance(result, Project)
        dt = result.input
        assert isinstance(dt, DerivedTable)
        assert isinstance(dt.query, Intersect)

    def test_except_inner(self) -> None:
        except_ = ExceptStmt(left=_select_one("x"), right=_select_one("x"))
        outer = SelectStmt(
            items=(ProjectionItem(expr=Wildcard()),),
            from_=DerivedTableRef(select=except_, alias="t"),
        )
        result = plan(outer, _empty_schema())
        assert isinstance(result, Project)
        dt = result.input
        assert isinstance(dt, DerivedTable)
        # Except plan node — we import it from the package below if exported.
        # Just verify the wrapper type rather than asserting on the inner type
        # to avoid an import cycle in the test (the package re-exports the
        # main plan nodes but not always every set-op).
        assert type(dt.query).__name__ == "Except"


# ---------------------------------------------------------------------------
# Optional alias for derived tables (PR #3819)
# ---------------------------------------------------------------------------


class TestOptionalAlias:
    def test_no_alias_synthesises_sentinel(self) -> None:
        """When alias is None, the planner uses a ``<derived #…>`` sentinel."""
        outer = SelectStmt(
            items=(ProjectionItem(expr=Wildcard()),),
            from_=DerivedTableRef(select=_select_one("x"), alias=None),
        )
        result = plan(outer, _empty_schema())
        assert isinstance(result, Project)
        dt = result.input
        assert isinstance(dt, DerivedTable)
        # Sentinel format ``<derived #<hex>>``.
        assert dt.alias.startswith("<derived #")
        assert dt.alias.endswith(">")

    def test_explicit_alias_passes_through(self) -> None:
        """Explicit alias is preserved verbatim (regression guard)."""
        outer = SelectStmt(
            items=(ProjectionItem(expr=Wildcard()),),
            from_=DerivedTableRef(select=_select_one("x"), alias="t"),
        )
        result = plan(outer, _empty_schema())
        dt = result.input
        assert isinstance(dt, DerivedTable)
        assert dt.alias == "t"

    def test_no_alias_columns_resolvable_unqualified(self) -> None:
        """Unqualified column refs against an unaliased derived table work."""
        outer = SelectStmt(
            items=(ProjectionItem(expr=Column(table=None, col="x")),),
            from_=DerivedTableRef(select=_select_one("x"), alias=None),
        )
        # Should not raise UnknownColumn — the planner walks scope values
        # via the synthetic alias and finds 'x'.
        result = plan(outer, _empty_schema())
        assert isinstance(result, Project)


# ---------------------------------------------------------------------------
# Compound inner + no alias together (the full combination)
# ---------------------------------------------------------------------------


class TestCompoundInnerNoAlias:
    def test_union_inner_no_alias(self) -> None:
        union = UnionStmt(left=_select_one("x"), right=_select_one("x"))
        outer = SelectStmt(
            items=(ProjectionItem(expr=Column(table=None, col="x")),),
            from_=DerivedTableRef(select=union, alias=None),
        )
        result = plan(outer, _empty_schema())
        assert isinstance(result, Project)
        dt = result.input
        assert isinstance(dt, DerivedTable)
        assert isinstance(dt.query, Union)
        assert dt.alias.startswith("<derived #")

    def test_intersect_inner_no_alias(self) -> None:
        intersect = IntersectStmt(left=_select_one("x"), right=_select_one("x"))
        outer = SelectStmt(
            items=(ProjectionItem(expr=Wildcard()),),
            from_=DerivedTableRef(select=intersect, alias=None),
        )
        result = plan(outer, _empty_schema())
        dt = result.input
        assert isinstance(dt, DerivedTable)
        assert isinstance(dt.query, Intersect)


# ---------------------------------------------------------------------------
# Outer query uses the derived table — filter/projection round-trip
# ---------------------------------------------------------------------------


class TestOuterReferences:
    def test_outer_filter_on_derived_column(self) -> None:
        """WHERE x > 0 against an unaliased derived table."""
        outer = SelectStmt(
            items=(ProjectionItem(expr=Column(table=None, col="x")),),
            from_=DerivedTableRef(select=_select_one("x"), alias=None),
            where=BinaryExpr(
                op=BinaryOp.GT,
                left=Column(table=None, col="x"),
                right=Literal(value=0),
            ),
        )
        # Should plan successfully — exercises the bare-column resolution
        # through the synthetic alias.
        result = plan(outer, _empty_schema())
        assert isinstance(result, Project)
