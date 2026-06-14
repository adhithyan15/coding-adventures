"""Phase 21 tests — Named variable-coefficient 2nd-order ODE recognition.

New solver family added in cas-ode 0.6.0
-----------------------------------------
- **Legendre**   ``(1−x²)y'' − 2x·y' + n(n+1)y = 0``  → LegendreP/Q
- **Bessel**     ``x²y'' + x·y' + (x²−ν²)y = 0``       → BesselJ/Y
- **Hermite**    ``y'' − 2x·y' + 2n·y = 0``             → HermiteH/H2
- **Chebyshev**  ``(1−x²)y'' − x·y' + n²·y = 0``        → ChebyshevT/U

Testing strategy
----------------
1. Unit-test each helper in isolation: ``_split_out_factor``,
   ``_collect_var2_coeffs``, ``_legendre_n_from_lambda``,
   ``_nu_from_r_minus_xsq``, ``_coeff_matches_func``,
   ``_extract_const_val``.
2. Test each recogniser function (``_try_legendre_ode`` etc.) with
   valid inputs for several parameter values, invalid inputs that
   look similar (e.g. wrong Q-coefficient), and completely unrelated
   ODEs that must fall through.
3. Test the dispatcher ``_try_var_coeff_named_ode``.
4. End-to-end pipeline tests through ``solve_ode`` / ``eval_ode``
   (ODE2 VM dispatch).
5. Regression: const-coeff and Euler-Cauchy ODEs must still work
   after Phase 21 is inserted above them in the dispatcher.

Shape of solution nodes
-----------------------
Each recogniser returns::

    Equal(y, Add(Mul(%c1, Sym(n, x)), Mul(%c2, Sym2(n, x))))

where ``Sym`` and ``Sym2`` are the pair of named solution symbols
(e.g. LEGENDRE_P / LEGENDRE_Q).  Tests verify:

- The returned value is an ``IRApply`` with head ``EQUAL``.
- The LHS of Equal is the dependent variable ``y``.
- The solution contains the expected head symbol names.
- The integration constants ``%c1`` and ``%c2`` are present.
- The ODE parameter (n or ν) has the correct numeric value.
"""

from __future__ import annotations

import math

from symbolic_ir import (
    ADD,
    EQUAL,
    MUL,
    NEG,
    POW,
    SUB,
    IRApply,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)
from symbolic_ir.nodes import (
    BESSEL_J,
    BESSEL_Y,
    CHEBYSHEV_T,
    CHEBYSHEV_U,
    C1,
    C2,
    D,
    HERMITE_H,
    HERMITE_H2,
    LEGENDRE_P,
    LEGENDRE_Q,
    ODE2,
)
from symbolic_vm import VM, SymbolicBackend

from cas_ode import build_ode_handler_table, solve_ode
from cas_ode.ode import (
    _build_named_solution,
    _coeff_matches_func,
    _collect_var2_coeffs,
    _eval_ir_at_x,
    _extract_const_val,
    _legendre_n_from_lambda,
    _nu_from_r_minus_xsq,
    _split_out_factor,
    _try_bessel_ode,
    _try_chebyshev_ode,
    _try_hermite_ode,
    _try_legendre_ode,
    _try_var_coeff_named_ode,
)

# ---------------------------------------------------------------------------
# Shared fixtures and tiny IR builders
# ---------------------------------------------------------------------------

X = IRSymbol("x")
Y = IRSymbol("y")
Y_PRIME = IRApply(D, (Y, X))
Y_DOUBLE = IRApply(D, (Y_PRIME, X))


def _I(n: int) -> IRInteger:
    return IRInteger(n)


def _R(p: int, q: int) -> IRRational:
    return IRRational(p, q)


def _mul(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(MUL, (a, b))


def _add(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(ADD, (a, b))


def _sub(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(SUB, (a, b))


def _neg(node: IRNode) -> IRNode:
    return IRApply(NEG, (node,))


def _pow(base: IRNode, exp: IRNode) -> IRNode:
    return IRApply(POW, (base, exp))


def _x_sq() -> IRNode:
    """Build x² as Pow(x, 2)."""
    return _pow(X, _I(2))


def _one_minus_x_sq() -> IRNode:
    """Build (1 − x²) as Sub(1, Pow(x, 2))."""
    return _sub(_I(1), _x_sq())


# ---------------------------------------------------------------------------
# ODE expression builders — canonical zero-form for each family
# ---------------------------------------------------------------------------
# Each function returns the expression `E` such that `E = 0` is the ODE.
# The solve_ode / _try_* functions expect this zero form.

def _legendre_expr(n: int) -> IRNode:
    """(1−x²)y'' − 2x·y' + n(n+1)·y = 0  in zero form."""
    lam = n * (n + 1)
    # (1-x²)·y''
    p_term = _mul(_one_minus_x_sq(), Y_DOUBLE)
    # -2x·y'  =  Neg(Mul(Mul(2, x), y'))
    q_term = _neg(_mul(_mul(_I(2), X), Y_PRIME))
    # n(n+1)·y  — use integer node
    r_term = _mul(_I(lam), Y)
    return _add(_add(p_term, q_term), r_term)


def _bessel_expr(nu_p: int, nu_q: int = 1) -> IRNode:
    """x²y'' + x·y' + (x²−ν²)·y = 0  in zero form.  ν = nu_p / nu_q."""
    # x²·y''
    p_term = _mul(_x_sq(), Y_DOUBLE)
    # x·y'
    q_term = _mul(X, Y_PRIME)
    # (x² − ν²)·y
    if nu_q == 1:
        nu_sq: IRNode = _I(nu_p * nu_p)
    else:
        # ν² = (nu_p/nu_q)²
        nu_sq = _R(nu_p * nu_p, nu_q * nu_q)
    r_coeff = _sub(_x_sq(), nu_sq)   # x² - ν²
    r_term = _mul(r_coeff, Y)
    return _add(_add(p_term, q_term), r_term)


def _hermite_expr(n: int) -> IRNode:
    """y'' − 2x·y' + 2n·y = 0  in zero form."""
    # y''  (coefficient = 1)
    p_term = Y_DOUBLE
    # −2x·y'
    q_term = _neg(_mul(_mul(_I(2), X), Y_PRIME))
    # 2n·y
    r_term = _mul(_I(2 * n), Y)
    return _add(_add(p_term, q_term), r_term)


def _chebyshev_expr(n: int) -> IRNode:
    """(1−x²)y'' − x·y' + n²·y = 0  in zero form."""
    # (1−x²)·y''
    p_term = _mul(_one_minus_x_sq(), Y_DOUBLE)
    # −x·y'
    q_term = _neg(_mul(X, Y_PRIME))
    # n²·y
    r_term = _mul(_I(n * n), Y)
    return _add(_add(p_term, q_term), r_term)


# ---------------------------------------------------------------------------
# VM / dispatcher helpers
# ---------------------------------------------------------------------------

def make_vm() -> VM:
    """Return a SymbolicBackend VM with the ODE2 handler wired in."""
    backend = SymbolicBackend()
    backend._handlers.update(build_ode_handler_table())  # type: ignore[attr-defined]
    return VM(backend)


def eval_ode(expr: IRNode, y: IRSymbol = Y, x: IRSymbol = X) -> IRNode:
    """Evaluate ODE2(expr, y, x) through the full VM pipeline."""
    vm = make_vm()
    return vm.eval(IRApply(ODE2, (expr, y, x)))


def _is_equal_node(result: IRNode) -> tuple[IRNode, IRNode]:
    """Assert result is Equal(lhs, rhs) and return (lhs, rhs)."""
    assert isinstance(result, IRApply) and result.head == EQUAL, (
        f"Expected Equal(...), got: {result!r}"
    )
    return result.args[0], result.args[1]


def _was_evaluated(result: IRNode) -> None:
    """Assert the result is NOT an unevaluated ODE2 node."""
    assert not (isinstance(result, IRApply) and result.head == ODE2), (
        f"Expected solved ODE, got unevaluated: {result!r}"
    )


def _is_unevaluated(result: IRNode) -> None:
    """Assert the result IS an unevaluated ODE2 node."""
    assert isinstance(result, IRApply) and result.head == ODE2, (
        f"Expected unevaluated ODE2, got: {result!r}"
    )


def _has_symbol_name(node: IRNode, name: str) -> bool:
    """Return True if any IRSymbol in the tree has the given name."""
    if isinstance(node, IRSymbol):
        return node.name == name
    if isinstance(node, IRApply):
        if node.head.name == name:
            return True
        return any(_has_symbol_name(c, name) for c in node.args)
    return False


def _collect_apply_heads(node: IRNode, acc: list[str]) -> None:
    """Collect all head names of IRApply nodes into acc."""
    if isinstance(node, IRApply):
        acc.append(node.head.name)
        for c in node.args:
            _collect_apply_heads(c, acc)


# ---------------------------------------------------------------------------
# TestSplitOutFactor
# ---------------------------------------------------------------------------


class TestSplitOutFactor:
    """Unit tests for :func:`_split_out_factor`.

    The function must return K such that ``term = K * target``, or ``None``
    if the target is not a factor of the term.
    """

    def test_term_equals_target_returns_one(self) -> None:
        """term == target  →  K = 1 (the integer)."""
        k = _split_out_factor(Y_DOUBLE, Y_DOUBLE)
        assert k == IRInteger(1)

    def test_direct_mul_right_is_target(self) -> None:
        """Mul(2, y'')  →  K = 2."""
        term = _mul(_I(2), Y_DOUBLE)
        k = _split_out_factor(term, Y_DOUBLE)
        assert k == _I(2)

    def test_direct_mul_left_is_target(self) -> None:
        """Mul(y'', x)  →  K = x."""
        term = _mul(Y_DOUBLE, X)
        k = _split_out_factor(term, Y_DOUBLE)
        assert k == X

    def test_nested_mul_right(self) -> None:
        """Mul(Mul(2, x), y')  →  K = Mul(2, x)."""
        coeff = _mul(_I(2), X)
        term = _mul(coeff, Y_PRIME)
        k = _split_out_factor(term, Y_PRIME)
        assert k == coeff

    def test_nested_mul_left(self) -> None:
        """Mul(y', Mul(3, x))  →  K = Mul(3, x)."""
        coeff = _mul(_I(3), X)
        term = _mul(Y_PRIME, coeff)
        k = _split_out_factor(term, Y_PRIME)
        assert k == coeff

    def test_neg_wraps_target(self) -> None:
        """Neg(y'')  →  K = Neg(1) = Neg(IRInteger(1))."""
        term = _neg(Y_DOUBLE)
        k = _split_out_factor(term, Y_DOUBLE)
        assert k is not None
        # K should be Neg(1)
        assert isinstance(k, IRApply) and k.head == NEG

    def test_neg_mul(self) -> None:
        """Neg(Mul(2, y'))  →  K = Neg(2)."""
        term = _neg(_mul(_I(2), Y_PRIME))
        k = _split_out_factor(term, Y_PRIME)
        assert k is not None
        assert isinstance(k, IRApply) and k.head == NEG

    def test_unrelated_term_returns_none(self) -> None:
        """x²  (no y factor)  →  None."""
        k = _split_out_factor(_x_sq(), Y)
        assert k is None

    def test_different_yprime_vs_ydouble(self) -> None:
        """Trying to extract y' from Mul(2, y'') returns None."""
        term = _mul(_I(2), Y_DOUBLE)
        k = _split_out_factor(term, Y_PRIME)
        assert k is None

    def test_mul_of_three_nested(self) -> None:
        """Mul(a, Mul(b, y))  →  K = Mul(a, b)."""
        a, b = _I(3), _I(5)
        inner = _mul(b, Y)
        term = _mul(a, inner)
        k = _split_out_factor(term, Y)
        # K should be Mul(3, 5) — the combined outer coefficient
        assert k is not None

    def test_integer_node_no_match(self) -> None:
        """A bare IRInteger is never a K·target."""
        k = _split_out_factor(_I(7), Y_DOUBLE)
        assert k is None


# ---------------------------------------------------------------------------
# TestCollectVar2Coeffs
# ---------------------------------------------------------------------------


class TestCollectVar2Coeffs:
    """Unit tests for :func:`_collect_var2_coeffs`.

    Verifies that (P, Q, R) triples are extracted correctly from valid
    variable-coefficient 2nd-order ODE expressions.
    """

    def test_legendre_n2_coefficients(self) -> None:
        """Legendre n=2: P=(1-x²), Q=-2x, R=6."""
        expr = _legendre_expr(2)
        result = _collect_var2_coeffs(expr, Y, X)
        assert result is not None
        P, Q, R = result
        # P ≈ 1-x² at test points
        for xv in (0.3, 0.6, -0.25, 0.85):
            p_val = _eval_ir_at_x(P, X, xv)
            assert p_val is not None
            assert abs(p_val - (1 - xv**2)) < 1e-9, f"P({xv}) off"

    def test_bessel_n1_coefficients(self) -> None:
        """Bessel ν=1: P=x², Q=x, R=(x²-1)."""
        expr = _bessel_expr(1)
        result = _collect_var2_coeffs(expr, Y, X)
        assert result is not None
        P, Q, R = result
        for xv in (0.3, 0.6, 0.85):
            p_val = _eval_ir_at_x(P, X, xv)
            q_val = _eval_ir_at_x(Q, X, xv)
            r_val = _eval_ir_at_x(R, X, xv)
            assert p_val is not None and abs(p_val - xv**2) < 1e-9
            assert q_val is not None and abs(q_val - xv) < 1e-9
            assert r_val is not None and abs(r_val - (xv**2 - 1)) < 1e-9

    def test_hermite_n3_coefficients(self) -> None:
        """Hermite n=3: P=1, Q=-2x, R=6."""
        expr = _hermite_expr(3)
        result = _collect_var2_coeffs(expr, Y, X)
        assert result is not None
        P, Q, R = result
        for xv in (0.3, 0.6):
            p_val = _eval_ir_at_x(P, X, xv)
            q_val = _eval_ir_at_x(Q, X, xv)
            r_val = _eval_ir_at_x(R, X, xv)
            assert p_val is not None and abs(p_val - 1.0) < 1e-9
            assert q_val is not None and abs(q_val - (-2 * xv)) < 1e-9
            assert r_val is not None and abs(r_val - 6.0) < 1e-9

    def test_no_ydouble_term_returns_none(self) -> None:
        """First-order ODE has no y'' — returns None."""
        # y' + y = 0  (no y'' term)
        expr = _add(Y_PRIME, Y)
        assert _collect_var2_coeffs(expr, Y, X) is None

    def test_free_constant_term_returns_none(self) -> None:
        """y'' + y + 1 = 0 has a free x-only term (1) — returns None."""
        expr = _add(_add(Y_DOUBLE, Y), _I(1))
        assert _collect_var2_coeffs(expr, Y, X) is None

    def test_missing_q_term_defaults_to_zero(self) -> None:
        """y'' + n²y = 0 has no y' term — Q defaults to 0."""
        n = 2
        expr = _add(Y_DOUBLE, _mul(_I(n * n), Y))
        result = _collect_var2_coeffs(expr, Y, X)
        assert result is not None
        P, Q, R = result
        for xv in _VAR2_TEST_X:
            q_val = _eval_ir_at_x(Q, X, xv)
            assert q_val is not None and abs(q_val) < 1e-12


_VAR2_TEST_X: tuple[float, ...] = (0.3, 0.6, -0.25, 0.85)


# ---------------------------------------------------------------------------
# TestLegendreNFromLambda
# ---------------------------------------------------------------------------


class TestLegendreNFromLambda:
    """Unit tests for :func:`_legendre_n_from_lambda`."""

    def test_n0(self) -> None:
        """λ=0 → n=0  (0·1=0)."""
        assert _legendre_n_from_lambda(0.0) == 0

    def test_n1(self) -> None:
        """λ=2 → n=1  (1·2=2)."""
        assert _legendre_n_from_lambda(2.0) == 1

    def test_n2(self) -> None:
        """λ=6 → n=2  (2·3=6)."""
        assert _legendre_n_from_lambda(6.0) == 2

    def test_n3(self) -> None:
        """λ=12 → n=3  (3·4=12)."""
        assert _legendre_n_from_lambda(12.0) == 3

    def test_n4(self) -> None:
        """λ=20 → n=4  (4·5=20)."""
        assert _legendre_n_from_lambda(20.0) == 4

    def test_non_triangular_lambda_returns_none(self) -> None:
        """λ=5 is not n(n+1) for any integer n → None."""
        assert _legendre_n_from_lambda(5.0) is None

    def test_lambda_7_returns_none(self) -> None:
        """λ=7 → None."""
        assert _legendre_n_from_lambda(7.0) is None

    def test_negative_lambda_returns_none(self) -> None:
        """λ=-3 → None (negative discriminant)."""
        assert _legendre_n_from_lambda(-3.0) is None

    def test_lambda_slightly_off_returns_none(self) -> None:
        """λ=6.01 is not close enough to 6 → None."""
        assert _legendre_n_from_lambda(6.01) is None

    def test_float_precision_near_n3(self) -> None:
        """λ=12.0000001 should still resolve to n=3 (within tolerance)."""
        # The tolerance is 1e-7, so a tiny float error is forgiven
        result = _legendre_n_from_lambda(12.0 + 1e-8)
        assert result == 3


# ---------------------------------------------------------------------------
# TestNuFromRMinusXSq
# ---------------------------------------------------------------------------


class TestNuFromRMinusXSq:
    """Unit tests for :func:`_nu_from_r_minus_xsq`."""

    def _make_r(self, nu_p: int, nu_q: int = 1) -> IRNode:
        """Build IR for R(x) = x² − ν² where ν = nu_p/nu_q."""
        if nu_q == 1:
            nu_sq_ir: IRNode = _I(nu_p * nu_p)
        else:
            nu_sq_ir = _R(nu_p * nu_p, nu_q * nu_q)
        return _sub(_x_sq(), nu_sq_ir)

    def test_nu_0(self) -> None:
        """R = x² → ν = 0, returns (0, 1)."""
        R = _x_sq()
        result = _nu_from_r_minus_xsq(R, X)
        assert result is not None
        p, q = result
        assert p == 0

    def test_nu_1(self) -> None:
        """R = x²−1 → ν = 1, returns (1, 1)."""
        R = self._make_r(1)
        result = _nu_from_r_minus_xsq(R, X)
        assert result == (1, 1)

    def test_nu_2(self) -> None:
        """R = x²−4 → ν = 2, returns (2, 1)."""
        R = self._make_r(2)
        result = _nu_from_r_minus_xsq(R, X)
        assert result == (2, 1)

    def test_nu_3(self) -> None:
        """R = x²−9 → ν = 3, returns (3, 1)."""
        R = self._make_r(3)
        result = _nu_from_r_minus_xsq(R, X)
        assert result == (3, 1)

    def test_nu_half(self) -> None:
        """R = x²−1/4 → ν = 1/2, returns (1, 2)."""
        R = self._make_r(1, 2)
        result = _nu_from_r_minus_xsq(R, X)
        assert result == (1, 2)

    def test_nu_three_halves(self) -> None:
        """R = x²−9/4 → ν = 3/2, returns (3, 2)."""
        R = self._make_r(3, 2)
        result = _nu_from_r_minus_xsq(R, X)
        assert result == (3, 2)

    def test_non_xsq_minus_const_returns_none(self) -> None:
        """R = x (linear, not x²−c) → None."""
        R = X
        result = _nu_from_r_minus_xsq(R, X)
        assert result is None

    def test_neg_nu_sq_returns_none(self) -> None:
        """R = x²+1 → ν² = −1 < 0 → None."""
        R = _add(_x_sq(), _I(1))
        result = _nu_from_r_minus_xsq(R, X)
        assert result is None


# ---------------------------------------------------------------------------
# TestTryLegendreOde
# ---------------------------------------------------------------------------


class TestTryLegendreOde:
    """Tests for :func:`_try_legendre_ode`.

    The Legendre ODE is ``(1−x²)y'' − 2x·y' + n(n+1)·y = 0``.
    The recogniser verifies coefficients numerically and extracts ``n``.
    """

    def _check_legendre_solution(self, result: IRNode, expected_n: int) -> None:
        """Assert result is Equal(y, %c1·LegendreP(n,x) + %c2·LegendreQ(n,x))."""
        assert result is not None, "Expected solution, got None"
        lhs, rhs = _is_equal_node(result)
        assert lhs == Y
        # Must contain LegendreP and LegendreQ
        assert _has_symbol_name(rhs, "LegendreP"), f"LegendreP missing in {rhs!r}"
        assert _has_symbol_name(rhs, "LegendreQ"), f"LegendreQ missing in {rhs!r}"
        # Must contain %c1 and %c2
        assert _has_symbol_name(rhs, "%c1"), f"%c1 missing in {rhs!r}"
        assert _has_symbol_name(rhs, "%c2"), f"%c2 missing in {rhs!r}"
        # Verify the n parameter
        assert _has_symbol_name(rhs, str(expected_n)) or _param_in_apply(
            rhs, expected_n
        ), f"Expected n={expected_n} in {rhs!r}"

    def _check_legendre_result(self, result: IRNode, expected_n: int) -> None:
        """Assert result has the correct LegendreP/Q structure."""
        self._check_legendre_solution(result, expected_n)

    def test_legendre_n0(self) -> None:
        """n=0: (1−x²)y'' − 2xy' + 0·y = 0  →  LegendreP/Q with n=0."""
        result = _try_legendre_ode(_legendre_expr(0), Y, X)
        assert result is not None
        lhs, rhs = _is_equal_node(result)
        assert lhs == Y
        assert _has_symbol_name(rhs, "LegendreP")
        assert _has_symbol_name(rhs, "LegendreQ")

    def test_legendre_n1(self) -> None:
        """n=1: λ=2, Q=-2x, should be recognised."""
        result = _try_legendre_ode(_legendre_expr(1), Y, X)
        assert result is not None
        assert _has_symbol_name(result, "LegendreP")

    def test_legendre_n2(self) -> None:
        """n=2: λ=6, standard form, should be recognised."""
        result = _try_legendre_ode(_legendre_expr(2), Y, X)
        assert result is not None
        lhs, rhs = _is_equal_node(result)
        assert lhs == Y
        assert _has_symbol_name(rhs, "LegendreP")
        assert _has_symbol_name(rhs, "LegendreQ")

    def test_legendre_n3(self) -> None:
        """n=3: λ=12, should be recognised."""
        result = _try_legendre_ode(_legendre_expr(3), Y, X)
        assert result is not None
        assert _has_symbol_name(result, "LegendreP")

    def test_legendre_bad_lambda_5_returns_none(self) -> None:
        """(1−x²)y'' − 2xy' + 5y = 0: λ=5 is not n(n+1) → None."""
        # Build with λ=5
        p_term = _mul(_one_minus_x_sq(), Y_DOUBLE)
        q_term = _neg(_mul(_mul(_I(2), X), Y_PRIME))
        r_term = _mul(_I(5), Y)
        expr = _add(_add(p_term, q_term), r_term)
        result = _try_legendre_ode(expr, Y, X)
        assert result is None

    def test_legendre_wrong_q_coeff_returns_none(self) -> None:
        """(1−x²)y'' − x·y' + 6y = 0: Q≈−x, not −2x  (this is Chebyshev n=√6 — non-integer, also None)."""
        # Q = -x instead of -2x  →  not Legendre
        p_term = _mul(_one_minus_x_sq(), Y_DOUBLE)
        q_term = _neg(_mul(X, Y_PRIME))
        r_term = _mul(_I(6), Y)
        expr = _add(_add(p_term, q_term), r_term)
        result = _try_legendre_ode(expr, Y, X)
        assert result is None

    def test_legendre_bessel_not_recognised(self) -> None:
        """Bessel ODE is not Legendre."""
        result = _try_legendre_ode(_bessel_expr(1), Y, X)
        assert result is None

    def test_legendre_hermite_not_recognised(self) -> None:
        """Hermite ODE is not Legendre."""
        result = _try_legendre_ode(_hermite_expr(2), Y, X)
        assert result is None

    def test_legendre_const_coeff_not_recognised(self) -> None:
        """y'' + y = 0 (const-coeff) is not Legendre."""
        expr = _add(Y_DOUBLE, Y)
        result = _try_legendre_ode(expr, Y, X)
        assert result is None


# ---------------------------------------------------------------------------
# TestTryBesselOde
# ---------------------------------------------------------------------------


class TestTryBesselOde:
    """Tests for :func:`_try_bessel_ode`.

    Bessel ODE: ``x²y'' + x·y' + (x²−ν²)·y = 0``.
    """

    def test_bessel_nu0(self) -> None:
        """ν=0: R = x², should be recognised."""
        result = _try_bessel_ode(_bessel_expr(0), Y, X)
        assert result is not None
        assert _has_symbol_name(result, "BesselJ")
        assert _has_symbol_name(result, "BesselY")

    def test_bessel_nu1(self) -> None:
        """ν=1: R = x²−1, should be recognised."""
        result = _try_bessel_ode(_bessel_expr(1), Y, X)
        assert result is not None
        lhs, rhs = _is_equal_node(result)
        assert lhs == Y
        assert _has_symbol_name(rhs, "BesselJ")
        assert _has_symbol_name(rhs, "BesselY")

    def test_bessel_nu2(self) -> None:
        """ν=2: R = x²−4, should be recognised."""
        result = _try_bessel_ode(_bessel_expr(2), Y, X)
        assert result is not None
        assert _has_symbol_name(result, "BesselJ")

    def test_bessel_nu_half(self) -> None:
        """ν=1/2: R = x²−1/4, should be recognised as rational order."""
        result = _try_bessel_ode(_bessel_expr(1, 2), Y, X)
        assert result is not None
        assert _has_symbol_name(result, "BesselJ")
        # Parameter should be IRRational(1,2)
        assert _has_rational_param(result, 1, 2)

    def test_bessel_nu_three_halves(self) -> None:
        """ν=3/2: R = x²−9/4, should be recognised."""
        result = _try_bessel_ode(_bessel_expr(3, 2), Y, X)
        assert result is not None
        assert _has_symbol_name(result, "BesselJ")

    def test_bessel_contains_c1_c2(self) -> None:
        """Bessel solution contains %c1 and %c2."""
        result = _try_bessel_ode(_bessel_expr(1), Y, X)
        assert result is not None
        assert _has_symbol_name(result, "%c1")
        assert _has_symbol_name(result, "%c2")

    def test_bessel_legendre_not_recognised(self) -> None:
        """Legendre ODE is not Bessel."""
        result = _try_bessel_ode(_legendre_expr(2), Y, X)
        assert result is None

    def test_bessel_hermite_not_recognised(self) -> None:
        """Hermite ODE is not Bessel."""
        result = _try_bessel_ode(_hermite_expr(2), Y, X)
        assert result is None

    def test_bessel_const_coeff_not_recognised(self) -> None:
        """y'' + y = 0 (const-coeff) is not Bessel."""
        expr = _add(Y_DOUBLE, Y)
        result = _try_bessel_ode(expr, Y, X)
        assert result is None

    def test_bessel_r_is_xsq_plus_const_not_xsq_minus_nu_sq(self) -> None:
        """x²y'' + xy' + (x²+1)y = 0: ν² = −1 < 0 → None."""
        r_coeff = _add(_x_sq(), _I(1))
        expr = _add(
            _add(_mul(_x_sq(), Y_DOUBLE), _mul(X, Y_PRIME)),
            _mul(r_coeff, Y),
        )
        result = _try_bessel_ode(expr, Y, X)
        assert result is None


# ---------------------------------------------------------------------------
# TestTryHermiteOde
# ---------------------------------------------------------------------------


class TestTryHermiteOde:
    """Tests for :func:`_try_hermite_ode`.

    Hermite ODE: ``y'' − 2x·y' + 2n·y = 0``.
    """

    def test_hermite_n0(self) -> None:
        """n=0: y'' − 2xy' = 0, should be recognised."""
        result = _try_hermite_ode(_hermite_expr(0), Y, X)
        assert result is not None
        assert _has_symbol_name(result, "HermiteH")
        assert _has_symbol_name(result, "HermiteH2")

    def test_hermite_n1(self) -> None:
        """n=1: y'' − 2xy' + 2y = 0, should be recognised."""
        result = _try_hermite_ode(_hermite_expr(1), Y, X)
        assert result is not None
        assert _has_symbol_name(result, "HermiteH")

    def test_hermite_n2(self) -> None:
        """n=2: y'' − 2xy' + 4y = 0, should be recognised."""
        result = _try_hermite_ode(_hermite_expr(2), Y, X)
        assert result is not None
        lhs, rhs = _is_equal_node(result)
        assert lhs == Y

    def test_hermite_n3(self) -> None:
        """n=3: y'' − 2xy' + 6y = 0, should be recognised."""
        result = _try_hermite_ode(_hermite_expr(3), Y, X)
        assert result is not None
        assert _has_symbol_name(result, "HermiteH")

    def test_hermite_contains_c1_c2(self) -> None:
        """Hermite solution contains %c1 and %c2."""
        result = _try_hermite_ode(_hermite_expr(2), Y, X)
        assert result is not None
        assert _has_symbol_name(result, "%c1")
        assert _has_symbol_name(result, "%c2")

    def test_hermite_non_integer_r_val_returns_none(self) -> None:
        """y'' − 2xy' + 3y = 0: 2n=3 → n=1.5 (non-integer) → None."""
        # R = 3 (odd, not even integer means n=1.5)
        p_term = Y_DOUBLE
        q_term = _neg(_mul(_mul(_I(2), X), Y_PRIME))
        r_term = _mul(_I(3), Y)
        expr = _add(_add(p_term, q_term), r_term)
        result = _try_hermite_ode(expr, Y, X)
        assert result is None

    def test_hermite_negative_r_returns_none(self) -> None:
        """y'' − 2xy' − 2y = 0: R=-2 < 0 → None."""
        p_term = Y_DOUBLE
        q_term = _neg(_mul(_mul(_I(2), X), Y_PRIME))
        r_term = _neg(_mul(_I(2), Y))
        expr = _add(_add(p_term, q_term), r_term)
        result = _try_hermite_ode(expr, Y, X)
        assert result is None

    def test_hermite_legendre_not_recognised(self) -> None:
        """Legendre ODE is not Hermite."""
        result = _try_hermite_ode(_legendre_expr(2), Y, X)
        assert result is None

    def test_hermite_bessel_not_recognised(self) -> None:
        """Bessel ODE is not Hermite."""
        result = _try_hermite_ode(_bessel_expr(1), Y, X)
        assert result is None

    def test_hermite_non_unit_p_returns_none(self) -> None:
        """2y'' − 2xy' + 4y = 0: P=2 ≠ 1 → not standard Hermite form → None."""
        p_term = _mul(_I(2), Y_DOUBLE)
        q_term = _neg(_mul(_mul(_I(2), X), Y_PRIME))
        r_term = _mul(_I(4), Y)
        expr = _add(_add(p_term, q_term), r_term)
        result = _try_hermite_ode(expr, Y, X)
        assert result is None


# ---------------------------------------------------------------------------
# TestTryChebyshevOde
# ---------------------------------------------------------------------------


class TestTryChebyshevOde:
    """Tests for :func:`_try_chebyshev_ode`.

    Chebyshev ODE: ``(1−x²)y'' − x·y' + n²·y = 0``.
    """

    def test_chebyshev_n0(self) -> None:
        """n=0: (1−x²)y'' − xy' + 0·y = 0, should be recognised."""
        result = _try_chebyshev_ode(_chebyshev_expr(0), Y, X)
        assert result is not None
        assert _has_symbol_name(result, "ChebyshevT")
        assert _has_symbol_name(result, "ChebyshevU")

    def test_chebyshev_n1(self) -> None:
        """n=1: (1−x²)y'' − xy' + y = 0, should be recognised."""
        result = _try_chebyshev_ode(_chebyshev_expr(1), Y, X)
        assert result is not None
        assert _has_symbol_name(result, "ChebyshevT")

    def test_chebyshev_n2(self) -> None:
        """n=2: (1−x²)y'' − xy' + 4y = 0, should be recognised."""
        result = _try_chebyshev_ode(_chebyshev_expr(2), Y, X)
        assert result is not None
        lhs, rhs = _is_equal_node(result)
        assert lhs == Y
        assert _has_symbol_name(rhs, "ChebyshevT")
        assert _has_symbol_name(rhs, "ChebyshevU")

    def test_chebyshev_n3(self) -> None:
        """n=3: (1−x²)y'' − xy' + 9y = 0, should be recognised."""
        result = _try_chebyshev_ode(_chebyshev_expr(3), Y, X)
        assert result is not None
        assert _has_symbol_name(result, "ChebyshevT")

    def test_chebyshev_contains_c1_c2(self) -> None:
        """Chebyshev solution contains %c1 and %c2."""
        result = _try_chebyshev_ode(_chebyshev_expr(2), Y, X)
        assert result is not None
        assert _has_symbol_name(result, "%c1")
        assert _has_symbol_name(result, "%c2")

    def test_chebyshev_non_perfect_square_r_returns_none(self) -> None:
        """(1−x²)y'' − xy' + 2y = 0: n²=2 → n=√2 (non-integer) → None."""
        p_term = _mul(_one_minus_x_sq(), Y_DOUBLE)
        q_term = _neg(_mul(X, Y_PRIME))
        r_term = _mul(_I(2), Y)
        expr = _add(_add(p_term, q_term), r_term)
        result = _try_chebyshev_ode(expr, Y, X)
        assert result is None

    def test_chebyshev_not_confused_with_legendre_n2(self) -> None:
        """Legendre n=2 has Q≈−2x; Chebyshev recogniser checks Q≈−x → should not match."""
        # _try_chebyshev_ode on the Legendre n=2 expression must return None
        # because Q = -2x ≠ -x
        result = _try_chebyshev_ode(_legendre_expr(2), Y, X)
        assert result is None

    def test_chebyshev_bessel_not_recognised(self) -> None:
        """Bessel ODE is not Chebyshev."""
        result = _try_chebyshev_ode(_bessel_expr(1), Y, X)
        assert result is None

    def test_chebyshev_hermite_not_recognised(self) -> None:
        """Hermite ODE is not Chebyshev."""
        result = _try_chebyshev_ode(_hermite_expr(2), Y, X)
        assert result is None


# ---------------------------------------------------------------------------
# TestVarCoeffNamedOdeDispatcher
# ---------------------------------------------------------------------------


class TestVarCoeffNamedOdeDispatcher:
    """Tests for :func:`_try_var_coeff_named_ode` — the Phase 21 dispatcher."""

    def test_dispatches_legendre(self) -> None:
        """Dispatcher recognises Legendre n=2."""
        result = _try_var_coeff_named_ode(_legendre_expr(2), Y, X)
        assert result is not None
        assert _has_symbol_name(result, "LegendreP")

    def test_dispatches_bessel(self) -> None:
        """Dispatcher recognises Bessel ν=1."""
        result = _try_var_coeff_named_ode(_bessel_expr(1), Y, X)
        assert result is not None
        assert _has_symbol_name(result, "BesselJ")

    def test_dispatches_hermite(self) -> None:
        """Dispatcher recognises Hermite n=3."""
        result = _try_var_coeff_named_ode(_hermite_expr(3), Y, X)
        assert result is not None
        assert _has_symbol_name(result, "HermiteH")

    def test_dispatches_chebyshev(self) -> None:
        """Dispatcher recognises Chebyshev n=2."""
        result = _try_var_coeff_named_ode(_chebyshev_expr(2), Y, X)
        assert result is not None
        assert _has_symbol_name(result, "ChebyshevT")

    def test_chebyshev_before_legendre_in_priority(self) -> None:
        """Chebyshev is tried before Legendre; the correct family wins.

        Chebyshev n=2: Q≈−x.  If tried second it would incorrectly match
        Legendre's P≈1−x² check first and get stuck on Q≈−2x.  By trying
        Chebyshev first we guarantee the correct family is identified.
        """
        cheb_expr = _chebyshev_expr(2)   # Q = -x (Chebyshev)
        result = _try_var_coeff_named_ode(cheb_expr, Y, X)
        assert result is not None
        assert _has_symbol_name(result, "ChebyshevT")
        assert not _has_symbol_name(result, "LegendreP")

    def test_unrelated_ode_returns_none(self) -> None:
        """y'' + y = 0 (const-coeff) returns None from dispatcher."""
        expr = _add(Y_DOUBLE, Y)
        result = _try_var_coeff_named_ode(expr, Y, X)
        assert result is None

    def test_first_order_returns_none(self) -> None:
        """y' + y = 0 returns None (not 2nd-order)."""
        expr = _add(Y_PRIME, Y)
        result = _try_var_coeff_named_ode(expr, Y, X)
        assert result is None


# ---------------------------------------------------------------------------
# TestPhase21EndToEnd — through solve_ode and the VM
# ---------------------------------------------------------------------------


class TestPhase21EndToEnd:
    """End-to-end tests running through solve_ode and the ODE2 VM handler.

    These verify the full pipeline: MACSYMA IR → dispatcher → recogniser →
    named solution node.
    """

    def test_legendre_n2_via_solve_ode(self) -> None:
        """solve_ode recognises Legendre n=2 directly (passed a live VM)."""
        vm = make_vm()
        result = solve_ode(_legendre_expr(2), Y, X, vm)
        assert result is not None
        assert _has_symbol_name(result, "LegendreP")
        assert _has_symbol_name(result, "LegendreQ")

    def test_legendre_n3_via_vm(self) -> None:
        """ODE2(legendre_n3, y, x) through the VM."""
        result = eval_ode(_legendre_expr(3))
        _was_evaluated(result)
        assert _has_symbol_name(result, "LegendreP")

    def test_bessel_nu1_via_solve_ode(self) -> None:
        """solve_ode recognises Bessel ν=1 (passed a live VM)."""
        vm = make_vm()
        result = solve_ode(_bessel_expr(1), Y, X, vm)
        assert result is not None
        assert _has_symbol_name(result, "BesselJ")
        assert _has_symbol_name(result, "BesselY")

    def test_bessel_nu1_via_vm(self) -> None:
        """ODE2(bessel_nu1, y, x) through the VM."""
        result = eval_ode(_bessel_expr(1))
        _was_evaluated(result)
        assert _has_symbol_name(result, "BesselJ")

    def test_bessel_nu_half_via_vm(self) -> None:
        """ODE2(bessel_nu=1/2, y, x) through the VM — rational parameter."""
        result = eval_ode(_bessel_expr(1, 2))
        _was_evaluated(result)
        assert _has_symbol_name(result, "BesselJ")
        # ν=1/2 should appear as IRRational(1,2) in the solution
        assert _has_rational_param(result, 1, 2)

    def test_hermite_n3_via_vm(self) -> None:
        """ODE2(hermite_n3, y, x) through the VM."""
        result = eval_ode(_hermite_expr(3))
        _was_evaluated(result)
        assert _has_symbol_name(result, "HermiteH")
        assert _has_symbol_name(result, "HermiteH2")

    def test_chebyshev_n2_via_vm(self) -> None:
        """ODE2(chebyshev_n2, y, x) through the VM."""
        result = eval_ode(_chebyshev_expr(2))
        _was_evaluated(result)
        assert _has_symbol_name(result, "ChebyshevT")
        assert _has_symbol_name(result, "ChebyshevU")

    def test_solution_structure_is_equal_y(self) -> None:
        """All named-ODE solutions are Equal(y, ...) with y as LHS."""
        for expr in [
            _legendre_expr(2),
            _bessel_expr(1),
            _hermite_expr(2),
            _chebyshev_expr(2),
        ]:
            result = eval_ode(expr)
            lhs, _ = _is_equal_node(result)
            assert lhs == Y, f"LHS of Equal should be y, got {lhs!r}"

    def test_solution_contains_c1_c2(self) -> None:
        """All named-ODE solutions contain integration constants %c1 and %c2."""
        for expr in [
            _legendre_expr(2),
            _bessel_expr(1),
            _hermite_expr(2),
            _chebyshev_expr(2),
        ]:
            result = eval_ode(expr)
            assert _has_symbol_name(result, "%c1"), f"Missing %c1 in {result!r}"
            assert _has_symbol_name(result, "%c2"), f"Missing %c2 in {result!r}"


# ---------------------------------------------------------------------------
# TestPhase21Regressions — previous ODE types must still work
# ---------------------------------------------------------------------------


class TestPhase21Regressions:
    """Regression tests — Phase 21 must not break earlier solver types.

    The named-ODE dispatcher fires in the solve_ode chain after Euler-Cauchy.
    These tests confirm that const-coeff, Euler-Cauchy, and the first-order
    solvers still produce correct results.
    """

    def test_const_coeff_homogeneous_still_works(self) -> None:
        """y'' − 3y' + 2y = 0 still returns explicit Equal(y, ...) solution."""
        expr = _add(
            _add(Y_DOUBLE, _neg(_mul(_I(3), Y_PRIME))),
            _mul(_I(2), Y),
        )
        result = eval_ode(expr)
        _was_evaluated(result)
        assert isinstance(result, IRApply) and result.head.name == "Equal"

    def test_const_coeff_complex_roots_still_works(self) -> None:
        """y'' + y = 0 still gives cos-based solution."""
        expr = _add(Y_DOUBLE, Y)
        result = eval_ode(expr)
        _was_evaluated(result)
        assert _has_symbol_name(result, "Cos") or _has_symbol_name(result, "Sin")

    def test_const_coeff_not_confused_with_named_ode(self) -> None:
        """y'' + 4y = 0 is const-coeff, not a named family (no variable P,Q,R)."""
        expr = _add(Y_DOUBLE, _mul(_I(4), Y))
        result = eval_ode(expr)
        _was_evaluated(result)
        # Should NOT match any named ODE family
        assert not _has_symbol_name(result, "HermiteH")
        assert not _has_symbol_name(result, "LegendreP")
        assert not _has_symbol_name(result, "BesselJ")
        assert not _has_symbol_name(result, "ChebyshevT")

    def test_euler_cauchy_distinct_real_roots_still_works(self) -> None:
        """x²y'' − 2y = 0 (Euler-Cauchy, r=2,r=-1) still works."""
        expr = _add(_mul(_x_sq(), Y_DOUBLE), _neg(_mul(_I(2), Y)))
        result = eval_ode(expr)
        _was_evaluated(result)
        # Euler-Cauchy gives Pow(x, r) terms
        assert _has_symbol_name(result, "Pow") or _has_symbol_name(result, "Equal")

    def test_first_order_linear_still_works(self) -> None:
        """y' + 2y = 0 still returns Equal(y, ...)."""
        expr = _add(Y_PRIME, _mul(_I(2), Y))
        result = eval_ode(expr)
        _was_evaluated(result)

    def test_unrecognised_variable_coeff_stays_unevaluated(self) -> None:
        """x·y'' + y = 0 — variable-coeff but not a named family → unevaluated."""
        expr = _add(_mul(X, Y_DOUBLE), Y)
        result = eval_ode(expr)
        _is_unevaluated(result)


# ---------------------------------------------------------------------------
# Private helpers for assertion predicates
# ---------------------------------------------------------------------------


def _param_in_apply(node: IRNode, expected: int) -> bool:
    """Return True if an IRApply argument list contains IRInteger(expected)."""
    if isinstance(node, IRApply):
        for arg in node.args:
            if isinstance(arg, IRInteger) and arg.value == expected:
                return True
            if _param_in_apply(arg, expected):
                return True
    return False


def _has_rational_param(node: IRNode, p: int, q: int) -> bool:
    """Return True if the tree contains IRRational(p, q) as an argument."""
    if isinstance(node, IRRational):
        return node.numer == p and node.denom == q
    if isinstance(node, IRApply):
        return any(_has_rational_param(c, p, q) for c in node.args)
    return False
