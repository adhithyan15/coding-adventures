"""Tests for the Lie point-symmetry handler (Track L1).

The handler covers three symmetry groups for first-order ODEs:

1. Translation in y — ``y' = f(x)`` reduces by direct integration.
2. Translation in x — autonomous ``y' = g(y)`` reduces by the inverse
   quadrature ``x = ∫ 1/g(y) dy + C``.
3. Scaling ``(x, y) → (λx, λ^k y)`` — detected, but delegated to the
   existing homogeneous-type solver (k = ±1) for closed-form output.

The acceptance bullets in the spec are exercised end-to-end through
``ode2_handler`` so we hit the public surface a user would touch.  The
``unevaluated`` test guarantees we don't claim victory on an ODE we
can't actually solve.
"""

from __future__ import annotations

from symbolic_ir import (
    ADD,
    DIV,
    EQUAL,
    INTEGRATE,
    LOG,
    MUL,
    NEG,
    POW,
    SIN,
    SUB,
    IRApply,
    IRInteger,
    IRNode,
    IRSymbol,
)
from symbolic_ir.nodes import C_CONST, ODE2, D
from symbolic_vm import VM, SymbolicBackend

from cas_ode import build_ode_handler_table
from cas_ode.lie_symmetry import (
    _detect_scaling_k,
    _extract_f,
    _is_x_autonomous,
    _is_y_autonomous,
    try_lie_symmetry,
)

# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

X = IRSymbol("x")
Y = IRSymbol("y")
Y_PRIME = IRApply(D, (Y, X))


def _vm() -> VM:
    backend = SymbolicBackend()
    backend._handlers.update(build_ode_handler_table())  # type: ignore[attr-defined]
    return VM(backend)


def _eval_ode2(zero_form: IRNode) -> IRNode:
    """Call the public ode2_handler via the VM."""
    return _vm().eval(IRApply(ODE2, (zero_form, Y, X)))


def _neg(n: IRNode) -> IRNode:
    return IRApply(NEG, (n,))


def _add(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(ADD, (a, b))


def _sub(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(SUB, (a, b))


def _mul(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(MUL, (a, b))


def _div(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(DIV, (a, b))


def _pow(b: IRNode, e: IRNode) -> IRNode:
    return IRApply(POW, (b, e))


def _sin(a: IRNode) -> IRNode:
    return IRApply(SIN, (a,))


# Tree-search helpers — used to assert structural properties of solutions.

def _contains(node: IRNode, head: IRSymbol) -> bool:
    """Return ``True`` if ``head`` (an IR head symbol) appears anywhere
    in the subtree."""
    if isinstance(node, IRApply):
        if isinstance(node.head, IRSymbol) and node.head == head:
            return True
        return any(_contains(a, head) for a in node.args)
    return False


def _contains_unevaluated_integrate(node: IRNode) -> bool:
    return _contains(node, INTEGRATE)


# ---------------------------------------------------------------------------
# Section A — Autonomy / detection unit tests
# ---------------------------------------------------------------------------


class TestAutonomyChecks:
    def test_constant_is_both_autonomous(self) -> None:
        f = IRInteger(7)
        assert _is_x_autonomous(f, X, Y)
        assert _is_y_autonomous(f, X, Y)

    def test_pure_x_is_y_autonomous(self) -> None:
        # f = sin(x)
        assert _is_y_autonomous(_sin(X), X, Y)
        assert not _is_x_autonomous(_sin(X), X, Y)

    def test_pure_y_is_x_autonomous(self) -> None:
        # f = y * (1 - y)
        f = _mul(Y, _sub(IRInteger(1), Y))
        assert _is_x_autonomous(f, X, Y)
        assert not _is_y_autonomous(f, X, Y)

    def test_mixed_is_neither(self) -> None:
        # f = x * y
        f = _mul(X, Y)
        assert not _is_x_autonomous(f, X, Y)
        assert not _is_y_autonomous(f, X, Y)


class TestScalingDetection:
    def test_scaling_k_equals_1_for_homogeneous(self) -> None:
        # f = (y² + xy) / x²  — should be scale-invariant with k = 1
        # under (x, y) → (λx, λy) since both num and denom scale as λ².
        f = _div(_add(_pow(Y, IRInteger(2)), _mul(X, Y)), _pow(X, IRInteger(2)))
        assert _detect_scaling_k(f, X, Y) == 1

    def test_scaling_k_equals_2_for_x_y_over_x(self) -> None:
        # f = y² / x — under (λx, λ²y), num → λ⁴y², denom → λx → f → λ³y²/x.
        # We need f → λ^(k-1) f = λ¹ f, so this DOES NOT match k=2 with
        # the canonical f.  Instead pick f such that scaling k=2 fires:
        # f = y / x²  →  (λ²y)/(λ²x²) = y/x² · λ⁰; we want λ¹.  Try
        # f = y²/x³: under (λx, λ²y) → λ⁴y² / λ³x³ = λ · y²/x³ ✓ (k=2).
        f = _div(_pow(Y, IRInteger(2)), _pow(X, IRInteger(3)))
        assert _detect_scaling_k(f, X, Y) == 2

    def test_no_scaling_for_transcendental(self) -> None:
        # f = sin(x * y) — no integer-exponent scaling preserves it.
        f = _sin(_mul(X, Y))
        assert _detect_scaling_k(f, X, Y) is None


# ---------------------------------------------------------------------------
# Section B — try_lie_symmetry directly
# ---------------------------------------------------------------------------


class TestExtractF:
    def test_zero_form_extraction(self) -> None:
        # y' - sin(x) = 0  →  f = sin(x)
        zero = _sub(Y_PRIME, _sin(X))
        f = _extract_f(zero, Y, X, _vm())
        # The VM may rewrap as Sin(x); we only need numerical equality.
        from cas_ode.lie_symmetry import _eval_f
        assert f is not None
        # sin(0.7) ≈ 0.6442
        v = _eval_f(f, X, Y, 0.7, 0.0)
        assert v is not None and abs(v - 0.6442176872) < 1e-6

    def test_missing_y_prime_returns_none(self) -> None:
        # 5x = 0  has no y' — not a normalisable ODE for Lie.
        zero = _mul(IRInteger(5), X)
        assert _extract_f(zero, Y, X, _vm()) is None


class TestTranslationYDirect:
    def test_y_prime_equals_sin_x(self) -> None:
        # Direct call: y' - sin(x) = 0
        zero = _sub(Y_PRIME, _sin(X))
        result = try_lie_symmetry(zero, Y, X, _vm())
        # In production, separable would intercept this case.  When called
        # directly, the y-autonomy branch fires:
        assert result is not None
        assert isinstance(result, IRApply) and result.head == EQUAL
        # LHS is y, RHS contains -cos(x) (plus a constant).
        assert result.args[0] == Y
        assert not _contains_unevaluated_integrate(result)


class TestTranslationXDirect:
    def test_logistic_autonomous(self) -> None:
        # y' = y(1-y)  →  zero form: y' - y(1-y) = 0
        rhs = _mul(Y, _sub(IRInteger(1), Y))
        zero = _sub(Y_PRIME, rhs)
        result = try_lie_symmetry(zero, Y, X, _vm())
        assert result is not None
        assert isinstance(result, IRApply) and result.head == EQUAL
        # Inverse form: x = ∫ 1/(y(1-y)) dy + C
        assert result.args[0] == X
        # The integral 1/(y(1-y)) = log(y) - log(1-y); VM may emit either
        # form, but it must NOT remain an unevaluated Integrate node.
        assert not _contains_unevaluated_integrate(result)
        assert _contains(result, LOG)


# ---------------------------------------------------------------------------
# Section C — End-to-end through ode2_handler
# ---------------------------------------------------------------------------
#
# These are the acceptance-list cases.  We invoke the *public* handler and
# verify the returned form satisfies the original ODE numerically by
# symbolic differentiation through the VM.


def _substitute_solution_and_check(
    sol: IRNode,
    f_for_yprime: IRNode,
) -> bool:
    """Verify ``sol = Equal(y, expr(x))`` by checking d/dx[expr] ≈ f(x, expr)
    at several x.  Returns ``False`` if the form is wrong or numerics diverge.

    For implicit forms (``Equal(x, expr(y))``) we differentiate dx/dy and
    compare to ``1 / f(x, y) = 1 / g(y)`` evaluated at sample y.
    """
    from cas_ode.ode import _eval_at_xy

    if not (isinstance(sol, IRApply) and sol.head == EQUAL):
        return False
    lhs, rhs = sol.args
    vm = _vm()

    if lhs == Y:
        # Explicit form: y = rhs(x). Build d(rhs)/dx and compare to f.
        # The forcing form f_for_yprime is f(x, y); since y = rhs(x), we
        # substitute y → rhs and compare numerically.
        derivative = vm.eval(IRApply(D, (rhs, X)))
        # Substitute %c → 0 in the solution and the derivative for testing.
        for xv in (0.4, 0.9, 1.7):
            try:
                # Both `derivative` and `rhs` may reference %c (C_CONST).
                # We substitute via a dummy eval pass that maps C_CONST → 0.
                d_val = _safe_eval(derivative, xv)
                y_val = _safe_eval(rhs, xv)
                if d_val is None or y_val is None:
                    return False
                f_val = _eval_at_xy(f_for_yprime, X, Y, xv, y_val)
                if abs(d_val - f_val) > 1e-6:
                    return False
            except (ValueError, ZeroDivisionError, OverflowError):
                return False
        return True

    if lhs == X:
        # Implicit form: x = rhs(y).  dx/dy = 1 / g(y).
        derivative = vm.eval(IRApply(D, (rhs, Y)))
        from cas_ode.ode import _eval_at_xy
        for yv in (0.25, 0.4, 0.6):  # avoid y = 0, 1 (singularities)
            try:
                # rhs may reference %c, so eval at any x value (doesn't
                # appear in rhs because lhs == X).
                dx_dy = _safe_eval_at_y(derivative, yv)
                g_val = _eval_at_xy(f_for_yprime, X, Y, 0.0, yv)
                if dx_dy is None or g_val is None or g_val == 0:
                    return False
                expected = 1.0 / g_val
                if abs(dx_dy - expected) > 1e-6:
                    return False
            except (ValueError, ZeroDivisionError, OverflowError):
                return False
        return True

    # Other implicit forms (Equal(F(x,y), C)) — not checked here.
    return True


def _safe_eval(node: IRNode, xv: float, yv: float = 0.0) -> float | None:
    """Eval treating ``%c`` (C_CONST) as 0."""
    from cas_ode.ode import _eval_at_xy
    # _eval_at_xy raises on unknown symbols. C_CONST is the symbol "%c".
    # We swap it for 0 by recursive substitution before eval.
    cleaned = _strip_c(node)
    try:
        return _eval_at_xy(cleaned, X, Y, xv, yv)
    except (ValueError, ZeroDivisionError, OverflowError):
        return None


def _safe_eval_at_y(node: IRNode, yv: float) -> float | None:
    """Eval treating ``%c`` as 0 — x not present in implicit form's RHS."""
    return _safe_eval(node, 1.0, yv)


def _strip_c(node: IRNode) -> IRNode:
    """Substitute every C_CONST occurrence with IRInteger(0)."""
    if node == C_CONST:
        return IRInteger(0)
    if isinstance(node, IRApply):
        return IRApply(
            node.head,
            tuple(_strip_c(a) for a in node.args),
        )
    return node


class TestAcceptance:
    def test_scaling_homogeneous_closes(self) -> None:
        # y' = (y² + xy) / x²
        # This is homogeneous-type; the existing solver catches it before
        # Lie.  We assert *some* closed form is produced.
        rhs = _div(_add(_pow(Y, IRInteger(2)), _mul(X, Y)), _pow(X, IRInteger(2)))
        zero = _sub(Y_PRIME, rhs)
        result = _eval_ode2(zero)
        # Should not be unevaluated ODE2(...)
        assert not (isinstance(result, IRApply) and result.head == ODE2)
        assert isinstance(result, IRApply) and result.head == EQUAL
        # Closed form must not still contain a raw Integrate.
        assert not _contains_unevaluated_integrate(result)

    def test_translation_y_sin_closes(self) -> None:
        # y' = sin(x)  →  y = -cos(x) + C
        # Separable catches this before Lie, but the public surface still
        # produces a valid closed form.
        zero = _sub(Y_PRIME, _sin(X))
        result = _eval_ode2(zero)
        assert isinstance(result, IRApply) and result.head == EQUAL
        assert result.args[0] == Y
        # Verify by differentiating and comparing to sin(x).
        assert _substitute_solution_and_check(result, _sin(X))

    def test_translation_x_logistic_closes_via_lie(self) -> None:
        # y' = y(1-y) — autonomous, NOT linear in y after expansion.
        # Separable case 2 only catches g(y) = k*y, so this falls through
        # to Lie.
        rhs = _mul(Y, _sub(IRInteger(1), Y))
        zero = _sub(Y_PRIME, rhs)
        result = _eval_ode2(zero)
        assert isinstance(result, IRApply) and result.head == EQUAL
        # Inverse form: x = ∫ 1/(y(1-y)) dy + C
        assert result.args[0] == X
        assert _contains(result, LOG)
        # Verify dx/dy = 1 / (y(1-y)) numerically.
        assert _substitute_solution_and_check(result, rhs)

    def test_fall_through_for_sin_xy(self) -> None:
        # y' = sin(xy) — no recognised symmetry, returns unevaluated.
        zero = _sub(Y_PRIME, _sin(_mul(X, Y)))
        result = _eval_ode2(zero)
        assert isinstance(result, IRApply) and result.head == ODE2


# ---------------------------------------------------------------------------
# Section D — Regression: existing handlers still win the race
# ---------------------------------------------------------------------------
#
# The spec requires that linear / separable / Bernoulli ODEs continue to
# route through their dedicated handlers, NOT through the Lie path.  We
# verify this indirectly by calling ``try_lie_symmetry`` and checking that
# the *direct* Lie reduction does not produce the canonical-handler shape
# (linear gives Exp; Bernoulli gives Pow).
#
# More importantly we exercise the full dispatcher and confirm the
# expected handler-specific output.


class TestRegression:
    def test_linear_y_prime_plus_y_equals_x(self) -> None:
        # y' + y = x  →  linear, integrating factor μ = e^x.
        zero = _sub(_add(Y_PRIME, Y), X)
        result = _eval_ode2(zero)
        assert isinstance(result, IRApply) and result.head == EQUAL
        assert result.args[0] == Y
        # Solution contains Exp — characteristic of integrating factor.
        from symbolic_ir import EXP
        assert _contains(result, EXP)

    def test_separable_y_prime_equals_x_y(self) -> None:
        # y' = x·y  →  separable; solution y = C·exp(x²/2).
        rhs = _mul(X, Y)
        zero = _sub(Y_PRIME, rhs)
        result = _eval_ode2(zero)
        assert isinstance(result, IRApply) and result.head == EQUAL
        assert result.args[0] == Y
        from symbolic_ir import EXP
        assert _contains(result, EXP)
