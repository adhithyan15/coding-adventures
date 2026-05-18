"""Tests for Phase 34: Weierstrass-substitution closed forms for
``∫ c / (a + b·sin(x)) dx`` and ``∫ c / (a + b·cos(x)) dx``.

The substitution ``u = tan(x/2)`` produces ``sin(x) = 2u/(1+u²)``,
``cos(x) = (1−u²)/(1+u²)``, ``dx = 2/(1+u²) du`` and reduces the
integrand to a rational function of ``u`` that integrates to an
arctan whenever ``a² > b²`` (the denominator never crosses zero on ℝ).

Closed forms emitted by Phase 34:

    ∫ 1/(a + b·sin x) dx  =  (2/√(a²−b²)) · arctan((a·tan(x/2) + b)/√(a²−b²))
    ∫ 1/(a + b·cos x) dx  =  (2/√(a²−b²)) · arctan(√((a−b)/(a+b)) · tan(x/2))

Both formulas are validated below by **numerical differentiation**: we
sample the integrand and the derivative of the returned closed form at
several values of ``x`` on the open intervals where ``tan(x/2)`` is
finite, and require the two to agree to a tight absolute tolerance.
Structural assertions on the exact IR shape are deliberately avoided
because the surrounding VM may re-canonicalise the arithmetic in
ways orthogonal to correctness.
"""

from __future__ import annotations

import math

import pytest
from symbolic_ir import (
    ADD,
    ATAN,
    COS,
    DIV,
    INTEGRATE,
    MUL,
    NEG,
    SIN,
    SQRT,
    SUB,
    TAN,
    IRApply,
    IRFloat,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

from symbolic_vm import VM, SymbolicBackend

X = IRSymbol("x")


@pytest.fixture
def vm() -> VM:
    return VM(SymbolicBackend())


def _integrate(f: IRNode) -> IRNode:
    return IRApply(INTEGRATE, (f, X))


def _eval_at(vm: VM, expr: IRNode, x_val: float) -> float:
    """Substitute ``x = x_val`` (as IRFloat) and evaluate to a float.

    Returns ``math.nan`` if the evaluation cannot fold to a number —
    callers should always assert real-number agreement and skip such
    samples explicitly when needed.
    """
    substituted = _subst(expr, X, IRFloat(x_val))
    folded = vm.eval(substituted)
    if isinstance(folded, IRFloat):
        return folded.value
    if isinstance(folded, IRInteger):
        return float(folded.value)
    if isinstance(folded, IRRational):
        return folded.numer / folded.denom
    return math.nan


def _subst(node: IRNode, var: IRSymbol, value: IRNode) -> IRNode:
    """Tiny structural substitution — replaces every ``var`` occurrence."""
    if node == var:
        return value
    if isinstance(node, IRApply):
        return IRApply(node.head, tuple(_subst(a, var, value) for a in node.args))
    return node


# ---------------------------------------------------------------------------
# ∫ 1/(a + b·sin(x)) dx — arctan form, a² > b²
# ---------------------------------------------------------------------------


def test_sin_two_plus_sin_returns_arctan_form(vm: VM) -> None:
    """``∫ 1/(2 + sin(x)) dx`` must close as ``(2/√3)·arctan(...)``."""
    integrand = IRApply(DIV, (IRInteger(1), IRApply(ADD, (IRInteger(2), IRApply(SIN, (X,))))))
    result = vm.eval(_integrate(integrand))
    # Must not stay as Integrate(...).
    assert not (isinstance(result, IRApply) and result.head == INTEGRATE), (
        f"Expected Phase 34 to close ∫ 1/(2+sin x) dx; got {result!r}"
    )
    # Should contain an Atan node somewhere.
    assert _contains_head(result, ATAN), (
        f"Expected an Atan in the closed form; got {result!r}"
    )


def test_sin_two_plus_sin_derivative_matches(vm: VM) -> None:
    """Numerically: ``d/dx[Φ(x)]`` must equal ``1/(2 + sin(x))``."""
    integrand = IRApply(DIV, (IRInteger(1), IRApply(ADD, (IRInteger(2), IRApply(SIN, (X,))))))
    phi = vm.eval(_integrate(integrand))
    # Sample on (−π, π) avoiding x = ±π where tan(x/2) blows up.
    for x_val in (-2.5, -1.0, -0.3, 0.0, 0.3, 1.0, 2.5):
        got = _numerical_derivative(vm, phi, x_val)
        expected = 1.0 / (2.0 + math.sin(x_val))
        assert math.isclose(got, expected, abs_tol=1e-4, rel_tol=1e-4), (
            f"At x={x_val}: derivative={got!r}, expected={expected!r}"
        )


def test_sin_perfect_square_discriminant(vm: VM) -> None:
    """``∫ 1/(5 + 3·sin(x)) dx``: ``a²−b² = 16`` is a perfect square — coefficient
    folds to an integer/rational and the result is exact (no Sqrt node)."""
    integrand = IRApply(
        DIV,
        (
            IRInteger(1),
            IRApply(ADD, (IRInteger(5), IRApply(MUL, (IRInteger(3), IRApply(SIN, (X,)))))),
        ),
    )
    result = vm.eval(_integrate(integrand))
    assert not _contains_head(result, SQRT), (
        f"a²−b²=16 should fold to 4 without leaving a Sqrt; got {result!r}"
    )
    # Verify numerically.
    for x_val in (-1.0, -0.2, 0.0, 0.7, 1.5):
        got = _numerical_derivative(vm, result, x_val)
        expected = 1.0 / (5.0 + 3.0 * math.sin(x_val))
        assert math.isclose(got, expected, abs_tol=1e-4, rel_tol=1e-4)


def test_sin_with_numerator_coefficient(vm: VM) -> None:
    """``∫ 3/(2 + sin(x)) dx`` must scale the closed form by 3."""
    integrand = IRApply(DIV, (IRInteger(3), IRApply(ADD, (IRInteger(2), IRApply(SIN, (X,))))))
    phi = vm.eval(_integrate(integrand))
    for x_val in (-1.0, -0.2, 0.0, 0.7, 1.5):
        got = _numerical_derivative(vm, phi, x_val)
        expected = 3.0 / (2.0 + math.sin(x_val))
        assert math.isclose(got, expected, abs_tol=1e-4, rel_tol=1e-4)


def test_sin_with_rational_coefficients(vm: VM) -> None:
    """``∫ 1/(3/2 + (1/2)·sin(x)) dx`` — rational a, b with a² > b².

    a = 3/2, b = 1/2 → disc = 9/4 − 1/4 = 2.
    """
    a = IRRational(3, 2)
    b = IRRational(1, 2)
    integrand = IRApply(
        DIV,
        (IRInteger(1), IRApply(ADD, (a, IRApply(MUL, (b, IRApply(SIN, (X,))))))),
    )
    phi = vm.eval(_integrate(integrand))
    assert not (isinstance(phi, IRApply) and phi.head == INTEGRATE)
    for x_val in (-1.5, -0.4, 0.0, 0.4, 1.5):
        got = _numerical_derivative(vm, phi, x_val)
        expected = 1.0 / (1.5 + 0.5 * math.sin(x_val))
        assert math.isclose(got, expected, abs_tol=1e-4, rel_tol=1e-4)


# ---------------------------------------------------------------------------
# ∫ 1/(a + b·cos(x)) dx — arctan form, a² > b², a > 0
# ---------------------------------------------------------------------------


def test_cos_two_plus_cos_closes(vm: VM) -> None:
    integrand = IRApply(DIV, (IRInteger(1), IRApply(ADD, (IRInteger(2), IRApply(COS, (X,))))))
    phi = vm.eval(_integrate(integrand))
    assert not (isinstance(phi, IRApply) and phi.head == INTEGRATE)
    for x_val in (-1.5, -0.4, 0.0, 0.4, 1.5):
        got = _numerical_derivative(vm, phi, x_val)
        expected = 1.0 / (2.0 + math.cos(x_val))
        assert math.isclose(got, expected, abs_tol=1e-4, rel_tol=1e-4)


def test_cos_five_plus_three_cos(vm: VM) -> None:
    """``∫ 1/(5 + 3·cos(x)) dx``. disc=16 (perfect square), (a−b)/(a+b)=2/8=1/4
    (also perfect-square ratio) — expect Sqrt-free output."""
    integrand = IRApply(
        DIV,
        (
            IRInteger(1),
            IRApply(ADD, (IRInteger(5), IRApply(MUL, (IRInteger(3), IRApply(COS, (X,)))))),
        ),
    )
    phi = vm.eval(_integrate(integrand))
    assert not _contains_head(phi, SQRT), f"Expected Sqrt-free closed form; got {phi!r}"
    for x_val in (-1.5, -0.4, 0.0, 0.4, 1.5):
        got = _numerical_derivative(vm, phi, x_val)
        expected = 1.0 / (5.0 + 3.0 * math.cos(x_val))
        assert math.isclose(got, expected, abs_tol=1e-4, rel_tol=1e-4)


# ---------------------------------------------------------------------------
# Operand-order robustness
# ---------------------------------------------------------------------------


def test_sin_operand_order_swapped(vm: VM) -> None:
    """``∫ 1/(sin(x) + 2) dx`` — constant on the right of Add. Same result."""
    integrand = IRApply(DIV, (IRInteger(1), IRApply(ADD, (IRApply(SIN, (X,)), IRInteger(2)))))
    phi = vm.eval(_integrate(integrand))
    # The handler might canonicalise the Add and still close, OR the VM may
    # re-sort args so the standard path fires.  Either way, the result must
    # NOT stay as a raw Integrate.
    assert not (isinstance(phi, IRApply) and phi.head == INTEGRATE), (
        f"Operand-swapped form should still close; got {phi!r}"
    )


# ---------------------------------------------------------------------------
# Fallthroughs — must stay unevaluated (Phase 34 deliberately defers)
# ---------------------------------------------------------------------------


def test_fallthrough_a_less_than_b(vm: VM) -> None:
    """``∫ 1/(1 + 2·sin(x)) dx`` — a²−b² = −3 < 0; log form not implemented."""
    integrand = IRApply(
        DIV,
        (
            IRInteger(1),
            IRApply(ADD, (IRInteger(1), IRApply(MUL, (IRInteger(2), IRApply(SIN, (X,)))))),
        ),
    )
    result = vm.eval(_integrate(integrand))
    assert isinstance(result, IRApply) and result.head == INTEGRATE, (
        f"a²<b² log form is deferred; expected unevaluated, got {result!r}"
    )


def test_fallthrough_a_equals_b(vm: VM) -> None:
    """``∫ 1/(1 + sin(x)) dx`` — a² = b²; linear-in-tan form not implemented."""
    integrand = IRApply(DIV, (IRInteger(1), IRApply(ADD, (IRInteger(1), IRApply(SIN, (X,))))))
    result = vm.eval(_integrate(integrand))
    assert isinstance(result, IRApply) and result.head == INTEGRATE


def test_fallthrough_non_bare_argument(vm: VM) -> None:
    """``∫ 1/(2 + sin(2x)) dx`` — argument inside Sin is not bare x.

    Phase 34 only handles the canonical bare-variable form.  A future
    Phase could compose with substitution to handle linear arguments;
    for now the integral remains unevaluated.
    """
    two_x = IRApply(MUL, (IRInteger(2), X))
    integrand = IRApply(
        DIV, (IRInteger(1), IRApply(ADD, (IRInteger(2), IRApply(SIN, (two_x,)))))
    )
    result = vm.eval(_integrate(integrand))
    assert isinstance(result, IRApply) and result.head == INTEGRATE


def test_fallthrough_symbolic_coefficients(vm: VM) -> None:
    """Symbolic ``a`` or ``b`` — can't decide sign of disc, must defer."""
    a_sym = IRSymbol("a")
    integrand = IRApply(DIV, (IRInteger(1), IRApply(ADD, (a_sym, IRApply(SIN, (X,))))))
    result = vm.eval(_integrate(integrand))
    assert isinstance(result, IRApply) and result.head == INTEGRATE


# ---------------------------------------------------------------------------
# Regression — Phase 34 must not steal integrals it doesn't own
# ---------------------------------------------------------------------------


def test_regression_pure_sin_still_works(vm: VM) -> None:
    """``∫ sin(x) dx = −cos(x)`` must continue to work after Phase 34 lands."""
    result = vm.eval(_integrate(IRApply(SIN, (X,))))
    # -cos(x) form — accept either Neg(Cos(x)) or Mul(-1, Cos(x)).
    cos_x = IRApply(COS, (X,))
    neg_cos = IRApply(NEG, (cos_x,))
    mul_neg = IRApply(MUL, (IRInteger(-1), cos_x))
    assert result in (neg_cos, mul_neg), f"Got {result!r}"


def test_regression_one_over_cos_unchanged(vm: VM) -> None:
    """``∫ 1/cos(x) dx`` (i.e. ∫ sec(x) dx) is NOT a Weierstrass case.

    The denominator is bare ``cos(x)`` without an additive constant, so
    ``_parse_a_plus_b_sincos`` returns None and Phase 34 does not engage.
    The result therefore stays as ``Integrate(...)`` (∫sec x dx has no
    elementary closed form in our pipeline yet).
    """
    integrand = IRApply(DIV, (IRInteger(1), IRApply(COS, (X,))))
    result = vm.eval(_integrate(integrand))
    # Either unevaluated, or some elementary fold the VM applies — but
    # specifically MUST NOT be a Phase 34 arctan-of-tan(x/2) artefact.
    if isinstance(result, IRApply) and result.head == ATAN:
        pytest.fail(f"Phase 34 incorrectly fired on ∫ 1/cos(x) dx: got {result!r}")


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _contains_head(node: IRNode, head: IRSymbol) -> bool:
    if isinstance(node, IRApply):
        if node.head == head:
            return True
        return any(_contains_head(a, head) for a in node.args)
    return False


def _numerical_derivative(vm: VM, expr: IRNode, x_val: float) -> float:
    """Central-difference derivative of ``expr`` w.r.t. x at ``x_val``.

    Uses a step of ``1e-5``.  Both ``expr(x_val + h)`` and
    ``expr(x_val − h)`` are evaluated via ``vm.eval`` on float-substituted
    copies, so the closed-form's structure doesn't matter — only the
    numeric output of the function does.
    """
    h = 1e-5
    fp = _eval_at(vm, expr, x_val + h)
    fm = _eval_at(vm, expr, x_val - h)
    return (fp - fm) / (2 * h)
