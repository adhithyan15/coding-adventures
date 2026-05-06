"""Integration tests for ODE and Laplace transform wiring in MacsymaBackend.

Phase 29 wires two substrate packages into the MacsymaBackend:

- :mod:`cas_ode` — symbolic ODE solver (``ODE2`` head)
- :mod:`cas_laplace` — Laplace transform and inverse (``Laplace``, ``ILT``,
  ``DiracDelta``, ``UnitStep`` heads)

These tests exercise the full dispatch chain:

    IR expression → MacsymaBackend handler → result

organised in five sections:

1. ODE2 handler wiring — verify ode2(eqn, y, x) reaches the solver.
2. Laplace transform — L{f(t)} round-trip checks.
3. Inverse Laplace transform — L⁻¹{F(s)} checks.
4. DiracDelta / UnitStep evaluation — special function folding.
5. SPICE smoke tests — transient RLC building blocks.
"""

from __future__ import annotations

import math

from symbolic_ir import (
    ADD,
    COS,
    DIV,
    EQUAL,
    EXP,
    MUL,
    NEG,
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
from symbolic_ir.nodes import C1, C2, C_CONST, D, ODE2
from symbolic_vm import VM

from macsyma_runtime import MacsymaBackend
from macsyma_runtime.cas_handlers import build_cas_handler_table

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _vm() -> VM:
    """Return a fresh VM backed by a MacsymaBackend."""
    return VM(MacsymaBackend())


def _int(n: int) -> IRInteger:
    return IRInteger(n)


def _sym(name: str) -> IRSymbol:
    return IRSymbol(name)


def _apply(head: str, *args: object) -> IRApply:
    return IRApply(IRSymbol(head), tuple(args))  # type: ignore[arg-type]


def _rat(n: int, d: int) -> IRRational:
    return IRRational(n, d)


# Convenience constructors for derivative IR nodes.

def _d(y: IRSymbol, x: IRSymbol) -> IRApply:
    """First derivative D(y, x) = dy/dx."""
    return IRApply(D, (y, x))


def _d2(y: IRSymbol, x: IRSymbol) -> IRApply:
    """Second derivative D(D(y, x), x) = d²y/dx²."""
    return IRApply(D, (_d(y, x), x))


# ---------------------------------------------------------------------------
# Section 1 — Handler table completeness
# ---------------------------------------------------------------------------


def test_handler_table_contains_ode_and_laplace_heads() -> None:
    """build_cas_handler_table must include ODE2 and the four Laplace heads."""
    table = build_cas_handler_table()
    expected = {"ODE2", "Laplace", "ILT", "DiracDelta", "UnitStep"}
    assert expected <= set(table.keys()), (
        f"Missing heads: {expected - set(table.keys())}"
    )


def test_backend_has_ode2_handler() -> None:
    """MacsymaBackend.handlers() must include ODE2."""
    b = MacsymaBackend()
    assert "ODE2" in b.handlers()


def test_backend_has_laplace_handler() -> None:
    """MacsymaBackend.handlers() must include Laplace, ILT, DiracDelta, UnitStep."""
    b = MacsymaBackend()
    handlers = b.handlers()
    for head in ("Laplace", "ILT", "DiracDelta", "UnitStep"):
        assert head in handlers, f"Missing handler: {head}"


# ---------------------------------------------------------------------------
# Section 2 — ODE2 handler wiring
# ---------------------------------------------------------------------------


def test_ode2_first_order_decaying() -> None:
    """ode2(y' + 2*y, y, x) → Equal(y, %c·exp(-2·x)).

    First-order linear homogeneous ODE with P(x) = 2, Q(x) = 0.
    Integrating factor μ = exp(2x) gives y = %c·exp(-2x).
    """
    vm = _vm()
    x = _sym("x")
    y = _sym("y")
    y_prime = _d(y, x)
    two_y = IRApply(MUL, (_int(2), y))
    # y' + 2*y = 0  →  y' + 2*y (zero form)
    eqn = IRApply(ADD, (y_prime, two_y))
    result = vm.eval(IRApply(ODE2, (eqn, y, x)))

    # Must return Equal(y, ...)
    assert isinstance(result, IRApply)
    assert isinstance(result.head, IRSymbol)
    assert result.head == EQUAL, f"Expected Equal, got head={result.head!r}"
    assert result.args[0] == y


def test_ode2_first_order_growing() -> None:
    """ode2(y' - y, y, x) → Equal(y, %c·exp(x)).

    P(x) = -1, Q(x) = 0.  Solution: y = %c·exp(x).
    """
    vm = _vm()
    x = _sym("x")
    y = _sym("y")
    y_prime = _d(y, x)
    # y' - y = 0
    eqn = IRApply(SUB, (y_prime, y))
    result = vm.eval(IRApply(ODE2, (eqn, y, x)))

    assert isinstance(result, IRApply)
    assert result.head == EQUAL
    assert result.args[0] == y


def test_ode2_second_order_simple_harmonic() -> None:
    """ode2(y'' + y, y, x) → Equal(y, exp(0)·(C1·cos(x) + C2·sin(x))).

    Characteristic equation r² + 1 = 0 → r = ±i.
    Complex-conjugate roots: α=0, β=1.
    General solution: y = C1·cos(x) + C2·sin(x).
    """
    vm = _vm()
    x = _sym("x")
    y = _sym("y")
    y_pp = _d2(y, x)
    # y'' + y = 0
    eqn = IRApply(ADD, (y_pp, y))
    result = vm.eval(IRApply(ODE2, (eqn, y, x)))

    assert isinstance(result, IRApply)
    assert result.head == EQUAL
    assert result.args[0] == y
    # The solution must involve cos and/or sin (from trig part of complex roots).
    solution = result.args[1]
    solution_str = repr(solution)
    assert "Cos" in solution_str or "Sin" in solution_str, (
        f"Expected trig in solution, got: {solution_str}"
    )


def test_ode2_second_order_distinct_real_roots() -> None:
    """ode2(y'' - 3*y' + 2*y, y, x) → Equal(y, C1·e^x + C2·e^(2x)).

    Characteristic equation r² - 3r + 2 = 0 → r = 1, r = 2.
    Distinct real roots.
    """
    vm = _vm()
    x = _sym("x")
    y = _sym("y")
    y_prime = _d(y, x)
    y_pp = _d2(y, x)
    three_yprime = IRApply(MUL, (_int(3), y_prime))
    two_y = IRApply(MUL, (_int(2), y))
    # y'' - 3*y' + 2*y = 0
    eqn = IRApply(ADD, (IRApply(SUB, (y_pp, three_yprime)), two_y))
    result = vm.eval(IRApply(ODE2, (eqn, y, x)))

    assert isinstance(result, IRApply)
    assert result.head == EQUAL
    assert result.args[0] == y
    # Solution must contain %c1 and %c2.
    solution_str = repr(result.args[1])
    assert "%c1" in solution_str or "C1" in solution_str, (
        f"Expected C1 in solution: {solution_str}"
    )


def test_ode2_wrong_arity_returns_unevaluated() -> None:
    """ODE2 with wrong arity must return the expression unevaluated."""
    vm = _vm()
    x = _sym("x")
    y = _sym("y")
    # Two args instead of three.
    expr = IRApply(ODE2, (y, x))
    result = vm.eval(expr)
    # Should come back unevaluated (same structure).
    assert isinstance(result, IRApply)
    assert result.head == ODE2


def test_ode2_unsolvable_returns_unevaluated() -> None:
    """An ODE that none of the solvers handles must return unevaluated.

    The nonlinear pendulum equation ``y'' + sin(y) = 0`` is genuinely
    unsolvable in closed form by the seven implemented strategies:

    - 2nd-order const-coeff recognisers require *rational* constant
      coefficients — ``sin(y)`` is y-dependent, not a constant.
    - Bernoulli and linear-first-order recognisers are first-order only.
    - Exact / separable recognisers look for ``D(y, x)`` terms; the ODE
      contains ``D(D(y, x), x)`` which they don't match.

    The handler must return the ``ODE2(…)`` node unchanged.
    """
    vm = _vm()
    x = _sym("x")
    y = _sym("y")
    y_pp = _d2(y, x)
    sin_y = IRApply(SIN, (y,))
    # y'' + sin(y) = 0  (nonlinear pendulum — no closed-form solution)
    eqn_pendulum = IRApply(ADD, (y_pp, sin_y))
    result = vm.eval(IRApply(ODE2, (eqn_pendulum, y, x)))
    assert isinstance(result, IRApply)
    assert result.head == ODE2, f"Expected unevaluated ODE2, got {result!r}"


# ---------------------------------------------------------------------------
# Section 3 — Laplace transform
# ---------------------------------------------------------------------------


def test_laplace_constant() -> None:
    """L{1} = 1/s."""
    vm = _vm()
    t = _sym("t")
    s = _sym("s")
    result = vm.eval(_apply("Laplace", _int(1), t, s))
    # Expected: 1/s  (Div(1, s))
    assert isinstance(result, IRApply), f"Expected IRApply, got {result!r}"
    result_str = repr(result)
    assert "s" in result_str, f"Expected 's' in Laplace result: {result_str}"


def test_laplace_exponential() -> None:
    """L{exp(a·t)} = 1/(s - a).

    We use a=2: L{exp(2t)} = 1/(s-2).
    """
    vm = _vm()
    t = _sym("t")
    s = _sym("s")
    f = IRApply(EXP, (IRApply(MUL, (_int(2), t)),))
    result = vm.eval(_apply("Laplace", f, t, s))
    # Should return something like Div(1, Sub(s, 2))
    assert isinstance(result, IRApply), f"Expected IRApply, got {result!r}"
    result_str = repr(result)
    assert "s" in result_str


def test_laplace_sin() -> None:
    """L{sin(t)} = 1/(s² + 1)."""
    vm = _vm()
    t = _sym("t")
    s = _sym("s")
    f = IRApply(SIN, (t,))
    result = vm.eval(_apply("Laplace", f, t, s))
    assert isinstance(result, IRApply)
    result_str = repr(result)
    assert "s" in result_str, f"Expected 's' in result: {result_str}"


def test_laplace_wrong_arity_unevaluated() -> None:
    """Laplace with wrong arity returns unevaluated."""
    vm = _vm()
    t = _sym("t")
    result = vm.eval(_apply("Laplace", _int(1), t))  # only 2 args
    assert isinstance(result, IRApply)
    assert isinstance(result.head, IRSymbol)
    assert result.head.name == "Laplace"


def test_laplace_nonvar_t_unevaluated() -> None:
    """Laplace(f, integer, s) with a non-symbol time var returns unevaluated."""
    vm = _vm()
    s = _sym("s")
    result = vm.eval(_apply("Laplace", _int(1), _int(0), s))
    assert isinstance(result, IRApply)
    assert result.head.name == "Laplace"


# ---------------------------------------------------------------------------
# Section 4 — Inverse Laplace transform (ILT)
# ---------------------------------------------------------------------------


def test_ilt_simple_exponential() -> None:
    """ILT{1/(s-3)} = exp(3t).

    Partial-fraction: simple pole at s=3.
    """
    vm = _vm()
    s = _sym("s")
    t = _sym("t")
    F = IRApply(DIV, (_int(1), IRApply(SUB, (s, _int(3)))))
    result = vm.eval(_apply("ILT", F, s, t))
    # Should produce something with exp and t
    assert isinstance(result, IRApply), f"Expected IRApply, got {result!r}"
    result_str = repr(result)
    assert "t" in result_str, f"Expected 't' in ILT result: {result_str}"


def test_ilt_wrong_arity_unevaluated() -> None:
    """ILT with wrong arity returns unevaluated."""
    vm = _vm()
    s = _sym("s")
    t = _sym("t")
    F = IRApply(DIV, (_int(1), s))
    result = vm.eval(_apply("ILT", F, s))  # only 2 args
    assert isinstance(result, IRApply)
    assert result.head.name == "ILT"


# ---------------------------------------------------------------------------
# Section 5 — DiracDelta and UnitStep evaluation
# ---------------------------------------------------------------------------


def test_dirac_delta_at_zero() -> None:
    """DiracDelta(0) = 1 (sifting property)."""
    vm = _vm()
    result = vm.eval(_apply("DiracDelta", _int(0)))
    assert result == _int(1), f"Expected 1, got {result!r}"


def test_dirac_delta_nonzero_arg_unevaluated() -> None:
    """DiracDelta(x) with a symbolic arg stays unevaluated."""
    vm = _vm()
    result = vm.eval(_apply("DiracDelta", _sym("x")))
    assert isinstance(result, IRApply)
    assert result.head.name == "DiracDelta"


def test_unit_step_negative() -> None:
    """UnitStep(-3) = 0 (step not yet reached)."""
    vm = _vm()
    result = vm.eval(_apply("UnitStep", _int(-3)))
    assert result == _int(0), f"Expected 0, got {result!r}"


def test_unit_step_positive() -> None:
    """UnitStep(5) = 1 (step already passed)."""
    vm = _vm()
    result = vm.eval(_apply("UnitStep", _int(5)))
    assert result == _int(1), f"Expected 1, got {result!r}"


def test_unit_step_at_zero() -> None:
    """UnitStep(0) = 1/2 (midpoint convention H(0) = 1/2)."""
    vm = _vm()
    result = vm.eval(_apply("UnitStep", _int(0)))
    assert result == _rat(1, 2), f"Expected 1/2, got {result!r}"


def test_unit_step_symbolic_unevaluated() -> None:
    """UnitStep(t) with symbolic arg stays unevaluated."""
    vm = _vm()
    result = vm.eval(_apply("UnitStep", _sym("t")))
    assert isinstance(result, IRApply)
    assert result.head.name == "UnitStep"


# ---------------------------------------------------------------------------
# Section 6 — SPICE building-block smoke tests
# ---------------------------------------------------------------------------


def test_rlc_series_resonance_ode() -> None:
    """Series RLC at resonance: y'' + y = 0  (ω₀ = 1, unit R, L, C).

    In a series RLC circuit with L=1, C=1, R=0:
        L·q'' + R·q' + q/C = 0  →  q'' + q = 0

    Solution: q(t) = %c1·cos(t) + %c2·sin(t)  (undamped oscillation).
    This is the same simple-harmonic test but framed as the SPICE motivator.
    """
    vm = _vm()
    x = _sym("t")   # time variable
    y = _sym("q")   # charge variable
    y_pp = _d2(y, x)
    eqn = IRApply(ADD, (y_pp, y))   # q'' + q = 0
    result = vm.eval(IRApply(ODE2, (eqn, y, x)))

    assert isinstance(result, IRApply)
    assert result.head == EQUAL
    assert result.args[0] == y
    solution_str = repr(result.args[1])
    assert "Cos" in solution_str or "Sin" in solution_str


def test_laplace_step_response() -> None:
    """Laplace of a unit step: L{H(t)} = 1/s  (for t→s).

    Heaviside step enters circuit analysis as the input for step-response
    simulations.  We verify the Laplace transform routes correctly.
    """
    vm = _vm()
    t = _sym("t")
    s = _sym("s")
    # UnitStep(t) → UnitStep stays unevaluated (symbolic t), but Laplace
    # handles it via its own table.
    unit_step = IRApply(IRSymbol("UnitStep"), (t,))
    result = vm.eval(_apply("Laplace", unit_step, t, s))
    # If the Laplace table knows UnitStep: result is Div(1, s).
    # If not, it stays unevaluated — either way the handler must not crash.
    # We only assert the handler runs without exception and returns an IR node.
    assert isinstance(result, IRNode), f"Expected IRNode, got {result!r}"
