"""Phase 14 integration tests — hyperbolic powers and exp × hyperbolic.

Also covers **Group E** matrix-operation handlers (Dot, Trace, Dimensions,
IdentityMatrix, ZeroMatrix, Rank, RowReduce) wired into SymbolicBackend.

Integration families tested
---------------------------
14a. ∫ exp(ax+b) · sinh(cx+d) dx  — double IBP closed form:

         exp(ax+b) · [a·sinh(cx+d) − c·cosh(cx+d)] / (a²−c²)

14b. ∫ exp(ax+b) · cosh(cx+d) dx  — double IBP closed form:

         exp(ax+b) · [a·cosh(cx+d) − c·sinh(cx+d)] / (a²−c²)

14c. ∫ sinh^n(ax+b) dx  — IBP reduction (n ≥ 2):

         I_n = (1/(na))·sinh^(n-1)·cosh − (n-1)/n · I_{n-2}
         Base: I_0 = x,  I_1 = cosh(ax+b)/a

14d. ∫ cosh^n(ax+b) dx  — IBP reduction (n ≥ 2):

         I_n = (1/(na))·cosh^(n-1)·sinh + (n-1)/n · I_{n-2}   [note: + sign]
         Base: I_0 = x,  I_1 = sinh(ax+b)/a

14e. ∫ sinh^m(ax+b)·cosh^n(ax+b) dx  — u-substitution when one exponent = 1:

         m=1: cosh^(n+1)/(n+1)/a
         n=1: sinh^(m+1)/(m+1)/a

Integration correctness is verified numerically: differentiate the
antiderivative at two test points and confirm the result matches the
original integrand.

Group E matrix handlers
-----------------------
All seven new handlers are exercised: Dot, Trace, Dimensions, IdentityMatrix,
ZeroMatrix, Rank, RowReduce.

Test points
-----------
- General: x₀ = 0.3, x₁ = 0.6
"""

from __future__ import annotations

import math

from symbolic_ir import (
    ADD,
    COSH,
    DIV,
    EXP,
    INTEGRATE,
    MUL,
    NEG,
    POW,
    SINH,
    SUB,
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
# Shared helpers
# ---------------------------------------------------------------------------

X = IRSymbol("x")
_INT = IRInteger
_RAT = lambda n, d: IRRational(n, d)  # noqa: E731

_TP = (0.3, 0.6)  # general test points

# IR head symbols used by matrix tests
_LIST = IRSymbol("List")
_MATRIX = IRSymbol("Matrix")
_DOT = IRSymbol("Dot")
_TRACE = IRSymbol("Trace")
_DIMENSIONS = IRSymbol("Dimensions")
_IDENTITY_MATRIX = IRSymbol("IdentityMatrix")
_ZERO_MATRIX = IRSymbol("ZeroMatrix")
_RANK = IRSymbol("Rank")
_ROW_REDUCE = IRSymbol("RowReduce")


def _make_vm() -> VM:
    return VM(SymbolicBackend())


def _eval_ir(node: IRNode, x_val: float) -> float:  # noqa: PLR0911
    """Numerically evaluate an IR tree at x = x_val."""
    if isinstance(node, IRInteger):
        return float(node.value)
    if isinstance(node, IRRational):
        return node.numer / node.denom
    if isinstance(node, IRFloat):
        return node.value
    if isinstance(node, IRSymbol):
        if node.name == "x":
            return x_val
        raise ValueError(f"Unknown symbol: {node.name}")
    if not isinstance(node, IRApply):
        raise TypeError(f"Unexpected node: {node!r}")
    head = node.head.name
    if head == "Add":
        return _eval_ir(node.args[0], x_val) + _eval_ir(node.args[1], x_val)
    if head == "Sub":
        return _eval_ir(node.args[0], x_val) - _eval_ir(node.args[1], x_val)
    if head == "Mul":
        return _eval_ir(node.args[0], x_val) * _eval_ir(node.args[1], x_val)
    if head == "Div":
        return _eval_ir(node.args[0], x_val) / _eval_ir(node.args[1], x_val)
    if head == "Neg":
        return -_eval_ir(node.args[0], x_val)
    if head == "Inv":
        return 1.0 / _eval_ir(node.args[0], x_val)
    if head == "Log":
        return math.log(abs(_eval_ir(node.args[0], x_val)))
    if head == "Pow":
        base = _eval_ir(node.args[0], x_val)
        exp = _eval_ir(node.args[1], x_val)
        return base**exp
    if head == "Sqrt":
        return math.sqrt(abs(_eval_ir(node.args[0], x_val)))
    if head == "Exp":
        return math.exp(_eval_ir(node.args[0], x_val))
    if head == "Sin":
        return math.sin(_eval_ir(node.args[0], x_val))
    if head == "Cos":
        return math.cos(_eval_ir(node.args[0], x_val))
    if head == "Sinh":
        return math.sinh(_eval_ir(node.args[0], x_val))
    if head == "Cosh":
        return math.cosh(_eval_ir(node.args[0], x_val))
    if head == "Tanh":
        return math.tanh(_eval_ir(node.args[0], x_val))
    if head == "Asin":
        return math.asin(_eval_ir(node.args[0], x_val))
    if head == "Acos":
        return math.acos(_eval_ir(node.args[0], x_val))
    if head == "Atan":
        return math.atan(_eval_ir(node.args[0], x_val))
    if head == "Asinh":
        return math.asinh(_eval_ir(node.args[0], x_val))
    if head == "Acosh":
        return math.acosh(_eval_ir(node.args[0], x_val))
    if head == "Atanh":
        return math.atanh(_eval_ir(node.args[0], x_val))
    raise ValueError(f"Unhandled head: {head}")


def _numerical_deriv(node: IRNode, x_val: float, h: float = 1e-7) -> float:
    return (_eval_ir(node, x_val + h) - _eval_ir(node, x_val - h)) / (2 * h)


def _check_antiderivative(
    integrand: IRNode,
    antideriv: IRNode,
    test_points: tuple[float, ...] = _TP,
    atol: float = 1e-6,
    rtol: float = 1e-6,
) -> None:
    """Verify F'(x) ≈ f(x) numerically at each test point."""
    for x_val in test_points:
        expected = _eval_ir(integrand, x_val)
        actual = _numerical_deriv(antideriv, x_val)
        tol = atol + rtol * abs(expected)
        assert abs(actual - expected) < tol, (
            f"At x={x_val}: F'={actual:.8f}, f={expected:.8f}, "
            f"diff={abs(actual - expected):.2e}"
        )


def _integrate_ir(vm: VM, integrand_ir: IRNode) -> IRNode:
    return vm.eval(IRApply(INTEGRATE, (integrand_ir, X)))


def _was_evaluated(f: IRNode, F: IRNode) -> None:
    assert IRApply(INTEGRATE, (f, X)) != F, (
        "Expected a closed-form antiderivative, got an unevaluated Integrate"
    )


def _is_unevaluated(f: IRNode, F: IRNode) -> None:
    assert IRApply(INTEGRATE, (f, X)) == F, (
        "Expected an unevaluated Integrate, got a closed form"
    )


# ---------------------------------------------------------------------------
# IR builder helpers
# ---------------------------------------------------------------------------


def _mul(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(MUL, (a, b))


def _add(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(ADD, (a, b))


def _sub(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(SUB, (a, b))


def _neg(a: IRNode) -> IRNode:
    return IRApply(NEG, (a,))


def _div(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(DIV, (a, b))


def _pow(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(POW, (a, b))


def _exp(arg: IRNode) -> IRNode:
    return IRApply(EXP, (arg,))


def _sinh(arg: IRNode) -> IRNode:
    return IRApply(SINH, (arg,))


def _cosh(arg: IRNode) -> IRNode:
    return IRApply(COSH, (arg,))


def _lin(a: int | IRNode, b: int = 0) -> IRNode:
    """Build a·x + b as an IR node."""
    a_node: IRNode = _INT(a) if isinstance(a, int) else a
    is_one = isinstance(a_node, IRInteger) and a_node.value == 1
    ax: IRNode = X if is_one else _mul(a_node, X)
    if b == 0:
        return ax
    return _add(ax, _INT(b))


# ---------------------------------------------------------------------------
# Matrix IR helpers
# ---------------------------------------------------------------------------


def _irow(*args: IRNode) -> IRApply:
    return IRApply(_LIST, tuple(args))


def _imat(*rows: IRApply) -> IRApply:
    return IRApply(_MATRIX, tuple(rows))


def _m2x2(a: int, b: int, c: int, d: int) -> IRApply:
    """Build an unevaluated 2×2 integer matrix."""
    return _imat(
        _irow(_INT(a), _INT(b)),
        _irow(_INT(c), _INT(d)),
    )


def _m3x3(vals: list[list[int]]) -> IRApply:
    """Build an unevaluated 3×3 integer matrix from nested list."""
    rows = tuple(_irow(*(_INT(v) for v in row)) for row in vals)
    return IRApply(_MATRIX, rows)


# ---------------------------------------------------------------------------
# Class 1: exp × sinh — double IBP closed form
# ---------------------------------------------------------------------------


class TestPhase14_ExpSinh:
    """∫ exp(ax+b) · sinh(cx+d) dx = exp(ax+b)·[a·sinh − c·cosh] / (a²−c²)."""

    def test_exp_2x_sinh_x(self) -> None:
        """∫ exp(2x)·sinh(x) dx  (a=2, c=1, D=3)."""
        vm = _make_vm()
        f = _mul(_exp(_lin(2)), _sinh(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_exp_x_sinh_2x(self) -> None:
        """∫ exp(x)·sinh(2x) dx  (a=1, c=2, D=−3)."""
        vm = _make_vm()
        f = _mul(_exp(X), _sinh(_lin(2)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_exp_3x_sinh_x(self) -> None:
        """∫ exp(3x)·sinh(x) dx  (a=3, c=1, D=8)."""
        vm = _make_vm()
        f = _mul(_exp(_lin(3)), _sinh(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_exp_2x_plus_1_sinh_x(self) -> None:
        """∫ exp(2x+1)·sinh(x) dx  (b≠0 on exponent side)."""
        vm = _make_vm()
        f = _mul(_exp(_lin(2, 1)), _sinh(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_exp_2x_sinh_x_plus_1(self) -> None:
        """∫ exp(2x)·sinh(x+1) dx  (d≠0 on hyperbolic side)."""
        vm = _make_vm()
        f = _mul(_exp(_lin(2)), _sinh(_add(X, _INT(1))))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_exp_3x_sinh_2x(self) -> None:
        """∫ exp(3x)·sinh(2x) dx  (a=3, c=2, D=5)."""
        vm = _make_vm()
        f = _mul(_exp(_lin(3)), _sinh(_lin(2)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_exp_sinh_commutativity(self) -> None:
        """∫ sinh(x)·exp(2x) dx = ∫ exp(2x)·sinh(x) dx (MUL arg order)."""
        vm = _make_vm()
        f1 = _mul(_exp(_lin(2)), _sinh(X))
        f2 = _mul(_sinh(X), _exp(_lin(2)))
        F1 = _integrate_ir(vm, f1)
        F2 = _integrate_ir(vm, f2)
        _check_antiderivative(f1, F1)
        _check_antiderivative(f2, F2)


# ---------------------------------------------------------------------------
# Class 2: exp × cosh — double IBP closed form
# ---------------------------------------------------------------------------


class TestPhase14_ExpCosh:
    """∫ exp(ax+b) · cosh(cx+d) dx = exp(ax+b)·[a·cosh − c·sinh] / (a²−c²)."""

    def test_exp_2x_cosh_x(self) -> None:
        """∫ exp(2x)·cosh(x) dx  (a=2, c=1, D=3)."""
        vm = _make_vm()
        f = _mul(_exp(_lin(2)), _cosh(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_exp_x_cosh_2x(self) -> None:
        """∫ exp(x)·cosh(2x) dx  (a=1, c=2, D=−3)."""
        vm = _make_vm()
        f = _mul(_exp(X), _cosh(_lin(2)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_exp_3x_cosh_2x(self) -> None:
        """∫ exp(3x)·cosh(2x) dx  (a=3, c=2, D=5)."""
        vm = _make_vm()
        f = _mul(_exp(_lin(3)), _cosh(_lin(2)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_exp_2x_plus_1_cosh_x_plus_1(self) -> None:
        """∫ exp(2x+1)·cosh(x+1) dx  (both shifted)."""
        vm = _make_vm()
        f = _mul(_exp(_lin(2, 1)), _cosh(_add(X, _INT(1))))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_exp_cosh_commutativity(self) -> None:
        """∫ cosh(x)·exp(2x) dx = ∫ exp(2x)·cosh(x) dx (MUL arg order)."""
        vm = _make_vm()
        f1 = _mul(_exp(_lin(2)), _cosh(X))
        f2 = _mul(_cosh(X), _exp(_lin(2)))
        F1 = _integrate_ir(vm, f1)
        F2 = _integrate_ir(vm, f2)
        _check_antiderivative(f1, F1)
        _check_antiderivative(f2, F2)


# ---------------------------------------------------------------------------
# Class 3: sinh^n power reduction
# ---------------------------------------------------------------------------


class TestPhase14_SinhPowers:
    """∫ sinh^n(ax+b) dx via IBP reduction formula."""

    def test_sinh_squared(self) -> None:
        """∫ sinh²(x) dx = (1/2)·sinh(x)·cosh(x) − x/2."""
        vm = _make_vm()
        f = _pow(_sinh(X), _INT(2))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_sinh_cubed(self) -> None:
        """∫ sinh³(x) dx = (1/3)·sinh²(x)·cosh(x) − (2/3)·cosh(x)."""
        vm = _make_vm()
        f = _pow(_sinh(X), _INT(3))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_sinh_fourth(self) -> None:
        """∫ sinh⁴(x) dx — applies reduction twice."""
        vm = _make_vm()
        f = _pow(_sinh(X), _INT(4))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_sinh_fifth(self) -> None:
        """∫ sinh⁵(x) dx — applies reduction three times, ending at sinh."""
        vm = _make_vm()
        f = _pow(_sinh(X), _INT(5))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_sinh_squared_2x(self) -> None:
        """∫ sinh²(2x) dx — linear arg with a=2."""
        vm = _make_vm()
        f = _pow(_sinh(_lin(2)), _INT(2))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_sinh_cubed_2x_plus_1(self) -> None:
        """∫ sinh³(2x+1) dx — linear arg with a=2, b=1."""
        vm = _make_vm()
        f = _pow(_sinh(_lin(2, 1)), _INT(3))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_sinh_squared_half_x(self) -> None:
        """∫ sinh²(x/2) dx — linear arg with a=1/2."""
        vm = _make_vm()
        arg = _div(X, _INT(2))
        f = _pow(_sinh(arg), _INT(2))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)


# ---------------------------------------------------------------------------
# Class 4: cosh^n power reduction
# ---------------------------------------------------------------------------


class TestPhase14_CoshPowers:
    """∫ cosh^n(ax+b) dx via IBP reduction formula.

    Key difference from sinh^n: the recursive term is +, not −, because
    d/dt[cosh] = sinh (positive).
    """

    def test_cosh_squared(self) -> None:
        """∫ cosh²(x) dx = (1/2)·cosh(x)·sinh(x) + x/2."""
        vm = _make_vm()
        f = _pow(_cosh(X), _INT(2))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_cosh_cubed(self) -> None:
        """∫ cosh³(x) dx = (1/3)·cosh²(x)·sinh(x) + (2/3)·sinh(x)."""
        vm = _make_vm()
        f = _pow(_cosh(X), _INT(3))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_cosh_fourth(self) -> None:
        """∫ cosh⁴(x) dx — applies reduction twice."""
        vm = _make_vm()
        f = _pow(_cosh(X), _INT(4))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_cosh_fifth(self) -> None:
        """∫ cosh⁵(x) dx — applies reduction three times, ending at cosh."""
        vm = _make_vm()
        f = _pow(_cosh(X), _INT(5))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_cosh_squared_2x(self) -> None:
        """∫ cosh²(2x) dx — linear arg with a=2."""
        vm = _make_vm()
        f = _pow(_cosh(_lin(2)), _INT(2))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_cosh_cubed_x_plus_1(self) -> None:
        """∫ cosh³(x+1) dx — constant shift b=1."""
        vm = _make_vm()
        f = _pow(_cosh(_add(X, _INT(1))), _INT(3))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_cosh_squared_half_x(self) -> None:
        """∫ cosh²(x/2) dx — linear arg with a=1/2."""
        vm = _make_vm()
        arg = _div(X, _INT(2))
        f = _pow(_cosh(arg), _INT(2))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)


# ---------------------------------------------------------------------------
# Class 5: sinh^m × cosh^n — u-substitution (one exponent = 1)
# ---------------------------------------------------------------------------


class TestPhase14_SinhCoshProduct:
    """∫ sinh^m · cosh^n dx via u-substitution when min(m,n) = 1."""

    def test_sinh_x_cosh_x(self) -> None:
        """∫ sinh(x)·cosh(x) dx = cosh²(x)/2  (or equivalently sinh²(x)/2)."""
        vm = _make_vm()
        f = _mul(_sinh(X), _cosh(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_sinh_x_cosh_squared_x(self) -> None:
        """∫ sinh(x)·cosh²(x) dx = cosh³(x)/3."""
        vm = _make_vm()
        f = _mul(_sinh(X), _pow(_cosh(X), _INT(2)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_sinh_x_cosh_cubed_x(self) -> None:
        """∫ sinh(x)·cosh³(x) dx = cosh⁴(x)/4."""
        vm = _make_vm()
        f = _mul(_sinh(X), _pow(_cosh(X), _INT(3)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_sinh_squared_x_cosh_x(self) -> None:
        """∫ sinh²(x)·cosh(x) dx = sinh³(x)/3."""
        vm = _make_vm()
        f = _mul(_pow(_sinh(X), _INT(2)), _cosh(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_sinh_cubed_x_cosh_x(self) -> None:
        """∫ sinh³(x)·cosh(x) dx = sinh⁴(x)/4."""
        vm = _make_vm()
        f = _mul(_pow(_sinh(X), _INT(3)), _cosh(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_cosh_squared_sinh_commutativity(self) -> None:
        """∫ cosh²(x)·sinh(x) dx = ∫ sinh(x)·cosh²(x) dx (MUL arg order)."""
        vm = _make_vm()
        f1 = _mul(_sinh(X), _pow(_cosh(X), _INT(2)))
        f2 = _mul(_pow(_cosh(X), _INT(2)), _sinh(X))
        F1 = _integrate_ir(vm, f1)
        F2 = _integrate_ir(vm, f2)
        _check_antiderivative(f1, F1)
        _check_antiderivative(f2, F2)

    def test_sinh_cosh_linear_arg(self) -> None:
        """∫ sinh(2x)·cosh(2x) dx = cosh²(2x)/4  (same linear arg, a=2)."""
        vm = _make_vm()
        f = _mul(_sinh(_lin(2)), _cosh(_lin(2)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)


# ---------------------------------------------------------------------------
# Class 6: Group E matrix handlers
# ---------------------------------------------------------------------------


class TestPhase14_MatrixOps:
    """Group E matrix-operation handlers: Dot, Trace, Dimensions,
    IdentityMatrix, ZeroMatrix, Rank, RowReduce."""

    # -- Dot (matrix product) -----------------------------------------------

    def test_dot_2x2(self) -> None:
        """Dot([[1,2],[3,4]], [[5,6],[7,8]]) = [[19,22],[43,50]]."""
        from cas_matrix import get_entry

        vm = _make_vm()
        A = vm.eval(_m2x2(1, 2, 3, 4))
        B = vm.eval(_m2x2(5, 6, 7, 8))
        result = vm.eval(IRApply(_DOT, (A, B)))
        assert isinstance(result, IRApply)
        assert result.head.name == "Matrix"
        assert get_entry(result, 1, 1) == _INT(19)
        assert get_entry(result, 1, 2) == _INT(22)
        assert get_entry(result, 2, 1) == _INT(43)
        assert get_entry(result, 2, 2) == _INT(50)

    def test_dot_identity(self) -> None:
        """Dot(I₂, A) = A for any 2×2 A."""
        from cas_matrix import get_entry

        vm = _make_vm()
        I2 = vm.eval(IRApply(_IDENTITY_MATRIX, (_INT(2),)))
        A = vm.eval(_m2x2(3, 7, 2, 5))
        result = vm.eval(IRApply(_DOT, (I2, A)))
        assert isinstance(result, IRApply)
        assert get_entry(result, 1, 1) == _INT(3)
        assert get_entry(result, 2, 2) == _INT(5)

    # -- Trace ---------------------------------------------------------------

    def test_trace_2x2(self) -> None:
        """Trace([[1,2],[3,4]]) = 1 + 4 = 5."""
        vm = _make_vm()
        M = vm.eval(_m2x2(1, 2, 3, 4))
        result = vm.eval(IRApply(_TRACE, (M,)))
        assert result == _INT(5)

    def test_trace_3x3(self) -> None:
        """Trace([[1,0,0],[0,2,0],[0,0,3]]) = 6."""
        vm = _make_vm()
        M = vm.eval(_m3x3([[1, 0, 0], [0, 2, 0], [0, 0, 3]]))
        result = vm.eval(IRApply(_TRACE, (M,)))
        assert result == _INT(6)

    # -- Dimensions ----------------------------------------------------------

    def test_dimensions_square(self) -> None:
        """Dimensions([[1,2],[3,4]]) = List(2, 2)."""
        vm = _make_vm()
        M = vm.eval(_m2x2(1, 2, 3, 4))
        result = vm.eval(IRApply(_DIMENSIONS, (M,)))
        assert isinstance(result, IRApply)
        assert result.head.name == "List"
        assert result.args[0] == _INT(2)
        assert result.args[1] == _INT(2)

    def test_dimensions_rectangular(self) -> None:
        """Dimensions(2×3 matrix) = List(2, 3)."""
        vm = _make_vm()
        M = vm.eval(IRApply(_MATRIX, (
            _irow(_INT(1), _INT(2), _INT(3)),
            _irow(_INT(4), _INT(5), _INT(6)),
        )))
        result = vm.eval(IRApply(_DIMENSIONS, (M,)))
        assert isinstance(result, IRApply)
        assert result.args[0] == _INT(2)
        assert result.args[1] == _INT(3)

    # -- IdentityMatrix ------------------------------------------------------

    def test_identity_matrix_2(self) -> None:
        """IdentityMatrix(2) = [[1,0],[0,1]]."""
        from cas_matrix import get_entry

        vm = _make_vm()
        result = vm.eval(IRApply(_IDENTITY_MATRIX, (_INT(2),)))
        assert isinstance(result, IRApply)
        assert result.head.name == "Matrix"
        assert get_entry(result, 1, 1) == _INT(1)
        assert get_entry(result, 1, 2) == _INT(0)
        assert get_entry(result, 2, 1) == _INT(0)
        assert get_entry(result, 2, 2) == _INT(1)

    def test_identity_matrix_3_diagonal(self) -> None:
        """IdentityMatrix(3): diagonal entries are 1, off-diagonal are 0."""
        from cas_matrix import get_entry

        vm = _make_vm()
        result = vm.eval(IRApply(_IDENTITY_MATRIX, (_INT(3),)))
        assert isinstance(result, IRApply)
        # Diagonal
        assert get_entry(result, 1, 1) == _INT(1)
        assert get_entry(result, 2, 2) == _INT(1)
        assert get_entry(result, 3, 3) == _INT(1)
        # Off-diagonal
        assert get_entry(result, 1, 2) == _INT(0)
        assert get_entry(result, 2, 3) == _INT(0)

    # -- ZeroMatrix ----------------------------------------------------------

    def test_zero_matrix_2x3(self) -> None:
        """ZeroMatrix(2, 3) — all entries are 0."""
        from cas_matrix import get_entry

        vm = _make_vm()
        result = vm.eval(IRApply(_ZERO_MATRIX, (_INT(2), _INT(3))))
        assert isinstance(result, IRApply)
        assert result.head.name == "Matrix"
        assert get_entry(result, 1, 1) == _INT(0)
        assert get_entry(result, 2, 3) == _INT(0)

    def test_zero_matrix_1arg(self) -> None:
        """ZeroMatrix(2) — square 2×2 zero matrix (1-argument form)."""
        from cas_matrix import get_entry

        vm = _make_vm()
        result = vm.eval(IRApply(_ZERO_MATRIX, (_INT(2),)))
        assert isinstance(result, IRApply)
        assert result.head.name == "Matrix"
        assert get_entry(result, 1, 2) == _INT(0)
        assert get_entry(result, 2, 1) == _INT(0)

    # -- Rank ----------------------------------------------------------------

    def test_rank_full_rank_2x2(self) -> None:
        """Rank([[1,2],[3,4]]) = 2 (full rank, det ≠ 0)."""
        vm = _make_vm()
        M = vm.eval(_m2x2(1, 2, 3, 4))
        result = vm.eval(IRApply(_RANK, (M,)))
        assert result == _INT(2)

    def test_rank_singular_2x2(self) -> None:
        """Rank([[1,2],[2,4]]) = 1 (row2 = 2·row1)."""
        vm = _make_vm()
        M = vm.eval(_m2x2(1, 2, 2, 4))
        result = vm.eval(IRApply(_RANK, (M,)))
        assert result == _INT(1)

    # -- RowReduce -----------------------------------------------------------

    def test_row_reduce_identity_unchanged(self) -> None:
        """RowReduce(I₂) = I₂."""
        from cas_matrix import get_entry

        vm = _make_vm()
        M = vm.eval(_m2x2(1, 0, 0, 1))
        result = vm.eval(IRApply(_ROW_REDUCE, (M,)))
        assert isinstance(result, IRApply)
        assert get_entry(result, 1, 1) == _INT(1)
        assert get_entry(result, 2, 2) == _INT(1)
        assert get_entry(result, 1, 2) == _INT(0)
        assert get_entry(result, 2, 1) == _INT(0)

    def test_row_reduce_full_rank_2x2(self) -> None:
        """RowReduce([[1,2],[3,4]]) = [[1,0],[0,1]]."""
        from cas_matrix import get_entry

        vm = _make_vm()
        M = vm.eval(_m2x2(1, 2, 3, 4))
        result = vm.eval(IRApply(_ROW_REDUCE, (M,)))
        assert isinstance(result, IRApply)
        assert result.head.name == "Matrix"
        assert get_entry(result, 1, 1) == _INT(1)
        assert get_entry(result, 1, 2) == _INT(0)
        assert get_entry(result, 2, 1) == _INT(0)
        assert get_entry(result, 2, 2) == _INT(1)


# ---------------------------------------------------------------------------
# Class 7: Fallthrough cases
# ---------------------------------------------------------------------------


class TestPhase14_Fallthroughs:
    """Integrals that Phase 14 cannot evaluate — should remain Integrate(...)."""

    def test_exp_sinh_degenerate_same_coeff(self) -> None:
        """∫ exp(x)·sinh(x) dx — a=c=1, D=0 → unevaluated."""
        vm = _make_vm()
        f = _mul(_exp(X), _sinh(X))
        F = _integrate_ir(vm, f)
        _is_unevaluated(f, F)

    def test_exp_cosh_degenerate_same_coeff(self) -> None:
        """∫ exp(x)·cosh(x) dx — a=c=1, D=0 → unevaluated."""
        vm = _make_vm()
        f = _mul(_exp(X), _cosh(X))
        F = _integrate_ir(vm, f)
        _is_unevaluated(f, F)

    def test_sinh_squared_cosh_squared(self) -> None:
        """∫ sinh²(x)·cosh²(x) dx — both exponents > 1, u-sub returns None."""
        vm = _make_vm()
        f = _mul(_pow(_sinh(X), _INT(2)), _pow(_cosh(X), _INT(2)))
        F = _integrate_ir(vm, f)
        _is_unevaluated(f, F)

    def test_sinh_power_non_integer_exp(self) -> None:
        """∫ sinh(x)^(1/2) dx — fractional exponent, not handled."""
        vm = _make_vm()
        f = _pow(_sinh(X), _RAT(1, 2))
        F = _integrate_ir(vm, f)
        _is_unevaluated(f, F)

    def test_sinh_power_nonlinear_arg(self) -> None:
        """∫ sinh(x²)² dx — non-linear argument, not handled."""
        vm = _make_vm()
        f = _pow(_sinh(_pow(X, _INT(2))), _INT(2))
        F = _integrate_ir(vm, f)
        _is_unevaluated(f, F)

    def test_sinh_cosh_different_linear_args(self) -> None:
        """∫ sinh(x)·cosh(2x) dx — different linear args, not handled."""
        vm = _make_vm()
        f = _mul(_sinh(X), _cosh(_lin(2)))
        F = _integrate_ir(vm, f)
        _is_unevaluated(f, F)


# ---------------------------------------------------------------------------
# Class 8: Regression tests — earlier phases must still work
# ---------------------------------------------------------------------------


class TestPhase14_Regressions:
    """Verify that Phase 14 additions do not break earlier phase integrals."""

    def test_phase13_x_sinh_x(self) -> None:
        """Phase 13 regression: ∫ x·sinh(x) dx (tabular IBP)."""
        vm = _make_vm()
        f = _mul(X, _sinh(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_phase13_x_cosh_x(self) -> None:
        """Phase 13 regression: ∫ x·cosh(x) dx (tabular IBP)."""
        vm = _make_vm()
        f = _mul(X, _cosh(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_phase12_x_asin_x(self) -> None:
        """Phase 12 regression: ∫ x·asin(x) dx."""
        from symbolic_ir import ASIN

        vm = _make_vm()
        f = _mul(X, IRApply(ASIN, (X,)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_phase4a_x_sin_x(self) -> None:
        """Phase 4a regression: ∫ x·sin(x) dx = sin(x) − x·cos(x)."""
        from symbolic_ir import SIN

        vm = _make_vm()
        f = _mul(X, IRApply(SIN, (X,)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_phase1_exp_x(self) -> None:
        """Phase 1 regression: ∫ exp(x) dx = exp(x)."""
        vm = _make_vm()
        f = _exp(X)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)


# ---------------------------------------------------------------------------
# Class 9: Macsyma string interface end-to-end tests
# ---------------------------------------------------------------------------


class TestPhase14_Macsyma:
    """End-to-end tests via the Macsyma string interface."""

    def _macsyma_integrate(self, integrand: str, var: str = "x") -> IRNode:
        from macsyma_compiler import compile_macsyma
        from macsyma_parser import parse_macsyma

        vm = VM(SymbolicBackend())
        ast = parse_macsyma(f"integrate({integrand}, {var});")
        ir = compile_macsyma(ast)[0]
        return vm.eval(ir)

    def test_integrate_exp_2x_sinh_x(self) -> None:
        """integrate(exp(2*x)*sinh(x), x) produces a closed form."""
        result = self._macsyma_integrate("exp(2*x)*sinh(x)")
        x_sym = IRSymbol("x")
        unevaluated = IRApply(
            INTEGRATE,
            (
                IRApply(MUL, (_exp(_mul(_INT(2), x_sym)), _sinh(x_sym))),
                x_sym,
            ),
        )
        assert result != unevaluated

    def test_integrate_sinh_squared(self) -> None:
        """integrate(sinh(x)^2, x) produces a closed form."""
        result = self._macsyma_integrate("sinh(x)^2")
        x_sym = IRSymbol("x")
        unevaluated = IRApply(
            INTEGRATE,
            (IRApply(POW, (IRApply(SINH, (x_sym,)), _INT(2))), x_sym),
        )
        assert result != unevaluated

    def test_integrate_cosh_cubed(self) -> None:
        """integrate(cosh(x)^3, x) produces a closed form."""
        result = self._macsyma_integrate("cosh(x)^3")
        x_sym = IRSymbol("x")
        unevaluated = IRApply(
            INTEGRATE,
            (IRApply(POW, (IRApply(COSH, (x_sym,)), _INT(3))), x_sym),
        )
        assert result != unevaluated

    def test_integrate_sinh_x_cosh_squared_x(self) -> None:
        """integrate(sinh(x)*cosh(x)^2, x) produces a closed form."""
        result = self._macsyma_integrate("sinh(x)*cosh(x)^2")
        x_sym = IRSymbol("x")
        unevaluated = IRApply(
            INTEGRATE,
            (
                IRApply(
                    MUL,
                    (
                        IRApply(SINH, (x_sym,)),
                        IRApply(POW, (IRApply(COSH, (x_sym,)), _INT(2))),
                    ),
                ),
                x_sym,
            ),
        )
        assert result != unevaluated
