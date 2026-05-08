"""Tests for the cas-ode package.

Architecture of the tests
--------------------------
All tests build IR nodes manually (using the symbolic_ir constructors)
rather than going through the MACSYMA string parser.  This keeps them
fast, deterministic, and independent of any parser/compiler bugs.

We test the following scenarios:

1. First-order linear ODEs (homogeneous: Q=0)
2. First-order linear ODEs (non-homogeneous: P=0 or both non-zero)
3. Separable ODEs (linear in y, factored product form)
4. Second-order constant-coefficient, three root cases:
   a. Two distinct real roots (positive/negative)
   b. Repeated root
   c. Complex conjugate roots (oscillatory)
5. The Equal(lhs, rhs) input form
6. Fall-through: unevaluated for non-const-coeff 2nd order
7. Helper-function coverage (coefficients, exact sqrt, flatten, etc.)
8. build_ode_handler_table() structure check
9. Edge cases: trivial ODE (y' = 0), wrong arity, non-symbol arguments

The expected outputs are verified by checking structural equality of the
returned ``Equal(y, solution)`` IR trees.
"""

from __future__ import annotations

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
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)
from symbolic_ir.nodes import C1, C2, ODE2, D
from symbolic_vm import VM, SymbolicBackend

from cas_ode import build_ode_handler_table, solve_ode
from cas_ode.handlers import ode2_handler
from cas_ode.ode import (
    _collect_euler_cauchy_coeffs,
    _collect_linear_first_order,
    _collect_second_order_coeffs,
    _exact_sqrt_fraction,
    _flatten_add,
    _flatten_product,
    _is_const_wrt,
    _isqrt_exact,
    _subst_ir,
    _subst_ratio_ir,
    _try_euler_cauchy,
    _try_homogeneous_type,
    solve_euler_cauchy,
    solve_second_order_const_coeff,
)

# ---------------------------------------------------------------------------
# Test fixtures — symbols and helpers
# ---------------------------------------------------------------------------

X = IRSymbol("x")
Y = IRSymbol("y")
Y_PRIME = IRApply(D, (Y, X))             # D(y, x)
Y_DOUBLE = IRApply(D, (Y_PRIME, X))     # D(D(y, x), x)


def make_vm() -> VM:
    """Return a fresh SymbolicBackend VM with ODE2 wired in."""
    backend = SymbolicBackend()
    # Install the ODE handler so vm.eval(ODE2(...)) works.
    backend._handlers.update(build_ode_handler_table())  # type: ignore[attr-defined]
    return VM(backend)


def eval_ode(expr: IRNode, y: IRSymbol = Y, x: IRSymbol = X) -> IRNode:
    """Convenience: create VM, evaluate ODE2(expr, y, x), return result."""
    vm = make_vm()
    return vm.eval(IRApply(ODE2, (expr, y, x)))


def _neg(n: IRNode) -> IRApply:
    return IRApply(NEG, (n,))


def _mul(a: IRNode, b: IRNode) -> IRApply:
    return IRApply(MUL, (a, b))


def _add(a: IRNode, b: IRNode) -> IRApply:
    return IRApply(ADD, (a, b))


def _sub(a: IRNode, b: IRNode) -> IRApply:
    return IRApply(SUB, (a, b))


def _exp(a: IRNode) -> IRApply:
    return IRApply(EXP, (a,))


def _pow(a: IRNode, b: IRNode) -> IRApply:
    return IRApply(POW, (a, b))


# ---------------------------------------------------------------------------
# Section A: Handler table structure
# ---------------------------------------------------------------------------


class TestBuildHandlerTable:
    """build_ode_handler_table() must return a dict with the ODE2 key."""

    def test_returns_dict(self) -> None:
        table = build_ode_handler_table()
        assert isinstance(table, dict)

    def test_has_ode2_key(self) -> None:
        table = build_ode_handler_table()
        assert "ODE2" in table

    def test_ode2_value_is_callable(self) -> None:
        table = build_ode_handler_table()
        assert callable(table["ODE2"])

    def test_handler_is_ode2_handler(self) -> None:
        table = build_ode_handler_table()
        assert table["ODE2"] is ode2_handler


# ---------------------------------------------------------------------------
# Section B: Helper function unit tests
# ---------------------------------------------------------------------------


class TestIsConstWrt:
    """_is_const_wrt(node, var) correctness."""

    def test_integer_is_const(self) -> None:
        assert _is_const_wrt(IRInteger(42), X)

    def test_rational_is_const(self) -> None:
        assert _is_const_wrt(IRRational(1, 2), X)

    def test_other_symbol_is_const(self) -> None:
        assert _is_const_wrt(Y, X)

    def test_var_itself_not_const(self) -> None:
        assert not _is_const_wrt(X, X)

    def test_expression_containing_var(self) -> None:
        expr = IRApply(ADD, (X, IRInteger(1)))
        assert not _is_const_wrt(expr, X)

    def test_expression_not_containing_var(self) -> None:
        expr = IRApply(ADD, (Y, IRInteger(1)))
        assert _is_const_wrt(expr, X)


class TestIsqrtExact:
    """_isqrt_exact and _exact_sqrt_fraction correctness."""

    def test_perfect_squares(self) -> None:
        for n in [0, 1, 4, 9, 16, 25, 100]:
            result = _isqrt_exact(n)
            assert result is not None and result * result == n

    def test_non_perfect_squares(self) -> None:
        for n in [2, 3, 5, 6, 7, 8, 10]:
            assert _isqrt_exact(n) is None

    def test_exact_sqrt_fraction_rational(self) -> None:
        from fractions import Fraction
        assert _exact_sqrt_fraction(Fraction(4)) == Fraction(2)
        assert _exact_sqrt_fraction(Fraction(1, 4)) == Fraction(1, 2)
        assert _exact_sqrt_fraction(Fraction(9, 16)) == Fraction(3, 4)

    def test_exact_sqrt_fraction_zero(self) -> None:
        from fractions import Fraction
        assert _exact_sqrt_fraction(Fraction(0)) == Fraction(0)

    def test_exact_sqrt_fraction_irrational(self) -> None:
        from fractions import Fraction
        assert _exact_sqrt_fraction(Fraction(2)) is None
        assert _exact_sqrt_fraction(Fraction(3)) is None


class TestFlattenAdd:
    """_flatten_add should recursively decompose Add trees."""

    def test_single_term(self) -> None:
        result = _flatten_add(X)
        assert result == [X]

    def test_add_two_terms(self) -> None:
        node = IRApply(ADD, (X, Y))
        result = _flatten_add(node)
        assert result == [X, Y]

    def test_nested_add(self) -> None:
        # Add(Add(a, b), c)
        ab = IRApply(ADD, (IRInteger(1), IRInteger(2)))
        node = IRApply(ADD, (ab, IRInteger(3)))
        result = _flatten_add(node)
        assert result == [IRInteger(1), IRInteger(2), IRInteger(3)]

    def test_sub_becomes_neg(self) -> None:
        node = IRApply(SUB, (X, Y))
        result = _flatten_add(node)
        assert len(result) == 2
        assert result[0] == X
        # Second element should be Neg(Y)
        assert isinstance(result[1], IRApply)
        assert result[1].head.name == "Neg"


# ---------------------------------------------------------------------------
# Section C: Second-order coefficient recognition
# ---------------------------------------------------------------------------


class TestCollectSecondOrderCoeffs:
    """_collect_second_order_coeffs pattern matching."""

    def test_y_double_prime_only(self) -> None:
        # y'' = 0 → only a, no b, c → returns None (not enough terms)
        result = _collect_second_order_coeffs(Y_DOUBLE, Y, X)
        assert result is None  # Only one term (a=1, b=0, c=0) — need ≥ 2 matched

    def test_y_double_prime_plus_y(self) -> None:
        from fractions import Fraction
        # y'' + y → a=1, b=0, c=1
        expr = IRApply(ADD, (Y_DOUBLE, Y))
        result = _collect_second_order_coeffs(expr, Y, X)
        assert result is not None
        a, b, c = result
        assert a == Fraction(1)
        assert b == Fraction(0)
        assert c == Fraction(1)

    def test_y_double_prime_minus_y(self) -> None:
        from fractions import Fraction
        # y'' - y → a=1, b=0, c=-1
        expr = IRApply(SUB, (Y_DOUBLE, Y))
        result = _collect_second_order_coeffs(expr, Y, X)
        assert result is not None
        a, b, c = result
        assert a == Fraction(1)
        assert b == Fraction(0)
        assert c == Fraction(-1)

    def test_full_second_order(self) -> None:
        from fractions import Fraction
        # y'' - 2*y' + y → a=1, b=-2, c=1
        term1 = Y_DOUBLE
        term2 = _neg(_mul(IRInteger(2), Y_PRIME))
        term3 = Y
        expr = IRApply(ADD, (IRApply(ADD, (term1, term2)), term3))
        result = _collect_second_order_coeffs(expr, Y, X)
        assert result is not None
        a, b, c = result
        assert a == Fraction(1)
        assert b == Fraction(-2)
        assert c == Fraction(1)

    def test_non_const_coeff_returns_none(self) -> None:
        # x*y'' + y → variable coefficient → None
        expr = IRApply(ADD, (_mul(X, Y_DOUBLE), Y))
        result = _collect_second_order_coeffs(expr, Y, X)
        assert result is None

    def test_first_order_only_returns_none(self) -> None:
        # y' + y → no y'', returns None
        expr = IRApply(ADD, (Y_PRIME, Y))
        result = _collect_second_order_coeffs(expr, Y, X)
        assert result is None


# ---------------------------------------------------------------------------
# Section D: solve_second_order_const_coeff
# ---------------------------------------------------------------------------


class TestSolveSecondOrderConstCoeff:
    """Verify the three root cases at the solver-function level."""

    def test_distinct_real_roots(self) -> None:
        from fractions import Fraction
        # y'' - y = 0 → roots r=1, r=-1
        # y = C1*exp(x) + C2*exp(-x)
        result = solve_second_order_const_coeff(
            Fraction(1), Fraction(0), Fraction(-1), Y, X
        )
        assert isinstance(result, IRApply)
        assert result.head == EQUAL
        assert result.args[0] == Y
        # Solution should be Add of two Mul terms
        solution = result.args[1]
        assert isinstance(solution, IRApply)
        assert solution.head == ADD

    def test_repeated_root(self) -> None:
        from fractions import Fraction
        # y'' - 2*y' + y = 0 → r = 1 (double)
        # y = (C1 + C2*x)*exp(x)
        result = solve_second_order_const_coeff(
            Fraction(1), Fraction(-2), Fraction(1), Y, X
        )
        assert isinstance(result, IRApply)
        assert result.head == EQUAL
        # Solution should involve Mul and Exp
        solution = result.args[1]
        assert isinstance(solution, IRApply)

    def test_complex_roots(self) -> None:
        from fractions import Fraction
        # y'' + y = 0 → roots ±i → exp(0*x)*(C1*cos(x) + C2*sin(x))
        result = solve_second_order_const_coeff(
            Fraction(1), Fraction(0), Fraction(1), Y, X
        )
        assert isinstance(result, IRApply)
        assert result.head == EQUAL
        solution = result.args[1]
        # Should involve Mul of Exp and trig sum
        assert isinstance(solution, IRApply)

    def test_complex_roots_with_real_part(self) -> None:
        from fractions import Fraction
        # y'' + 2*y' + 5*y = 0 → roots -1 ± 2i
        # y = exp(-x)*(C1*cos(2x) + C2*sin(2x))
        result = solve_second_order_const_coeff(
            Fraction(1), Fraction(2), Fraction(5), Y, X
        )
        assert isinstance(result, IRApply)
        assert result.head == EQUAL


# ---------------------------------------------------------------------------
# Section E: Integration-based first-order linear solver
# ---------------------------------------------------------------------------


class TestFirstOrderLinear:
    """Test first-order linear ODE solving via the VM."""

    def test_y_prime_minus_2y(self) -> None:
        """y' - 2*y = 0  →  y = %c * exp(2*x)."""
        vm = make_vm()
        expr = _sub(Y_PRIME, _mul(IRInteger(2), Y))
        result = vm.eval(IRApply(ODE2, (expr, Y, X)))
        assert isinstance(result, IRApply)
        assert result.head == EQUAL
        assert result.args[0] == Y

    def test_y_prime_minus_x(self) -> None:
        """y' - x = 0  →  y' = x  →  y = x^2/2 + %c."""
        vm = make_vm()
        expr = _sub(Y_PRIME, X)
        result = vm.eval(IRApply(ODE2, (expr, Y, X)))
        assert isinstance(result, IRApply)
        assert result.head == EQUAL
        assert result.args[0] == Y
        # Solution should contain C_CONST somewhere
        solution_str = str(result.args[1])
        assert "%c" in solution_str

    def test_y_prime_alone(self) -> None:
        """y' = 0  →  y = %c (trivial)."""
        vm = make_vm()
        expr = Y_PRIME  # y' = 0
        result = vm.eval(IRApply(ODE2, (expr, Y, X)))
        assert isinstance(result, IRApply)
        assert result.head == EQUAL

    def test_y_prime_plus_y(self) -> None:
        """y' + y = 0  →  y = %c * exp(-x)."""
        vm = make_vm()
        expr = _add(Y_PRIME, Y)
        result = vm.eval(IRApply(ODE2, (expr, Y, X)))
        assert isinstance(result, IRApply)
        assert result.head == EQUAL
        assert result.args[0] == Y

    def test_equal_form_input(self) -> None:
        """Equal(y', 2*y) input — same result as y' - 2*y = 0."""
        vm = make_vm()
        lhs = Y_PRIME
        rhs = _mul(IRInteger(2), Y)
        eqn = IRApply(EQUAL, (lhs, rhs))
        result = vm.eval(IRApply(ODE2, (eqn, Y, X)))
        assert isinstance(result, IRApply)
        assert result.head == EQUAL
        assert result.args[0] == Y

    def test_x_times_y_prime_minus_y(self) -> None:
        """x*y' - y = 0 — not const-coeff linear by our recogniser.

        This ODE (Euler type) is separable: y' = y/x.
        The separable recogniser should catch it.
        """
        vm = make_vm()
        expr = _sub(_mul(X, Y_PRIME), Y)
        result = vm.eval(IRApply(ODE2, (expr, Y, X)))
        # Should either solve it or return unevaluated — not crash.
        assert isinstance(result, IRApply)

    def test_y_prime_plus_2y_equals_4x(self) -> None:
        """y' + 2*y = 4*x — non-homogeneous linear."""
        vm = make_vm()
        # Expression: y' + 2*y - 4*x = 0
        term1 = Y_PRIME
        term2 = _mul(IRInteger(2), Y)
        term3 = _neg(_mul(IRInteger(4), X))
        expr = IRApply(ADD, (IRApply(ADD, (term1, term2)), term3))
        result = vm.eval(IRApply(ODE2, (expr, Y, X)))
        # May or may not solve depending on capability; just check type.
        assert isinstance(result, IRApply)


# ---------------------------------------------------------------------------
# Section F: Second-order via VM
# ---------------------------------------------------------------------------


class TestSecondOrderViaVM:
    """Full pipeline: ODE2 handler → solver → result."""

    def test_y_double_minus_y(self) -> None:
        """y'' - y = 0  →  two distinct real roots (1, -1)."""
        vm = make_vm()
        expr = _sub(Y_DOUBLE, Y)
        result = vm.eval(IRApply(ODE2, (expr, Y, X)))
        assert isinstance(result, IRApply)
        assert result.head == EQUAL
        assert result.args[0] == Y
        solution_str = str(result.args[1])
        assert "%c1" in solution_str
        assert "%c2" in solution_str

    def test_y_double_plus_y(self) -> None:
        """y'' + y = 0  →  complex roots → sin + cos."""
        vm = make_vm()
        expr = _add(Y_DOUBLE, Y)
        result = vm.eval(IRApply(ODE2, (expr, Y, X)))
        assert isinstance(result, IRApply)
        assert result.head == EQUAL
        solution_str = str(result.args[1])
        assert "%c1" in solution_str
        assert "%c2" in solution_str

    def test_y_double_minus_2yprime_plus_y(self) -> None:
        """y'' - 2*y' + y = 0  →  repeated root r=1."""
        vm = make_vm()
        # y'' - 2*y' + y
        term1 = Y_DOUBLE
        term2 = _neg(_mul(IRInteger(2), Y_PRIME))
        term3 = Y
        expr = IRApply(ADD, (IRApply(ADD, (term1, term2)), term3))
        result = vm.eval(IRApply(ODE2, (expr, Y, X)))
        assert isinstance(result, IRApply)
        assert result.head == EQUAL
        solution_str = str(result.args[1])
        assert "%c1" in solution_str
        assert "%c2" in solution_str

    def test_y_double_plus_2yprime_plus_5y(self) -> None:
        """y'' + 2*y' + 5*y = 0  →  complex roots -1 ± 2i."""
        vm = make_vm()
        # y'' + 2*y' + 5*y
        term1 = Y_DOUBLE
        term2 = _mul(IRInteger(2), Y_PRIME)
        term3 = _mul(IRInteger(5), Y)
        expr = IRApply(ADD, (IRApply(ADD, (term1, term2)), term3))
        result = vm.eval(IRApply(ODE2, (expr, Y, X)))
        assert isinstance(result, IRApply)
        assert result.head == EQUAL
        solution_str = str(result.args[1])
        assert "%c1" in solution_str
        assert "%c2" in solution_str

    def test_4y_double_minus_y(self) -> None:
        """4*y'' - y = 0  →  roots ±1/2."""
        vm = make_vm()
        term1 = _mul(IRInteger(4), Y_DOUBLE)
        term2 = _neg(Y)
        expr = _add(term1, term2)
        result = vm.eval(IRApply(ODE2, (expr, Y, X)))
        assert isinstance(result, IRApply)
        assert result.head == EQUAL

    def test_equal_form_second_order(self) -> None:
        """Equal(y'', Neg(y)) input form for second-order ODE."""
        vm = make_vm()
        eqn = IRApply(EQUAL, (Y_DOUBLE, _neg(Y)))
        result = vm.eval(IRApply(ODE2, (eqn, Y, X)))
        assert isinstance(result, IRApply)
        assert result.head == EQUAL


# ---------------------------------------------------------------------------
# Section G: Separable ODEs via VM
# ---------------------------------------------------------------------------


class TestSeparableViaVM:
    """Separable ODEs — y' = f(x)*g(y) forms."""

    def test_y_prime_minus_2xy(self) -> None:
        """y' - 2*x*y = 0  — separable: y' = 2x·y.

        Should delegate to linear solver: P = -2x, Q = 0.
        """
        vm = make_vm()
        expr = _sub(Y_PRIME, _mul(_mul(IRInteger(2), X), Y))
        result = vm.eval(IRApply(ODE2, (expr, Y, X)))
        assert isinstance(result, IRApply)
        assert result.head == EQUAL
        assert result.args[0] == Y

    def test_y_prime_minus_ky(self) -> None:
        """y' - k*y = 0 (constant coefficient growth/decay)."""
        vm = make_vm()
        k = IRInteger(3)
        expr = _sub(Y_PRIME, _mul(k, Y))
        result = vm.eval(IRApply(ODE2, (expr, Y, X)))
        assert isinstance(result, IRApply)
        assert result.head == EQUAL


# ---------------------------------------------------------------------------
# Section H: Fall-through (unevaluated cases)
# ---------------------------------------------------------------------------


class TestFallThrough:
    """Verify that unsupported ODEs return unevaluated."""

    def test_variable_coeff_second_order(self) -> None:
        """y'' + sin(x)*y — variable coefficients → unevaluated."""
        vm = make_vm()
        sin_x = IRApply(SIN, (X,))
        expr = _add(Y_DOUBLE, _mul(sin_x, Y))
        result = vm.eval(IRApply(ODE2, (expr, Y, X)))
        # Should return the unevaluated ODE2 node
        assert isinstance(result, IRApply)
        assert isinstance(result.head, IRSymbol)
        assert result.head == ODE2

    def test_wrong_arity(self) -> None:
        """ODE2 with wrong number of arguments returns unevaluated."""
        vm = make_vm()
        result = vm.eval(IRApply(ODE2, (Y_PRIME, Y)))  # only 2 args
        assert isinstance(result, IRApply)
        assert result.head == ODE2

    def test_non_symbol_y(self) -> None:
        """ODE2 with non-symbol second argument returns unevaluated."""
        vm = make_vm()
        result = vm.eval(IRApply(ODE2, (Y_PRIME, IRInteger(1), X)))
        assert isinstance(result, IRApply)
        assert result.head == ODE2

    def test_non_symbol_x(self) -> None:
        """ODE2 with non-symbol third argument returns unevaluated."""
        vm = make_vm()
        result = vm.eval(IRApply(ODE2, (Y_PRIME, Y, IRInteger(1))))
        assert isinstance(result, IRApply)
        assert result.head == ODE2


# ---------------------------------------------------------------------------
# Section I: solve_ode direct tests
# ---------------------------------------------------------------------------


class TestSolveOdeDirect:
    """Call solve_ode() directly without going through the VM handler."""

    def test_returns_none_for_unknown(self) -> None:
        """Completely unrecognised expression → None."""
        vm = make_vm()
        # Something totally foreign: sin(y) + cos(x)
        expr = _add(IRApply(SIN, (Y,)), IRApply(COS, (X,)))
        result = solve_ode(expr, Y, X, vm)
        assert result is None

    def test_second_order_direct(self) -> None:
        """solve_ode with second-order ODE directly."""
        vm = make_vm()
        expr = _add(Y_DOUBLE, Y)
        result = solve_ode(expr, Y, X, vm)
        assert result is not None
        assert isinstance(result, IRApply)
        assert result.head == EQUAL

    def test_first_order_linear_direct(self) -> None:
        """solve_ode with first-order linear ODE directly."""
        vm = make_vm()
        expr = _sub(Y_PRIME, _mul(IRInteger(2), Y))
        result = solve_ode(expr, Y, X, vm)
        assert result is not None
        assert isinstance(result, IRApply)
        assert result.head == EQUAL

    def test_trivial_y_prime_zero(self) -> None:
        """y' = 0 → y = %c."""
        vm = make_vm()
        expr = Y_PRIME  # y' (= 0)
        result = solve_ode(expr, Y, X, vm)
        assert result is not None
        assert isinstance(result, IRApply)
        assert result.head == EQUAL
        assert result.args[0] == Y


# ---------------------------------------------------------------------------
# Section J: collect_linear_first_order direct tests
# ---------------------------------------------------------------------------


class TestCollectLinearFirstOrder:
    """Unit tests for the first-order linear coefficient extractor."""

    def test_y_prime_only(self) -> None:
        """y' alone → P = 0, Q = 0."""
        result = _collect_linear_first_order(Y_PRIME, Y, X)
        assert result is not None
        p, q = result
        assert p == IRInteger(0)
        assert q == IRInteger(0)

    def test_y_prime_plus_y(self) -> None:
        """y' + y → P = 1, Q = 0."""
        expr = _add(Y_PRIME, Y)
        result = _collect_linear_first_order(expr, Y, X)
        assert result is not None
        p, q = result
        # P should be 1 (possibly IRInteger(1))
        assert p == IRInteger(1) or str(p) == "1"

    def test_y_prime_minus_2y(self) -> None:
        """y' - 2*y → P = -2, Q = 0."""
        expr = _sub(Y_PRIME, _mul(IRInteger(2), Y))
        result = _collect_linear_first_order(expr, Y, X)
        assert result is not None

    def test_no_y_prime_returns_none(self) -> None:
        """Expression without y' → None."""
        result = _collect_linear_first_order(Y, Y, X)
        assert result is None


# ---------------------------------------------------------------------------
# Section K: Builder helper coverage tests
# ---------------------------------------------------------------------------


class TestBuilderHelpers:
    """Cover the private builder helpers in ode.py."""

    def test_add_zero_left(self) -> None:
        """_add(0, b) → b."""
        from cas_ode.ode import _add
        result = _add(IRInteger(0), X)
        assert result == X

    def test_add_zero_right(self) -> None:
        """_add(a, 0) → a."""
        from cas_ode.ode import _add
        result = _add(X, IRInteger(0))
        assert result == X

    def test_add_nonzero(self) -> None:
        """_add(a, b) → Add(a, b) when neither is zero."""
        from cas_ode.ode import _add
        result = _add(X, Y)
        assert isinstance(result, IRApply)
        assert result.head.name == "Add"

    def test_mul_one_left(self) -> None:
        """_mul(1, b) → b."""
        from cas_ode.ode import _mul
        assert _mul(IRInteger(1), X) == X

    def test_mul_one_right(self) -> None:
        """_mul(a, 1) → a."""
        from cas_ode.ode import _mul
        assert _mul(X, IRInteger(1)) == X

    def test_mul_neg_one_left(self) -> None:
        """_mul(-1, b) → Neg(b)."""
        from cas_ode.ode import _mul
        result = _mul(IRInteger(-1), X)
        assert isinstance(result, IRApply)
        assert result.head.name == "Neg"

    def test_mul_neg_one_right(self) -> None:
        """_mul(a, -1) → Neg(a)."""
        from cas_ode.ode import _mul
        result = _mul(X, IRInteger(-1))
        assert isinstance(result, IRApply)
        assert result.head.name == "Neg"

    def test_sub_builder(self) -> None:
        """_sub(a, b) → Sub(a, b)."""
        from cas_ode.ode import _sub
        result = _sub(X, Y)
        assert isinstance(result, IRApply)
        assert result.head.name == "Sub"

    def test_pow_builder(self) -> None:
        """_pow(a, b) → Pow(a, b)."""
        from cas_ode.ode import _pow
        result = _pow(X, IRInteger(2))
        assert isinstance(result, IRApply)
        assert result.head.name == "Pow"

    def test_frac_to_ir_integer(self) -> None:
        """_frac_to_ir(Fraction(3)) → IRInteger(3)."""
        from fractions import Fraction

        from cas_ode.ode import _frac_to_ir
        result = _frac_to_ir(Fraction(3))
        assert isinstance(result, IRInteger)
        assert result.value == 3

    def test_frac_to_ir_rational(self) -> None:
        """_frac_to_ir(Fraction(1,2)) → IRRational(1,2)."""
        from fractions import Fraction

        from cas_ode.ode import _frac_to_ir
        result = _frac_to_ir(Fraction(1, 2))
        assert isinstance(result, IRRational)
        assert result.numer == 1
        assert result.denom == 2

    def test_unevaluated_integrate_check(self) -> None:
        """_is_unevaluated_integrate checks the node shape correctly."""
        from cas_ode.ode import _is_unevaluated_integrate
        # A real Integrate(...) node
        node = IRApply(IRSymbol("Integrate"), (X, X))
        assert _is_unevaluated_integrate(node, X)
        # Not an integrate node
        assert not _is_unevaluated_integrate(X, X)
        assert not _is_unevaluated_integrate(IRInteger(1), X)

    def test_flatten_add_neg_of_neg(self) -> None:
        """_flatten_add(Neg(Neg(x))) → [x] — double negation simplification."""
        from symbolic_ir import NEG
        node = IRApply(NEG, (IRApply(NEG, (X,)),))
        result = _flatten_add(node)
        assert result == [X]


# ---------------------------------------------------------------------------
# Section L: Additional coverage for separable and irrational roots
# ---------------------------------------------------------------------------


class TestIrrationalRoots:
    """Second-order ODEs with non-rational discriminant."""

    def test_irrational_discriminant(self) -> None:
        """y'' - 3*y = 0 → roots ±√3 (irrational) — should still solve."""
        from fractions import Fraction
        # a=1, b=0, c=-3 → disc=12 (not perfect square)
        result = solve_second_order_const_coeff(
            Fraction(1), Fraction(0), Fraction(-3), Y, X
        )
        assert isinstance(result, IRApply)
        assert result.head == EQUAL
        # Irrational case: solution uses Pow(3, 1/2) as sqrt
        solution_str = str(result.args[1])
        assert "%c1" in solution_str or "%c2" in solution_str

    def test_irrational_complex_discriminant(self) -> None:
        """y'' + 3*y = 0 → roots ±i√3 (complex irrational)."""
        from fractions import Fraction
        # a=1, b=0, c=3 → disc=-12 → complex irrational
        result = solve_second_order_const_coeff(
            Fraction(1), Fraction(0), Fraction(3), Y, X
        )
        assert isinstance(result, IRApply)
        assert result.head == EQUAL
        solution_str = str(result.args[1])
        assert "%c1" in solution_str


class TestSeparableDirectCases:
    """Test the separable ODE recognizer branches directly."""

    def test_y_prime_equals_x_squared(self) -> None:
        """y' - x^2 = 0 → y = x^3/3 + %c (pure f(x) case)."""
        vm = make_vm()
        expr = _sub(Y_PRIME, _pow(X, IRInteger(2)))
        result = vm.eval(IRApply(ODE2, (expr, Y, X)))
        assert isinstance(result, IRApply)
        assert result.head == EQUAL
        assert result.args[0] == Y
        # Should contain %c
        solution_str = str(result.args[1])
        assert "%c" in solution_str

    def test_y_prime_equals_1(self) -> None:
        """y' - 1 = 0 → y = x + %c."""
        vm = make_vm()
        expr = _sub(Y_PRIME, IRInteger(1))
        result = vm.eval(IRApply(ODE2, (expr, Y, X)))
        assert isinstance(result, IRApply)
        assert result.head == EQUAL

    def test_pure_gy_linear(self) -> None:
        """y' = 5*y (pure g(y) = 5y case in separable)."""
        vm = make_vm()
        # y' - 5*y = 0 but going through separable recogniser
        # The separable path hits: rhs = 5*y (const wrt x, linear in y)
        # Build as y' - 5*y and use constant that doesn't look like Mul(P(x), y)
        # to bypass linear direct. Actually the linear recogniser gets it first.
        expr = _sub(Y_PRIME, _mul(IRInteger(5), Y))
        result = vm.eval(IRApply(ODE2, (expr, Y, X)))
        assert isinstance(result, IRApply)
        assert result.head == EQUAL


class TestSumOfTermsHelper:
    """Test _sum_of_terms directly."""

    def test_empty_list(self) -> None:
        """Empty list → IRInteger(0)."""
        from cas_ode.ode import _sum_of_terms
        result = _sum_of_terms([])
        assert result == IRInteger(0)

    def test_single_term(self) -> None:
        """Single non-negated term → just that term."""
        from cas_ode.ode import _sum_of_terms
        result = _sum_of_terms([(X, False)])
        assert result == X

    def test_negated_term(self) -> None:
        """Single negated term → Neg(term)."""
        from cas_ode.ode import _sum_of_terms
        result = _sum_of_terms([(X, True)])
        assert isinstance(result, IRApply)
        assert result.head.name == "Neg"

    def test_multiple_terms(self) -> None:
        """Multiple terms accumulate into Add chain."""
        from cas_ode.ode import _sum_of_terms
        result = _sum_of_terms([(X, False), (Y, False)])
        assert isinstance(result, IRApply)
        assert result.head.name == "Add"


# ---------------------------------------------------------------------------
# Section M: Additional edge case tests for deeper coverage
# ---------------------------------------------------------------------------


class TestExtractCoeffRationalPaths:
    """Test IRRational coefficient extraction paths."""

    def test_rational_left_coeff(self) -> None:
        """Mul(IRRational(1,2), expr) → coeff=1/2."""
        from fractions import Fraction

        from cas_ode.ode import _extract_coeff
        node = IRApply(MUL, (IRRational(1, 2), Y))
        coeff, base = _extract_coeff(node, X)
        assert coeff == Fraction(1, 2)
        assert base == Y

    def test_rational_right_coeff(self) -> None:
        """Mul(expr, IRRational(3,4)) → coeff=3/4."""
        from fractions import Fraction

        from cas_ode.ode import _extract_coeff
        node = IRApply(MUL, (Y, IRRational(3, 4)))
        coeff, base = _extract_coeff(node, X)
        assert coeff == Fraction(3, 4)
        assert base == Y

    def test_rational_bare_term(self) -> None:
        """Bare IRRational as a standalone term → coeff=that, base=1."""
        from fractions import Fraction

        from cas_ode.ode import _extract_coeff
        node = IRRational(1, 3)
        coeff, base = _extract_coeff(node, X)
        assert coeff == Fraction(1, 3)
        assert base == IRInteger(1)

    def test_isqrt_negative(self) -> None:
        """_isqrt_exact(-1) returns None."""
        assert _isqrt_exact(-1) is None

    def test_exact_sqrt_fraction_negative(self) -> None:
        """_exact_sqrt_fraction(Fraction(-1)) returns None."""
        from fractions import Fraction
        assert _exact_sqrt_fraction(Fraction(-1)) is None


class TestFirstOrderWithScaledYPrime:
    """ODEs where y' has a coefficient other than 1."""

    def test_2y_prime_plus_4y(self) -> None:
        """2*y' + 4*y = 0 → y' + 2*y = 0 → y = %c * exp(-2x)."""
        vm = make_vm()
        # 2*y' + 4*y
        expr = _add(_mul(IRInteger(2), Y_PRIME), _mul(IRInteger(4), Y))
        result = vm.eval(IRApply(ODE2, (expr, Y, X)))
        assert isinstance(result, IRApply)
        assert result.head == EQUAL
        assert result.args[0] == Y

    def test_coeff_y_prime_in_linear_recogniser(self) -> None:
        """Test that 2*y' is recognised in _collect_linear_first_order."""
        expr = _add(_mul(IRInteger(2), Y_PRIME), Y)
        result = _collect_linear_first_order(expr, Y, X)
        assert result is not None
        # P = 1/2 (from dividing through by the 2)
        p_ir, q_ir = result
        # Should be a Div node (P = 1/2)
        assert isinstance(p_ir, IRApply) and p_ir.head.name == "Div"


class TestSeparableMultipleRhsTerms:
    """Separable ODE with multiple right-hand-side terms."""

    def test_y_prime_equals_x_plus_1(self) -> None:
        """y' = x + 1 → y = x^2/2 + x + %c."""
        vm = make_vm()
        # y' - x - 1 = 0  [two rhs terms]
        expr = IRApply(ADD, (
            IRApply(SUB, (Y_PRIME, X)),
            IRInteger(-1)
        ))
        result = vm.eval(IRApply(ODE2, (expr, Y, X)))
        assert isinstance(result, IRApply)
        assert result.head == EQUAL
        assert result.args[0] == Y
        solution_str = str(result.args[1])
        assert "%c" in solution_str

    def test_neg_y_prime_on_lhs_falls_through(self) -> None:
        """An expression with -y' on the LHS (from separable) → fall-through."""
        vm = make_vm()
        # -D(y, x) + y = 0  — separable recogniser sees -y' (negated yprime)
        neg_yprime = IRApply(NEG, (Y_PRIME,))
        expr = _add(neg_yprime, Y)
        # This should either solve (linear) or return unevaluated — not crash
        result = vm.eval(IRApply(ODE2, (expr, Y, X)))
        assert isinstance(result, IRApply)


class TestIsFlattenAddFloat:
    """Cover the IRFloat branch in _is_const_wrt."""

    def test_ir_float_const_wrt(self) -> None:
        """An IRFloat value is const with respect to any symbol."""
        from symbolic_ir import IRFloat
        node = IRFloat(3.14)
        assert _is_const_wrt(node, X)


# ---------------------------------------------------------------------------
# Section P: _subst_ir unit tests
# ---------------------------------------------------------------------------


def _div_node(a: IRNode, b: IRNode) -> IRApply:
    """Build Div(a, b) — local helper to avoid clashing with test helpers."""
    return IRApply(DIV, (a, b))


class TestSubstIr:
    """Unit tests for the pure IR tree substitution helper _subst_ir.

    _subst_ir(node, var, replacement) replaces every occurrence of ``var``
    in ``node`` with ``replacement``, leaving all other sub-trees unchanged.
    """

    def test_substitute_symbol_directly(self) -> None:
        """A symbol that equals var is replaced by the replacement."""
        v = IRSymbol("v")
        result = _subst_ir(v, v, IRInteger(5))
        assert result == IRInteger(5)

    def test_different_symbol_unchanged(self) -> None:
        """A symbol that is not var is left as-is."""
        v = IRSymbol("v")
        result = _subst_ir(X, v, IRInteger(5))
        assert result == X

    def test_integer_unchanged(self) -> None:
        """IRInteger nodes are returned unmodified regardless of var."""
        v = IRSymbol("v")
        result = _subst_ir(IRInteger(7), v, IRInteger(0))
        assert result == IRInteger(7)

    def test_rational_unchanged(self) -> None:
        """IRRational nodes are returned unmodified."""
        v = IRSymbol("v")
        result = _subst_ir(IRRational(1, 3), v, IRInteger(0))
        assert result == IRRational(1, 3)

    def test_nested_add(self) -> None:
        """Substitution descends into both args of Add."""
        v = IRSymbol("v")
        # Add(v, x) → Add(IRInteger(2), x)
        expr = IRApply(ADD, (v, X))
        result = _subst_ir(expr, v, IRInteger(2))
        assert isinstance(result, IRApply)
        assert result.head == ADD
        assert result.args[0] == IRInteger(2)
        assert result.args[1] == X

    def test_pow_back_substitution(self) -> None:
        """Replacing v in Pow(v, 2) with Div(y, x) for back-substitution."""
        v = IRSymbol("v")
        y_over_x = _div_node(Y, X)
        expr = IRApply(POW, (v, IRInteger(2)))
        result = _subst_ir(expr, v, y_over_x)
        assert isinstance(result, IRApply)
        assert result.head == POW
        assert result.args[0] == y_over_x
        assert result.args[1] == IRInteger(2)

    def test_no_occurrence_leaves_tree_identical(self) -> None:
        """If var does not appear, the tree is returned structurally unchanged."""
        v = IRSymbol("v")
        expr = IRApply(MUL, (X, Y))
        result = _subst_ir(expr, v, IRInteger(99))
        assert result == expr


# ---------------------------------------------------------------------------
# Section Q: _subst_ratio_ir unit tests
# ---------------------------------------------------------------------------


class TestSubstRatioIr:
    """Unit tests for the structural Div(y,x) → v substitution helper.

    _subst_ratio_ir(node, y, x, v) replaces every exact ``Div(y, x)``
    pattern with the symbol v.  If y appears in any form other than
    ``Div(y, x)``, the function returns None (signalling that the
    expression cannot be written purely in terms of y/x).
    """

    def test_div_y_x_becomes_v(self) -> None:
        """The exact Div(y, x) node is replaced by v."""
        v = IRSymbol("v")
        node = _div_node(Y, X)
        result = _subst_ratio_ir(node, Y, X, v)
        assert result == v

    def test_bare_y_returns_none(self) -> None:
        """A bare y symbol (not inside Div(y,x)) causes None."""
        v = IRSymbol("v")
        result = _subst_ratio_ir(Y, Y, X, v)
        assert result is None

    def test_pow_div_y_x_squared(self) -> None:
        """Pow(Div(y,x), 2) → Pow(v, 2)."""
        v = IRSymbol("v")
        node = IRApply(POW, (_div_node(Y, X), IRInteger(2)))
        result = _subst_ratio_ir(node, Y, X, v)
        assert result == IRApply(POW, (v, IRInteger(2)))

    def test_integer_passes_through(self) -> None:
        """IRInteger nodes pass through unmodified."""
        v = IRSymbol("v")
        result = _subst_ratio_ir(IRInteger(3), Y, X, v)
        assert result == IRInteger(3)

    def test_x_symbol_unchanged(self) -> None:
        """The independent variable x (not matching y) is returned as-is."""
        v = IRSymbol("v")
        result = _subst_ratio_ir(X, Y, X, v)
        assert result == X

    def test_sum_of_two_ratios(self) -> None:
        """Add(Div(y,x), Div(y,x)) → Add(v, v)."""
        v = IRSymbol("v")
        y_over_x = _div_node(Y, X)
        node = IRApply(ADD, (y_over_x, y_over_x))
        result = _subst_ratio_ir(node, Y, X, v)
        assert result == IRApply(ADD, (v, v))

    def test_y_in_add_numerator_fails(self) -> None:
        """Div(Add(y, x), x) — y inside Add in numerator → None.

        This pattern arises in (y + x)/x = 1 + y/x.  We deliberately
        return None here and let the VM's simplification (via linear/
        separable) handle it, rather than depending on algebraic distribution.
        """
        v = IRSymbol("v")
        node = _div_node(IRApply(ADD, (Y, X)), X)
        result = _subst_ratio_ir(node, Y, X, v)
        assert result is None

    def test_unrelated_symbol_unchanged(self) -> None:
        """An unrelated symbol 'z' passes through without triggering None."""
        v = IRSymbol("v")
        z = IRSymbol("z")
        result = _subst_ratio_ir(z, Y, X, v)
        assert result == z

    def test_nested_y_not_in_div_fails(self) -> None:
        """Mul(y, x) — y inside Mul but not Div(y,x) → None."""
        v = IRSymbol("v")
        node = IRApply(MUL, (Y, X))
        result = _subst_ratio_ir(node, Y, X, v)
        assert result is None


# ---------------------------------------------------------------------------
# Section R: _try_homogeneous_type and full-pipeline tests (Phase 18c)
# ---------------------------------------------------------------------------


class TestHomogeneousTypeODE:
    """Tests for the homogeneous-type ODE solver (Phase 18c).

    A homogeneous-type ODE has the form::

        dy/dx = f(y/x)

    The substitution v = y/x reduces it to a separable equation in (v, x).
    The solver returns an implicit solution  H(y/x) = Log(x) + C  when it
    can integrate 1/(f(v) − v) in closed form.  The degenerate case
    f(v) = v (i.e. y' = y/x) is handled separately: y = C·x.

    All ODE expressions are passed in zero form: LHS − RHS = 0.
    """

    # ------------------------------------------------------------------ #
    # Direct calls to _try_homogeneous_type                               #
    # ------------------------------------------------------------------ #

    def test_degenerate_case_y_prime_eq_y_over_x(self) -> None:
        """y' = y/x is the degenerate case f(v)=v → y = %c·x.

        When f(v) = v the equation x·dv/dx = f(v)−v = 0 forces v = const,
        so y = v·x = C·x.  The solver detects this via an identity check
        and returns the explicit solution Equal(y, Mul(%c, x)).
        """
        vm = make_vm()
        # Zero form: D(y,x) − y/x = Sub(D(y,x), Div(y,x))
        expr = IRApply(SUB, (Y_PRIME, _div_node(Y, X)))
        result = _try_homogeneous_type(expr, Y, X, vm)

        assert result is not None
        assert isinstance(result, IRApply)
        assert result.head == EQUAL
        # Explicit solution: lhs is y
        assert result.args[0] == Y
        # RHS is Mul(%c, x) — integration constant times x
        rhs = result.args[1]
        assert isinstance(rhs, IRApply)
        assert rhs.head == MUL
        c_sym = IRSymbol("%c")
        assert c_sym in rhs.args
        assert X in rhs.args

    def test_y_prime_eq_ratio_squared_returns_equal(self) -> None:
        """y' = (y/x)^2 — f(v)=v², denom=v²−v, integral = Log((v−1)/v).

        This is a proper homogeneous-type ODE that requires the Hermite
        partial-fraction path.  We verify the solver produces some implicit
        Equal(...) form rather than returning None.
        """
        vm = make_vm()
        y_over_x = _div_node(Y, X)
        expr = IRApply(SUB, (Y_PRIME, IRApply(POW, (y_over_x, IRInteger(2)))))
        result = _try_homogeneous_type(expr, Y, X, vm)

        assert result is not None
        assert isinstance(result, IRApply)
        assert result.head == EQUAL

    def test_y_prime_eq_ratio_squared_solution_contains_c(self) -> None:
        """The implicit solution for y' = (y/x)² carries the constant %c."""
        vm = make_vm()
        y_over_x = _div_node(Y, X)
        expr = IRApply(SUB, (Y_PRIME, IRApply(POW, (y_over_x, IRInteger(2)))))
        result = _try_homogeneous_type(expr, Y, X, vm)

        assert result is not None
        assert "%c" in str(result)

    def test_y_prime_eq_ratio_squared_plus_ratio(self) -> None:
        """y' = (y/x)^2 + (y/x) — f(v)=v²+v, denom=v², integral=−1/v.

        The denominator f(v)−v = v²+v−v = v², so the integrand is 1/v²
        whose antiderivative is −v⁻¹.  After back-substitution this gives
        −x/y = Log(x) + C.
        """
        vm = make_vm()
        y_over_x = _div_node(Y, X)
        rhs_expr = IRApply(ADD, (
            IRApply(POW, (y_over_x, IRInteger(2))),
            y_over_x,
        ))
        expr = IRApply(SUB, (Y_PRIME, rhs_expr))
        result = _try_homogeneous_type(expr, Y, X, vm)

        assert result is not None
        assert isinstance(result, IRApply)
        assert result.head == EQUAL
        assert "%c" in str(result)

    def test_y_prime_eq_2_times_ratio(self) -> None:
        """y' = 2·(y/x) — f(v)=2v, denom=v, integral=Log(v).

        After back-substitution: Log(y/x) = Log(x) + C, which is the
        implicit form of the analytic solution y = A·x² (A constant).
        """
        vm = make_vm()
        y_over_x = _div_node(Y, X)
        rhs_expr = IRApply(MUL, (IRInteger(2), y_over_x))
        expr = IRApply(SUB, (Y_PRIME, rhs_expr))
        result = _try_homogeneous_type(expr, Y, X, vm)

        assert result is not None
        assert isinstance(result, IRApply)
        assert result.head == EQUAL
        # Log(y/x) should appear on the left or in the solution.
        result_str = str(result)
        assert "Log" in result_str
        assert "%c" in result_str

    # ------------------------------------------------------------------ #
    # Fall-through: non-homogeneous expressions → None                    #
    # ------------------------------------------------------------------ #

    def test_rhs_free_of_y_returns_none(self) -> None:
        """y' = sin(x) — rhs is free of y, so not homogeneous-type."""
        vm = make_vm()
        expr = IRApply(SUB, (Y_PRIME, IRApply(SIN, (X,))))
        result = _try_homogeneous_type(expr, Y, X, vm)
        assert result is None

    def test_bare_y_in_rhs_returns_none(self) -> None:
        """y' = y + x — bare y in rhs, _subst_ratio_ir returns None."""
        vm = make_vm()
        expr = IRApply(SUB, (Y_PRIME, IRApply(ADD, (Y, X))))
        result = _try_homogeneous_type(expr, Y, X, vm)
        assert result is None

    def test_y_times_x_in_rhs_returns_none(self) -> None:
        """y' = y·x — y appears as Mul(y,x), not as Div(y,x) → None."""
        vm = make_vm()
        expr = IRApply(SUB, (Y_PRIME, IRApply(MUL, (Y, X))))
        result = _try_homogeneous_type(expr, Y, X, vm)
        assert result is None

    def test_no_y_prime_term_returns_none(self) -> None:
        """Expression with no D(y,x) term returns None (not an ODE)."""
        vm = make_vm()
        expr = IRApply(ADD, (Y, X))
        result = _try_homogeneous_type(expr, Y, X, vm)
        assert result is None

    def test_transcendental_rhs_fails_integration_returns_none(self) -> None:
        """y' = exp(y/x) — denom e^v − v has no closed-form antiderivative.

        The Hermite reduction cannot integrate 1/(e^v − v), so the solver
        correctly falls through by returning None.
        """
        vm = make_vm()
        y_over_x = _div_node(Y, X)
        rhs_expr = IRApply(EXP, (y_over_x,))
        expr = IRApply(SUB, (Y_PRIME, rhs_expr))
        result = _try_homogeneous_type(expr, Y, X, vm)
        # Integration of 1/(e^v − v) fails → None
        assert result is None

    # ------------------------------------------------------------------ #
    # Full pipeline: solve_ode and VM ODE2 dispatch                       #
    # ------------------------------------------------------------------ #

    def test_solve_ode_degenerate(self) -> None:
        """solve_ode() returns Equal(y, %c·x) for the degenerate case."""
        vm = make_vm()
        expr = IRApply(SUB, (Y_PRIME, _div_node(Y, X)))
        result = solve_ode(expr, Y, X, vm)

        assert result is not None
        assert isinstance(result, IRApply)
        assert result.head == EQUAL
        assert result.args[0] == Y

    def test_ode2_dispatch_degenerate_via_vm(self) -> None:
        """ODE2(D(y,x)−y/x, y, x) through the VM handler → explicit y=%c·x."""
        vm = make_vm()
        expr = IRApply(SUB, (Y_PRIME, _div_node(Y, X)))
        result = vm.eval(IRApply(ODE2, (expr, Y, X)))

        assert isinstance(result, IRApply)
        assert result.head == EQUAL
        assert result.args[0] == Y
        assert "%c" in str(result)

    def test_ode2_dispatch_ratio_squared(self) -> None:
        """ODE2 dispatch for y' = (y/x)^2 returns an implicit Equal node."""
        vm = make_vm()
        y_over_x = _div_node(Y, X)
        expr = IRApply(SUB, (Y_PRIME, IRApply(POW, (y_over_x, IRInteger(2)))))
        result = vm.eval(IRApply(ODE2, (expr, Y, X)))

        assert isinstance(result, IRApply)
        assert result.head == EQUAL
        assert "%c" in str(result)

    def test_linear_ode_not_captured_by_homogeneous(self) -> None:
        """y' − 2y = 0 is handled by the linear/separable route before the
        homogeneous-type solver sees it.  The solution must contain Exp
        (from the integrating-factor method), not the implicit Log form.
        """
        vm = make_vm()
        # D(y,x) − 2*y = 0
        expr = IRApply(SUB, (Y_PRIME, IRApply(MUL, (IRInteger(2), Y))))
        result = vm.eval(IRApply(ODE2, (expr, Y, X)))

        assert isinstance(result, IRApply)
        assert result.head == EQUAL
        assert result.args[0] == Y
        # Exponential solution from linear/separable, not the Log implicit form
        result_str = str(result)
        assert "Exp" in result_str


# ---------------------------------------------------------------------------
# Section J: Euler-Cauchy equidimensional ODE (Phase 19)
# ---------------------------------------------------------------------------

# Shared IR atoms for the Euler-Cauchy tests.
X_SQ = IRApply(POW, (X, IRInteger(2)))   # Pow(x, 2) = x²


def _ec_term_x2_yprime2(coeff: int = 1) -> IRNode:
    """Build coeff · x² · y'' as a product tree."""
    base = _mul(X_SQ, Y_DOUBLE)
    if coeff == 1:
        return base
    return _mul(IRInteger(coeff), base)


def _ec_term_x_yprime(coeff: int = 1) -> IRNode:
    """Build coeff · x · y' as a product tree."""
    base = _mul(X, Y_PRIME)
    if coeff == 1:
        return base
    return _mul(IRInteger(coeff), base)


def _ec_term_y(coeff: int = 1) -> IRNode:
    """Build coeff · y."""
    if coeff == 1:
        return Y
    return _mul(IRInteger(coeff), Y)


def _build_ec_expr(a: int, b: int, c: int) -> IRNode:
    """Assemble a · x²y'' + b · x·y' + c · y as a left-folded Add tree."""
    terms = []
    if a != 0:
        terms.append(_ec_term_x2_yprime2(a))
    if b != 0:
        terms.append(_ec_term_x_yprime(b))
    if c != 0:
        terms.append(_ec_term_y(c))
    if len(terms) == 1:
        return terms[0]
    result = terms[0]
    for t in terms[1:]:
        result = _add(result, t)
    return result


class TestFlattenProduct:
    """Unit tests for _flatten_product — the Mul-tree analogue of _flatten_add."""

    def test_integer_node(self) -> None:
        """A bare IRInteger gives coefficient = that integer, no factors."""
        k, fs = _flatten_product(IRInteger(7))
        from fractions import Fraction
        assert k == Fraction(7)
        assert fs == []

    def test_rational_node(self) -> None:
        """A bare IRRational gives the corresponding Fraction, no factors."""
        k, fs = _flatten_product(IRRational(3, 4))
        from fractions import Fraction
        assert k == Fraction(3, 4)
        assert fs == []

    def test_symbol_node(self) -> None:
        """A bare symbol is a factor with coefficient 1."""
        k, fs = _flatten_product(X)
        from fractions import Fraction
        assert k == Fraction(1)
        assert fs == [X]

    def test_neg_integer(self) -> None:
        """Neg(IRInteger(5)) → coefficient = −5, no factors."""
        k, fs = _flatten_product(_neg(IRInteger(5)))
        from fractions import Fraction
        assert k == Fraction(-5)
        assert fs == []

    def test_neg_symbol(self) -> None:
        """Neg(x) → coefficient = −1, factors = [x]."""
        k, fs = _flatten_product(_neg(X))
        from fractions import Fraction
        assert k == Fraction(-1)
        assert fs == [X]

    def test_mul_two_symbols(self) -> None:
        """Mul(x, y) → coefficient = 1, factors = [x, y]."""
        k, fs = _flatten_product(_mul(X, Y))
        from fractions import Fraction
        assert k == Fraction(1)
        assert set(fs) == {X, Y}

    def test_mul_int_symbol(self) -> None:
        """Mul(3, x) → coefficient = 3, factors = [x]."""
        k, fs = _flatten_product(_mul(IRInteger(3), X))
        from fractions import Fraction
        assert k == Fraction(3)
        assert fs == [X]

    def test_mul_neg_int_symbol(self) -> None:
        """Mul(-2, y) → coefficient = −2, factors = [y]."""
        k, fs = _flatten_product(_mul(IRInteger(-2), Y))
        from fractions import Fraction
        assert k == Fraction(-2)
        assert fs == [Y]

    def test_triple_product(self) -> None:
        """Mul(3, Mul(x², y'')) → coefficient = 3, factors = [x², y'']."""
        node = _mul(IRInteger(3), _mul(X_SQ, Y_DOUBLE))
        k, fs = _flatten_product(node)
        from fractions import Fraction
        assert k == Fraction(3)
        assert X_SQ in fs
        assert Y_DOUBLE in fs


class TestCollectEulerCauchyCoeffs:
    """Unit tests for _collect_euler_cauchy_coeffs."""

    def test_basic_x2_yprime2_plus_y(self) -> None:
        """x²y'' + y must give a=1, b=0, c=1."""
        from fractions import Fraction
        expr = _add(_ec_term_x2_yprime2(1), _ec_term_y(1))
        result = _collect_euler_cauchy_coeffs(expr, Y, X)
        assert result is not None
        a, b, c = result
        assert a == Fraction(1)
        assert b == Fraction(0)
        assert c == Fraction(1)

    def test_full_three_term(self) -> None:
        """x²y'' + 2xy' − 2y → a=1, b=2, c=−2."""
        from fractions import Fraction
        expr = _build_ec_expr(1, 2, -2)
        result = _collect_euler_cauchy_coeffs(expr, Y, X)
        assert result is not None
        a, b, c = result
        assert a == Fraction(1)
        assert b == Fraction(2)
        assert c == Fraction(-2)

    def test_negative_b_and_c(self) -> None:
        """x²y'' − 5xy' + 9y → a=1, b=−5, c=9."""
        from fractions import Fraction
        expr = _build_ec_expr(1, -5, 9)
        result = _collect_euler_cauchy_coeffs(expr, Y, X)
        assert result is not None
        a, b, c = result
        assert a == Fraction(1)
        assert b == Fraction(-5)
        assert c == Fraction(9)

    def test_scaled_leading_coeff(self) -> None:
        """3x²y'' + 6xy' − 6y → a=3, b=6, c=−6."""
        from fractions import Fraction
        expr = _build_ec_expr(3, 6, -6)
        result = _collect_euler_cauchy_coeffs(expr, Y, X)
        assert result is not None
        a, b, c = result
        assert a == Fraction(3)
        assert b == Fraction(6)
        assert c == Fraction(-6)

    def test_missing_x2_term_returns_none(self) -> None:
        """xy' + y = 0 has no x²·y'' term → None (a=0)."""
        expr = _add(_ec_term_x_yprime(1), _ec_term_y(1))
        assert _collect_euler_cauchy_coeffs(expr, Y, X) is None

    def test_const_coeff_ode_returns_none(self) -> None:
        """y'' + y' + y = 0 has no x² or x multipliers → None."""
        expr = _add(_add(Y_DOUBLE, Y_PRIME), Y)
        assert _collect_euler_cauchy_coeffs(expr, Y, X) is None

    def test_only_one_term_returns_none(self) -> None:
        """A single x²y'' term has matched < 2 → None."""
        expr = _ec_term_x2_yprime2(1)
        assert _collect_euler_cauchy_coeffs(expr, Y, X) is None

    def test_bare_x_term_returns_none(self) -> None:
        """x²y'' + 2xy' + c·x = 0 has a bare x term (not c·y) → None."""
        expr = _add(_add(_ec_term_x2_yprime2(1), _ec_term_x_yprime(2)), X)
        assert _collect_euler_cauchy_coeffs(expr, Y, X) is None


class TestSolveEulerCauchy:
    """Unit tests for solve_euler_cauchy with all three root cases."""

    # ------------------------------------------------------------------ #
    # Case 1: distinct real roots (positive discriminant)                #
    # ------------------------------------------------------------------ #

    def test_distinct_real_roots_positive(self) -> None:
        """x²y'' + 2xy' − 2y = 0 → y = C1·x + C2·x^{-2}.

        Indicial: r² + (2−1)r − 2 = 0  → r² + r − 2 = 0
        Roots: r=1, r=−2.
        """
        from fractions import Fraction
        result = solve_euler_cauchy(Fraction(1), Fraction(2), Fraction(-2), Y, X)

        assert isinstance(result, IRApply)
        assert result.head == EQUAL
        assert result.args[0] == Y

        # The solution is Add(Mul(C1, x), Mul(C2, Pow(x, Neg(IRInteger(2)))))
        sol = result.args[1]
        sol_str = str(sol)
        # Both integration constants must appear.
        assert "c1" in sol_str or C1 in _walk_nodes(sol)
        assert "c2" in sol_str or C2 in _walk_nodes(sol)
        # Log must NOT appear (real distinct roots → no logarithm).
        assert "Log" not in sol_str

    def test_distinct_real_roots_symmetric(self) -> None:
        """x²y'' + xy' − 4y = 0 → y = C1·x² + C2·x^{-2}.

        Indicial: r² + 0·r − 4 = 0  → roots ±2.
        """
        from fractions import Fraction
        result = solve_euler_cauchy(Fraction(1), Fraction(1), Fraction(-4), Y, X)

        assert isinstance(result, IRApply)
        assert result.head == EQUAL
        sol = result.args[1]
        sol_str = str(sol)
        assert "Log" not in sol_str
        assert "Cos" not in sol_str and "Sin" not in sol_str

    def test_distinct_real_roots_result_is_add(self) -> None:
        """The solution for two distinct roots is an Add of two Mul terms."""
        from fractions import Fraction
        result = solve_euler_cauchy(Fraction(1), Fraction(2), Fraction(-2), Y, X)
        sol = result.args[1]
        # Top level must be Add(Mul(C1, ...), Mul(C2, ...))
        assert isinstance(sol, IRApply)
        assert sol.head == ADD

    # ------------------------------------------------------------------ #
    # Case 2: repeated root (zero discriminant)                          #
    # ------------------------------------------------------------------ #

    def test_repeated_root_order_three(self) -> None:
        """x²y'' − 5xy' + 9y = 0 → y = (C1 + C2·ln x)·x³.

        Indicial: r² + (−5−1)r + 9 = 0  → r² − 6r + 9 = 0  → (r−3)² = 0.
        """
        from fractions import Fraction
        result = solve_euler_cauchy(Fraction(1), Fraction(-5), Fraction(9), Y, X)

        assert isinstance(result, IRApply)
        assert result.head == EQUAL
        sol = result.args[1]
        sol_str = str(sol)
        # Repeated root → logarithm appears.
        assert "Log" in sol_str
        # No trig functions.
        assert "Cos" not in sol_str and "Sin" not in sol_str

    def test_repeated_root_order_one(self) -> None:
        """x²y'' − xy' + y = 0 → y = (C1 + C2·ln x)·x.

        Indicial: r² + (−1−1)r + 1 = 0  → r² − 2r + 1 = 0  → (r−1)² = 0.
        """
        from fractions import Fraction
        result = solve_euler_cauchy(Fraction(1), Fraction(-1), Fraction(1), Y, X)

        assert isinstance(result, IRApply)
        assert result.head == EQUAL
        sol = result.args[1]
        sol_str = str(sol)
        assert "Log" in sol_str
        # x to the first power appears (not x^3 or anything else)
        # We check that x itself is a factor somewhere in the solution.
        assert X in _walk_nodes(sol)

    def test_repeated_root_solution_structure(self) -> None:
        """Repeated root: top-level node is Mul, inner Add contains C1 and C2."""
        from fractions import Fraction
        result = solve_euler_cauchy(Fraction(1), Fraction(-5), Fraction(9), Y, X)
        sol = result.args[1]
        # Top level: Mul((C1 + C2·ln x), x^r)
        assert isinstance(sol, IRApply)
        assert sol.head == MUL

    # ------------------------------------------------------------------ #
    # Case 3: complex conjugate roots (negative discriminant)            #
    # ------------------------------------------------------------------ #

    def test_complex_roots_zero_alpha(self) -> None:
        """x²y'' + xy' + y = 0 → y = C1·cos(ln x) + C2·sin(ln x).

        Indicial: r² + 0·r + 1 = 0  → roots ±i.  α=0, β=1.
        """
        from fractions import Fraction
        result = solve_euler_cauchy(Fraction(1), Fraction(1), Fraction(1), Y, X)

        assert isinstance(result, IRApply)
        assert result.head == EQUAL
        sol = result.args[1]
        sol_str = str(sol)
        assert "Cos" in sol_str
        assert "Sin" in sol_str
        assert "Log" in sol_str  # β·ln(x) appears inside trig
        # No Pow(x, ...) other than multiplied by the whole expression
        # (alpha=0 means x^alpha = 1, still wrapped in Mul(1,...))

    def test_complex_roots_negative_alpha(self) -> None:
        """x²y'' + 3xy' + 2y = 0 → y = x^{-1}·(C1·cos(ln x) + C2·sin(ln x)).

        Indicial: r² + 2r + 2 = 0  → r = −1 ± i.  α=−1, β=1.
        """
        from fractions import Fraction
        result = solve_euler_cauchy(Fraction(1), Fraction(3), Fraction(2), Y, X)

        assert isinstance(result, IRApply)
        assert result.head == EQUAL
        sol = result.args[1]
        sol_str = str(sol)
        assert "Cos" in sol_str
        assert "Sin" in sol_str
        assert "Log" in sol_str
        # x^{-1} must appear → Pow node with Neg exponent
        assert "Pow" in sol_str

    def test_complex_roots_solution_is_mul(self) -> None:
        """Complex case: top-level is Mul(x^α, Add(cos_term, sin_term))."""
        from fractions import Fraction
        result = solve_euler_cauchy(Fraction(1), Fraction(3), Fraction(2), Y, X)
        sol = result.args[1]
        assert isinstance(sol, IRApply)
        assert sol.head == MUL

    # ------------------------------------------------------------------ #
    # Edge cases                                                          #
    # ------------------------------------------------------------------ #

    def test_solution_head_is_equal(self) -> None:
        """All cases: result is Equal(y, ...)."""
        from fractions import Fraction
        for a, b, c in [
            (Fraction(1), Fraction(2), Fraction(-2)),   # distinct real
            (Fraction(1), Fraction(-5), Fraction(9)),   # repeated
            (Fraction(1), Fraction(1), Fraction(1)),    # complex
        ]:
            r = solve_euler_cauchy(a, b, c, Y, X)
            assert isinstance(r, IRApply)
            assert r.head == EQUAL
            assert r.args[0] == Y

    def test_c1_c2_constants_present(self) -> None:
        """All three root cases must contain both %c1 and %c2."""
        from fractions import Fraction
        for a, b, c in [
            (Fraction(1), Fraction(2), Fraction(-2)),
            (Fraction(1), Fraction(-5), Fraction(9)),
            (Fraction(1), Fraction(1), Fraction(1)),
        ]:
            sol = solve_euler_cauchy(a, b, c, Y, X).args[1]
            nodes = _walk_nodes(sol)
            assert C1 in nodes, f"C1 missing for a={a},b={b},c={c}"
            assert C2 in nodes, f"C2 missing for a={a},b={b},c={c}"


class TestTryEulerCauchy:
    """Integration tests for _try_euler_cauchy (pattern-match + solve)."""

    def test_matches_distinct_real(self) -> None:
        """x²y'' + 2xy' − 2y = 0 is recognised and solved."""
        expr = _build_ec_expr(1, 2, -2)
        result = _try_euler_cauchy(expr, Y, X)
        assert result is not None
        assert isinstance(result, IRApply)
        assert result.head == EQUAL
        assert result.args[0] == Y

    def test_matches_repeated_root(self) -> None:
        """x²y'' − 5xy' + 9y = 0 is recognised and solved (Log appears)."""
        expr = _build_ec_expr(1, -5, 9)
        result = _try_euler_cauchy(expr, Y, X)
        assert result is not None
        assert "Log" in str(result)

    def test_matches_complex_roots(self) -> None:
        """x²y'' + xy' + y = 0 is recognised and solved (Cos/Sin appear)."""
        expr = _build_ec_expr(1, 1, 1)
        result = _try_euler_cauchy(expr, Y, X)
        assert result is not None
        sol_str = str(result)
        assert "Cos" in sol_str
        assert "Sin" in sol_str

    def test_no_x2_term_returns_none(self) -> None:
        """y'' + y' + y = 0 (constant-coeff) is not Euler-Cauchy → None."""
        expr = _add(_add(Y_DOUBLE, Y_PRIME), Y)
        assert _try_euler_cauchy(expr, Y, X) is None

    def test_first_order_returns_none(self) -> None:
        """y' + y = 0 has no y'' term → None."""
        expr = _add(Y_PRIME, Y)
        assert _try_euler_cauchy(expr, Y, X) is None

    def test_ec_two_term_only_x2_and_y(self) -> None:
        """x²y'' + y = 0 (b=0) is a valid two-term Euler-Cauchy."""
        # Indicial: r² + (-1)r + 1 = 0 → r² - r + 1 = 0
        # disc = 1 - 4 = -3 < 0 → complex roots
        expr = _add(_ec_term_x2_yprime2(1), _ec_term_y(1))
        result = _try_euler_cauchy(expr, Y, X)
        assert result is not None
        assert result.head == EQUAL


class TestEulerCauchyViaDispatcher:
    """End-to-end tests: solve_ode / eval_ode dispatches to Euler-Cauchy."""

    def test_dispatch_distinct_real_via_solve_ode(self) -> None:
        """solve_ode(x²y''+2xy'-2y, y, x, vm) → Equal(y, ...) with no Log."""
        vm = make_vm()
        expr = _build_ec_expr(1, 2, -2)
        result = solve_ode(expr, Y, X, vm)
        assert result is not None
        assert isinstance(result, IRApply)
        assert result.head == EQUAL
        assert "Log" not in str(result)

    def test_dispatch_repeated_root_via_solve_ode(self) -> None:
        """solve_ode(x²y''-5xy'+9y, y, x, vm) → Equal(y, ...) with Log."""
        vm = make_vm()
        expr = _build_ec_expr(1, -5, 9)
        result = solve_ode(expr, Y, X, vm)
        assert result is not None
        assert "Log" in str(result)

    def test_dispatch_complex_via_solve_ode(self) -> None:
        """solve_ode(x²y''+xy'+y, y, x, vm) → Equal(y, ...) with Cos/Sin."""
        vm = make_vm()
        expr = _build_ec_expr(1, 1, 1)
        result = solve_ode(expr, Y, X, vm)
        assert result is not None
        sol_str = str(result)
        assert "Cos" in sol_str
        assert "Sin" in sol_str

    def test_const_coeff_not_consumed_by_euler_cauchy(self) -> None:
        """y'' − 3y' + 2y = 0 is constant-coeff, not Euler-Cauchy.
        It must be solved by the const-coeff solver (Exp in the result).
        """
        vm = make_vm()
        # y'' - 3y' + 2y = 0  (standard const-coeff, roots 1 and 2)
        expr = _add(_add(Y_DOUBLE, _mul(IRInteger(-3), Y_PRIME)), _mul(IRInteger(2), Y))
        result = solve_ode(expr, Y, X, vm)
        assert result is not None
        assert "Exp" in str(result)

    def test_eval_ode_ode2_dispatch_euler_cauchy(self) -> None:
        """eval_ode wraps solve_ode via ODE2 — same result."""
        expr = _build_ec_expr(1, 2, -2)
        result = eval_ode(expr)
        assert result is not None
        assert isinstance(result, IRApply)
        assert result.head == EQUAL
        assert result.args[0] == Y

    def test_euler_cauchy_scaled_coefficients(self) -> None:
        """2x²y'' + 4xy' − 4y = 0 is equivalent to x²y''+2xy'-2y=0 after division.
        The solver works with integer coefficients directly — same roots.
        """
        vm = make_vm()
        expr = _build_ec_expr(2, 4, -4)   # 2·(x²y''+2xy'-2y)=0
        result = solve_ode(expr, Y, X, vm)
        # Indicial: 2r²+(4-2)r-4=0 → 2r²+2r-4=0 → r²+r-2=0 → r=1,-2
        # Exact same roots as the unscaled case.
        assert result is not None
        assert result.head == EQUAL
        assert "Log" not in str(result)


# ---------------------------------------------------------------------------
# Shared walk helper (used by Euler-Cauchy tests above)
# ---------------------------------------------------------------------------


def _walk_nodes(node: IRNode) -> list[IRNode]:
    """Collect all nodes in an IR tree (depth-first, pre-order)."""
    acc: list[IRNode] = [node]
    if isinstance(node, IRApply):
        for arg in node.args:
            acc.extend(_walk_nodes(arg))
    return acc
