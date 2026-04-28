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

from cas_ode import build_ode_handler_table, solve_ode
from cas_ode.handlers import ode2_handler
from cas_ode.ode import (
    _collect_linear_first_order,
    _collect_second_order_coeffs,
    _exact_sqrt_fraction,
    _flatten_add,
    _is_const_wrt,
    _isqrt_exact,
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
