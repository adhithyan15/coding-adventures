"""Tests for cas-mnewton — Newton's method numeric root finder.

Test strategy
-------------
We test three layers:

1. The pure ``mnewton_solve`` function directly, by providing simple
   Python-function-based eval/diff stubs. This verifies the algorithm
   in isolation.

2. Via the VM handler ``MNewton(f, x, x0)`` using a real
   ``SymbolicBackend`` with the mnewton handler table registered.
   This tests end-to-end wiring.

3. Edge cases: non-numeric x0, f'=0 at x0, non-symbol var, wrong arity.

Known exact roots used as ground truth
---------------------------------------
- ``x - 2 = 0``       → root at x = 2.0
- ``x^2 - 2 = 0``     → root at x = sqrt(2) ≈ 1.41421356…
- ``x^2 - 4 = 0``     → roots at ±2; starting near 2 → +2
- ``x^3 - 8 = 0``     → root at x = 2.0
- ``x^2 - 9 = 0``     → root at x = 3 (from x0=2) or x=-3 (from x0=-2)
- ``x^2 = 0``         → root at x = 0 with f'=0 at x0=0 → unevaluated
"""

from __future__ import annotations

import math

import pytest
from symbolic_ir import (
    POW,
    SIN,
    SUB,
    IRApply,
    IRFloat,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

# ---------------------------------------------------------------------------
# VM and backend helpers
# ---------------------------------------------------------------------------
# We import from symbolic_vm here — this package is installed for tests.
from symbolic_vm import VM, SymbolicBackend

from cas_mnewton import MNewtonError, build_mnewton_handler_table
from cas_mnewton.newton import _ir_to_float


def make_vm() -> VM:
    """Build a SymbolicBackend VM with the MNewton handler registered."""
    backend = SymbolicBackend()
    # Merge in our MNewton handler table.
    for head_name, handler in build_mnewton_handler_table().items():
        backend._handlers[head_name] = handler
    return VM(backend)


# Common symbols and head
_X = IRSymbol("x")
_Y = IRSymbol("y")
_MNEWTON = IRSymbol("MNewton")

# ---------------------------------------------------------------------------
# Helper: build MNewton(f, x, x0) apply node
# ---------------------------------------------------------------------------


def mnewton_apply(f: IRNode, x: IRSymbol, x0: IRNode) -> IRApply:
    return IRApply(_MNEWTON, (f, x, x0))


def mnewton_apply_tol(f: IRNode, x: IRSymbol, x0: IRNode, tol: float) -> IRApply:
    return IRApply(_MNEWTON, (f, x, x0, IRFloat(tol)))


# ---------------------------------------------------------------------------
# Section 1: _ir_to_float helper unit tests
# ---------------------------------------------------------------------------


def test_ir_to_float_integer() -> None:
    assert _ir_to_float(IRInteger(3)) == 3.0


def test_ir_to_float_float() -> None:
    assert _ir_to_float(IRFloat(1.5)) == 1.5


def test_ir_to_float_rational() -> None:
    val = _ir_to_float(IRRational(1, 2))
    assert val == pytest.approx(0.5)


def test_ir_to_float_symbol_returns_none() -> None:
    assert _ir_to_float(IRSymbol("x")) is None


def test_ir_to_float_apply_returns_none() -> None:
    node = IRApply(IRSymbol("Add"), (IRInteger(1), IRInteger(2)))
    assert _ir_to_float(node) is None


# ---------------------------------------------------------------------------
# Section 2: mnewton_solve via VM (integration tests)
# ---------------------------------------------------------------------------


def test_linear_x_minus_2() -> None:
    """f = x - 2 has an exact root at x = 2. Any starting point converges."""
    vm = make_vm()
    # f(x) = x - 2  =  Sub(x, 2)
    f = IRApply(SUB, (_X, IRInteger(2)))
    result = vm.eval(mnewton_apply(f, _X, IRFloat(0.0)))
    assert isinstance(result, IRFloat)
    assert abs(result.value - 2.0) < 1e-9


def test_linear_converges_from_negative_x0() -> None:
    """f = x - 2 from x0 = -100 still converges to 2 in one Newton step."""
    vm = make_vm()
    f = IRApply(SUB, (_X, IRInteger(2)))
    result = vm.eval(mnewton_apply(f, _X, IRFloat(-100.0)))
    assert isinstance(result, IRFloat)
    assert abs(result.value - 2.0) < 1e-9


def test_quadratic_sqrt2_from_1_5() -> None:
    """f = x^2 - 2; root is sqrt(2) ≈ 1.41421356. Classic Newton demo."""
    vm = make_vm()
    # f(x) = x^2 - 2  =  Sub(Pow(x, 2), 2)
    f = IRApply(SUB, (IRApply(POW, (_X, IRInteger(2))), IRInteger(2)))
    result = vm.eval(mnewton_apply(f, _X, IRFloat(1.5)))
    assert isinstance(result, IRFloat)
    assert abs(result.value - math.sqrt(2)) < 1e-8


def test_quadratic_sqrt2_from_10() -> None:
    """Same f = x^2 - 2 from a further starting point."""
    vm = make_vm()
    f = IRApply(SUB, (IRApply(POW, (_X, IRInteger(2))), IRInteger(2)))
    result = vm.eval(mnewton_apply(f, _X, IRFloat(10.0)))
    assert isinstance(result, IRFloat)
    assert abs(result.value - math.sqrt(2)) < 1e-8


def test_quadratic_negative_root() -> None:
    """f = x^2 - 4; starting from x0 = -3 converges to -2."""
    vm = make_vm()
    f = IRApply(SUB, (IRApply(POW, (_X, IRInteger(2))), IRInteger(4)))
    result = vm.eval(mnewton_apply(f, _X, IRFloat(-3.0)))
    assert isinstance(result, IRFloat)
    assert abs(result.value - (-2.0)) < 1e-9


def test_quadratic_positive_root() -> None:
    """f = x^2 - 4; starting from x0 = 3 converges to +2."""
    vm = make_vm()
    f = IRApply(SUB, (IRApply(POW, (_X, IRInteger(2))), IRInteger(4)))
    result = vm.eval(mnewton_apply(f, _X, IRFloat(3.0)))
    assert isinstance(result, IRFloat)
    assert abs(result.value - 2.0) < 1e-9


def test_cubic_x3_minus_8() -> None:
    """f = x^3 - 8; real root at x = 2."""
    vm = make_vm()
    f = IRApply(SUB, (IRApply(POW, (_X, IRInteger(3))), IRInteger(8)))
    result = vm.eval(mnewton_apply(f, _X, IRFloat(1.0)))
    assert isinstance(result, IRFloat)
    assert abs(result.value - 2.0) < 1e-8


def test_cubic_different_x0() -> None:
    """f = x^3 - 8; different starting point still converges."""
    vm = make_vm()
    f = IRApply(SUB, (IRApply(POW, (_X, IRInteger(3))), IRInteger(8)))
    result = vm.eval(mnewton_apply(f, _X, IRFloat(4.0)))
    assert isinstance(result, IRFloat)
    assert abs(result.value - 2.0) < 1e-8


def test_x0_exactly_at_root() -> None:
    """If x0 is exactly the root, f(x0) = 0 and we return x0 immediately."""
    vm = make_vm()
    # f = x - 5; x0 = 5 → f(5) = 0 → converges on first check
    f = IRApply(SUB, (_X, IRInteger(5)))
    result = vm.eval(mnewton_apply(f, _X, IRFloat(5.0)))
    assert isinstance(result, IRFloat)
    assert result.value == pytest.approx(5.0)


def test_x0_as_irinteger() -> None:
    """x0 can be an IRInteger — it gets converted to float internally."""
    vm = make_vm()
    f = IRApply(SUB, (_X, IRInteger(7)))
    result = vm.eval(mnewton_apply(f, _X, IRInteger(0)))
    assert isinstance(result, IRFloat)
    assert abs(result.value - 7.0) < 1e-9


def test_x0_as_irrational() -> None:
    """x0 can be an IRRational (exact fraction)."""
    vm = make_vm()
    f = IRApply(SUB, (_X, IRInteger(2)))
    result = vm.eval(mnewton_apply(f, _X, IRRational(3, 2)))  # x0 = 1.5
    assert isinstance(result, IRFloat)
    assert abs(result.value - 2.0) < 1e-9


def test_precision_1e8() -> None:
    """Resulting root is within 1e-8 of the expected value."""
    vm = make_vm()
    # sqrt(3) via x^2 - 3 = 0
    f = IRApply(SUB, (IRApply(POW, (_X, IRInteger(2))), IRInteger(3)))
    result = vm.eval(mnewton_apply(f, _X, IRFloat(2.0)))
    assert isinstance(result, IRFloat)
    assert abs(result.value - math.sqrt(3)) < 1e-8


def test_non_symbol_var_returns_unevaluated() -> None:
    """If the 'variable' argument is not an IRSymbol, return unevaluated."""
    vm = make_vm()
    f = IRApply(SUB, (_X, IRInteger(2)))
    # Pass IRInteger(1) instead of a symbol as var — this is malformed.
    expr = IRApply(_MNEWTON, (f, IRInteger(1), IRFloat(0.0)))
    result = vm.eval(expr)
    # Should come back unevaluated (the original IRApply).
    assert result == expr


def test_non_numeric_x0_returns_unevaluated() -> None:
    """If x0 is a symbol (not numeric), return unevaluated."""
    vm = make_vm()
    f = IRApply(SUB, (_X, IRInteger(2)))
    # x0 = IRSymbol("a") — no numerical value
    expr = mnewton_apply(f, _X, IRSymbol("a"))
    result = vm.eval(expr)
    # mnewton_solve returns f_ir when x0 is not numeric,
    # so the VM wraps it back or returns the function itself.
    # The result should NOT be an IRFloat.
    assert not isinstance(result, IRFloat)


def test_wrong_arity_returns_unevaluated() -> None:
    """MNewton with 2 args (too few) returns the expression unevaluated."""
    vm = make_vm()
    f = IRApply(SUB, (_X, IRInteger(2)))
    expr = IRApply(_MNEWTON, (f, _X))  # only 2 args
    result = vm.eval(expr)
    assert result == expr


def test_wrong_arity_5_returns_unevaluated() -> None:
    """MNewton with 5 args (too many) returns the expression unevaluated."""
    vm = make_vm()
    f = IRApply(SUB, (_X, IRInteger(2)))
    expr = IRApply(_MNEWTON, (f, _X, IRFloat(0.0), IRFloat(1e-10), IRFloat(0.0)))
    result = vm.eval(expr)
    assert result == expr


def test_derivative_zero_at_x0_returns_unevaluated() -> None:
    """f = x^2 - 1, x0 = 0: f'(0) = 0, Newton step undefined → unevaluated.

    We use f = x^2 - 1 (not x^2) because x0=0 is NOT a root of f, yet
    f'(0) = 0, so the Newton step is undefined. The handler must return
    the original expression unevaluated.
    """
    vm = make_vm()
    # f(x) = x^2 - 1; f'(x) = 2x; f'(0) = 0, but f(0) = -1 ≠ 0
    f = IRApply(SUB, (IRApply(POW, (_X, IRInteger(2))), IRInteger(1)))
    expr = mnewton_apply(f, _X, IRFloat(0.0))
    result = vm.eval(expr)
    # Should NOT be an IRFloat (derivative is zero at x0=0).
    assert not isinstance(result, IRFloat)


def test_custom_tolerance() -> None:
    """MNewton(f, x, x0, tol) uses the custom tolerance."""
    vm = make_vm()
    f = IRApply(SUB, (IRApply(POW, (_X, IRInteger(2))), IRInteger(2)))
    # Very loose tolerance — should still converge with fewer iterations.
    result = vm.eval(mnewton_apply_tol(f, _X, IRFloat(1.5), 1e-4))
    assert isinstance(result, IRFloat)
    # The result should still be close to sqrt(2), just not as precisely.
    assert abs(result.value - math.sqrt(2)) < 1e-3


def test_sin_root_near_pi() -> None:
    """sin(x) has a root at x=pi. Starting near 3.0 should converge."""
    vm = make_vm()
    # f(x) = sin(x)
    f = IRApply(SIN, (_X,))
    result = vm.eval(mnewton_apply(f, _X, IRFloat(3.0)))
    assert isinstance(result, IRFloat)
    assert abs(result.value - math.pi) < 1e-8


def test_quadratic_sqrt9() -> None:
    """f = x^2 - 9; root at 3.0 from starting point 2.0."""
    vm = make_vm()
    f = IRApply(SUB, (IRApply(POW, (_X, IRInteger(2))), IRInteger(9)))
    result = vm.eval(mnewton_apply(f, _X, IRFloat(2.0)))
    assert isinstance(result, IRFloat)
    assert abs(result.value - 3.0) < 1e-9


def test_mnewton_error_raised_for_zero_derivative() -> None:
    """mnewton_solve raises MNewtonError when f'=0 at starting x (non-root).

    Use f = x^2 - 1 at x0 = 0: f(0) = -1 (not a root), f'(0) = 0 (flat
    tangent). Newton's method cannot proceed — MNewtonError is raised.
    """
    from symbolic_vm.derivative import _diff

    from cas_mnewton.newton import mnewton_solve

    vm = make_vm()
    # f = x^2 - 1; f'(0) = 0; f(0) = -1 (not a root)
    f = IRApply(SUB, (IRApply(POW, (_X, IRInteger(2))), IRInteger(1)))
    with pytest.raises(MNewtonError):
        mnewton_solve(f, _X, IRFloat(0.0), vm.eval, _diff)
