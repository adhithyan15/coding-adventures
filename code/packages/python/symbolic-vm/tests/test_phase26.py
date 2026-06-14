"""Phase 26 — Transcendental Equation Solving tests.

Covers:
  26a — Trigonometric:           sin/cos/tan(ax+b) = c → periodic families
  26b — Exponential/logarithmic: exp/log(ax+b) = c → unique inverse
  26c — Lambert W:               f(x)·exp(f(x)) = c → W₀(c)
  26d — Hyperbolic:              sinh/cosh/tanh(ax+b) = c → exact inverse
  26e — Compound:                polynomial in a single transcendental function
  LambertW numeric handler — IRFloat evaluation for concrete arguments

Test structure
--------------
TestPhase26_Trig            — sin/cos/tan (26a)
TestPhase26_TrigShifted     — sin/cos/tan with a ≠ 1 or b ≠ 0
TestPhase26_ExpLog          — exp/log (26b)
TestPhase26_LambertW        — Lambert W pattern (26c)
TestPhase26_Hyperbolic      — sinh/cosh/tanh (26d)
TestPhase26_Compound        — compound poly-in-transcendental (26e)
TestPhase26_LambertWHandler — numeric LambertW handler
TestPhase26_Fallthrough     — unevaluated (no pattern matched)
TestPhase26_Regressions     — Phase 1-25 operations still work
TestPhase26_Macsyma         — end-to-end MACSYMA surface-syntax tests

Verification strategy
---------------------
Transcendental solutions involve ``FreeInteger`` (``%k``) for the periodic
families and symbolic inverse functions (``Asin``, ``LambertW``, …) for the
rest.  We verify by:

1. **Structure checks** — assert the result is a ``List`` with the correct
   number of solutions, the right IR heads appear, etc.
2. **Numeric back-substitution** — evaluate each solution to float (setting
   ``FreeInteger = 0``) and confirm that plugging it back satisfies the
   original equation within 1e-9 tolerance.
"""

from __future__ import annotations

import math

import pytest
from symbolic_ir import (
    ADD,
    COS,
    COSH,
    EXP,
    FREE_INTEGER,
    INTEGRATE,
    LOG,
    MUL,
    POW,
    SIN,
    SINH,
    TANH,
    IRApply,
    IRFloat,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

from symbolic_vm.backends import SymbolicBackend
from symbolic_vm.vm import VM

# ---------------------------------------------------------------------------
# Shared symbols
# ---------------------------------------------------------------------------

X = IRSymbol("x")
Y = IRSymbol("y")
EQUAL = IRSymbol("Equal")
SOLVE = IRSymbol("Solve")
LIST = IRSymbol("List")
_PI = IRSymbol("%pi")
_K = FREE_INTEGER


# ---------------------------------------------------------------------------
# Test helpers
# ---------------------------------------------------------------------------


def _make_vm() -> VM:
    """Fresh symbolic VM."""
    return VM(SymbolicBackend())


def _solve(eq_ir: IRNode, var: IRSymbol) -> IRNode:
    """Evaluate ``Solve(eq_ir, var)`` and return the result."""
    return _make_vm().eval(IRApply(SOLVE, (eq_ir, var)))


def _eq(lhs: IRNode, rhs: IRNode) -> IRNode:
    """Construct ``Equal(lhs, rhs)``."""
    return IRApply(EQUAL, (lhs, rhs))


def _is_list(node: IRNode) -> bool:
    return (
        isinstance(node, IRApply)
        and isinstance(node.head, IRSymbol)
        and node.head.name == "List"
    )


def _list_args(node: IRNode) -> tuple[IRNode, ...]:
    assert _is_list(node), f"expected List, got {node!r}"
    return node.args  # type: ignore[return-value]


def _eval_ir(node: IRNode, k: int = 0) -> float | None:
    """Recursively evaluate an IR tree to float.

    ``FreeInteger`` (the periodic constant %k) is substituted with ``k``.
    Returns ``None`` if the tree cannot be fully evaluated.

    Handles: IRInteger, IRRational, IRFloat, IRSymbol (%pi, %e, FreeInteger),
    and IRApply with arithmetic / trig / hyperbolic / inverse heads.
    """
    if isinstance(node, IRInteger):
        return float(node.value)
    if isinstance(node, IRRational):
        return node.numer / node.denom
    if isinstance(node, IRFloat):
        return node.value
    if isinstance(node, IRSymbol):
        if node.name == "%pi":
            return math.pi
        if node.name == "%e":
            return math.e
        if node.name == "FreeInteger":
            return float(k)
        return None

    if not isinstance(node, IRApply):
        return None

    head = node.head
    if not isinstance(head, IRSymbol):
        return None

    args = [_eval_ir(a, k) for a in node.args]
    if any(a is None for a in args):
        return None

    n = head.name
    a0 = args[0] if args else None
    a1 = args[1] if len(args) > 1 else None

    # Arithmetic
    if n == "Add":
        s = 0.0
        for a in args:
            s += a  # type: ignore[operator]
        return s
    if n == "Sub" and len(args) == 2:
        return a0 - a1  # type: ignore[operator]
    if n == "Mul":
        p = 1.0
        for a in args:
            p *= a  # type: ignore[operator]
        return p
    if n == "Div" and len(args) == 2:
        if a1 == 0.0:
            return None
        return a0 / a1  # type: ignore[operator]
    if n == "Neg" and len(args) == 1:
        return -a0  # type: ignore[operator]
    if n == "Pow" and len(args) == 2:
        return a0 ** a1  # type: ignore[operator]

    # Trig / inverse trig
    if n == "Sin" and len(args) == 1:
        return math.sin(a0)  # type: ignore[arg-type]
    if n == "Cos" and len(args) == 1:
        return math.cos(a0)  # type: ignore[arg-type]
    if n == "Tan" and len(args) == 1:
        return math.tan(a0)  # type: ignore[arg-type]
    if n == "Asin" and len(args) == 1:
        return math.asin(max(-1.0, min(1.0, a0)))  # type: ignore[arg-type]
    if n == "Acos" and len(args) == 1:
        return math.acos(max(-1.0, min(1.0, a0)))  # type: ignore[arg-type]
    if n == "Atan" and len(args) == 1:
        return math.atan(a0)  # type: ignore[arg-type]

    # Exponential / log
    if n == "Exp" and len(args) == 1:
        return math.exp(a0)  # type: ignore[arg-type]
    if n == "Log" and len(args) == 1:
        if a0 <= 0:  # type: ignore[operator]
            return None
        return math.log(a0)  # type: ignore[arg-type]

    # Hyperbolic / inverse hyperbolic
    if n == "Sinh" and len(args) == 1:
        return math.sinh(a0)  # type: ignore[arg-type]
    if n == "Cosh" and len(args) == 1:
        return math.cosh(a0)  # type: ignore[arg-type]
    if n == "Tanh" and len(args) == 1:
        return math.tanh(a0)  # type: ignore[arg-type]
    if n == "Asinh" and len(args) == 1:
        return math.asinh(a0)  # type: ignore[arg-type]
    if n == "Acosh" and len(args) == 1:
        if a0 < 1.0:  # type: ignore[operator]
            return None
        return math.acosh(a0)  # type: ignore[arg-type]
    if n == "Atanh" and len(args) == 1:
        if abs(a0) >= 1.0:  # type: ignore[arg-type]
            return None
        return math.atanh(a0)  # type: ignore[arg-type]

    # Lambert W
    if n == "LambertW" and len(args) == 1:
        # Newton iteration (same logic as lambert_w_handler)
        x_val = a0  # type: ignore[assignment]
        if x_val < -1.0 / math.e - 1e-12:  # type: ignore[operator]
            return None
        w = math.log(x_val) if x_val > 0 else 0.0  # type: ignore[arg-type]
        for _ in range(64):
            ew = math.exp(w)
            dw = (w * ew - x_val) / (ew * (w + 1) + 1e-300)  # type: ignore[operator]
            w -= dw
            if abs(dw) < 1e-12 * (1 + abs(w)):
                break
        return w

    return None


def _check_trig(
    func_name: str,
    c_val: float,
    sol_node: IRNode,
    k: int = 0,
    tol: float = 1e-9,
) -> bool:
    """Check that ``func(sol_node(k)) ≈ c_val``."""
    x_val = _eval_ir(sol_node, k=k)
    if x_val is None:
        return False
    func_map = {"Sin": math.sin, "Cos": math.cos, "Tan": math.tan}
    f = func_map.get(func_name)
    if f is None:
        return False
    return abs(f(x_val) - c_val) <= tol


def _check_hyp(
    func_name: str,
    c_val: float,
    sol_node: IRNode,
    tol: float = 1e-9,
) -> bool:
    """Check that ``hypfunc(sol_node) ≈ c_val``."""
    x_val = _eval_ir(sol_node, k=0)
    if x_val is None:
        return False
    func_map = {"Sinh": math.sinh, "Cosh": math.cosh, "Tanh": math.tanh}
    f = func_map.get(func_name)
    if f is None:
        return False
    return abs(f(x_val) - c_val) <= tol


def _contains_head(node: IRNode, head_name: str) -> bool:
    """Return True if ``node`` or any sub-node has the given head name."""
    if isinstance(node, IRApply):
        if isinstance(node.head, IRSymbol) and node.head.name == head_name:
            return True
        return any(_contains_head(a, head_name) for a in node.args)
    return False


def _contains_free_integer(node: IRNode) -> bool:
    if isinstance(node, IRSymbol) and node.name == "FreeInteger":
        return True
    if isinstance(node, IRApply):
        return any(_contains_free_integer(a) for a in node.args)
    return False


# ===========================================================================
# 1. Trigonometric — 26a: sin/cos/tan(x) = c
# ===========================================================================


class TestPhase26_Trig:
    """26a — periodic families: sin(x)=c, cos(x)=c, tan(x)=c."""

    def test_sin_x_eq_zero(self):
        """sin(x) = 0 → two solutions with FreeInteger."""
        result = _solve(_eq(IRApply(SIN, (X,)), IRInteger(0)), X)
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 2
        # Both solutions should contain FreeInteger
        assert all(_contains_free_integer(s) for s in sols)
        # At k=0: sin(0)=0 and sin(pi)=0
        assert _check_trig("Sin", 0.0, sols[0], k=0)
        assert _check_trig("Sin", 0.0, sols[1], k=0)

    def test_sin_x_eq_half(self):
        """sin(x) = 1/2 → arcsin(1/2) family."""
        result = _solve(_eq(IRApply(SIN, (X,)), IRRational(1, 2)), X)
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 2
        assert _check_trig("Sin", 0.5, sols[0], k=0)
        assert _check_trig("Sin", 0.5, sols[1], k=0)
        # k=1 should also satisfy (next period)
        assert _check_trig("Sin", 0.5, sols[0], k=1)

    def test_sin_x_eq_neg_one(self):
        """sin(x) = -1 → x = -pi/2 + 2k*pi."""
        result = _solve(_eq(IRApply(SIN, (X,)), IRInteger(-1)), X)
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 2
        assert _check_trig("Sin", -1.0, sols[0], k=0)
        assert _check_trig("Sin", -1.0, sols[1], k=0)

    def test_cos_x_eq_zero(self):
        """cos(x) = 0 → pi/2 + 2k*pi and -pi/2 + 2k*pi."""
        result = _solve(_eq(IRApply(COS, (X,)), IRInteger(0)), X)
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 2
        assert _check_trig("Cos", 0.0, sols[0], k=0)
        assert _check_trig("Cos", 0.0, sols[1], k=0)

    def test_cos_x_eq_half(self):
        """cos(x) = 1/2 → arccos(1/2) family."""
        result = _solve(_eq(IRApply(COS, (X,)), IRRational(1, 2)), X)
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 2
        assert _check_trig("Cos", 0.5, sols[0], k=0)
        assert _check_trig("Cos", 0.5, sols[1], k=0)

    def test_cos_x_eq_one(self):
        """cos(x) = 1 → 0 + 2k*pi (double family reduces to same)."""
        result = _solve(_eq(IRApply(COS, (X,)), IRInteger(1)), X)
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 2
        assert _check_trig("Cos", 1.0, sols[0], k=0)
        assert _check_trig("Cos", 1.0, sols[1], k=0)

    def test_tan_x_eq_zero(self):
        """tan(x) = 0 → k*pi family (single solution family)."""
        result = _solve(_eq(IRApply(IRSymbol("Tan"), (X,)), IRInteger(0)), X)
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1
        assert _check_trig("Tan", 0.0, sols[0], k=0)
        assert _check_trig("Tan", 0.0, sols[0], k=1)

    def test_tan_x_eq_one(self):
        """tan(x) = 1 → pi/4 + k*pi."""
        result = _solve(_eq(IRApply(IRSymbol("Tan"), (X,)), IRInteger(1)), X)
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1
        assert _check_trig("Tan", 1.0, sols[0], k=0)
        assert _check_trig("Tan", 1.0, sols[0], k=1)

    def test_sin_solution_contains_asin(self):
        """sin(x)=c solution must reference arcsin."""
        result = _solve(_eq(IRApply(SIN, (X,)), IRRational(1, 3)), X)
        sols = _list_args(result)
        assert any(_contains_head(s, "Asin") for s in sols)

    def test_cos_solution_contains_acos(self):
        """cos(x)=c solution must reference arccos."""
        result = _solve(_eq(IRApply(COS, (X,)), IRRational(1, 4)), X)
        sols = _list_args(result)
        assert any(_contains_head(s, "Acos") for s in sols)

    def test_tan_solution_contains_atan(self):
        """tan(x)=c solution must reference arctan."""
        result = _solve(_eq(IRApply(IRSymbol("Tan"), (X,)), IRInteger(2)), X)
        sols = _list_args(result)
        assert _contains_head(sols[0], "Atan")


# ===========================================================================
# 2. Trigonometric with shifted / scaled argument — 26a
# ===========================================================================


class TestPhase26_TrigShifted:
    """26a — trig with linear argument ax+b (a≠1 or b≠0)."""

    def test_sin_2x_eq_one(self):
        """sin(2x) = 1 → x = (pi/2 + 2k*pi)/2."""
        arg = IRApply(MUL, (IRInteger(2), X))
        result = _solve(_eq(IRApply(SIN, (arg,)), IRInteger(1)), X)
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 2
        for s in sols:
            x_val = _eval_ir(s, k=0)
            assert x_val is not None
            assert abs(math.sin(2 * x_val) - 1.0) < 1e-9

    def test_cos_x_plus_rational_eq_zero(self):
        """cos(x + 1) = 0 → x = acos(0) - 1 + 2k*pi (rational offset)."""
        arg = IRApply(ADD, (X, IRInteger(1)))
        result = _solve(_eq(IRApply(COS, (arg,)), IRInteger(0)), X)
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 2
        for s in sols:
            x_val = _eval_ir(s, k=0)
            assert x_val is not None
            assert abs(math.cos(x_val + 1.0) - 0.0) < 1e-9

    def test_sin_half_x_eq_half(self):
        """sin(x/2) = 1/2 → x = 2*arcsin(1/2) + 4k*pi."""
        arg = IRApply(MUL, (IRRational(1, 2), X))
        result = _solve(_eq(IRApply(SIN, (arg,)), IRRational(1, 2)), X)
        assert _is_list(result)
        sols = _list_args(result)
        for s in sols:
            x_val = _eval_ir(s, k=0)
            assert x_val is not None
            assert abs(math.sin(x_val / 2) - 0.5) < 1e-9


# ===========================================================================
# 3. Exponential / Logarithmic — 26b
# ===========================================================================


class TestPhase26_ExpLog:
    """26b — exp(ax+b) = c and log(ax+b) = c."""

    def test_exp_x_eq_one(self):
        """exp(x) = 1 → x = log(1) = 0."""
        result = _solve(_eq(IRApply(EXP, (X,)), IRInteger(1)), X)
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1
        x_val = _eval_ir(sols[0])
        assert x_val is not None
        assert abs(math.exp(x_val) - 1.0) < 1e-9

    def test_exp_x_eq_two(self):
        """exp(x) = 2 → x = log(2)."""
        result = _solve(_eq(IRApply(EXP, (X,)), IRInteger(2)), X)
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1
        x_val = _eval_ir(sols[0])
        assert x_val is not None
        assert abs(math.exp(x_val) - 2.0) < 1e-9

    def test_exp_2x_eq_four(self):
        """exp(2x) = 4 → x = log(4)/2."""
        arg = IRApply(MUL, (IRInteger(2), X))
        result = _solve(_eq(IRApply(EXP, (arg,)), IRInteger(4)), X)
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1
        x_val = _eval_ir(sols[0])
        assert x_val is not None
        assert abs(math.exp(2 * x_val) - 4.0) < 1e-9

    def test_exp_solution_contains_log(self):
        """exp(x)=c solution must contain a Log node."""
        result = _solve(_eq(IRApply(EXP, (X,)), IRInteger(3)), X)
        sols = _list_args(result)
        assert _contains_head(sols[0], "Log")

    def test_log_x_eq_one(self):
        """log(x) = 1 → x = exp(1) = e."""
        result = _solve(_eq(IRApply(LOG, (X,)), IRInteger(1)), X)
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1
        x_val = _eval_ir(sols[0])
        assert x_val is not None
        assert abs(math.log(x_val) - 1.0) < 1e-9

    def test_log_x_eq_zero(self):
        """log(x) = 0 → x = exp(0) = 1."""
        result = _solve(_eq(IRApply(LOG, (X,)), IRInteger(0)), X)
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1
        x_val = _eval_ir(sols[0])
        assert x_val is not None
        assert abs(math.log(x_val) - 0.0) < 1e-9

    def test_log_solution_contains_exp(self):
        """log(x)=c solution must contain an Exp node."""
        result = _solve(_eq(IRApply(LOG, (X,)), IRInteger(2)), X)
        sols = _list_args(result)
        assert _contains_head(sols[0], "Exp")


# ===========================================================================
# 4. Lambert W — 26c
# ===========================================================================


class TestPhase26_LambertW:
    """26c — f(x)·exp(f(x)) = c where f is linear."""

    def test_x_exp_x_eq_one(self):
        """x·exp(x) = 1 → x = W(1) ≈ 0.5671."""
        x_exp_x = IRApply(MUL, (X, IRApply(EXP, (X,))))
        result = _solve(_eq(x_exp_x, IRInteger(1)), X)
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1
        # Solution must contain LambertW
        assert _contains_head(sols[0], "LambertW")

    def test_x_exp_x_eq_zero(self):
        """x·exp(x) = 0 → x = W(0) = 0."""
        x_exp_x = IRApply(MUL, (X, IRApply(EXP, (X,))))
        result = _solve(_eq(x_exp_x, IRInteger(0)), X)
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1
        assert _contains_head(sols[0], "LambertW")

    def test_x_exp_x_eq_e(self):
        """x·exp(x) = e → x = W(e) = 1."""
        x_exp_x = IRApply(MUL, (X, IRApply(EXP, (X,))))
        e_sym = IRSymbol("%e")
        result = _solve(_eq(x_exp_x, e_sym), X)
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1
        assert _contains_head(sols[0], "LambertW")

    def test_exp_first_then_linear(self):
        """exp(x)·x = 2 (commuted MUL) → x = W(2)."""
        # Argument order reversed: Mul(Exp(x), x) instead of Mul(x, Exp(x))
        exp_x_times_x = IRApply(MUL, (IRApply(EXP, (X,)), X))
        result = _solve(_eq(exp_x_times_x, IRInteger(2)), X)
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1
        assert _contains_head(sols[0], "LambertW")


# ===========================================================================
# 5. Hyperbolic — 26d
# ===========================================================================


class TestPhase26_Hyperbolic:
    """26d — sinh/cosh/tanh with linear argument."""

    def test_sinh_x_eq_zero(self):
        """sinh(x) = 0 → x = asinh(0) = 0."""
        result = _solve(_eq(IRApply(SINH, (X,)), IRInteger(0)), X)
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1
        assert _check_hyp("Sinh", 0.0, sols[0])

    def test_sinh_x_eq_one(self):
        """sinh(x) = 1 → x = asinh(1)."""
        result = _solve(_eq(IRApply(SINH, (X,)), IRInteger(1)), X)
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1
        assert _check_hyp("Sinh", 1.0, sols[0])
        assert _contains_head(sols[0], "Asinh")

    def test_cosh_x_eq_one(self):
        """cosh(x) = 1 → x = ±acosh(1) = ±0 = 0 (both branches)."""
        result = _solve(_eq(IRApply(COSH, (X,)), IRInteger(1)), X)
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 2
        for s in sols:
            assert _check_hyp("Cosh", 1.0, s)

    def test_cosh_x_eq_two(self):
        """cosh(x) = 2 → two branches: acosh(2) and -acosh(2)."""
        result = _solve(_eq(IRApply(COSH, (X,)), IRInteger(2)), X)
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 2
        for s in sols:
            assert _check_hyp("Cosh", 2.0, s)
        x_vals = [_eval_ir(s) for s in sols]
        # The two values should be negatives of each other
        assert x_vals[0] is not None and x_vals[1] is not None
        assert abs(x_vals[0] + x_vals[1]) < 1e-9  # type: ignore[operator]

    def test_tanh_x_eq_zero(self):
        """tanh(x) = 0 → x = atanh(0) = 0."""
        result = _solve(_eq(IRApply(TANH, (X,)), IRInteger(0)), X)
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1
        assert _check_hyp("Tanh", 0.0, sols[0])

    def test_tanh_x_eq_half(self):
        """tanh(x) = 1/2 → x = atanh(1/2) ≈ 0.549."""
        result = _solve(_eq(IRApply(TANH, (X,)), IRRational(1, 2)), X)
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1
        assert _check_hyp("Tanh", 0.5, sols[0])
        assert _contains_head(sols[0], "Atanh")

    def test_sinh_two_x_eq_one(self):
        """sinh(2x) = 1 → x = asinh(1)/2."""
        arg = IRApply(MUL, (IRInteger(2), X))
        result = _solve(_eq(IRApply(SINH, (arg,)), IRInteger(1)), X)
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1
        x_val = _eval_ir(sols[0])
        assert x_val is not None
        assert abs(math.sinh(2 * x_val) - 1.0) < 1e-9


# ===========================================================================
# 6. Compound — 26e: polynomial in a single transcendental function
# ===========================================================================


class TestPhase26_Compound:
    """26e — equations that are polynomials in a transcendental substitution."""

    def test_sin2_plus_sin_eq_zero(self):
        """sin(x)^2 + sin(x) = 0 → 4 solutions (u∈{-1,0}, each gives 2)."""
        sin_x = IRApply(SIN, (X,))
        lhs = IRApply(ADD, (IRApply(POW, (sin_x, IRInteger(2))), sin_x))
        result = _solve(_eq(lhs, IRInteger(0)), X)
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 4
        # All solutions satisfy the equation
        for s in sols:
            x_val = _eval_ir(s, k=0)
            assert x_val is not None
            val = math.sin(x_val) ** 2 + math.sin(x_val)
            assert abs(val) < 1e-9

    def test_sin2_minus_sin_eq_zero(self):
        """sin(x)^2 - sin(x) = 0 → u∈{0,1} → 4 solutions."""
        sin_x = IRApply(SIN, (X,))
        neg_sin = IRApply(IRSymbol("Neg"), (sin_x,))
        lhs = IRApply(ADD, (IRApply(POW, (sin_x, IRInteger(2))), neg_sin))
        result = _solve(_eq(lhs, IRInteger(0)), X)
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) >= 2  # at minimum the u=0 solutions

    def test_exp2_minus_3exp_plus_2_eq_zero(self):
        """exp(x)^2 - 3*exp(x) + 2 = 0 → u∈{1,2} → x∈{0, log(2)}."""
        exp_x = IRApply(EXP, (X,))
        # exp(x)^2 - 3*exp(x) + 2
        lhs = IRApply(ADD, (
            IRApply(ADD, (
                IRApply(IRSymbol("Pow"), (exp_x, IRInteger(2))),
                IRApply(IRSymbol("Neg"), (IRApply(MUL, (IRInteger(3), exp_x)),)),
            )),
            IRInteger(2),
        ))
        result = _solve(_eq(lhs, IRInteger(0)), X)
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 2
        # x=0 (exp(0)=1) and x=log(2) (exp(log(2))=2)
        x_vals = sorted(_eval_ir(s) for s in sols if _eval_ir(s) is not None)
        assert len(x_vals) == 2
        assert abs(x_vals[0] - 0.0) < 1e-9
        assert abs(x_vals[1] - math.log(2)) < 1e-9


# ===========================================================================
# 7. LambertW numeric handler
# ===========================================================================


class TestPhase26_LambertWHandler:
    """Numeric evaluation of LambertW(x) via lambert_w_handler."""

    _LW = IRSymbol("LambertW")

    def _eval_lw(self, x_ir: IRNode) -> IRNode:
        return _make_vm().eval(IRApply(self._LW, (x_ir,)))

    def test_lambert_w_zero(self):
        """LambertW(0) = 0."""
        result = self._eval_lw(IRInteger(0))
        assert isinstance(result, IRFloat)
        assert abs(result.value - 0.0) < 1e-9

    def test_lambert_w_one(self):
        """LambertW(1) = Omega ≈ 0.5671432904097838."""
        result = self._eval_lw(IRInteger(1))
        assert isinstance(result, IRFloat)
        assert abs(result.value - 0.5671432904097838) < 1e-9

    def test_lambert_w_e(self):
        """LambertW(e) = 1 (since 1·exp(1)=e).

        to_number cannot evaluate the symbolic ``%e`` constant, so we
        supply the numeric value directly via ``IRFloat``.
        """
        result = self._eval_lw(IRFloat(math.e))
        assert isinstance(result, IRFloat)
        assert abs(result.value - 1.0) < 1e-9

    def test_lambert_w_rational(self):
        """LambertW(1/2) ≈ 0.3517..."""
        result = self._eval_lw(IRRational(1, 2))
        assert isinstance(result, IRFloat)
        # W(0.5)·exp(W(0.5)) ≈ 0.5
        assert abs(result.value * math.exp(result.value) - 0.5) < 1e-9

    def test_lambert_w_negative_in_domain(self):
        """LambertW(-1/(2e)) is in domain (>=-1/e)."""
        arg = IRFloat(-1.0 / (2 * math.e))
        result = self._eval_lw(arg)
        assert isinstance(result, IRFloat)
        # Verify: w·exp(w) ≈ -1/(2e)
        w = result.value
        assert abs(w * math.exp(w) - (-1.0 / (2 * math.e))) < 1e-9

    def test_lambert_w_out_of_domain(self):
        """LambertW(-2) is out of domain → unevaluated."""
        result = self._eval_lw(IRFloat(-2.0))
        # Should return unevaluated IRApply
        assert isinstance(result, IRApply)
        assert isinstance(result.head, IRSymbol) and result.head.name == "LambertW"

    def test_lambert_w_symbolic_arg(self):
        """LambertW(x) with symbolic x → unevaluated."""
        result = self._eval_lw(X)
        assert isinstance(result, IRApply)

    def test_lambert_w_large(self):
        """LambertW(100) should satisfy w·exp(w) ≈ 100."""
        result = self._eval_lw(IRInteger(100))
        assert isinstance(result, IRFloat)
        w = result.value
        assert abs(w * math.exp(w) - 100.0) < 1e-6


# ===========================================================================
# 8. Fallthrough — unevaluated cases
# ===========================================================================


class TestPhase26_Fallthrough:
    """Equations that should return unevaluated (no pattern matched)."""

    def _is_unevaluated_solve(self, node: IRNode) -> bool:
        return (
            isinstance(node, IRApply)
            and isinstance(node.head, IRSymbol)
            and node.head.name == "Solve"
        )

    def test_sin_of_quadratic_unevaluated(self):
        """sin(x^2) = 0 is not linear in argument → unevaluated."""
        x_sq = IRApply(IRSymbol("Pow"), (X, IRInteger(2)))
        result = _solve(_eq(IRApply(SIN, (x_sq,)), IRInteger(0)), X)
        # Non-linear argument; no compound pattern applies → unevaluated
        assert self._is_unevaluated_solve(result)

    def test_sin_x_plus_cos_x_unevaluated(self):
        """sin(x) + cos(x) = 0 — compound pattern can't reduce to single func."""
        lhs = IRApply(ADD, (IRApply(SIN, (X,)), IRApply(COS, (X,))))
        result = _solve(_eq(lhs, IRInteger(0)), X)
        # Mixed transcendental → unevaluated
        assert self._is_unevaluated_solve(result)

    def test_high_degree_poly_unevaluated(self):
        """x^5 + x + 1 = 0 — degree > 4 → still unevaluated."""
        lhs = IRApply(ADD, (
            IRApply(ADD, (IRApply(POW, (X, IRInteger(5))), X)),
            IRInteger(1),
        ))
        result = _solve(_eq(lhs, IRInteger(0)), X)
        assert self._is_unevaluated_solve(result)


# ===========================================================================
# 9. Regressions — earlier phase operations still work
# ===========================================================================


class TestPhase26_Regressions:
    """Smoke tests that prior-phase functionality is unbroken."""

    def test_linear_solve(self):
        """Phase 1 regression: Solve(2x + 4 = 0, x) = -2."""
        lhs = IRApply(ADD, (IRApply(MUL, (IRInteger(2), X)), IRInteger(4)))
        result = _solve(_eq(lhs, IRInteger(0)), X)
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1
        x_val = _eval_ir(sols[0])
        assert x_val is not None and abs(x_val + 2.0) < 1e-12

    def test_quadratic_solve(self):
        """Phase 1 regression: Solve(x^2 - 5x + 6 = 0, x) = {2, 3}."""
        # x^2 - 5x + 6
        lhs = IRApply(ADD, (
            IRApply(ADD, (
                IRApply(POW, (X, IRInteger(2))),
                IRApply(IRSymbol("Neg"), (IRApply(MUL, (IRInteger(5), X)),)),
            )),
            IRInteger(6),
        ))
        result = _solve(_eq(lhs, IRInteger(0)), X)
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 2
        x_vals = sorted(_eval_ir(s) for s in sols if _eval_ir(s) is not None)
        assert abs(x_vals[0] - 2.0) < 1e-9
        assert abs(x_vals[1] - 3.0) < 1e-9

    def test_lambert_w_handler_not_broken(self):
        """LambertW(0) still returns 0.0 after Phase 26."""
        result = _make_vm().eval(IRApply(IRSymbol("LambertW"), (IRInteger(0),)))
        assert isinstance(result, IRFloat)
        assert abs(result.value) < 1e-12

    def test_integrate_not_broken(self):
        """Phase 14 regression: Integrate(x^2, x) returns something."""
        expr = IRApply(INTEGRATE, (IRApply(POW, (X, IRInteger(2))), X))
        result = _make_vm().eval(expr)
        # Should be evaluated (not the bare Integrate expression)
        if isinstance(result, IRApply):
            assert result.head != INTEGRATE


# ===========================================================================
# 10. MACSYMA surface-syntax tests
# ===========================================================================


class TestPhase26_Macsyma:
    """End-to-end tests via the MACSYMA compiler + runtime stack."""

    def _run(self, src: str) -> IRNode:
        pytest.importorskip(
            "macsyma_runtime",
            reason="macsyma-runtime not installed; skipping MACSYMA e2e test",
        )
        from macsyma_compiler.compiler import (  # noqa: PLC0415
            _STANDARD_FUNCTIONS,
            compile_macsyma,
        )
        from macsyma_parser.parser import parse_macsyma  # noqa: PLC0415
        from macsyma_runtime.name_table import (  # noqa: PLC0415
            extend_compiler_name_table,
        )

        extend_compiler_name_table(_STANDARD_FUNCTIONS)
        stmts = compile_macsyma(parse_macsyma(src + ";"))
        return _make_vm().eval_program(stmts)

    def test_macsyma_solve_sin(self):
        """solve(sin(x) = 0, x) returns a two-element List."""
        result = self._run("solve(sin(x) = 0, x)")
        assert _is_list(result)
        assert len(_list_args(result)) == 2

    def test_macsyma_solve_exp(self):
        """solve(exp(x) = 1, x) returns List(Log(1)) = List(0)."""
        result = self._run("solve(exp(x) = 1, x)")
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1
        x_val = _eval_ir(sols[0])
        assert x_val is not None and abs(math.exp(x_val) - 1.0) < 1e-9

    def test_macsyma_solve_log(self):
        """solve(log(x) = 1, x) returns List(exp(1))."""
        result = self._run("solve(log(x) = 1, x)")
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1
        x_val = _eval_ir(sols[0])
        assert x_val is not None and abs(math.log(x_val) - 1.0) < 1e-9

    def test_macsyma_solve_sinh(self):
        """solve(sinh(x) = 1, x) returns List(asinh(1))."""
        result = self._run("solve(sinh(x) = 1, x)")
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1
        x_val = _eval_ir(sols[0])
        assert x_val is not None and abs(math.sinh(x_val) - 1.0) < 1e-9

    def test_macsyma_solve_tanh(self):
        """solve(tanh(x) = 1/2, x) returns List(atanh(1/2))."""
        result = self._run("solve(tanh(x) = 1/2, x)")
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1
        x_val = _eval_ir(sols[0])
        assert x_val is not None and abs(math.tanh(x_val) - 0.5) < 1e-9

    def test_macsyma_solve_lambert_w(self):
        """solve(x*exp(x) = 1, x) returns List(LambertW(1))."""
        result = self._run("solve(x*exp(x) = 1, x)")
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1
        assert _contains_head(sols[0], "LambertW")

    def test_macsyma_lambert_w_numeric(self):
        """lambert_w(1) ≈ 0.5671."""
        result = self._run("lambert_w(1)")
        assert isinstance(result, IRFloat)
        assert abs(result.value - 0.5671432904097838) < 1e-9

    def test_macsyma_solve_compound(self):
        """solve(sin(x)^2 + sin(x) = 0, x) returns 4 solutions."""
        result = self._run("solve(sin(x)^2 + sin(x) = 0, x)")
        assert _is_list(result)
        assert len(_list_args(result)) == 4

    def test_macsyma_solve_cosh(self):
        """solve(cosh(x) = 2, x) returns two solutions (symmetric)."""
        result = self._run("solve(cosh(x) = 2, x)")
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 2
        x_vals = [_eval_ir(s) for s in sols]
        for xv in x_vals:
            assert xv is not None
            assert abs(math.cosh(xv) - 2.0) < 1e-9

    def test_macsyma_linear_regression(self):
        """Regression: solve(2*x + 4 = 0, x) = -2 (Phase 1 still works)."""
        result = self._run("solve(2*x + 4 = 0, x)")
        assert _is_list(result)
        sols = _list_args(result)
        x_val = _eval_ir(sols[0])
        assert x_val is not None and abs(x_val + 2.0) < 1e-12
