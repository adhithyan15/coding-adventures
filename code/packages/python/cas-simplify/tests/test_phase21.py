"""Phase 21 tests — assumption framework, radcan, logcontract/logexpand,
exponentialize, and DeMoivre.

Test organisation
-----------------
Six test classes, each covering one feature cluster:

1. ``TestAssumptionContext``  — assume/forget/query API
2. ``TestRadcan``             — radical canonicalization rules
3. ``TestLogcontract``        — log contraction rules
4. ``TestLogexpand``          — log expansion rules
5. ``TestExponentialize``     — trig/hyp → exponential form
6. ``TestDeMoivre``           — complex exponential decomposition
"""

from __future__ import annotations

from symbolic_ir import (
    ADD,
    COS,
    COSH,
    DIV,
    EQUAL,
    EXP,
    GREATER,
    GREATER_EQUAL,
    LESS,
    LOG,
    MUL,
    NOT_EQUAL,
    POW,
    SIN,
    SINH,
    SQRT,
    SUB,
    TAN,
    TANH,
    IRApply,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

from cas_simplify.assumptions import AssumptionContext
from cas_simplify.exponentialize import demoivre, exponentialize
from cas_simplify.logcontract import logcontract, logexpand
from cas_simplify.radcan import radcan

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

x = IRSymbol("x")
y = IRSymbol("y")
a = IRSymbol("a")
b = IRSymbol("b")
n = IRSymbol("n")

_I = IRSymbol("ImaginaryUnit")


def _greater_zero(sym: IRSymbol) -> IRApply:
    """Build Greater(sym, 0)."""
    return IRApply(GREATER, (sym, IRInteger(0)))


def _less_zero(sym: IRSymbol) -> IRApply:
    """Build Less(sym, 0)."""
    return IRApply(LESS, (sym, IRInteger(0)))


def _fresh_ctx() -> AssumptionContext:
    """Return a new, empty AssumptionContext."""
    return AssumptionContext()


def _ctx_pos(*syms: IRSymbol) -> AssumptionContext:
    """Return a context in which all given symbols are assumed positive."""
    ctx = _fresh_ctx()
    for s in syms:
        ctx.assume_relation(_greater_zero(s))
    return ctx


# ===========================================================================
# 1. AssumptionContext
# ===========================================================================


class TestAssumptionContext:
    """Unit tests for the AssumptionContext API."""

    def test_empty_context_returns_none(self) -> None:
        """A brand-new context knows nothing — every query is None / False."""
        ctx = _fresh_ctx()
        assert ctx.is_positive("x") is None
        assert ctx.is_negative("x") is None
        assert ctx.sign_of("x") is None
        assert ctx.is_integer("x") is False
        assert ctx.has_any_facts("x") is False

    def test_assume_greater_zero(self) -> None:
        """assume(x > 0) marks x as positive."""
        ctx = _fresh_ctx()
        ctx.assume_relation(_greater_zero(x))
        assert ctx.is_positive("x") is True
        assert ctx.is_negative("x") is False
        assert ctx.sign_of("x") == 1

    def test_assume_less_zero(self) -> None:
        """assume(x < 0) marks x as negative."""
        ctx = _fresh_ctx()
        ctx.assume_relation(_less_zero(x))
        assert ctx.is_negative("x") is True
        assert ctx.is_positive("x") is False
        assert ctx.sign_of("x") == -1

    def test_assume_equal_zero(self) -> None:
        """assume(x = 0) marks x as zero."""
        ctx = _fresh_ctx()
        ctx.assume_relation(IRApply(EQUAL, (x, IRInteger(0))))
        assert ctx.sign_of("x") == 0
        assert ctx.is_positive("x") is False

    def test_assume_not_equal_zero(self) -> None:
        """assume(x ≠ 0) does not reveal sign but blocks equal-zero query."""
        ctx = _fresh_ctx()
        ctx.assume_relation(IRApply(NOT_EQUAL, (x, IRInteger(0))))
        assert ctx.sign_of("x") is None  # direction still unknown
        assert ctx.is_true_relation(IRApply(EQUAL, (x, IRInteger(0)))) is False

    def test_assume_greater_equal_zero(self) -> None:
        """assume(x ≥ 0) marks x as nonneg."""
        ctx = _fresh_ctx()
        ctx.assume_relation(IRApply(GREATER_EQUAL, (x, IRInteger(0))))
        assert ctx.is_nonneg("x") is True
        assert ctx.is_negative("x") is False

    def test_assume_property_positive(self) -> None:
        """assume_property(x, positive) is equivalent to assume(x > 0)."""
        ctx = _fresh_ctx()
        ctx.assume_property(x, IRSymbol("positive"))
        assert ctx.is_positive("x") is True

    def test_assume_property_integer(self) -> None:
        """assume_property(n, integer) marks n as an integer."""
        ctx = _fresh_ctx()
        ctx.assume_property(n, IRSymbol("integer"))
        assert ctx.is_integer("n") is True

    def test_forget_relation(self) -> None:
        """forget(x > 0) removes the positive fact."""
        ctx = _fresh_ctx()
        ctx.assume_relation(_greater_zero(x))
        assert ctx.is_positive("x") is True
        ctx.forget_relation(_greater_zero(x))
        assert ctx.is_positive("x") is None

    def test_forget_all(self) -> None:
        """forget_all() clears every recorded fact."""
        ctx = _fresh_ctx()
        ctx.assume_relation(_greater_zero(x))
        ctx.assume_property(n, IRSymbol("integer"))
        ctx.forget_all()
        assert ctx.is_positive("x") is None
        assert ctx.is_integer("n") is False

    def test_is_true_relation_positive(self) -> None:
        """is_true_relation(Greater(x,0)) returns True after assume(x>0)."""
        ctx = _ctx_pos(x)
        rel = IRApply(GREATER, (x, IRInteger(0)))
        assert ctx.is_true_relation(rel) is True

    def test_is_true_relation_unknown(self) -> None:
        """is_true_relation on an unrecorded symbol returns None."""
        ctx = _fresh_ctx()
        rel = IRApply(GREATER, (y, IRInteger(0)))
        assert ctx.is_true_relation(rel) is None

    def test_multiple_symbols_independent(self) -> None:
        """Assumptions for x do not bleed into y."""
        ctx = _fresh_ctx()
        ctx.assume_relation(_greater_zero(x))
        assert ctx.is_positive("x") is True
        assert ctx.is_positive("y") is None


# ===========================================================================
# 2. Radcan
# ===========================================================================


class TestRadcan:
    """Unit tests for radcan() — radical canonicalization."""

    def test_passthrough_symbol(self) -> None:
        """A bare symbol is unchanged."""
        assert radcan(x) == x

    def test_passthrough_integer(self) -> None:
        """A literal integer is unchanged."""
        assert radcan(IRInteger(5)) == IRInteger(5)

    def test_sqrt_perfect_square_int(self) -> None:
        """Sqrt(4) → 2."""
        expr = IRApply(SQRT, (IRInteger(4),))
        assert radcan(expr) == IRInteger(2)

    def test_sqrt_perfect_square_large(self) -> None:
        """Sqrt(9) → 3."""
        expr = IRApply(SQRT, (IRInteger(9),))
        assert radcan(expr) == IRInteger(3)

    def test_sqrt_non_perfect_square_unchanged(self) -> None:
        """Sqrt(2) stays as Sqrt(2) — not a perfect square."""
        expr = IRApply(SQRT, (IRInteger(2),))
        result = radcan(expr)
        assert result == expr

    def test_sqrt_x_squared_positive(self) -> None:
        """Sqrt(x²) → x when x > 0."""
        ctx = _ctx_pos(x)
        expr = IRApply(SQRT, (IRApply(POW, (x, IRInteger(2))),))
        assert radcan(expr, ctx) == x

    def test_sqrt_x_squared_no_ctx(self) -> None:
        """Sqrt(x²) stays unevaluated without positivity context."""
        expr = IRApply(SQRT, (IRApply(POW, (x, IRInteger(2))),))
        result = radcan(expr)
        # Should not simplify to x — sign unknown.
        assert result != x

    def test_sqrt_mul_perfect_square_factor(self) -> None:
        """Sqrt(x²*y) → x*Sqrt(y) when x > 0."""
        ctx = _ctx_pos(x)
        inner = IRApply(MUL, (IRApply(POW, (x, IRInteger(2))), y))
        expr = IRApply(SQRT, (inner,))
        result = radcan(expr, ctx)
        # Should produce Mul(x, Sqrt(y))
        assert isinstance(result, IRApply)
        assert result.head == MUL
        assert x in result.args
        assert IRApply(SQRT, (y,)) in result.args

    def test_sqrt_merge_product(self) -> None:
        """Sqrt(a) * Sqrt(b) → Sqrt(a*b)."""
        expr = IRApply(MUL, (IRApply(SQRT, (a,)), IRApply(SQRT, (b,))))
        result = radcan(expr)
        assert isinstance(result, IRApply)
        assert result.head == SQRT
        inner = result.args[0]
        # Inner should be Mul(a, b) or Mul(b, a).
        assert isinstance(inner, IRApply) and inner.head == MUL
        assert set(inner.args) == {a, b}

    def test_sqrt_integer_product(self) -> None:
        """Sqrt(4) * Sqrt(9) simplifies to 2 * 3.

        radcan's bottom-up pass first collapses Sqrt(4) → 2 and Sqrt(9) → 3,
        then the Mul(2, 3) no longer contains Sqrt nodes so the product stays
        as Mul(2, 3).  Numeric folding (2*3 → 6) is the job of cas_simplify's
        simplify() pipeline, not radcan.
        """
        expr = IRApply(MUL, (
            IRApply(SQRT, (IRInteger(4),)),
            IRApply(SQRT, (IRInteger(9),)),
        ))
        result = radcan(expr)
        # Either already folded to 6 by future integration, or still Mul(2, 3).
        expected_mul = IRApply(MUL, (IRInteger(2), IRInteger(3)))
        assert result == IRInteger(6) or result == expected_mul

    def test_pow_sqrt_sq(self) -> None:
        """Pow(Sqrt(x), 2) → x."""
        expr = IRApply(POW, (IRApply(SQRT, (x,)), IRInteger(2)))
        assert radcan(expr) == x

    def test_exp_log_cancel(self) -> None:
        """Exp(Log(x)) → x."""
        expr = IRApply(EXP, (IRApply(LOG, (x,)),))
        assert radcan(expr) == x

    def test_log_exp_cancel(self) -> None:
        """Log(Exp(x)) → x."""
        expr = IRApply(LOG, (IRApply(EXP, (x,)),))
        assert radcan(expr) == x

    def test_common_rational_exponent_collection(self) -> None:
        """a^(1/3) * b^(1/3) → (a*b)^(1/3)."""
        third = IRRational(1, 3)
        expr = IRApply(MUL, (
            IRApply(POW, (a, third)),
            IRApply(POW, (b, third)),
        ))
        result = radcan(expr)
        assert isinstance(result, IRApply) and result.head == POW
        base = result.args[0]
        exp = result.args[1]
        assert exp == third
        assert isinstance(base, IRApply) and base.head == MUL
        assert set(base.args) == {a, b}


# ===========================================================================
# 3. Logcontract
# ===========================================================================


class TestLogcontract:
    """Unit tests for logcontract() — combine log sums."""

    def test_passthrough_no_log(self) -> None:
        """An expression without Log is unchanged."""
        expr = IRApply(ADD, (x, y))
        assert logcontract(expr) == expr

    def test_sum_two_logs(self) -> None:
        """log(a) + log(b) → log(a*b)."""
        expr = IRApply(ADD, (IRApply(LOG, (a,)), IRApply(LOG, (b,))))
        result = logcontract(expr)
        assert isinstance(result, IRApply)
        assert result.head == LOG
        inner = result.args[0]
        assert isinstance(inner, IRApply) and inner.head == MUL

    def test_sum_three_logs(self) -> None:
        """log(a) + log(b) + log(x) → log(a*b*x)."""
        expr = IRApply(ADD, (
            IRApply(LOG, (a,)),
            IRApply(LOG, (b,)),
            IRApply(LOG, (x,)),
        ))
        result = logcontract(expr)
        assert result.head == LOG
        inner = result.args[0]
        assert isinstance(inner, IRApply) and inner.head == MUL
        assert len(inner.args) == 3

    def test_sum_log_plus_non_log(self) -> None:
        """log(a) + x is unchanged (only one Log)."""
        expr = IRApply(ADD, (IRApply(LOG, (a,)), x))
        result = logcontract(expr)
        # Only one Log — rule doesn't fire, result should be equivalent.
        # Actually logcontract still returns the original (unchanged).
        assert result == expr

    def test_difference_two_logs(self) -> None:
        """log(a) - log(b) → log(a/b)."""
        expr = IRApply(SUB, (IRApply(LOG, (a,)), IRApply(LOG, (b,))))
        result = logcontract(expr)
        assert result.head == LOG
        inner = result.args[0]
        assert isinstance(inner, IRApply) and inner.head == DIV

    def test_coeff_times_log_integer(self) -> None:
        """2 * log(x) → log(x^2)."""
        expr = IRApply(MUL, (IRInteger(2), IRApply(LOG, (x,))))
        result = logcontract(expr)
        assert result.head == LOG
        inner = result.args[0]
        assert isinstance(inner, IRApply) and inner.head == POW
        assert inner.args[0] == x
        assert inner.args[1] == IRInteger(2)

    def test_coeff_times_log_rational(self) -> None:
        """(1/2) * log(x) → log(x^(1/2))."""
        half = IRRational(1, 2)
        expr = IRApply(MUL, (half, IRApply(LOG, (x,))))
        result = logcontract(expr)
        assert result.head == LOG
        inner = result.args[0]
        assert isinstance(inner, IRApply) and inner.head == POW
        assert inner.args[1] == half

    def test_non_numeric_coeff_no_contract(self) -> None:
        """x * log(y) is NOT contracted (x is not numeric)."""
        expr = IRApply(MUL, (x, IRApply(LOG, (y,))))
        result = logcontract(expr)
        assert result.head == MUL  # unchanged

    def test_nested_sum_contracts(self) -> None:
        """Bottom-up: log inside nested add gets contracted."""
        inner_sum = IRApply(ADD, (IRApply(LOG, (a,)), IRApply(LOG, (b,))))
        outer = IRApply(ADD, (x, inner_sum))
        result = logcontract(outer)
        # The inner Add(log(a), log(b)) should contract to log(a*b),
        # leaving Add(x, log(a*b)).
        assert isinstance(result, IRApply) and result.head == ADD
        log_node = next(
            (
                arg
                for arg in result.args
                if isinstance(arg, IRApply) and arg.head == LOG
            ),
            None,
        )
        assert log_node is not None

    def test_sum_two_logs_structure(self) -> None:
        """Verify the merged product contains both original radicands."""
        log_a = IRApply(LOG, (a,))
        log_b = IRApply(LOG, (b,))
        result = logcontract(IRApply(ADD, (log_a, log_b)))
        inner = result.args[0]
        assert a in inner.args
        assert b in inner.args

    def test_add_two_logs_partial(self) -> None:
        """Add(log(a), c, log(b)) → Add(c, log(a*b))."""
        c = IRSymbol("c")
        expr = IRApply(ADD, (IRApply(LOG, (a,)), c, IRApply(LOG, (b,))))
        result = logcontract(expr)
        assert isinstance(result, IRApply) and result.head == ADD
        assert c in result.args


# ===========================================================================
# 4. Logexpand
# ===========================================================================


class TestLogexpand:
    """Unit tests for logexpand() — distribute a log over products / powers."""

    def test_passthrough_no_log(self) -> None:
        """An expression without Log is unchanged."""
        assert logexpand(x) == x

    def test_log_power_integer(self) -> None:
        """log(x^3) → 3*log(x)."""
        expr = IRApply(LOG, (IRApply(POW, (x, IRInteger(3))),))
        result = logexpand(expr)
        assert isinstance(result, IRApply) and result.head == MUL
        assert IRInteger(3) in result.args
        log_part = next(
            (a for a in result.args if isinstance(a, IRApply) and a.head == LOG),
            None,
        )
        assert log_part is not None and log_part.args[0] == x

    def test_log_power_rational(self) -> None:
        """log(x^(1/2)) → (1/2)*log(x)."""
        half = IRRational(1, 2)
        expr = IRApply(LOG, (IRApply(POW, (x, half)),))
        result = logexpand(expr)
        assert isinstance(result, IRApply) and result.head == MUL
        assert half in result.args

    def test_log_product_two_factors(self) -> None:
        """log(a*b) → log(a) + log(b)."""
        expr = IRApply(LOG, (IRApply(MUL, (a, b)),))
        result = logexpand(expr)
        assert isinstance(result, IRApply) and result.head == ADD

    def test_log_product_three_factors(self) -> None:
        """log(a*b*x) produces a nested Add."""
        expr = IRApply(LOG, (IRApply(MUL, (a, b, x)),))
        result = logexpand(expr)
        # Should be Add(...) structure with three log terms total.
        def _count_logs(node: IRNode) -> int:
            if isinstance(node, IRApply) and node.head == LOG:
                return 1
            if isinstance(node, IRApply):
                return sum(_count_logs(arg) for arg in node.args)
            return 0
        assert _count_logs(result) == 3

    def test_log_quotient(self) -> None:
        """log(a/b) → log(a) - log(b)."""
        expr = IRApply(LOG, (IRApply(DIV, (a, b)),))
        result = logexpand(expr)
        assert isinstance(result, IRApply) and result.head == SUB
        assert result.args[0] == IRApply(LOG, (a,))
        assert result.args[1] == IRApply(LOG, (b,))

    def test_log_plain_symbol_unchanged(self) -> None:
        """log(x) (no product/power/quotient) is unchanged."""
        expr = IRApply(LOG, (x,))
        assert logexpand(expr) == expr

    def test_nested_log_power(self) -> None:
        """Nested: log(x^3 * y^2) expands both powers inside the product."""
        inner = IRApply(MUL, (
            IRApply(POW, (x, IRInteger(3))),
            IRApply(POW, (y, IRInteger(2))),
        ))
        expr = IRApply(LOG, (inner,))
        result = logexpand(expr)
        # After expanding log(x^3 * y^2) → log(x^3) + log(y^2)
        # → 3*log(x) + 2*log(y)
        # (because expansion recurses)
        assert result.head == ADD

    def test_ctx_parameter_accepted(self) -> None:
        """logexpand(expr, ctx) accepts a context without raising."""
        ctx = _ctx_pos(x)
        expr = IRApply(LOG, (IRApply(POW, (x, IRInteger(2))),))
        result = logexpand(expr, ctx)
        assert result is not None

    def test_log_power_negative_int(self) -> None:
        """log(x^(-1)) → (-1)*log(x)."""
        expr = IRApply(LOG, (IRApply(POW, (x, IRInteger(-1))),))
        result = logexpand(expr)
        assert isinstance(result, IRApply) and result.head == MUL
        assert IRInteger(-1) in result.args

    def test_logexpand_returns_irnode(self) -> None:
        """logexpand always returns an IRNode (never None)."""
        assert isinstance(logexpand(x), IRNode)
        assert isinstance(logexpand(IRInteger(0)), IRNode)


# ===========================================================================
# 5. Exponentialize
# ===========================================================================


class TestExponentialize:
    """Unit tests for exponentialize() — trig/hyp → exponential form."""

    def test_sin_structure(self) -> None:
        """sin(x) → Div(Sub(Exp(Mul(i,x)), Exp(Mul(i,Neg(x)))), Mul(2,i))."""
        result = exponentialize(IRApply(SIN, (x,)))
        assert isinstance(result, IRApply)
        assert result.head == DIV
        numerator, denominator = result.args
        assert numerator.head == SUB
        assert denominator.head == MUL

    def test_cos_structure(self) -> None:
        """cos(x) → Div(Add(exp(...), exp(...)), 2)."""
        result = exponentialize(IRApply(COS, (x,)))
        assert isinstance(result, IRApply) and result.head == DIV
        numerator, denominator = result.args
        assert numerator.head == ADD
        assert denominator == IRInteger(2)

    def test_tan_structure(self) -> None:
        """tan(x) → Div(numerator, Add(exp(ix), exp(-ix)))."""
        result = exponentialize(IRApply(TAN, (x,)))
        assert isinstance(result, IRApply) and result.head == DIV
        _, denominator = result.args
        assert denominator.head == ADD

    def test_sinh_structure(self) -> None:
        """sinh(x) → Div(Sub(exp(x), exp(-x)), 2)."""
        result = exponentialize(IRApply(SINH, (x,)))
        assert isinstance(result, IRApply) and result.head == DIV
        numerator, denominator = result.args
        assert numerator.head == SUB
        assert denominator == IRInteger(2)

    def test_cosh_structure(self) -> None:
        """cosh(x) → Div(Add(exp(x), exp(-x)), 2)."""
        result = exponentialize(IRApply(COSH, (x,)))
        assert isinstance(result, IRApply) and result.head == DIV
        numerator, denominator = result.args
        assert numerator.head == ADD
        assert denominator == IRInteger(2)

    def test_tanh_structure(self) -> None:
        """tanh(x) → Div(Sub(exp(x), exp(-x)), Add(exp(x), exp(-x)))."""
        result = exponentialize(IRApply(TANH, (x,)))
        assert isinstance(result, IRApply) and result.head == DIV
        numerator, denominator = result.args
        assert numerator.head == SUB
        assert denominator.head == ADD

    def test_sin_numerator_uses_exp(self) -> None:
        """The numerator of exponentialize(sin(x)) contains Exp nodes."""
        result = exponentialize(IRApply(SIN, (x,)))
        numerator = result.args[0]
        assert any(
            isinstance(arg, IRApply) and arg.head == EXP
            for arg in numerator.args
        )

    def test_nested_sin_cos(self) -> None:
        """exponentialize recurses: cos(sin(x)) becomes Exp-based."""
        inner = IRApply(SIN, (x,))
        expr = IRApply(COS, (inner,))
        result = exponentialize(expr)
        # After exponentialize(sin(x)) → Div(...), then cos(Div(...)) expands
        assert isinstance(result, IRApply) and result.head == DIV

    def test_passthrough_add(self) -> None:
        """Non-trig heads pass through; only child trig nodes get replaced."""
        expr = IRApply(ADD, (IRApply(SIN, (x,)), y))
        result = exponentialize(expr)
        assert isinstance(result, IRApply) and result.head == ADD
        # First arg should be exponentialize(sin(x))
        assert result.args[0].head == DIV

    def test_denominator_sin_contains_imaginary(self) -> None:
        """The denominator Mul(2, i) contains ImaginaryUnit."""
        result = exponentialize(IRApply(SIN, (x,)))
        denominator = result.args[1]
        assert _I in denominator.args

    def test_sinh_denominator_is_two(self) -> None:
        """sinh(x) denominator is exactly IRInteger(2) — real, no i."""
        result = exponentialize(IRApply(SINH, (x,)))
        assert result.args[1] == IRInteger(2)


# ===========================================================================
# 6. DeMoivre
# ===========================================================================


class TestDeMoivre:
    """Unit tests for demoivre() — exp(a + b·i) decomposition."""

    def test_pure_imaginary_bare_i(self) -> None:
        """exp(i) → cos(1) + i*sin(1)  [i itself as argument]."""
        result = demoivre(IRApply(EXP, (_I,)))
        assert isinstance(result, IRApply) and result.head == ADD

    def test_exp_i_times_y(self) -> None:
        """exp(i*y) → cos(y) + i*sin(y)."""
        arg = IRApply(MUL, (_I, y))
        result = demoivre(IRApply(EXP, (arg,)))
        assert isinstance(result, IRApply) and result.head == ADD
        cos_part, i_sin_part = result.args
        assert cos_part.head == COS
        assert cos_part.args[0] == y
        assert i_sin_part.head == MUL
        assert _I in i_sin_part.args

    def test_exp_real_plus_imag(self) -> None:
        """exp(x + i*y) → exp(x) * (cos(y) + i*sin(y))."""
        arg = IRApply(ADD, (x, IRApply(MUL, (_I, y))))
        result = demoivre(IRApply(EXP, (arg,)))
        assert isinstance(result, IRApply) and result.head == MUL
        exp_part = next(
            (a for a in result.args if isinstance(a, IRApply) and a.head == EXP),
            None,
        )
        assert exp_part is not None
        assert exp_part.args[0] == x

    def test_exp_real_only_unchanged(self) -> None:
        """exp(x) with no imaginary part is unchanged."""
        expr = IRApply(EXP, (x,))
        assert demoivre(expr) == expr

    def test_exp_real_plus_imag_trig_structure(self) -> None:
        """exp(x + i*y): the trig factor is Add(cos(y), Mul(i, sin(y)))."""
        arg = IRApply(ADD, (x, IRApply(MUL, (_I, y))))
        result = demoivre(IRApply(EXP, (arg,)))
        trig_factor = next(
            (a for a in result.args if isinstance(a, IRApply) and a.head == ADD),
            None,
        )
        assert trig_factor is not None
        assert trig_factor.args[0].head == COS
        assert trig_factor.args[1].head == MUL

    def test_passthrough_non_exp(self) -> None:
        """demoivre leaves non-Exp nodes unchanged."""
        expr = IRApply(ADD, (x, y))
        assert demoivre(expr) == expr

    def test_nested_exp(self) -> None:
        """demoivre recurses into nested expressions."""
        inner = IRApply(EXP, (IRApply(MUL, (_I, x)),))
        outer = IRApply(ADD, (inner, y))
        result = demoivre(outer)
        # Inner exp(i*x) should be decomposed.
        inner_result = result.args[0]
        assert inner_result.head == ADD  # cos(x) + i*sin(x)

    def test_exp_i_times_y_reversed_mul(self) -> None:
        """exp(y*i) (reversed mul order) → cos(y) + i*sin(y)."""
        arg = IRApply(MUL, (y, _I))
        result = demoivre(IRApply(EXP, (arg,)))
        assert isinstance(result, IRApply) and result.head == ADD

    def test_pure_imaginary_result_has_cos_sin(self) -> None:
        """exp(i*y) result contains Cos and Sin nodes."""
        arg = IRApply(MUL, (_I, y))
        result = demoivre(IRApply(EXP, (arg,)))
        has_cos = any(
            isinstance(a, IRApply) and a.head == COS for a in result.args
        )
        has_sin_in_mul = any(
            isinstance(a, IRApply)
            and a.head == MUL
            and any(isinstance(m, IRApply) and m.head == SIN for m in a.args)
            for a in result.args
        )
        assert has_cos
        assert has_sin_in_mul

    def test_exp_imaginary_unit_itself(self) -> None:
        """exp(ImaginaryUnit) decomposes: result is Add(Cos(1), Mul(i, Sin(1)))."""
        result = demoivre(IRApply(EXP, (_I,)))
        # imag coeff is IRInteger(1)
        assert isinstance(result, IRApply) and result.head == ADD
        cos_part, i_sin_part = result.args
        assert isinstance(cos_part, IRApply) and cos_part.head == COS
        assert cos_part.args[0] == IRInteger(1)

    def test_returns_irnode(self) -> None:
        """demoivre always returns an IRNode."""
        assert isinstance(demoivre(x), IRNode)
        assert isinstance(demoivre(IRInteger(0)), IRNode)
