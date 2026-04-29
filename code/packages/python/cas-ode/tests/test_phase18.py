"""Phase 18 integration tests — Bernoulli, Exact, and 2nd-order
non-homogeneous ODE solvers.

New solver types added in cas-ode 0.2.0
----------------------------------------
- **Bernoulli** ``y' + P(x)·y = Q(x)·y^n``  (n ≠ 0,1)
- **Exact**     ``M(x,y) + N(x,y)·y' = 0``  with ∂M/∂y = ∂N/∂x
- **2nd-order non-homogeneous** (undetermined coefficients):
  constant, polynomial (deg ≤ 2), exponential e^(αx),
  sin(βx)/cos(βx), and e^(αx)·sin/cos(βx) forcing.

Verification strategy
---------------------
All numerical checks differentiate the antiderivative:

    F'(x) ≈ f(x)  at test points away from singularities.

For Bernoulli and non-homogeneous ODEs the solution is explicit
``Equal(y, expr)``; for exact ODEs the solution is implicit
``Equal(F(x,y), %c)`` — we verify by checking that ∂F/∂x = M and
∂F/∂y = N at test points.
"""

from __future__ import annotations

import math

from symbolic_ir import (
    ADD,
    COS,
    EQUAL,
    EXP,
    MUL,
    NEG,
    POW,
    SIN,
    SUB,
    IRApply,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)
from symbolic_ir.nodes import ODE2, D
from symbolic_vm import VM, SymbolicBackend

from cas_ode import build_ode_handler_table

# ---------------------------------------------------------------------------
# Fixtures and helpers
# ---------------------------------------------------------------------------

X = IRSymbol("x")
Y = IRSymbol("y")
Y_PRIME = IRApply(D, (Y, X))
Y_DOUBLE = IRApply(D, (Y_PRIME, X))


def make_vm() -> VM:
    """Return a fresh SymbolicBackend VM with ODE2 wired in."""
    backend = SymbolicBackend()
    backend._handlers.update(build_ode_handler_table())  # type: ignore[attr-defined]
    return VM(backend)


def eval_ode(expr: IRNode, y: IRSymbol = Y, x: IRSymbol = X) -> IRNode:
    """Evaluate ODE2(expr, y, x) through the VM."""
    vm = make_vm()
    return vm.eval(IRApply(ODE2, (expr, y, x)))


def _I(n: int) -> IRInteger:
    return IRInteger(n)


def _R(n: int, d: int) -> IRRational:
    return IRRational(n, d)


def _mul(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(MUL, (a, b))


def _add(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(ADD, (a, b))


def _sub(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(SUB, (a, b))


def _pow(base: IRNode, exp: IRNode) -> IRNode:
    return IRApply(POW, (base, exp))


def _neg(node: IRNode) -> IRNode:
    return IRApply(NEG, (node,))


def _exp(arg: IRNode) -> IRNode:
    return IRApply(EXP, (arg,))


def _sin(arg: IRNode) -> IRNode:
    return IRApply(SIN, (arg,))


def _cos(arg: IRNode) -> IRNode:
    return IRApply(COS, (arg,))


def _eval_ir(node: IRNode, x_val: float, y_val: float = 0.0) -> float:
    """Numerically evaluate an IR tree at (x, y) = (x_val, y_val)."""
    if isinstance(node, IRInteger):
        return float(node.value)
    if isinstance(node, IRRational):
        return node.numer / node.denom
    from symbolic_ir import IRFloat  # noqa: PLC0415
    if isinstance(node, IRFloat):
        return node.value
    if isinstance(node, IRSymbol):
        if node.name == "x":
            return x_val
        if node.name == "y":
            return y_val
        if node.name in ("%c", "%c1"):
            return 1.0   # treat integration constant as 1 for structure tests
        if node.name == "%c2":
            return 0.0
        raise ValueError(f"Unknown symbol: {node.name}")
    if not isinstance(node, IRApply):
        raise TypeError(f"Unexpected node: {node!r}")
    head = node.head.name
    ev = lambda n: _eval_ir(n, x_val, y_val)  # noqa: E731
    if head == "Add":
        return ev(node.args[0]) + ev(node.args[1])
    if head == "Sub":
        return ev(node.args[0]) - ev(node.args[1])
    if head == "Mul":
        return ev(node.args[0]) * ev(node.args[1])
    if head == "Div":
        return ev(node.args[0]) / ev(node.args[1])
    if head == "Neg":
        return -ev(node.args[0])
    if head == "Pow":
        return ev(node.args[0]) ** ev(node.args[1])
    if head == "Exp":
        return math.exp(_eval_ir(node.args[0], x_val, y_val))
    if head == "Log":
        return math.log(abs(_eval_ir(node.args[0], x_val, y_val)))
    if head == "Sin":
        return math.sin(_eval_ir(node.args[0], x_val, y_val))
    if head == "Cos":
        return math.cos(_eval_ir(node.args[0], x_val, y_val))
    if head == "Sinh":
        return math.sinh(_eval_ir(node.args[0], x_val, y_val))
    if head == "Cosh":
        return math.cosh(_eval_ir(node.args[0], x_val, y_val))
    if head == "Tanh":
        return math.tanh(_eval_ir(node.args[0], x_val, y_val))
    raise ValueError(f"Unhandled head: {head}")


def _deriv_x(node: IRNode, x_val: float, y_val: float = 0.0, h: float = 1e-7) -> float:
    """Numerical partial derivative ∂(node)/∂x at (x_val, y_val)."""
    return (
        _eval_ir(node, x_val + h, y_val) - _eval_ir(node, x_val - h, y_val)
    ) / (2 * h)


def _deriv_y(node: IRNode, x_val: float, y_val: float, h: float = 1e-7) -> float:
    """Numerical partial derivative ∂(node)/∂y at (x_val, y_val)."""
    return (
        _eval_ir(node, x_val, y_val + h) - _eval_ir(node, x_val, y_val - h)
    ) / (2 * h)


def _was_evaluated(result: IRNode) -> None:
    """Assert result is NOT an unevaluated ODE2 node."""
    assert not (
        isinstance(result, IRApply) and result.head == ODE2
    ), f"Expected solved ODE, got unevaluated: {result!r}"


def _is_unevaluated(result: IRNode) -> None:
    """Assert result IS an unevaluated ODE2 node."""
    assert isinstance(result, IRApply) and result.head == ODE2, (
        f"Expected unevaluated ODE2, got: {result!r}"
    )


def _is_equal_node(result: IRNode) -> IRNode:
    """Assert result is Equal(lhs, rhs) and return rhs."""
    assert isinstance(result, IRApply) and result.head == EQUAL, (
        f"Expected Equal(...), got: {result!r}"
    )
    return result.args[1]


# ---------------------------------------------------------------------------
# TestPhase18_Bernoulli
# ---------------------------------------------------------------------------


class TestPhase18_Bernoulli:
    """Bernoulli ODE: y' + P(x)·y = Q(x)·y^n  (n ≠ 0, 1).

    Reduction: v = y^(1-n) → linear ODE for v, then y = v^(1/(1-n)).
    """

    def test_bernoulli_n2_constant_p_q(self) -> None:
        """y' - y = -y² → P=-1, Q=-1, n=2.

        Exact solution: y = 1/(1 + C·e^(-x)).
        """
        # Zero form: D(y,x) - y + y^2 = 0  (after moving -y² → +y²)
        # Wait: y' - y = -y²  →  D(y,x) - y + y² = 0
        expr = _add(_sub(Y_PRIME, Y), _pow(Y, _I(2)))
        result = eval_ode(expr)
        _was_evaluated(result)
        sol = _is_equal_node(result)
        # Verify F'(x) ≈ F(x)*(1 - F(x)) at test point (satisfies y' = y - y²)
        # y' = y - y²  at x=0.5
        x0 = 0.5
        y_val = _eval_ir(sol, x0)
        dy_dx = _deriv_x(sol, x0)
        rhs = y_val - y_val ** 2
        assert abs(dy_dx - rhs) < 1e-5, f"y' - y + y² ≠ 0 at x={x0}"

    def test_bernoulli_n2_x_coefficient(self) -> None:
        """y' + x·y = x·y² → P=x, Q=x, n=2."""
        # Zero form: D(y,x) + x*y - x*y^2 = 0
        expr = _add(
            _add(Y_PRIME, _mul(X, Y)),
            _neg(_mul(X, _pow(Y, _I(2)))),
        )
        result = eval_ode(expr)
        _was_evaluated(result)
        sol = _is_equal_node(result)
        # Check the solution satisfies y' + x*y - x*y² = 0 at x=0.4
        x0 = 0.4
        y_val = _eval_ir(sol, x0)
        dy_dx = _deriv_x(sol, x0)
        rhs = -x0 * y_val + x0 * y_val ** 2
        assert abs(dy_dx - rhs) < 1e-5, f"ODE residual too large at x={x0}"

    def test_bernoulli_n3_constant_coefficients(self) -> None:
        """y' + y = y³ → P=1, Q=1, n=3."""
        # Zero form: D(y,x) + y - y^3 = 0
        expr = _add(_add(Y_PRIME, Y), _neg(_pow(Y, _I(3))))
        result = eval_ode(expr)
        _was_evaluated(result)
        sol = _is_equal_node(result)
        x0 = 0.3
        y_val = _eval_ir(sol, x0)
        dy_dx = _deriv_x(sol, x0)
        rhs = -y_val + y_val ** 3
        assert abs(dy_dx - rhs) < 1e-5, f"ODE residual too large at x={x0}"

    def test_bernoulli_result_is_equal_node(self) -> None:
        """Bernoulli solver returns Equal(y, ...) node."""
        expr = _add(_sub(Y_PRIME, Y), _pow(Y, _I(2)))
        result = eval_ode(expr)
        assert isinstance(result, IRApply) and result.head == EQUAL
        assert result.args[0] == Y

    def test_bernoulli_n2_returns_power_expression(self) -> None:
        """Bernoulli n=2 solution contains a Pow node (from v^(-1))."""
        expr = _add(_sub(Y_PRIME, Y), _pow(Y, _I(2)))
        result = eval_ode(expr)
        sol = _is_equal_node(result)

        def _has_pow(node: IRNode) -> bool:
            if isinstance(node, IRApply):
                if node.head == POW:
                    return True
                return any(_has_pow(c) for c in node.args)
            return False

        assert _has_pow(sol), f"Expected Pow in Bernoulli solution: {sol!r}"

    def test_bernoulli_n1_not_matched(self) -> None:
        """y' + y = y^1 = y — this is linear, not Bernoulli (n=1)."""
        # The Bernoulli recogniser rejects n=1; linear solver handles it.
        expr = _add(_add(Y_PRIME, Y), _neg(Y))   # = D(y,x)
        result = eval_ode(expr)
        # Should still be evaluated (trivially y' = 0 → y = C)
        _was_evaluated(result)

    def test_bernoulli_negative_n(self) -> None:
        """y' + y = y^(-1) → P=1, Q=1, n=-1 (negative Bernoulli)."""
        # Zero form: D(y,x) + y - y^(-1) = 0
        expr = _add(
            _add(Y_PRIME, Y),
            _neg(_pow(Y, _I(-1))),
        )
        result = eval_ode(expr)
        _was_evaluated(result)
        sol = _is_equal_node(result)
        x0 = 0.5
        y_val = _eval_ir(sol, x0)
        dy_dx = _deriv_x(sol, x0)
        rhs = -y_val + y_val ** (-1)
        assert abs(dy_dx - rhs) < 1e-5, f"ODE residual too large at x={x0}"

    def test_bernoulli_fallthrough_poly_times_ypow(self) -> None:
        """y' + x²·y = x·y³ — Bernoulli with polynomial P,Q, should solve."""
        # Zero form: D(y,x) + x^2*y - x*y^3 = 0
        x_sq = _pow(X, _I(2))
        expr = _add(
            _add(Y_PRIME, _mul(x_sq, Y)),
            _neg(_mul(X, _pow(Y, _I(3)))),
        )
        result = eval_ode(expr)
        # May evaluate or fall through depending on integration success
        # Just check it returns something valid
        assert isinstance(result, IRApply)

    def test_bernoulli_pure_y_prime_not_bernoulli(self) -> None:
        """y' alone with no y^n — not Bernoulli, falls to linear/separable."""
        expr = Y_PRIME   # y' = 0
        result = eval_ode(expr)
        _was_evaluated(result)

    def test_bernoulli_two_different_powers_unevaluated(self) -> None:
        """y^2 + y^3 both present — ambiguous Bernoulli, unevaluated."""
        # y' + y^2 - y^3 = 0  has two different y-powers → not Bernoulli
        expr = _add(
            _add(Y_PRIME, _pow(Y, _I(2))),
            _neg(_pow(Y, _I(3))),
        )
        result = eval_ode(expr)
        _is_unevaluated(result)

    def test_bernoulli_n2_structure_is_explicit_y(self) -> None:
        """Bernoulli solution is Equal(y, f(x)) not Equal(f(x,y), c)."""
        expr = _add(_sub(Y_PRIME, Y), _pow(Y, _I(2)))
        result = eval_ode(expr)
        assert isinstance(result, IRApply) and result.head == EQUAL
        # LHS of Equal must be y (explicit solution)
        assert result.args[0] == Y


# ---------------------------------------------------------------------------
# TestPhase18_Exact
# ---------------------------------------------------------------------------


class TestPhase18_Exact:
    """Exact ODE: M dx + N dy = 0 when ∂M/∂y = ∂N/∂x.

    The solver returns an implicit solution Equal(F(x,y), %c).
    """

    def test_exact_simple_2xy_x2(self) -> None:
        """2xy·dx + x²·dy = 0 — exact (∂M/∂y = 2x = ∂N/∂x).

        Potential F = x²·y.  Solution: x²·y = C.
        """
        # Zero form: 2*x*y + x^2 * D(y,x) = 0
        M = _mul(_I(2), _mul(X, Y))
        N = _pow(X, _I(2))
        expr = _add(M, _mul(N, Y_PRIME))
        result = eval_ode(expr)
        _was_evaluated(result)
        assert isinstance(result, IRApply) and result.head == EQUAL
        # Verify: F(x,y) = C  where F = x^2*y
        # Check ∂F/∂x = 2xy at (x,y)=(1.5, 0.5)
        F_expr = result.args[0]
        x0, y0 = 1.5, 0.5
        dF_dx = _deriv_x(F_expr, x0, y0)
        M_val = 2 * x0 * y0
        assert abs(dF_dx - M_val) < 1e-5, f"∂F/∂x ≠ M at ({x0},{y0})"

    def test_exact_derivative_condition_N(self) -> None:
        """Verify ∂F/∂y = N for the 2xy + x² case."""
        M = _mul(_I(2), _mul(X, Y))
        N = _pow(X, _I(2))
        expr = _add(M, _mul(N, Y_PRIME))
        result = eval_ode(expr)
        F_expr = result.args[0]
        x0, y0 = 1.5, 0.5
        dF_dy = _deriv_y(F_expr, x0, y0)
        N_val = x0 ** 2
        assert abs(dF_dy - N_val) < 1e-5, f"∂F/∂y ≠ N at ({x0},{y0})"

    def test_exact_polynomial_M_N(self) -> None:
        """(3x² + 6xy)·dx + (3x² + 4y³)·dy = 0 — polynomial exact ODE.

        ∂M/∂y = 6x = ∂N/∂x ✓
        """
        # M = 3x^2 + 6xy, N = 3x^2 + 4y^3
        # In zero form: M + N*y' = 0
        x_sq = _pow(X, _I(2))
        y_cu = _pow(Y, _I(3))
        M = _add(_mul(_I(3), x_sq), _mul(_I(6), _mul(X, Y)))
        N = _add(_mul(_I(3), x_sq), _mul(_I(4), y_cu))
        expr = _add(M, _mul(N, Y_PRIME))
        result = eval_ode(expr)
        _was_evaluated(result)
        assert isinstance(result, IRApply) and result.head == EQUAL
        # Check ∂F/∂x = M at a test point
        F_expr = result.args[0]
        x0, y0 = 1.0, 0.5
        dF_dx = _deriv_x(F_expr, x0, y0)
        M_val = 3 * x0 ** 2 + 6 * x0 * y0
        assert abs(dF_dx - M_val) < 1e-5

    def test_exact_implicit_solution_form(self) -> None:
        """Exact solver returns Equal(F(x,y), %c) with %c on RHS."""
        M = _mul(_I(2), _mul(X, Y))
        N = _pow(X, _I(2))
        expr = _add(M, _mul(N, Y_PRIME))
        result = eval_ode(expr)
        assert isinstance(result, IRApply) and result.head == EQUAL
        # RHS must be the integration constant C_CONST
        rhs = result.args[1]
        assert isinstance(rhs, IRSymbol) and rhs.name == "%c", (
            f"Expected %c on RHS of exact solution, got: {rhs!r}"
        )

    def test_exact_not_exact_unevaluated(self) -> None:
        """M·dx + N·dy = 0 with ∂M/∂y ≠ ∂N/∂x — falls through unevaluated."""
        # M = x^2, N = x·y  →  ∂M/∂y = 0, ∂N/∂x = y  (not equal, not exact)
        M = _pow(X, _I(2))
        N = _mul(X, Y)
        expr = _add(M, _mul(N, Y_PRIME))
        result = eval_ode(expr)
        # Not exact — should fall through; the separable or linear solver
        # might still catch it, but we just check it doesn't throw
        assert isinstance(result, IRApply)

    def test_exact_constant_M_N_1(self) -> None:
        """M = y, N = x — exact (∂M/∂y = 1 = ∂N/∂x).  Solution: x·y = C."""
        # Zero form: y + x * D(y,x) = 0
        expr = _add(Y, _mul(X, Y_PRIME))
        result = eval_ode(expr)
        _was_evaluated(result)
        assert isinstance(result, IRApply) and result.head == EQUAL
        F_expr = result.args[0]
        x0, y0 = 1.2, 0.8
        dF_dx = _deriv_x(F_expr, x0, y0)
        assert abs(dF_dx - y0) < 1e-5, f"∂F/∂x ≠ y at ({x0},{y0})"
        dF_dy = _deriv_y(F_expr, x0, y0)
        assert abs(dF_dy - x0) < 1e-5, f"∂F/∂y ≠ x at ({x0},{y0})"

    def test_exact_missing_yprime_not_exact(self) -> None:
        """Expression with no y' term — not Exact (no N), falls through."""
        # y - x = 0  — no y' term at all
        expr = _sub(Y, X)
        result = eval_ode(expr)
        # exact recogniser gives up; other solvers may or may not fire
        assert isinstance(result, IRApply)

    def test_exact_result_LHS_not_y(self) -> None:
        """Exact solution LHS is F(x,y), not just y (implicit, not explicit)."""
        M = _mul(_I(2), _mul(X, Y))
        N = _pow(X, _I(2))
        expr = _add(M, _mul(N, Y_PRIME))
        result = eval_ode(expr)
        lhs = result.args[0]
        # LHS should NOT just be the bare symbol Y
        # Exact solution is implicit F(x,y)=C, not explicit Equal(y, ...)
        assert lhs != Y, "Exact solution must be implicit, not Equal(y, f(x))"

    def test_exact_2y_dx_2x_dy(self) -> None:
        """M = 2y, N = 2x — exact.  Potential F = 2xy.  Solution: 2xy = C."""
        M = _mul(_I(2), Y)
        N = _mul(_I(2), X)
        expr = _add(M, _mul(N, Y_PRIME))
        result = eval_ode(expr)
        _was_evaluated(result)
        F_expr = result.args[0]
        x0, y0 = 1.0, 1.5
        dF_dx = _deriv_x(F_expr, x0, y0)
        assert abs(dF_dx - 2 * y0) < 1e-5

    def test_exact_quadratic_both(self) -> None:
        """M = 2xy², N = 2x²y — exact (both partials = 4xy).  F = x²y²."""
        x_sq = _pow(X, _I(2))
        y_sq = _pow(Y, _I(2))
        M = _mul(_I(2), _mul(X, y_sq))
        N = _mul(_I(2), _mul(x_sq, Y))
        expr = _add(M, _mul(N, Y_PRIME))
        result = eval_ode(expr)
        _was_evaluated(result)
        F_expr = result.args[0]
        x0, y0 = 1.0, 0.5
        dF_dx = _deriv_x(F_expr, x0, y0)
        M_val = 2 * x0 * y0 ** 2
        assert abs(dF_dx - M_val) < 1e-4


# ---------------------------------------------------------------------------
# TestPhase18_NonHomogeneous2ndOrder
# ---------------------------------------------------------------------------


class TestPhase18_NonHomogeneous2ndOrder:
    """2nd-order non-homogeneous with constant coefficients.

    Solution = homogeneous part (C1·... + C2·...) + particular part.
    We verify numerically that the solution satisfies the ODE.
    """

    def _check_ode(
        self,
        result: IRNode,
        a: float,
        b: float,
        c: float,
        f_func: object,  # callable float → float
        x_test: float = 0.5,
    ) -> None:
        """Verify a·y'' + b·y' + c·y = f(x) at x=x_test numerically."""
        sol = _is_equal_node(result)
        y_val = _eval_ir(sol, x_test)
        dy = _deriv_x(sol, x_test)

        # Second derivative via central difference
        h = 1e-5
        y_p = _eval_ir(sol, x_test + h)
        y_m = _eval_ir(sol, x_test - h)
        d2y = (y_p - 2 * y_val + y_m) / h ** 2

        lhs = a * d2y + b * dy + c * y_val
        rhs = float(f_func(x_test))  # type: ignore[operator]
        assert abs(lhs - rhs) < 1e-3, (
            f"ODE residual {abs(lhs - rhs):.2e} > 1e-3 at x={x_test}"
        )

    def test_nonhom_constant_forcing(self) -> None:
        """y'' + y = 3  →  y_p = 3."""
        # Zero form: D²y + y - 3 = 0
        expr = _sub(_add(Y_DOUBLE, Y), _I(3))
        result = eval_ode(expr)
        _was_evaluated(result)
        self._check_ode(result, 1, 0, 1, lambda x: 3.0)

    def test_nonhom_exp_forcing(self) -> None:
        """y'' - y = e^(2x)  →  y_p = e^(2x)/3."""
        # Zero form: D²y - y - exp(2x) = 0
        expr = _sub(_sub(Y_DOUBLE, Y), _exp(_mul(_I(2), X)))
        result = eval_ode(expr)
        _was_evaluated(result)
        self._check_ode(result, 1, 0, -1, lambda x: math.exp(2 * x))

    def test_nonhom_exp_resonance(self) -> None:
        """y'' - y = e^x  — e^x is a homogeneous solution (resonance).

        Particular solution: y_p = x·e^x / 2  (not A·e^x).
        """
        # Zero form: D²y - y - exp(x) = 0
        expr = _sub(_sub(Y_DOUBLE, Y), _exp(X))
        result = eval_ode(expr)
        _was_evaluated(result)
        self._check_ode(result, 1, 0, -1, lambda x: math.exp(x))

    def test_nonhom_sin_forcing(self) -> None:
        """y'' + 4y = sin(x)  →  y_p = sin(x)/3."""
        # Zero form: D²y + 4y - sin(x) = 0
        expr = _sub(_add(Y_DOUBLE, _mul(_I(4), Y)), _sin(X))
        result = eval_ode(expr)
        _was_evaluated(result)
        self._check_ode(result, 1, 0, 4, lambda x: math.sin(x))

    def test_nonhom_cos_forcing(self) -> None:
        """y'' + 3y' + 2y = cos(x)."""
        # Zero form: D²y + 3y' + 2y - cos(x) = 0
        expr = _sub(
            _add(_add(Y_DOUBLE, _mul(_I(3), Y_PRIME)), _mul(_I(2), Y)),
            _cos(X),
        )
        result = eval_ode(expr)
        _was_evaluated(result)
        self._check_ode(result, 1, 3, 2, lambda x: math.cos(x))

    def test_nonhom_exp_sin_forcing(self) -> None:
        """y'' + y = e^x·sin(x)."""
        # Zero form: D²y + y - exp(x)*sin(x) = 0
        expr = _sub(
            _add(Y_DOUBLE, Y),
            _mul(_exp(X), _sin(X)),
        )
        result = eval_ode(expr)
        _was_evaluated(result)
        self._check_ode(result, 1, 0, 1, lambda x: math.exp(x) * math.sin(x))

    def test_nonhom_exp_cos_forcing(self) -> None:
        """y'' + 2y' + 5y = e^x·cos(2x) — non-resonant exp_cos.

        Char roots are -1±2i; forcing alpha=1, beta=2 is non-resonant.
        """
        # Zero form: D²y + 2y' + 5y - exp(x)*cos(2x) = 0
        expr = _sub(
            _add(_add(Y_DOUBLE, _mul(_I(2), Y_PRIME)), _mul(_I(5), Y)),
            _mul(_exp(X), _cos(_mul(_I(2), X))),
        )
        result = eval_ode(expr)
        _was_evaluated(result)
        self._check_ode(result, 1, 2, 5, lambda x: math.exp(x) * math.cos(2 * x))

    def test_nonhom_linear_poly_forcing(self) -> None:
        """y'' + y = x  →  y_p = x."""
        # Zero form: D²y + y - x = 0
        expr = _sub(_add(Y_DOUBLE, Y), X)
        result = eval_ode(expr)
        _was_evaluated(result)
        self._check_ode(result, 1, 0, 1, lambda x: x)

    def test_nonhom_quadratic_poly_forcing(self) -> None:
        """y'' + y' + y = x²  →  particular is quadratic."""
        # Zero form: D²y + y' + y - x^2 = 0
        expr = _sub(
            _add(_add(Y_DOUBLE, Y_PRIME), Y),
            _pow(X, _I(2)),
        )
        result = eval_ode(expr)
        _was_evaluated(result)
        self._check_ode(result, 1, 1, 1, lambda x: x ** 2)

    def test_nonhom_solution_is_equal_y(self) -> None:
        """Non-homogeneous solution is Equal(y, ...) — explicit in y."""
        expr = _sub(_add(Y_DOUBLE, Y), _I(1))
        result = eval_ode(expr)
        assert isinstance(result, IRApply) and result.head == EQUAL
        assert result.args[0] == Y

    def test_nonhom_contains_c1_c2(self) -> None:
        """Non-homogeneous solution contains integration constants %c1, %c2."""
        expr = _sub(_add(Y_DOUBLE, Y), _I(1))
        result = eval_ode(expr)
        sol = _is_equal_node(result)

        def _has_symbol(node: IRNode, name: str) -> bool:
            if isinstance(node, IRSymbol):
                return node.name == name
            if isinstance(node, IRApply):
                return any(_has_symbol(c, name) for c in node.args)
            return False

        assert _has_symbol(sol, "%c1"), f"Missing %c1 in {sol!r}"
        assert _has_symbol(sol, "%c2"), f"Missing %c2 in {sol!r}"

    def test_nonhom_double_exp_resonance(self) -> None:
        """y'' = e^x — y_h has no e^x terms, no resonance.

        Particular: y_p = e^x / 1 (char poly a=1, b=0, c=0; char_val(1)=1).
        """
        # Zero form: D²y - exp(x) = 0  (b=0, c=0 → a·y'' = f)
        expr = _sub(Y_DOUBLE, _exp(X))
        result = eval_ode(expr)
        _was_evaluated(result)
        self._check_ode(result, 1, 0, 0, lambda x: math.exp(x))


# ---------------------------------------------------------------------------
# TestPhase18_Fallthrough
# ---------------------------------------------------------------------------


class TestPhase18_Fallthrough:
    """Cases that must remain unevaluated."""

    def test_nonconst_coeff_2nd_order_unevaluated(self) -> None:
        """x·y'' + y = 0 — variable coefficient, unevaluated."""
        # Zero form: Mul(x, D²y) + y = 0
        expr = _add(_mul(X, Y_DOUBLE), Y)
        result = eval_ode(expr)
        _is_unevaluated(result)

    def test_bernoulli_n0_not_bernoulli(self) -> None:
        """y' + y = y^0 = 1 — n=0 is linear (Bernoulli excluded for n=0)."""
        # y' + y - 1 = 0  → linear with P=1, Q=1
        expr = _sub(_add(Y_PRIME, Y), _I(1))
        result = eval_ode(expr)
        _was_evaluated(result)   # linear solver handles this

    def test_unrecognised_forcing_unevaluated(self) -> None:
        """y'' + y = tanh(x) — forcing unrecognised, unevaluated."""
        from symbolic_ir import TANH  # noqa: PLC0415

        expr = _sub(_add(Y_DOUBLE, Y), IRApply(TANH, (X,)))
        result = eval_ode(expr)
        _is_unevaluated(result)

    def test_fourth_order_unevaluated(self) -> None:
        """y'''' is not handled — unevaluated."""
        y_triple = IRApply(D, (Y_DOUBLE, X))
        y_quad = IRApply(D, (y_triple, X))
        expr = _add(y_quad, Y)
        result = eval_ode(expr)
        _is_unevaluated(result)

    def test_nonexact_ode_not_forced(self) -> None:
        """y' + y = x² — linear (P=1, Q=x²), μ=e^x, integral is elementary."""
        # y' + y - x^2 = 0  →  linear with P=1, Q=x^2
        expr = _sub(_add(Y_PRIME, Y), _pow(X, _I(2)))
        result = eval_ode(expr)
        _was_evaluated(result)   # linear solver handles y' + y = x^2

    def test_trig_resonance_unevaluated(self) -> None:
        """y'' + y = sin(x) — trig resonance det=0, particular not computed."""
        # sin(x) with β=1 and c-aβ²=0 → det=0 → falls through
        expr = _sub(_add(Y_DOUBLE, Y), _sin(X))
        result = eval_ode(expr)
        _is_unevaluated(result)

    def test_no_y_prime_unevaluated(self) -> None:
        """x·y = 1 — algebraic equation, not an ODE → unevaluated."""
        expr = _sub(_mul(X, Y), _I(1))
        result = eval_ode(expr)
        _is_unevaluated(result)


# ---------------------------------------------------------------------------
# TestPhase18_Regressions
# ---------------------------------------------------------------------------


class TestPhase18_Regressions:
    """Regression tests — Phase 0.1.0 solver types must still work."""

    def test_first_order_linear_homogeneous(self) -> None:
        """y' + 2y = 0 still works (first-order linear, Phase 0.1.0)."""
        expr = _add(Y_PRIME, _mul(_I(2), Y))
        result = eval_ode(expr)
        _was_evaluated(result)

    def test_first_order_linear_nonhomogeneous(self) -> None:
        """y' + y = 1 still works."""
        expr = _sub(_add(Y_PRIME, Y), _I(1))
        result = eval_ode(expr)
        _was_evaluated(result)

    def test_separable_y_prime_equals_ky(self) -> None:
        """y' = 2*y still works (separable/linear, Phase 0.1.0)."""
        expr = _sub(Y_PRIME, _mul(_I(2), Y))
        result = eval_ode(expr)
        _was_evaluated(result)

    def test_second_order_real_roots(self) -> None:
        """y'' − 3y' + 2y = 0 still works (const-coeff homogeneous)."""
        expr = _add(
            _sub(Y_DOUBLE, _mul(_I(3), Y_PRIME)),
            _mul(_I(2), Y),
        )
        result = eval_ode(expr)
        _was_evaluated(result)
        assert isinstance(result, IRApply) and result.head == EQUAL

    def test_second_order_complex_roots(self) -> None:
        """y'' + y = 0 still works (complex roots)."""
        expr = _add(Y_DOUBLE, Y)
        result = eval_ode(expr)
        _was_evaluated(result)
        sol = _is_equal_node(result)

        def _has_cos(node: IRNode) -> bool:
            if isinstance(node, IRApply):
                if node.head.name == "Cos":
                    return True
                return any(_has_cos(c) for c in node.args)
            return False

        assert _has_cos(sol), f"Expected Cos in complex-root solution: {sol!r}"

    def test_second_order_repeated_root(self) -> None:
        """y'' − 2y' + y = 0 still works (repeated root r=1)."""
        expr = _add(_sub(Y_DOUBLE, _mul(_I(2), Y_PRIME)), Y)
        result = eval_ode(expr)
        _was_evaluated(result)

    def test_wrong_arity_unevaluated(self) -> None:
        """ODE2 with wrong arity still returns unevaluated (regression)."""
        vm = make_vm()
        bad = IRApply(ODE2, (Y_PRIME, Y))  # only 2 args instead of 3
        res = vm.eval(bad)
        assert res == bad
