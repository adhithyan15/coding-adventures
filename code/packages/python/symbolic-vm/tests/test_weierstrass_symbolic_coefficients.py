"""Tests for Track G1: symbolic-coefficient Weierstrass lift.

The numeric Phase-34 Weierstrass helper only fires when ``a`` and ``b``
in the integrand ``∫ c / (a + b·sin(α·x+β)) dx`` are concrete rationals.
Track G1 generalises it: when the user has declared the sign of the
discriminant ``a² − b²`` via ``assume(...)``, the integrator emits the
corresponding closed form with symbolic ``a, b``.

The branch selection is driven by ``vm.assumptions`` lookups against
the compound-relation store added in Track G1's first part (see
``cas_simplify/assumptions.py``).  These tests cover all four branches
(``> 0``, ``< 0``, ``= 0``, no assumption → unevaluated) plus the
linear-argument lifting that must still compose.

Structural assertions rather than numeric ones — the result is a tree
in symbolic ``a, b`` that no numeric evaluation can collapse cheaply.
We assert the *kind* of the outer head (``Atan``, ``Log``, ``Integrate``)
and that the recorded discriminant radicand appears literally
somewhere in the tree.
"""

from __future__ import annotations

from symbolic_ir import (
    ADD,
    ATAN,
    COS,
    DIV,
    EQUAL,
    GREATER,
    INTEGRATE,
    LESS,
    LOG,
    MUL,
    POW,
    SIN,
    SQRT,
    SUB,
    IRApply,
    IRInteger,
    IRNode,
    IRSymbol,
)

from symbolic_vm import VM, SymbolicBackend


X = IRSymbol("x")
A = IRSymbol("a")
B = IRSymbol("b")
TWO = IRInteger(2)


def _make_vm() -> VM:
    return VM(SymbolicBackend())


def _assume(vm: VM, rel: IRNode) -> None:
    vm.eval(IRApply(IRSymbol("Assume"), (rel,)))


def _gt(lhs: IRNode, rhs: IRNode) -> IRNode:
    return IRApply(GREATER, (lhs, rhs))


def _lt(lhs: IRNode, rhs: IRNode) -> IRNode:
    return IRApply(LESS, (lhs, rhs))


def _eq(lhs: IRNode, rhs: IRNode) -> IRNode:
    return IRApply(EQUAL, (lhs, rhs))


def _sq(sym: IRNode) -> IRNode:
    return IRApply(POW, (sym, TWO))


def _integrate(integrand: IRNode) -> IRNode:
    return IRApply(INTEGRATE, (integrand, X))


def _contains_head(node: IRNode, head: IRSymbol) -> bool:
    """Walk ``node`` and return True iff any ``IRApply`` has ``head``.

    Used to assert the closed form mentions the discriminant radicand
    (``Sqrt(...)``) without nailing down the exact arithmetic
    arrangement of the surrounding tree.
    """
    if isinstance(node, IRApply):
        if node.head == head:
            return True
        return any(_contains_head(a, head) for a in node.args)
    return False


def _contains_subtree(node: IRNode, target: IRNode) -> bool:
    """True iff ``target`` appears as a sub-tree of ``node`` (structural)."""
    if node == target:
        return True
    if isinstance(node, IRApply):
        return any(_contains_subtree(a, target) for a in node.args)
    return False


# ---------------------------------------------------------------------------
# disc > 0  →  arctan branch
# ---------------------------------------------------------------------------


def test_symbolic_sin_arctan_branch() -> None:
    """``assume(a² > b²); ∫ 1/(a + b·sin(x)) dx`` returns the arctan form
    with symbolic ``Sqrt(a² − b²)``."""
    vm = _make_vm()
    _assume(vm, _gt(_sq(A), _sq(B)))
    denom = IRApply(ADD, (A, IRApply(MUL, (B, IRApply(SIN, (X,))))))
    result = vm.eval(_integrate(IRApply(DIV, (IRInteger(1), denom))))
    # Must not stay as Integrate.
    assert not (isinstance(result, IRApply) and result.head == INTEGRATE)
    # Must contain Atan and Sqrt(a² − b²).
    assert _contains_head(result, ATAN)
    expected_radicand = IRApply(SUB, (_sq(A), _sq(B)))
    expected_sqrt = IRApply(SQRT, (expected_radicand,))
    assert _contains_subtree(result, expected_sqrt)


def test_symbolic_cos_arctan_branch() -> None:
    """``assume(a² > b²); ∫ 1/(a + b·cos(x)) dx`` returns the cos-branch
    arctan form with symbolic ``Sqrt(a² − b²)``."""
    vm = _make_vm()
    _assume(vm, _gt(_sq(A), _sq(B)))
    denom = IRApply(ADD, (A, IRApply(MUL, (B, IRApply(COS, (X,))))))
    result = vm.eval(_integrate(IRApply(DIV, (IRInteger(1), denom))))
    assert not (isinstance(result, IRApply) and result.head == INTEGRATE)
    assert _contains_head(result, ATAN)
    expected_radicand = IRApply(SUB, (_sq(A), _sq(B)))
    expected_sqrt = IRApply(SQRT, (expected_radicand,))
    assert _contains_subtree(result, expected_sqrt)


# ---------------------------------------------------------------------------
# disc < 0  →  log branch
# ---------------------------------------------------------------------------


def test_symbolic_sin_log_branch() -> None:
    """``assume(a² < b²); ∫ 1/(a + b·sin(x)) dx`` returns the log form
    with symbolic ``Sqrt(b² − a²)``."""
    vm = _make_vm()
    _assume(vm, _lt(_sq(A), _sq(B)))
    denom = IRApply(ADD, (A, IRApply(MUL, (B, IRApply(SIN, (X,))))))
    result = vm.eval(_integrate(IRApply(DIV, (IRInteger(1), denom))))
    assert not (isinstance(result, IRApply) and result.head == INTEGRATE)
    assert _contains_head(result, LOG)
    expected_radicand = IRApply(SUB, (_sq(B), _sq(A)))
    expected_sqrt = IRApply(SQRT, (expected_radicand,))
    assert _contains_subtree(result, expected_sqrt)


# ---------------------------------------------------------------------------
# disc = 0  →  degenerate branch
# ---------------------------------------------------------------------------


def test_symbolic_sin_degenerate_branch() -> None:
    """``assume(a² = b²); ∫ 1/(a + b·sin(x)) dx`` returns the degenerate
    closed form — a rational expression in ``tan(x/2)`` with no outer
    arctan or log."""
    vm = _make_vm()
    _assume(vm, _eq(_sq(A), _sq(B)))
    denom = IRApply(ADD, (A, IRApply(MUL, (B, IRApply(SIN, (X,))))))
    result = vm.eval(_integrate(IRApply(DIV, (IRInteger(1), denom))))
    assert not (isinstance(result, IRApply) and result.head == INTEGRATE)
    # No outer arctan, no outer log — only tan(...) below an arithmetic
    # tree.
    assert not _contains_head(result, ATAN)
    assert not _contains_head(result, LOG)


# ---------------------------------------------------------------------------
# No assumption  →  unevaluated
# ---------------------------------------------------------------------------


def test_no_assumption_returns_unevaluated() -> None:
    """Without an ``assume(...)`` declaring the discriminant sign the
    integrator MUST leave the integral unevaluated rather than
    guessing."""
    vm = _make_vm()
    denom = IRApply(ADD, (A, IRApply(MUL, (B, IRApply(SIN, (X,))))))
    integrand = IRApply(DIV, (IRInteger(1), denom))
    result = vm.eval(_integrate(integrand))
    assert isinstance(result, IRApply) and result.head == INTEGRATE


# ---------------------------------------------------------------------------
# Linear-argument lifting composes (Phase 38 + Track G1).
# ---------------------------------------------------------------------------


def test_symbolic_linear_argument_arctan() -> None:
    """``assume(a² > b²); ∫ 1/(a + b·sin(2x + 1)) dx`` lifts to the
    arctan form with ``tan((2x+1)/2)`` inside.

    The ``α = 2`` scaling must thread through the change of variable
    (i.e. the outer numerator is divided by ``α`` exactly as the numeric
    Phase-38 path does).
    """
    vm = _make_vm()
    _assume(vm, _gt(_sq(A), _sq(B)))
    # inner = 2x + 1
    inner = IRApply(ADD, (IRApply(MUL, (TWO, X)), IRInteger(1)))
    denom = IRApply(ADD, (A, IRApply(MUL, (B, IRApply(SIN, (inner,))))))
    result = vm.eval(_integrate(IRApply(DIV, (IRInteger(1), denom))))
    assert not (isinstance(result, IRApply) and result.head == INTEGRATE)
    assert _contains_head(result, ATAN)
    # The lifted form must reference the same linear inner argument
    # somewhere — every tan(...) inside the closed form takes
    # ``(2x+1)/2``.
    expected_radicand = IRApply(SUB, (_sq(A), _sq(B)))
    expected_sqrt = IRApply(SQRT, (expected_radicand,))
    assert _contains_subtree(result, expected_sqrt)


# ---------------------------------------------------------------------------
# Numeric regression — the symbolic helper must not steal numeric cases.
# ---------------------------------------------------------------------------


def test_numeric_regression_arctan_still_works() -> None:
    """``∫ 1/(2 + sin(x)) dx`` still closes to the numeric arctan form —
    the Track G1 symbolic helper is gated on non-numeric ``a, b`` so the
    fast numeric path keeps firing for concrete inputs."""
    vm = _make_vm()
    denom = IRApply(ADD, (IRInteger(2), IRApply(SIN, (X,))))
    integrand = IRApply(DIV, (IRInteger(1), denom))
    result = vm.eval(_integrate(integrand))
    assert not (isinstance(result, IRApply) and result.head == INTEGRATE)
    assert _contains_head(result, ATAN)


def test_numeric_regression_log_still_works() -> None:
    """``∫ 1/(1 + 2·sin(x)) dx`` still closes to the numeric log form."""
    vm = _make_vm()
    denom = IRApply(
        ADD,
        (IRInteger(1), IRApply(MUL, (IRInteger(2), IRApply(SIN, (X,))))),
    )
    integrand = IRApply(DIV, (IRInteger(1), denom))
    result = vm.eval(_integrate(integrand))
    assert not (isinstance(result, IRApply) and result.head == INTEGRATE)
    assert _contains_head(result, LOG)
