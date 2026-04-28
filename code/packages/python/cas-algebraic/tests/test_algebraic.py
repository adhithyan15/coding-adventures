"""Tests for cas_algebraic — polynomial factoring over Q[√d].

Coverage target: 80%+.

Test plan
---------
1.  factor_over_extension — core algorithm, 20+ cases
2.  _is_rational_square helper
3.  alg_factor_handler — VM integration
4.  build_alg_factor_handler_table
5.  Edge cases: degree 0, 1, multi-variable, rational functions

Naming convention: test_<function>_<scenario>.
"""

from __future__ import annotations

from fractions import Fraction

from symbolic_ir import (
    ADD,
    MUL,
    NEG,
    POW,
    SQRT,
    SUB,
    IRApply,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

from cas_algebraic import factor_over_extension
from cas_algebraic.algebraic import (
    _is_rational_square,
    _try_split_depressed_quartic,
    _try_split_quadratic,
)
from cas_algebraic.handlers import (
    _alg_coeff_to_ir,
    _extract_d,
    _ir_to_poly_coeffs,
    _poly_add,
    _poly_mul,
    build_alg_factor_handler_table,
)

# ===========================================================================
# Fixtures / helpers
# ===========================================================================

x = IRSymbol("x")


class _MinimalVM:
    """Minimal recursive VM stub — no symbolic_vm dependency.

    Evaluates an IR tree bottom-up: first eval each argument, then dispatch
    to a registered handler.  Handlers are keyed by head name (string).

    This mirrors the real VM's evaluation order without pulling in the full
    symbolic_vm stack.  The same pattern is used in cas-complex tests to
    avoid the circular-dependency: symbolic_vm → cas_algebraic → symbolic_vm.

    The ``AlgFactor`` head is treated as a *held* head — its arguments are
    NOT pre-evaluated before the handler is called.  (In the real VM this is
    done via ``_HELD_HEADS``.)  For the current test expressions (which are
    already in canonical form) this makes no difference, but it matches the
    intended semantics.
    """

    # Heads whose arguments should NOT be recursively evaluated before
    # dispatching to the registered handler (mirrors _HELD_HEADS in the
    # real SymbolicBackend).
    _held: frozenset[str] = frozenset({"AlgFactor"})

    def __init__(self) -> None:
        self._handlers: dict = {}

    def eval(self, node: IRNode) -> IRNode:
        if not isinstance(node, IRApply):
            return node
        if not isinstance(node.head, IRSymbol):
            return node
        head_name = node.head.name
        # For held heads, pass the original args to the handler unchanged.
        if head_name not in self._held:
            evaled_args = tuple(self.eval(a) for a in node.args)
            if evaled_args != node.args:
                node = IRApply(node.head, evaled_args)
        handler = self._handlers.get(head_name)
        if handler is not None:
            result = handler(self, node)
            if result is not node:
                return result
        return node


def _make_vm() -> _MinimalVM:
    """Build a MinimalVM wired with the AlgFactor handler.

    No symbolic_vm import — the AlgFactor handler now uses its own
    _ir_to_poly_coeffs helper instead of symbolic_vm.polynomial_bridge.
    """
    from cas_algebraic import build_alg_factor_handler_table

    vm = _MinimalVM()
    vm._handlers.update(build_alg_factor_handler_table())
    return vm


def _sqrt(d: int) -> IRApply:
    """Convenience: build Sqrt(d) IR node."""
    return IRApply(SQRT, (IRInteger(d),))


def _alg_factor_expr(poly_ir, d: int) -> IRApply:
    """Build AlgFactor(poly, Sqrt(d)) IR."""
    return IRApply(IRSymbol("AlgFactor"), (poly_ir, _sqrt(d)))


# ===========================================================================
# Section 1: _is_rational_square
# ===========================================================================


class TestIsRationalSquare:
    """Tests for the _is_rational_square helper."""

    def test_integer_perfect_square(self):
        assert _is_rational_square(Fraction(4)) == Fraction(2)
        assert _is_rational_square(Fraction(9)) == Fraction(3)
        assert _is_rational_square(Fraction(1)) == Fraction(1)
        assert _is_rational_square(Fraction(0)) == Fraction(0)

    def test_integer_not_square(self):
        assert _is_rational_square(Fraction(2)) is None
        assert _is_rational_square(Fraction(3)) is None
        assert _is_rational_square(Fraction(5)) is None

    def test_rational_perfect_square(self):
        assert _is_rational_square(Fraction(1, 4)) == Fraction(1, 2)
        assert _is_rational_square(Fraction(9, 4)) == Fraction(3, 2)
        assert _is_rational_square(Fraction(4, 9)) == Fraction(2, 3)

    def test_rational_not_square(self):
        assert _is_rational_square(Fraction(1, 2)) is None
        assert _is_rational_square(Fraction(3, 4)) is None

    def test_negative_returns_none(self):
        assert _is_rational_square(Fraction(-1)) is None
        assert _is_rational_square(Fraction(-4)) is None


# ===========================================================================
# Section 2: factor_over_extension — the core algorithm
# ===========================================================================


class TestFactorOverExtension:
    """Test the top-level factor_over_extension dispatcher."""

    # --- Degree ≤ 1 edge cases ---

    def test_degree_zero_returns_none(self):
        """A constant polynomial cannot split."""
        assert factor_over_extension([5], 2) is None

    def test_degree_one_returns_none(self):
        """A linear polynomial is trivially irreducible."""
        assert factor_over_extension([0, 1], 2) is None

    def test_empty_returns_none(self):
        """Empty coefficient list is degenerate — return None."""
        assert factor_over_extension([], 2) is None

    # --- Quadratic cases (Pattern 2) ---

    def test_x2_minus_2_over_sqrt2(self):
        """x² − 2 = (x − √2)(x + √2) over Q[√2]."""
        result = factor_over_extension([-2, 0, 1], 2)
        assert result is not None
        assert len(result) == 2

    def test_x2_minus_3_over_sqrt3(self):
        """x² − 3 = (x − √3)(x + √3) over Q[√3]."""
        result = factor_over_extension([-3, 0, 1], 3)
        assert result is not None
        assert len(result) == 2

    def test_x2_minus_5_over_sqrt5(self):
        """x² − 5 = (x − √5)(x + √5) over Q[√5]."""
        result = factor_over_extension([-5, 0, 1], 5)
        assert result is not None
        assert len(result) == 2

    def test_x2_plus_1_irreducible_over_sqrt2(self):
        """x² + 1 is irreducible over Q[√2] (discriminant −4, and −4/2 is not a square).

        The discriminant of x²+1 is b²−4c = 0−4 = −4.
        (−4)/(4·2) = −1/2 < 0, so no real split.
        """
        result = factor_over_extension([1, 0, 1], 2)
        assert result is None

    def test_x2_plus_2_irreducible_over_sqrt2(self):
        """x² + 2 is irreducible over Q[√2].

        Discriminant = −8. −8/(4·2) = −1 < 0.
        """
        result = factor_over_extension([2, 0, 1], 2)
        assert result is None

    def test_x2_with_linear_term_splits(self):
        """x² − 3x + 2 = (x−1)(x−2) factors over Q already → None from our function.

        This is already reducible over Z, so factor_over_extension returns None
        (no NEW algebraic splitting; both factors are already in Z).
        """
        # x² − 3x + 2 = (x−1)(x−2), reducible over Z.
        # factor_over_extension returns None when no new splitting is found
        # over Q[√d] vs Q.
        result = factor_over_extension([2, -3, 1], 2)
        assert result is None

    # --- Quartic cases (Pattern 1) ---

    def test_x4_plus_1_over_sqrt2(self):
        """x⁴ + 1 = (x²+√2x+1)(x²−√2x+1) over Q[√2].

        This is the canonical example.  Verify we get 2 quadratic factors.
        """
        result = factor_over_extension([1, 0, 0, 0, 1], 2)
        assert result is not None
        assert len(result) == 2
        # Each factor should be a degree-2 algebraic polynomial (3 coefficients).
        assert len(result[0]) == 3
        assert len(result[1]) == 3

    def test_x4_plus_1_coefficients_correct(self):
        """Verify exact coefficients of x⁴+1 factored over Q[√2].

        Expected:
          h1 = x² + √2x + 1 → [(1,0), (0,1), (1,0)]
          h2 = x² − √2x + 1 → [(1,0), (0,-1), (1,0)]
        """
        result = factor_over_extension([1, 0, 0, 0, 1], 2)
        assert result is not None
        assert len(result) == 2

        h1, h2 = result
        # Constant term: 1 + 0·√2
        assert h1[0] == (Fraction(1), Fraction(0))
        assert h2[0] == (Fraction(1), Fraction(0))
        # x-coefficient: 0 + (±1)·√2
        # One factor has +1, the other -1
        radicals = {h1[1][1], h2[1][1]}
        assert radicals == {Fraction(1), Fraction(-1)}
        # x² coefficient: 1 + 0·√2
        assert h1[2] == (Fraction(1), Fraction(0))
        assert h2[2] == (Fraction(1), Fraction(0))

    def test_x4_plus_1_over_sqrt3_no_split(self):
        """x⁴ + 1 does NOT split over Q[√3].

        For x⁴+1 (p=0, q=1), s=1, r² = (2-0)/3 = 2/3, not a perfect square.
        """
        result = factor_over_extension([1, 0, 0, 0, 1], 3)
        assert result is None

    def test_x4_minus_5x2_plus_5_does_not_split(self):
        """x⁴ − 5x² + 5 has irrational s, so Pattern 1 doesn't apply."""
        # q = 5, not a perfect square, so s = √5 which is not rational.
        # Therefore no algebraic split of this form exists.
        result = factor_over_extension([5, 0, -5, 0, 1], 5)
        assert result is None

    # --- Already-reducible polynomials ---

    def test_x4_minus_1_reducible_over_z(self):
        """x⁴ − 1 = (x−1)(x+1)(x²+1) over Z; no new split over Q[√2].

        The factor (x²+1) has discriminant −4, which is not ≥0, so no
        quadratic splitting over Q[√2] either.
        """
        # x⁴ − 1 = (x²−1)(x²+1) → (x−1)(x+1)(x²+1) over Z.
        # x²+1 doesn't split over Q[√2] (disc = −4).
        result = factor_over_extension([-1, 0, 0, 0, 1], 2)
        assert result is None

    def test_x5_minus_1_no_new_split(self):
        """x⁵ − 1; cyclotomic factor x⁴+x³+x²+x+1 is not depressed → None."""
        # x⁵ − 1; the degree-4 factor x⁴+x³+x²+x+1 has non-zero x³ and x¹
        # coefficients, so Pattern 1 cannot apply.  Returns None.
        result = factor_over_extension([-1, 0, 0, 0, 0, 1], 2)
        assert result is None

    # --- Additional cases ---

    def test_x2_minus_2_coefficients(self):
        """Verify exact factor coefficients for x²−2 over Q[√2].

        Root is √2, so factors are (x − √2)(x + √2).
        h1 = (x − √2): constant term is 0 + (−1)·√2, linear term is 1.
        h2 = (x + √2): constant term is 0 + 1·√2, linear term is 1.
        """
        result = factor_over_extension([-2, 0, 1], 2)
        assert result is not None
        assert len(result) == 2

        constants = {result[0][0][1], result[1][0][1]}
        assert constants == {Fraction(1), Fraction(-1)}
        # Both linear terms should be purely rational (coefficient 1).
        assert result[0][1] == (Fraction(1), Fraction(0))
        assert result[1][1] == (Fraction(1), Fraction(0))

    def test_quadratic_split_over_sqrt5(self):
        """x² − 5 = (x−√5)(x+√5) over Q[√5]."""
        result = factor_over_extension([-5, 0, 1], 5)
        assert result is not None
        assert len(result) == 2

    def test_non_monic_quadratic_handled(self):
        """2x² − 4 = 2(x²−2) → (x−√2)(x+√2) with content stripped.

        After stripping content 2, the primitive part is x²−2 which
        splits. Our function returns factors or None based on the full
        analysis; confirm it doesn't crash.
        """
        # 2(x²-2): content=2, primitive part=x²-2, which splits.
        result = factor_over_extension([-4, 0, 2], 2)
        # Split found (content is factored out by factor_integer_polynomial).
        assert result is not None


# ===========================================================================
# Section 3: _try_split_quadratic direct tests
# ===========================================================================


class TestTrySplitQuadratic:
    """Direct tests of the quadratic splitter."""

    def test_x2_minus_d(self):
        """[−d, 0, 1] always splits over Q[√d]."""
        for d in [2, 3, 5, 7]:
            result = _try_split_quadratic([-d, 0, 1], d)
            assert result is not None, f"x²−{d} should split over Q[√{d}]"
            assert len(result) == 2

    def test_non_monic_returns_none(self):
        """Non-monic quadratic → None (we require monic input)."""
        # [1, 0, 2] is 2x²+1, not monic
        result = _try_split_quadratic([1, 0, 2], 2)
        assert result is None

    def test_wrong_degree_returns_none(self):
        """Wrong degree → None."""
        result = _try_split_quadratic([1, 0, 0, 1], 2)
        assert result is None

    def test_disc_not_square_returns_none(self):
        """If disc/d is not a rational square, return None."""
        # x² + x + 1: disc = 1 − 4 = −3 < 0 → None
        result = _try_split_quadratic([1, 1, 1], 2)
        assert result is None


# ===========================================================================
# Section 4: _try_split_depressed_quartic direct tests
# ===========================================================================


class TestTrySplitDepressedQuartic:
    """Direct tests of the quartic splitter."""

    def test_x4_plus_1_over_sqrt2(self):
        result = _try_split_depressed_quartic([1, 0, 0, 0, 1], 2)
        assert result is not None
        assert len(result) == 2

    def test_wrong_length_returns_none(self):
        """Not a degree-4 polynomial."""
        assert _try_split_depressed_quartic([1, 0, 1], 2) is None

    def test_not_monic_returns_none(self):
        """Non-monic quartic → None."""
        assert _try_split_depressed_quartic([1, 0, 0, 0, 2], 2) is None

    def test_not_depressed_returns_none(self):
        """Quartic with x³ term → None."""
        assert _try_split_depressed_quartic([1, 0, 0, 1, 1], 2) is None

    def test_q_not_perfect_square_returns_none(self):
        """q = 2 is not a perfect square → None."""
        assert _try_split_depressed_quartic([2, 0, 0, 0, 1], 2) is None


# ===========================================================================
# Section 5: _extract_d tests
# ===========================================================================


class TestExtractD:
    """Tests for the Sqrt(d) extractor."""

    def test_sqrt_integer(self):
        assert _extract_d(_sqrt(2)) == 2
        assert _extract_d(_sqrt(5)) == 5
        assert _extract_d(_sqrt(3)) == 3

    def test_not_sqrt_returns_none(self):
        assert _extract_d(IRInteger(2)) is None
        assert _extract_d(IRSymbol("x")) is None

    def test_sqrt_symbol_returns_none(self):
        """Sqrt(x) is not an integer extension."""
        sqrt_x = IRApply(SQRT, (IRSymbol("x"),))
        assert _extract_d(sqrt_x) is None

    def test_sqrt_perfect_square_returns_none(self):
        """√4 = 2 ∈ Q, so the extension Q[√4] = Q is trivial."""
        assert _extract_d(_sqrt(4)) is None
        assert _extract_d(_sqrt(9)) is None
        assert _extract_d(_sqrt(1)) is None

    def test_sqrt_negative_returns_none(self):
        """Negative d has no real extension."""
        assert _extract_d(_sqrt(-2)) is None

    def test_sqrt_wrong_arity_returns_none(self):
        """Sqrt with 0 or 2 args → None."""
        assert _extract_d(IRApply(SQRT, ())) is None
        assert _extract_d(IRApply(SQRT, (IRInteger(2), IRInteger(3)))) is None


# ===========================================================================
# Section 6: build_alg_factor_handler_table
# ===========================================================================


def test_build_alg_factor_handler_table_returns_dict():
    """The table must have 'AlgFactor' key."""
    table = build_alg_factor_handler_table()
    assert isinstance(table, dict)
    assert "AlgFactor" in table
    assert callable(table["AlgFactor"])


# ===========================================================================
# Section 7: alg_factor_handler — VM integration
# ===========================================================================


class TestAlgFactorHandler:
    """Integration tests for the AlgFactor VM handler."""

    def test_wrong_arity_returns_unevaluated(self):
        """AlgFactor with 1 or 3 args returns unevaluated."""
        vm = _make_vm()
        # 1 arg
        expr1 = IRApply(IRSymbol("AlgFactor"), (IRInteger(1),))
        assert vm.eval(expr1) == expr1
        # 3 args
        expr3 = IRApply(
            IRSymbol("AlgFactor"),
            (IRInteger(1), _sqrt(2), IRInteger(3)),
        )
        assert vm.eval(expr3) == expr3

    def test_non_sqrt_second_arg_unevaluated(self):
        """Second arg must be Sqrt(n)."""
        vm = _make_vm()
        poly_ir = IRApply(ADD, (IRApply(MUL, (x, x)), IRInteger(1)))
        expr = IRApply(IRSymbol("AlgFactor"), (poly_ir, IRInteger(2)))
        assert vm.eval(expr) == expr

    def test_no_variable_returns_inner(self):
        """Numeric-only polynomial returns unevaluated (no variable)."""
        vm = _make_vm()
        expr = IRApply(IRSymbol("AlgFactor"), (IRInteger(5), _sqrt(2)))
        # _find_variable returns None; handler returns expr unchanged.
        result = vm.eval(expr)
        assert result == expr

    def test_x4_plus_1_over_sqrt2_evaluates(self):
        """AlgFactor(x^4+1, Sqrt(2)) should produce a Mul of two factors."""
        vm = _make_vm()
        # Build x^4 + 1 IR.
        x4 = IRApply(MUL, (IRApply(MUL, (IRApply(MUL, (x, x)), x)), x))
        poly_ir = IRApply(ADD, (x4, IRInteger(1)))
        expr = _alg_factor_expr(poly_ir, 2)
        result = vm.eval(expr)
        # Result should be a Mul node (two factors).
        assert isinstance(result, IRApply)
        assert result.head == MUL
        assert len(result.args) == 2

    def test_irreducible_returns_unevaluated(self):
        """x^2 + 1 is irreducible over Q[√2] → unevaluated AlgFactor."""
        vm = _make_vm()
        # x^2 + 1
        poly_ir = IRApply(
            ADD, (IRApply(MUL, (x, x)), IRInteger(1))
        )
        expr = _alg_factor_expr(poly_ir, 2)
        result = vm.eval(expr)
        assert isinstance(result, IRApply)
        assert result.head == IRSymbol("AlgFactor")

    def test_perfect_square_d_returns_unevaluated(self):
        """Sqrt(4) is rational; AlgFactor returns unevaluated."""
        vm = _make_vm()
        poly_ir = IRApply(ADD, (IRApply(MUL, (x, x)), IRInteger(1)))
        expr = IRApply(IRSymbol("AlgFactor"), (poly_ir, _sqrt(4)))
        result = vm.eval(expr)
        assert result == expr

    def test_non_polynomial_input_unevaluated(self):
        """If the poly_ir is not a polynomial (contains non-poly head) → unevaluated."""
        vm = _make_vm()
        # Sin(x) is not a polynomial — _ir_to_poly_coeffs returns None.
        sin_x = IRApply(IRSymbol("Sin"), (x,))
        expr = IRApply(IRSymbol("AlgFactor"), (sin_x, _sqrt(2)))
        result = vm.eval(expr)
        assert result == expr

    def test_pow_polynomial_evaluates(self):
        """AlgFactor(x^2 - 2, Sqrt(2)) uses Pow node → two linear factors."""
        vm = _make_vm()
        # x^2 - 2 using Pow IR: Sub(Pow(x, 2), 2)
        poly_ir = IRApply(SUB, (IRApply(POW, (x, IRInteger(2))), IRInteger(2)))
        expr = _alg_factor_expr(poly_ir, 2)
        result = vm.eval(expr)
        # Should factor as (x - sqrt(2))(x + sqrt(2)).
        assert isinstance(result, IRApply)
        assert result.head == MUL

    def test_neg_polynomial_evaluates(self):
        """AlgFactor(Neg(x^2+1), Sqrt(2)) — Neg node handled by _ir_to_poly_coeffs."""
        vm = _make_vm()
        # Neg(x^2 + 1) — irreducible over Q[√2], so returns unevaluated.
        pos_poly = IRApply(ADD, (IRApply(MUL, (x, x)), IRInteger(1)))
        neg_poly = IRApply(NEG, (pos_poly,))
        expr = _alg_factor_expr(neg_poly, 2)
        result = vm.eval(expr)
        # x^2+1 is irreducible; Neg(...) is also irreducible.
        assert isinstance(result, IRApply)
        assert result.head == IRSymbol("AlgFactor")


# ===========================================================================
# Section 8: _poly_add, _poly_mul — polynomial arithmetic helpers
# ===========================================================================


class TestPolyHelpers:
    """Unit tests for the polynomial arithmetic helpers."""

    def test_poly_add_equal_length(self):
        """Add two polynomials of the same degree."""
        a = [Fraction(1), Fraction(2)]   # 1 + 2x
        b = [Fraction(3), Fraction(4)]   # 3 + 4x
        assert _poly_add(a, b) == [Fraction(4), Fraction(6)]  # 4 + 6x

    def test_poly_add_unequal_length(self):
        """Shorter polynomial is zero-padded."""
        a = [Fraction(1), Fraction(2), Fraction(3)]  # 1 + 2x + 3x^2
        b = [Fraction(10)]                            # 10
        result = _poly_add(a, b)
        assert result == [Fraction(11), Fraction(2), Fraction(3)]

    def test_poly_mul_constants(self):
        """Multiplying two constants gives a constant."""
        assert _poly_mul([Fraction(3)], [Fraction(4)]) == [Fraction(12)]

    def test_poly_mul_degree_1(self):
        """(1 + x)(1 - x) = 1 - x^2."""
        a = [Fraction(1), Fraction(1)]   # 1 + x
        b = [Fraction(1), Fraction(-1)]  # 1 - x
        result = _poly_mul(a, b)
        assert result == [Fraction(1), Fraction(0), Fraction(-1)]

    def test_poly_mul_empty_returns_zero(self):
        """Multiplying by empty list returns [0]."""
        assert _poly_mul([], [Fraction(2)]) == [Fraction(0)]


# ===========================================================================
# Section 9: _ir_to_poly_coeffs — IR → polynomial coefficient extraction
# ===========================================================================


class TestIrToPolyCoeffs:
    """Unit tests for the _ir_to_poly_coeffs IR → polynomial converter."""

    def test_integer_constant(self):
        """IRInteger maps to a constant polynomial."""
        assert _ir_to_poly_coeffs(IRInteger(5), x) == [Fraction(5)]

    def test_rational_constant(self):
        """IRRational maps to a constant polynomial with Fraction value."""
        node = IRRational(1, 2)
        result = _ir_to_poly_coeffs(node, x)
        assert result == [Fraction(1, 2)]

    def test_variable_x(self):
        """IRSymbol(x) → [0, 1] (the polynomial x)."""
        assert _ir_to_poly_coeffs(x, x) == [Fraction(0), Fraction(1)]

    def test_other_symbol_returns_none(self):
        """A symbol that is not x → None (unknown variable)."""
        y = IRSymbol("y")
        assert _ir_to_poly_coeffs(y, x) is None

    def test_sub_polynomial(self):
        """Sub(x, 1) → x - 1 = [-1, 1]."""
        node = IRApply(SUB, (x, IRInteger(1)))
        assert _ir_to_poly_coeffs(node, x) == [Fraction(-1), Fraction(1)]

    def test_pow_polynomial(self):
        """Pow(x, 2) → x^2 = [0, 0, 1]."""
        node = IRApply(POW, (x, IRInteger(2)))
        assert _ir_to_poly_coeffs(node, x) == [Fraction(0), Fraction(0), Fraction(1)]

    def test_pow_zero_exponent(self):
        """Pow(x, 0) → 1 = [1]."""
        node = IRApply(POW, (x, IRInteger(0)))
        assert _ir_to_poly_coeffs(node, x) == [Fraction(1)]

    def test_pow_negative_exponent_returns_none(self):
        """Pow(x, -1) is a rational function → None."""
        node = IRApply(POW, (x, IRInteger(-1)))
        assert _ir_to_poly_coeffs(node, x) is None

    def test_neg_polynomial(self):
        """Neg(x + 1) → -x - 1 = [-1, -1]."""
        plus = IRApply(ADD, (x, IRInteger(1)))
        node = IRApply(NEG, (plus,))
        assert _ir_to_poly_coeffs(node, x) == [Fraction(-1), Fraction(-1)]

    def test_unknown_head_returns_none(self):
        """An IR head not handled (e.g. Sin) → None."""
        node = IRApply(IRSymbol("Sin"), (x,))
        assert _ir_to_poly_coeffs(node, x) is None

    def test_add_with_none_subexpr_returns_none(self):
        """If either sub-expression of Add is not a polynomial → None."""
        sin_x = IRApply(IRSymbol("Sin"), (x,))
        node = IRApply(ADD, (sin_x, IRInteger(1)))
        assert _ir_to_poly_coeffs(node, x) is None

    def test_mul_with_none_subexpr_returns_none(self):
        """Mul where one factor is non-polynomial → None."""
        y = IRSymbol("y")
        node = IRApply(MUL, (x, y))  # x*y — two variables
        assert _ir_to_poly_coeffs(node, x) is None

    def test_pow_with_non_integer_exponent_returns_none(self):
        """Pow(x, x) — symbolic exponent → None."""
        node = IRApply(POW, (x, x))
        assert _ir_to_poly_coeffs(node, x) is None

    def test_pow_with_none_base_returns_none(self):
        """Pow(Sin(x), 2) — non-polynomial base → None."""
        sin_x = IRApply(IRSymbol("Sin"), (x,))
        node = IRApply(POW, (sin_x, IRInteger(2)))
        assert _ir_to_poly_coeffs(node, x) is None


# ===========================================================================
# Section 10: _alg_coeff_to_ir — algebraic coefficient → IR
# ===========================================================================


class TestAlgCoeffToIr:
    """Unit tests for _alg_coeff_to_ir edge cases not covered by main tests."""

    def test_fractional_rational_coefficient(self):
        """rational=1/2, radical=0 → IRRational(1, 2)."""
        sqrt_ir = IRApply(SQRT, (IRInteger(2),))
        result = _alg_coeff_to_ir(Fraction(1, 2), Fraction(0), sqrt_ir)
        assert isinstance(result, IRRational)
        assert result.numer == 1
        assert result.denom == 2

    def test_non_unit_radical_coefficient(self):
        """rational=0, radical=3/2 → Mul(3/2, Sqrt(2))."""
        sqrt_ir = IRApply(SQRT, (IRInteger(2),))
        result = _alg_coeff_to_ir(Fraction(0), Fraction(3, 2), sqrt_ir)
        # Should be Mul(IRRational(3,2), sqrt_ir)
        assert isinstance(result, IRApply)
        assert result.head == MUL

    def test_negative_unit_radical(self):
        """rational=0, radical=-1 → Neg(Sqrt(2))."""
        sqrt_ir = IRApply(SQRT, (IRInteger(2),))
        result = _alg_coeff_to_ir(Fraction(0), Fraction(-1), sqrt_ir)
        assert isinstance(result, IRApply)
        assert result.head == IRSymbol("Neg")

    def test_both_rational_and_radical(self):
        """rational=1, radical=1 → Add(1, Sqrt(2))."""
        sqrt_ir = IRApply(SQRT, (IRInteger(2),))
        result = _alg_coeff_to_ir(Fraction(1), Fraction(1), sqrt_ir)
        assert isinstance(result, IRApply)
        assert result.head == ADD
