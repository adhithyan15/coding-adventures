"""ConstantFolding — arithmetic, comparison, boolean, NULL propagation."""

from __future__ import annotations

from sql_planner import (
    BinaryExpr,
    BinaryOp,
    Column,
    Expr,
    Filter,
    IsNotNull,
    IsNull,
    Literal,
    Project,
    ProjectionItem,
    Scan,
    UnaryExpr,
    UnaryOp,
)

from sql_optimizer import ConstantFolding


def fold_expr(expr: Expr) -> Expr:
    """Helper: fold an expression by wrapping in a Filter and unwrapping."""
    p = Filter(input=Scan(table="t"), predicate=expr)
    return ConstantFolding()(p).predicate


class TestArithmetic:
    def test_add(self) -> None:
        e = BinaryExpr(op=BinaryOp.ADD, left=Literal(1), right=Literal(2))
        assert fold_expr(e) == Literal(3)

    def test_mul(self) -> None:
        e = BinaryExpr(op=BinaryOp.MUL, left=Literal(10), right=Literal(4))
        assert fold_expr(e) == Literal(40)

    def test_integer_div(self) -> None:
        e = BinaryExpr(op=BinaryOp.DIV, left=Literal(7), right=Literal(2))
        assert fold_expr(e) == Literal(3)

    def test_string_concat_via_add(self) -> None:
        # SQL-style || is not modeled; but ADD on strings gives concat in
        # Python semantics. We still fold it because the types are known.
        e = BinaryExpr(op=BinaryOp.ADD, left=Literal("a"), right=Literal("b"))
        assert fold_expr(e) == Literal("ab")

    def test_mod(self) -> None:
        e = BinaryExpr(op=BinaryOp.MOD, left=Literal(10), right=Literal(3))
        assert fold_expr(e) == Literal(1)

    def test_division_by_zero_not_folded(self) -> None:
        e = BinaryExpr(op=BinaryOp.DIV, left=Literal(1), right=Literal(0))
        out = fold_expr(e)
        assert isinstance(out, BinaryExpr)

    def test_nested_folds_bottom_up(self) -> None:
        # (1+2) * (3+4) = 3 * 7 = 21
        e = BinaryExpr(
            op=BinaryOp.MUL,
            left=BinaryExpr(op=BinaryOp.ADD, left=Literal(1), right=Literal(2)),
            right=BinaryExpr(op=BinaryOp.ADD, left=Literal(3), right=Literal(4)),
        )
        assert fold_expr(e) == Literal(21)


class TestComparison:
    def test_less_than_true(self) -> None:
        e = BinaryExpr(op=BinaryOp.LT, left=Literal(1), right=Literal(2))
        assert fold_expr(e) == Literal(True)

    def test_equal_false(self) -> None:
        e = BinaryExpr(op=BinaryOp.EQ, left=Literal("a"), right=Literal("b"))
        assert fold_expr(e) == Literal(False)

    def test_null_comparison_yields_null(self) -> None:
        e = BinaryExpr(op=BinaryOp.EQ, left=Literal(None), right=Literal(None))
        assert fold_expr(e) == Literal(None)


class TestBooleanSimplification:
    def test_true_and_x(self) -> None:
        x = Column(table="t", col="x")
        e = BinaryExpr(op=BinaryOp.AND, left=Literal(True), right=x)
        assert fold_expr(e) == x

    def test_false_and_x(self) -> None:
        x = Column(table="t", col="x")
        e = BinaryExpr(op=BinaryOp.AND, left=Literal(False), right=x)
        assert fold_expr(e) == Literal(False)

    def test_true_or_x(self) -> None:
        x = Column(table="t", col="x")
        e = BinaryExpr(op=BinaryOp.OR, left=Literal(True), right=x)
        assert fold_expr(e) == Literal(True)

    def test_false_or_x(self) -> None:
        x = Column(table="t", col="x")
        e = BinaryExpr(op=BinaryOp.OR, left=Literal(False), right=x)
        assert fold_expr(e) == x

    def test_not_true(self) -> None:
        assert fold_expr(UnaryExpr(op=UnaryOp.NOT, operand=Literal(True))) == Literal(False)

    def test_not_false(self) -> None:
        assert fold_expr(UnaryExpr(op=UnaryOp.NOT, operand=Literal(False))) == Literal(True)

    def test_not_null(self) -> None:
        assert fold_expr(UnaryExpr(op=UnaryOp.NOT, operand=Literal(None))) == Literal(None)

    def test_neg_literal(self) -> None:
        assert fold_expr(UnaryExpr(op=UnaryOp.NEG, operand=Literal(5))) == Literal(-5)


class TestNullPropagation:
    def test_null_plus_5(self) -> None:
        e = BinaryExpr(op=BinaryOp.ADD, left=Literal(None), right=Literal(5))
        assert fold_expr(e) == Literal(None)

    def test_null_times_0(self) -> None:
        e = BinaryExpr(op=BinaryOp.MUL, left=Literal(None), right=Literal(0))
        assert fold_expr(e) == Literal(None)

    def test_is_null_true(self) -> None:
        assert fold_expr(IsNull(operand=Literal(None))) == Literal(True)

    def test_is_null_false(self) -> None:
        assert fold_expr(IsNull(operand=Literal(5))) == Literal(False)

    def test_is_not_null_true(self) -> None:
        assert fold_expr(IsNotNull(operand=Literal(5))) == Literal(True)

    def test_is_not_null_false(self) -> None:
        assert fold_expr(IsNotNull(operand=Literal(None))) == Literal(False)


class TestIdempotent:
    def test_fold_twice(self) -> None:
        e = BinaryExpr(op=BinaryOp.ADD, left=Literal(1), right=Literal(2))
        once = fold_expr(e)
        twice = fold_expr(once)
        assert once == twice


class TestFoldsAcrossPlanNodes:
    def test_folds_in_project_items(self) -> None:
        p = Project(
            input=Scan(table="t"),
            items=(
                ProjectionItem(
                    expr=BinaryExpr(op=BinaryOp.ADD, left=Literal(1), right=Literal(1)),
                    alias="two",
                ),
            ),
        )
        out = ConstantFolding()(p)
        assert out.items[0].expr == Literal(2)
