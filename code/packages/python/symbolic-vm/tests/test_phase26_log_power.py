"""Tests for Phase 26: log-power integration via IBP reduction.

Phase 26 adds two new patterns to the ``Integrate`` handler:

1. ``∫ log(ax+b)^n dx`` — pure log-power (POW(LOG, n) head, n ≥ 2).
   Uses the reduction formula:
       F_n(x) = (ax+b)/a · log(ax+b)^n  −  n · F_{n-1}(x)
   with base case F_0(x) = x.

2. ``∫ Q(x) · log(x)^n dx`` — polynomial × log-power (MUL head, n ≥ 2).
   Uses term-by-term linearity and the reduction:
       ∫ x^k · log(x)^n dx = x^(k+1)/(k+1) · log(x)^n
                              − n/(k+1) · ∫ x^k · log(x)^(n-1) dx

Both patterns recurse down to n = 1 or n = 0, which are handled by existing
rules (``_try_log_product`` / elementary-function table).
"""

from __future__ import annotations

import math

import pytest
from symbolic_ir import (
    ADD,
    INTEGRATE,
    LOG,
    MUL,
    NEG,
    POW,
    SUB,
    IRApply,
    IRInteger,
    IRRational,
    IRSymbol,
)

from symbolic_vm import VM, SymbolicBackend

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

X = IRSymbol("x")


@pytest.fixture
def vm() -> VM:
    return VM(SymbolicBackend())


def _integrate(f: IRApply) -> IRApply:
    return IRApply(INTEGRATE, (f, X))


def _contains_head(node: IRApply, head: IRSymbol) -> bool:
    """Recursively check whether any sub-node has the given head."""
    if isinstance(node, IRApply):
        if node.head == head:
            return True
        return any(_contains_head(a, head) for a in node.args)
    return False


def _logx_pow(n: int) -> IRApply:
    """Return ``POW(LOG(x), n)`` IR node."""
    return IRApply(POW, (IRApply(LOG, (X,)), IRInteger(n)))


def _eval_ir(node: IRApply, x_val: float) -> float:
    """Numerically evaluate an IR tree by substitution (simple cases only)."""
    if isinstance(node, IRInteger):
        return float(node.value)
    if isinstance(node, IRRational):
        return node.numer / node.denom
    if isinstance(node, IRSymbol):
        if node == X:
            return x_val
        raise ValueError(f"unknown symbol {node}")
    assert isinstance(node, IRApply)
    h = node.head
    if h == ADD:
        return sum(_eval_ir(a, x_val) for a in node.args)
    if h == SUB:
        a, b = node.args
        return _eval_ir(a, x_val) - _eval_ir(b, x_val)
    if h == MUL:
        result = 1.0
        for a in node.args:
            result *= _eval_ir(a, x_val)
        return result
    if h == NEG:
        return -_eval_ir(node.args[0], x_val)
    if h == POW:
        base, exp = node.args
        return _eval_ir(base, x_val) ** _eval_ir(exp, x_val)
    from symbolic_ir import DIV
    if h == DIV:
        a, b = node.args
        return _eval_ir(a, x_val) / _eval_ir(b, x_val)
    if h == LOG:
        return math.log(_eval_ir(node.args[0], x_val))
    raise ValueError(f"unsupported head {h}")


def _numerical_definite(integrand_fn, a: float, b: float, n: int = 10_000) -> float:
    """Simple trapezoidal rule for ground-truth comparison."""
    h = (b - a) / n
    total = 0.5 * (integrand_fn(a) + integrand_fn(b))
    for i in range(1, n):
        total += integrand_fn(a + i * h)
    return total * h


# ---------------------------------------------------------------------------
# Test 1 — log(x)^2 is no longer unevaluated
# ---------------------------------------------------------------------------


def test_log_squared_is_closed(vm: VM) -> None:
    """``∫ log(x)^2 dx`` must return a closed form (no ``Integrate`` head)."""
    out = vm.eval(_integrate(_logx_pow(2)))
    assert not _contains_head(out, INTEGRATE), (
        f"expected closed form, got unevaluated: {out}"
    )
    assert _contains_head(out, LOG), f"expected LOG in result, got: {out}"


# ---------------------------------------------------------------------------
# Test 2 — log(x)^2 numerical correctness
# ---------------------------------------------------------------------------


def test_log_squared_numeric(vm: VM) -> None:
    """``∫ log(x)^2 dx = x·log(x)^2 - 2x·log(x) + 2x``.

    Verify ∫₁^2 log(x)^2 dx by comparing the antiderivative difference
    F(2) − F(1) with the trapezoidal numerical integral.
    """
    out = vm.eval(_integrate(_logx_pow(2)))
    f2 = _eval_ir(out, 2.0)
    f1 = _eval_ir(out, 1.0)
    antideriv_diff = f2 - f1

    numerical = _numerical_definite(lambda xv: math.log(xv) ** 2, 1.0, 2.0)
    assert abs(antideriv_diff - numerical) < 1e-5, (
        f"antiderivative diff {antideriv_diff:.8f} != numerical {numerical:.8f}"
    )


# ---------------------------------------------------------------------------
# Test 3 — log(x)^3 is closed
# ---------------------------------------------------------------------------


def test_log_cubed_is_closed(vm: VM) -> None:
    """``∫ log(x)^3 dx`` must return a closed form."""
    out = vm.eval(_integrate(_logx_pow(3)))
    assert not _contains_head(out, INTEGRATE), f"expected closed form, got: {out}"
    assert _contains_head(out, LOG), f"expected LOG in result, got: {out}"


# ---------------------------------------------------------------------------
# Test 4 — log(x)^3 numerical correctness
# ---------------------------------------------------------------------------


def test_log_cubed_numeric(vm: VM) -> None:
    """Verify ``∫₁^3 log(x)^3 dx`` via antiderivative difference."""
    out = vm.eval(_integrate(_logx_pow(3)))
    f3 = _eval_ir(out, 3.0)
    f1 = _eval_ir(out, 1.0)
    antideriv_diff = f3 - f1

    numerical = _numerical_definite(lambda xv: math.log(xv) ** 3, 1.0, 3.0)
    assert abs(antideriv_diff - numerical) < 1e-4, (
        f"antiderivative diff {antideriv_diff:.8f} != numerical {numerical:.8f}"
    )


# ---------------------------------------------------------------------------
# Test 5 — x * log(x)^2 is closed
# ---------------------------------------------------------------------------


def test_x_times_log_squared_is_closed(vm: VM) -> None:
    """``∫ x·log(x)^2 dx`` must return a closed form."""
    integrand = IRApply(MUL, (X, _logx_pow(2)))
    out = vm.eval(_integrate(integrand))
    assert not _contains_head(out, INTEGRATE), f"expected closed form, got: {out}"
    assert _contains_head(out, LOG), f"expected LOG in result, got: {out}"


# ---------------------------------------------------------------------------
# Test 6 — x * log(x)^2 numerical correctness
# ---------------------------------------------------------------------------


def test_x_times_log_squared_numeric(vm: VM) -> None:
    """Verify ``∫₁^2 x·log(x)^2 dx`` via antiderivative difference."""
    integrand = IRApply(MUL, (X, _logx_pow(2)))
    out = vm.eval(_integrate(integrand))
    f2 = _eval_ir(out, 2.0)
    f1 = _eval_ir(out, 1.0)
    antideriv_diff = f2 - f1

    numerical = _numerical_definite(lambda xv: xv * math.log(xv) ** 2, 1.0, 2.0)
    assert abs(antideriv_diff - numerical) < 1e-5, (
        f"antiderivative diff {antideriv_diff:.8f} != numerical {numerical:.8f}"
    )


# ---------------------------------------------------------------------------
# Test 7 — x^2 * log(x)^2 is closed
# ---------------------------------------------------------------------------


def test_x_squared_times_log_squared_is_closed(vm: VM) -> None:
    """``∫ x^2·log(x)^2 dx`` must return a closed form."""
    x_sq = IRApply(POW, (X, IRInteger(2)))
    integrand = IRApply(MUL, (x_sq, _logx_pow(2)))
    out = vm.eval(_integrate(integrand))
    assert not _contains_head(out, INTEGRATE), f"expected closed form, got: {out}"
    assert _contains_head(out, LOG), f"expected LOG in result, got: {out}"


# ---------------------------------------------------------------------------
# Test 8 — x^2 * log(x)^2 numerical correctness
# ---------------------------------------------------------------------------


def test_x_squared_times_log_squared_numeric(vm: VM) -> None:
    """Verify ``∫₁^2 x^2·log(x)^2 dx`` via antiderivative difference."""
    x_sq = IRApply(POW, (X, IRInteger(2)))
    integrand = IRApply(MUL, (x_sq, _logx_pow(2)))
    out = vm.eval(_integrate(integrand))
    f2 = _eval_ir(out, 2.0)
    f1 = _eval_ir(out, 1.0)
    antideriv_diff = f2 - f1

    numerical = _numerical_definite(lambda xv: xv**2 * math.log(xv) ** 2, 1.0, 2.0)
    assert abs(antideriv_diff - numerical) < 1e-4, (
        f"antiderivative diff {antideriv_diff:.8f} != numerical {numerical:.8f}"
    )


# ---------------------------------------------------------------------------
# Test 9 — log(2x+1)^2 (linear shift) is closed
# ---------------------------------------------------------------------------


def test_log_linear_squared_is_closed(vm: VM) -> None:
    """``∫ log(2x+1)^2 dx`` — linear shift is handled by the reduction formula."""
    from symbolic_ir import ADD as _ADD
    arg = IRApply(_ADD, (IRApply(MUL, (IRInteger(2), X)), IRInteger(1)))  # 2x+1
    integrand = IRApply(POW, (IRApply(LOG, (arg,)), IRInteger(2)))
    out = vm.eval(_integrate(integrand))
    assert not _contains_head(out, INTEGRATE), f"expected closed form, got: {out}"
    assert _contains_head(out, LOG), f"expected LOG in result, got: {out}"


# ---------------------------------------------------------------------------
# Test 10 — log(2x+1)^2 numerical correctness
# ---------------------------------------------------------------------------


def test_log_linear_squared_numeric(vm: VM) -> None:
    """Verify ``∫₀^1 log(2x+1)^2 dx`` via antiderivative difference."""
    from symbolic_ir import ADD as _ADD
    arg = IRApply(_ADD, (IRApply(MUL, (IRInteger(2), X)), IRInteger(1)))
    integrand = IRApply(POW, (IRApply(LOG, (arg,)), IRInteger(2)))
    out = vm.eval(_integrate(integrand))
    f1 = _eval_ir(out, 1.0)
    f0 = _eval_ir(out, 0.0)
    antideriv_diff = f1 - f0

    numerical = _numerical_definite(lambda xv: math.log(2 * xv + 1) ** 2, 0.0, 1.0)
    assert abs(antideriv_diff - numerical) < 1e-4, (
        f"antiderivative diff {antideriv_diff:.8f} != numerical {numerical:.8f}"
    )


# ---------------------------------------------------------------------------
# Test 11 — log(x)^4 is closed (depth-4 recursion)
# ---------------------------------------------------------------------------


def test_log_pow4_is_closed(vm: VM) -> None:
    """``∫ log(x)^4 dx`` must return a closed form (four-level recursion)."""
    out = vm.eval(_integrate(_logx_pow(4)))
    assert not _contains_head(out, INTEGRATE), f"expected closed form, got: {out}"
    assert _contains_head(out, LOG), f"expected LOG in result, got: {out}"


# ---------------------------------------------------------------------------
# Test 12 — log(x)^1 regression: still handled by existing Phase 3 rule
# ---------------------------------------------------------------------------


def test_log_linear_regression(vm: VM) -> None:
    """``∫ log(x) dx = x·log(x) - x`` still works after Phase 26 is added.

    Phase 26 only fires for n ≥ 2; n = 1 must remain on the existing
    elementary-function path to avoid double-coverage.
    """
    log_x = IRApply(LOG, (X,))
    out = vm.eval(_integrate(log_x))
    assert not _contains_head(out, INTEGRATE), f"expected closed form, got: {out}"
    # Structural: should be x·log(x) - x (or equivalent SUB/MUL shape).
    assert _contains_head(out, LOG), f"expected LOG in result, got: {out}"
    # Numerical spot check at x = e: F(e) - F(1) = 1.
    fe = _eval_ir(out, math.e)
    f1 = _eval_ir(out, 1.0)
    assert abs((fe - f1) - 1.0) < 1e-9, (
        f"∫₁^e log(x) dx should be 1, got {fe - f1}"
    )
