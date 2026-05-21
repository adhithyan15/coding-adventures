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


class TestIsDistinctFrom:
    """Constant-folding of the NULL-safe IS [NOT] DISTINCT FROM operators."""

    def test_distinct_two_literals_unequal(self) -> None:
        e = BinaryExpr(op=BinaryOp.IS_DISTINCT_FROM, left=Literal(1), right=Literal(2))
        assert fold_expr(e) == Literal(True)

    def test_distinct_two_literals_equal(self) -> None:
        e = BinaryExpr(op=BinaryOp.IS_DISTINCT_FROM, left=Literal(1), right=Literal(1))
        assert fold_expr(e) == Literal(False)

    def test_distinct_null_left(self) -> None:
        e = BinaryExpr(op=BinaryOp.IS_DISTINCT_FROM, left=Literal(None), right=Literal(1))
        assert fold_expr(e) == Literal(True)

    def test_distinct_null_right(self) -> None:
        e = BinaryExpr(op=BinaryOp.IS_DISTINCT_FROM, left=Literal(1), right=Literal(None))
        assert fold_expr(e) == Literal(True)

    def test_distinct_both_null(self) -> None:
        e = BinaryExpr(op=BinaryOp.IS_DISTINCT_FROM, left=Literal(None), right=Literal(None))
        assert fold_expr(e) == Literal(False)

    def test_not_distinct_equal(self) -> None:
        e = BinaryExpr(op=BinaryOp.IS_NOT_DISTINCT_FROM, left=Literal(5), right=Literal(5))
        assert fold_expr(e) == Literal(True)

    def test_not_distinct_both_null(self) -> None:
        e = BinaryExpr(op=BinaryOp.IS_NOT_DISTINCT_FROM, left=Literal(None), right=Literal(None))
        assert fold_expr(e) == Literal(True)

    def test_not_distinct_mixed_null(self) -> None:
        e = BinaryExpr(op=BinaryOp.IS_NOT_DISTINCT_FROM, left=Literal(None), right=Literal(1))
        assert fold_expr(e) == Literal(False)

    def test_not_distinct_different_values(self) -> None:
        e = BinaryExpr(op=BinaryOp.IS_NOT_DISTINCT_FROM, left=Literal(1), right=Literal(2))
        assert fold_expr(e) == Literal(False)

    def test_distinct_with_column_not_folded(self) -> None:
        # When one side is a column (not a literal), we can't fold.
        e = BinaryExpr(op=BinaryOp.IS_DISTINCT_FROM, left=Column("t", "x"), right=Literal(1))
        assert fold_expr(e) == e


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


# ---------------------------------------------------------------------------
# Bitwise operators — &, |, <<, >>, ~.  See sql_vm.operators for the
# matching VM semantics; the folder must agree byte-for-byte on every
# literal input so query plans don't drift between code paths.
# ---------------------------------------------------------------------------


class TestBitwiseFolding:
    def test_and(self) -> None:
        e = BinaryExpr(op=BinaryOp.BIT_AND, left=Literal(5), right=Literal(3))
        assert fold_expr(e) == Literal(1)

    def test_or(self) -> None:
        e = BinaryExpr(op=BinaryOp.BIT_OR, left=Literal(5), right=Literal(3))
        assert fold_expr(e) == Literal(7)

    def test_shl_basic(self) -> None:
        e = BinaryExpr(op=BinaryOp.BIT_SHL, left=Literal(1), right=Literal(4))
        assert fold_expr(e) == Literal(16)

    def test_shl_wraps_at_64(self) -> None:
        # 1 << 63 wraps to -2**63 in 64-bit signed two's complement.
        e = BinaryExpr(op=BinaryOp.BIT_SHL, left=Literal(1), right=Literal(63))
        assert fold_expr(e) == Literal(-(2**63))

    def test_shl_saturates(self) -> None:
        e = BinaryExpr(op=BinaryOp.BIT_SHL, left=Literal(1), right=Literal(64))
        assert fold_expr(e) == Literal(0)

    def test_shl_negative_count(self) -> None:
        e = BinaryExpr(op=BinaryOp.BIT_SHL, left=Literal(16), right=Literal(-2))
        assert fold_expr(e) == Literal(4)

    def test_shl_far_negative(self) -> None:
        e = BinaryExpr(op=BinaryOp.BIT_SHL, left=Literal(16), right=Literal(-100))
        assert fold_expr(e) == Literal(0)
        e2 = BinaryExpr(op=BinaryOp.BIT_SHL, left=Literal(-1), right=Literal(-100))
        assert fold_expr(e2) == Literal(-1)

    def test_shr_basic(self) -> None:
        e = BinaryExpr(op=BinaryOp.BIT_SHR, left=Literal(16), right=Literal(2))
        assert fold_expr(e) == Literal(4)

    def test_shr_arithmetic(self) -> None:
        e = BinaryExpr(op=BinaryOp.BIT_SHR, left=Literal(-8), right=Literal(1))
        assert fold_expr(e) == Literal(-4)

    def test_shr_saturates_positive(self) -> None:
        e = BinaryExpr(op=BinaryOp.BIT_SHR, left=Literal(123), right=Literal(64))
        assert fold_expr(e) == Literal(0)

    def test_shr_saturates_negative(self) -> None:
        e = BinaryExpr(op=BinaryOp.BIT_SHR, left=Literal(-1), right=Literal(64))
        assert fold_expr(e) == Literal(-1)

    def test_shr_negative_count(self) -> None:
        e = BinaryExpr(op=BinaryOp.BIT_SHR, left=Literal(4), right=Literal(-2))
        assert fold_expr(e) == Literal(16)

    def test_shr_far_negative_count(self) -> None:
        e = BinaryExpr(op=BinaryOp.BIT_SHR, left=Literal(4), right=Literal(-100))
        assert fold_expr(e) == Literal(0)

    def test_not_unary(self) -> None:
        e = UnaryExpr(op=UnaryOp.BIT_NOT, operand=Literal(5))
        assert fold_expr(e) == Literal(-6)

    def test_not_neg_one(self) -> None:
        e = UnaryExpr(op=UnaryOp.BIT_NOT, operand=Literal(-1))
        assert fold_expr(e) == Literal(0)

    def test_not_float_truncates(self) -> None:
        e = UnaryExpr(op=UnaryOp.BIT_NOT, operand=Literal(5.9))
        assert fold_expr(e) == Literal(-6)

    def test_not_null_propagates(self) -> None:
        e = UnaryExpr(op=UnaryOp.BIT_NOT, operand=Literal(None))
        assert fold_expr(e) == Literal(None)

    def test_not_with_bool_not_folded(self) -> None:
        # bool ~ is implementation-defined — folder leaves it for the VM.
        e = UnaryExpr(op=UnaryOp.BIT_NOT, operand=Literal(True))
        assert fold_expr(e) == e

    def test_null_propagation_binary(self) -> None:
        e = BinaryExpr(op=BinaryOp.BIT_AND, left=Literal(None), right=Literal(3))
        assert fold_expr(e) == Literal(None)

    def test_column_not_folded(self) -> None:
        e = BinaryExpr(op=BinaryOp.BIT_AND, left=Column("t", "x"), right=Literal(3))
        assert fold_expr(e) == e

    def test_string_operand_not_folded(self) -> None:
        # _apply_binary raises TypeError for string operands, so the
        # folder catches it and leaves the expression intact for the VM
        # to report at runtime.
        e = BinaryExpr(op=BinaryOp.BIT_AND, left=Literal("foo"), right=Literal(3))
        assert fold_expr(e) == e
