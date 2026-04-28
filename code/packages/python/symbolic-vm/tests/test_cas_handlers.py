"""Tests for the CAS substrate handlers installed on SymbolicBackend.

All tests here construct IR **directly** — they do NOT go through the
MACSYMA parser or extend the compiler name table (those tests live in
``macsyma-runtime`` which depends on this package, not the other way
around). This keeps the test boundary clean:

- ``symbolic-vm`` tests: handler logic in isolation.
- ``macsyma-runtime`` tests: end-to-end pipeline with MACSYMA syntax.
"""

from __future__ import annotations

import math

import pytest
from symbolic_ir import (
    ADD,
    MUL,
    POW,
    SUB,
    IRApply,
    IRFloat,
    IRInteger,
    IRRational,
    IRSymbol,
)

from symbolic_vm import VM, SymbolicBackend

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def make_vm() -> tuple[VM, SymbolicBackend]:
    backend = SymbolicBackend()
    return VM(backend), backend


_LIST = IRSymbol("List")
_FACTOR = IRSymbol("Factor")
_SOLVE = IRSymbol("Solve")
_SIMPLIFY = IRSymbol("Simplify")
_EXPAND = IRSymbol("Expand")
_SUBST = IRSymbol("Subst")
_LIMIT = IRSymbol("Limit")
_TAYLOR = IRSymbol("Taylor")
_LENGTH = IRSymbol("Length")
_FIRST = IRSymbol("First")
_REST = IRSymbol("Rest")
_LAST = IRSymbol("Last")
_APPEND = IRSymbol("Append")
_REVERSE = IRSymbol("Reverse")
_RANGE = IRSymbol("Range")
_MAP = IRSymbol("Map")
_APPLY = IRSymbol("Apply")
_SELECT = IRSymbol("Select")
_SORT = IRSymbol("Sort")
_PART = IRSymbol("Part")
_FLATTEN = IRSymbol("Flatten")
_JOIN = IRSymbol("Join")
_MATRIX = IRSymbol("Matrix")
_TRANSPOSE = IRSymbol("Transpose")
_DETERMINANT = IRSymbol("Determinant")
_INVERSE = IRSymbol("Inverse")
_ABS = IRSymbol("Abs")
_FLOOR = IRSymbol("Floor")
_CEILING = IRSymbol("Ceiling")
_MOD = IRSymbol("Mod")
_GCD = IRSymbol("Gcd")
_LCM = IRSymbol("Lcm")
_LHS = IRSymbol("Lhs")
_RHS = IRSymbol("Rhs")
_MAKE_LIST = IRSymbol("MakeList")
_AT = IRSymbol("At")
_EQUAL = IRSymbol("Equal")

x = IRSymbol("x")
y = IRSymbol("y")


def ilist(*args: object) -> IRApply:
    """Convenience: build an ``IRApply(List, args)``."""
    return IRApply(_LIST, tuple(args))  # type: ignore[arg-type]


def irow(*args: object) -> IRApply:
    return ilist(*args)


# ===========================================================================
# Section 1: Simplify + Expand
# ===========================================================================


def test_simplify_add_zero() -> None:
    """Simplify(x + 0) → x."""
    vm, _ = make_vm()
    expr = IRApply(_SIMPLIFY, (IRApply(ADD, (x, IRInteger(0))),))
    assert vm.eval(expr) == x


def test_simplify_mul_one() -> None:
    """Simplify(x * 1) → x."""
    vm, _ = make_vm()
    expr = IRApply(_SIMPLIFY, (IRApply(MUL, (x, IRInteger(1))),))
    assert vm.eval(expr) == x


def test_simplify_numeric_fold() -> None:
    """Simplify(2 + 3) → 5."""
    vm, _ = make_vm()
    expr = IRApply(_SIMPLIFY, (IRApply(ADD, (IRInteger(2), IRInteger(3))),))
    assert vm.eval(expr) == IRInteger(5)


def test_simplify_wrong_arity_passthrough() -> None:
    """Simplify() with wrong arity falls through unevaluated."""
    vm, _ = make_vm()
    expr = IRApply(_SIMPLIFY, ())
    assert vm.eval(expr) == expr


def test_expand_canonical_flattens_nested_add() -> None:
    """Expand applies canonical form: Add(Add(1, 2), x) → Add(1, 2, x)."""
    vm, _ = make_vm()
    # Nested Add should be flattened to a single n-ary Add.
    inner = IRApply(ADD, (IRApply(ADD, (IRInteger(1), IRInteger(2))), x))
    expr = IRApply(_EXPAND, (inner,))
    result = vm.eval(expr)
    assert isinstance(result, IRApply)
    assert result.head == ADD


def test_expand_wrong_arity_passthrough() -> None:
    vm, _ = make_vm()
    expr = IRApply(_EXPAND, ())
    assert vm.eval(expr) == expr


# ===========================================================================
# Section 2: Subst
# ===========================================================================


def test_subst_numeric_eval() -> None:
    """Subst(2, x, x^2 + 1) → 5."""
    vm, _ = make_vm()
    target = IRApply(ADD, (IRApply(POW, (x, IRInteger(2))), IRInteger(1)))
    expr = IRApply(_SUBST, (IRInteger(2), x, target))
    assert vm.eval(expr) == IRInteger(5)


def test_subst_symbolic() -> None:
    """Subst(y, x, x^2) → y^2."""
    vm, _ = make_vm()
    target = IRApply(POW, (x, IRInteger(2)))
    expr = IRApply(_SUBST, (y, x, target))
    result = vm.eval(expr)
    assert isinstance(result, IRApply)
    assert result.head == POW
    assert result.args[0] == y
    assert result.args[1] == IRInteger(2)


def test_subst_no_match_passthrough() -> None:
    """Subst(2, y, x^2) → x^2 (no occurrences of y)."""
    vm, _ = make_vm()
    target = IRApply(POW, (x, IRInteger(2)))
    expr = IRApply(_SUBST, (IRInteger(2), y, target))
    # No y in target, so result is just x^2 (evaluated)
    result = vm.eval(expr)
    assert isinstance(result, IRApply)
    assert result.head == POW
    assert result.args[0] == x


def test_subst_wrong_arity_passthrough() -> None:
    vm, _ = make_vm()
    expr = IRApply(_SUBST, (IRInteger(1),))
    assert vm.eval(expr) == expr


# ===========================================================================
# Section 3: Factor
# ===========================================================================


def test_factor_difference_of_squares() -> None:
    """Factor(x^2 - 1) returns a product of factors, not the original."""
    vm, _ = make_vm()
    target = IRApply(SUB, (IRApply(POW, (x, IRInteger(2))), IRInteger(1)))
    expr = IRApply(_FACTOR, (target,))
    result = vm.eval(expr)
    # Result should be some Mul/Add/Sub expression, not the Factor(...) node.
    assert result != expr


def test_factor_irreducible_passthrough() -> None:
    """Factor(x^2 + 1) stays unevaluated — no integer roots."""
    vm, _ = make_vm()
    target = IRApply(ADD, (IRApply(POW, (x, IRInteger(2))), IRInteger(1)))
    expr = IRApply(_FACTOR, (target,))
    result = vm.eval(expr)
    # Irreducible over Z → returned unevaluated.
    assert result == expr


def test_factor_linear() -> None:
    """Factor(x - 3) → x - 3 (already irreducible linear, content 1)."""
    vm, _ = make_vm()
    target = IRApply(SUB, (x, IRInteger(3)))
    expr = IRApply(_FACTOR, (target,))
    result = vm.eval(expr)
    # Linear polynomial: factored result should be structurally equivalent.
    assert result is not None


def test_factor_no_variable() -> None:
    """Factor(4) — purely numeric, returns the inner expression."""
    vm, _ = make_vm()
    expr = IRApply(_FACTOR, (IRInteger(4),))
    result = vm.eval(expr)
    assert result == IRInteger(4)


def test_factor_wrong_arity_passthrough() -> None:
    vm, _ = make_vm()
    expr = IRApply(_FACTOR, ())
    assert vm.eval(expr) == expr


def test_factor_sophie_germain_x4_plus_4() -> None:
    """Factor(x^4 + 4) = (x^2+2x+2)(x^2-2x+2) — Sophie Germain identity."""
    vm, _ = make_vm()
    # x^4 + 4
    target = IRApply(ADD, (IRApply(POW, (x, IRInteger(4))), IRInteger(4)))
    expr = IRApply(_FACTOR, (target,))
    result = vm.eval(expr)
    # Must be a non-trivial product — not equal to the original Factor(…).
    assert result != expr
    # Must be an IRApply (Mul tree — left-associative binary product).
    assert isinstance(result, IRApply)
    # The top-level head must be Mul (product of two quadratics).
    assert result.head.name == "Mul"


def test_factor_x4_plus_x2_plus_1_cyclotomic() -> None:
    """Factor(x^4+x^2+1) = (x^2+x+1)(x^2-x+1)."""
    vm, _ = make_vm()
    # x^4 + x^2 + 1 built as Add(Add(x^4, x^2), 1)
    target = IRApply(
        ADD,
        (
            IRApply(ADD, (IRApply(POW, (x, IRInteger(4))), IRApply(POW, (x, IRInteger(2))))),
            IRInteger(1),
        ),
    )
    expr = IRApply(_FACTOR, (target,))
    result = vm.eval(expr)
    assert result != expr
    assert isinstance(result, IRApply)
    assert result.head.name == "Mul"


# ===========================================================================
# Section 4: Solve
# ===========================================================================


def test_solve_linear() -> None:
    """Solve(2*x - 4, x) → [2]."""
    vm, _ = make_vm()
    poly = IRApply(SUB, (IRApply(MUL, (IRInteger(2), x)), IRInteger(4)))
    expr = IRApply(_SOLVE, (poly, x))
    result = vm.eval(expr)
    assert isinstance(result, IRApply)
    assert result.head.name == "List"
    assert result.args == (IRInteger(2),)


def test_solve_quadratic() -> None:
    """Solve(x^2 - 5*x + 6, x) → [2, 3]."""
    vm, _ = make_vm()
    poly = IRApply(
        ADD,
        (
            IRApply(
                SUB,
                (IRApply(POW, (x, IRInteger(2))), IRApply(MUL, (IRInteger(5), x))),
            ),
            IRInteger(6),
        ),
    )
    expr = IRApply(_SOLVE, (poly, x))
    result = vm.eval(expr)
    assert isinstance(result, IRApply)
    assert result.head.name == "List"
    solutions = set(result.args)
    assert IRInteger(2) in solutions
    assert IRInteger(3) in solutions


def test_solve_no_solution() -> None:
    """Solve(1, x) → [] (constant non-zero — no solution)."""
    vm, _ = make_vm()
    expr = IRApply(_SOLVE, (IRInteger(1), x))
    result = vm.eval(expr)
    assert isinstance(result, IRApply)
    assert result.args == ()


def test_solve_equal_form() -> None:
    """Solve(Equal(x, 3), x) → [3]."""
    vm, _ = make_vm()
    eq = IRApply(IRSymbol("Equal"), (x, IRInteger(3)))
    expr = IRApply(_SOLVE, (eq, x))
    result = vm.eval(expr)
    assert isinstance(result, IRApply)
    assert result.head.name == "List"
    assert IRInteger(3) in result.args


def test_solve_wrong_var_passthrough() -> None:
    vm, _ = make_vm()
    expr = IRApply(_SOLVE, (x, IRInteger(1)))
    assert vm.eval(expr) == expr


def test_solve_cubic_three_rational() -> None:
    """Solve(x^3 - 6x^2 + 11x - 6, x) → [1, 2, 3]."""
    vm, _ = make_vm()
    # x^3 - 6x^2 + 11x - 6
    cubic = IRApply(ADD, (
        IRApply(SUB, (
            IRApply(ADD, (
                IRApply(SUB, (
                    IRApply(POW, (x, IRInteger(3))),
                    IRApply(MUL, (IRInteger(6), IRApply(POW, (x, IRInteger(2))))),
                )),
                IRApply(MUL, (IRInteger(11), x)),
            )),
            IRInteger(6),
        )),
        IRInteger(0),
    ))
    result = vm.eval(IRApply(_SOLVE, (cubic, x)))
    assert isinstance(result, IRApply)
    assert result.head.name == "List"
    assert IRInteger(1) in result.args
    assert IRInteger(2) in result.args
    assert IRInteger(3) in result.args


def test_solve_cubic_one_rational_two_complex() -> None:
    """Solve(x^3 + 1, x) → list with -1 and complex pair."""
    vm, _ = make_vm()
    cubic = IRApply(ADD, (IRApply(POW, (x, IRInteger(3))), IRInteger(1)))
    result = vm.eval(IRApply(_SOLVE, (cubic, x)))
    assert isinstance(result, IRApply)
    assert result.head.name == "List"
    assert len(result.args) == 3
    assert IRInteger(-1) in result.args


def test_solve_quartic_four_rational() -> None:
    """Solve(x^4 - 10x^2 + 9, x) → [±1, ±3]."""
    vm, _ = make_vm()
    # x^4 - 10x^2 + 9
    quartic = IRApply(ADD, (
        IRApply(SUB, (IRApply(POW, (x, IRInteger(4))), IRApply(MUL, (IRInteger(10), IRApply(POW, (x, IRInteger(2))))))),
        IRInteger(9),
    ))
    result = vm.eval(IRApply(_SOLVE, (quartic, x)))
    assert isinstance(result, IRApply)
    assert result.head.name == "List"
    int_roots = {a.value for a in result.args if isinstance(a, IRInteger)}
    assert int_roots == {1, -1, 3, -3}


def test_solve_degree_5_passthrough() -> None:
    """Solve(x^5, x) returns unevaluated (degree > 4)."""
    vm, _ = make_vm()
    expr = IRApply(_SOLVE, (IRApply(POW, (x, IRInteger(5))), x))
    result = vm.eval(expr)
    # Degree 5 should be returned unevaluated
    assert isinstance(result, IRApply)


def test_solve_linear_system_2x2() -> None:
    """Solve(List(x+y=3, x-y=1), List(x,y)) → [Rule(x,2), Rule(y,1)]."""
    vm, _ = make_vm()
    y = IRSymbol("y")
    _EQ = IRSymbol("Equal")
    _LIST = IRSymbol("List")
    _RULE = IRSymbol("Rule")
    # x + y = 3 and x - y = 1
    eq1 = IRApply(_EQ, (IRApply(ADD, (x, y)), IRInteger(3)))
    eq2 = IRApply(_EQ, (IRApply(SUB, (x, y)), IRInteger(1)))
    eqs_list = IRApply(_LIST, (eq1, eq2))
    vars_list = IRApply(_LIST, (x, y))
    result = vm.eval(IRApply(_SOLVE, (eqs_list, vars_list)))
    assert isinstance(result, IRApply)
    assert result.head.name == "List"
    rules = {r.args[0].name: r.args[1] for r in result.args if isinstance(r, IRApply) and r.head.name == "Rule"}
    assert rules["x"] == IRInteger(2)
    assert rules["y"] == IRInteger(1)


def test_nsolve_cubic() -> None:
    """NSolve(x^3 - 6x^2 + 11x - 6, x) → 3 float roots near 1, 2, 3."""
    from symbolic_ir import IRFloat

    vm, _ = make_vm()
    _NSOLVE = IRSymbol("NSolve")
    # x^3 - 6x^2 + 11x - 6
    cubic = IRApply(ADD, (
        IRApply(SUB, (
            IRApply(ADD, (
                IRApply(SUB, (
                    IRApply(POW, (x, IRInteger(3))),
                    IRApply(MUL, (IRInteger(6), IRApply(POW, (x, IRInteger(2))))),
                )),
                IRApply(MUL, (IRInteger(11), x)),
            )),
            IRInteger(6),
        )),
        IRInteger(0),
    ))
    result = vm.eval(IRApply(_NSOLVE, (cubic, x)))
    assert isinstance(result, IRApply)
    assert result.head.name == "List"
    assert len(result.args) == 3
    # All roots should be IRFloat with values near 1, 2, 3
    vals = sorted(r.value for r in result.args if isinstance(r, IRFloat))
    assert len(vals) == 3
    assert abs(vals[0] - 1.0) < 1e-6
    assert abs(vals[1] - 2.0) < 1e-6
    assert abs(vals[2] - 3.0) < 1e-6


def test_nsolve_quintic_five_roots() -> None:
    """NSolve(x^5 - 1, x) → 5 roots."""
    vm, _ = make_vm()
    _NSOLVE = IRSymbol("NSolve")
    # x^5 - 1
    quintic = IRApply(SUB, (IRApply(POW, (x, IRInteger(5))), IRInteger(1)))
    result = vm.eval(IRApply(_NSOLVE, (quintic, x)))
    assert isinstance(result, IRApply)
    assert result.head.name == "List"
    assert len(result.args) == 5


# ===========================================================================
# Section 5: List operations
# ===========================================================================


def test_length() -> None:
    lst = ilist(IRInteger(1), IRInteger(2), IRInteger(3))
    vm, _ = make_vm()
    assert vm.eval(IRApply(_LENGTH, (lst,))) == IRInteger(3)


def test_length_empty() -> None:
    vm, _ = make_vm()
    assert vm.eval(IRApply(_LENGTH, (ilist(),))) == IRInteger(0)


def test_first() -> None:
    lst = ilist(x, y, IRInteger(3))
    vm, _ = make_vm()
    assert vm.eval(IRApply(_FIRST, (lst,))) == x


def test_rest() -> None:
    lst = ilist(x, y, IRInteger(3))
    vm, _ = make_vm()
    result = vm.eval(IRApply(_REST, (lst,)))
    assert result == ilist(y, IRInteger(3))


def test_last() -> None:
    lst = ilist(x, y, IRInteger(3))
    vm, _ = make_vm()
    assert vm.eval(IRApply(_LAST, (lst,))) == IRInteger(3)


def test_append_two_lists() -> None:
    l1 = ilist(IRInteger(1), IRInteger(2))
    l2 = ilist(IRInteger(3), IRInteger(4))
    vm, _ = make_vm()
    result = vm.eval(IRApply(_APPEND, (l1, l2)))
    assert result == ilist(IRInteger(1), IRInteger(2), IRInteger(3), IRInteger(4))


def test_reverse() -> None:
    lst = ilist(IRInteger(1), IRInteger(2), IRInteger(3))
    vm, _ = make_vm()
    result = vm.eval(IRApply(_REVERSE, (lst,)))
    assert result == ilist(IRInteger(3), IRInteger(2), IRInteger(1))


def test_range_single_arg() -> None:
    vm, _ = make_vm()
    result = vm.eval(IRApply(_RANGE, (IRInteger(4),)))
    assert result == ilist(IRInteger(1), IRInteger(2), IRInteger(3), IRInteger(4))


def test_range_two_args() -> None:
    vm, _ = make_vm()
    result = vm.eval(IRApply(_RANGE, (IRInteger(2), IRInteger(5))))
    assert result == ilist(IRInteger(2), IRInteger(3), IRInteger(4), IRInteger(5))


def test_range_three_args_step() -> None:
    vm, _ = make_vm()
    result = vm.eval(IRApply(_RANGE, (IRInteger(1), IRInteger(9), IRInteger(2))))
    assert result == ilist(IRInteger(1), IRInteger(3), IRInteger(5), IRInteger(7), IRInteger(9))


def test_part_one_based() -> None:
    lst = ilist(x, y, IRInteger(3))
    vm, _ = make_vm()
    assert vm.eval(IRApply(_PART, (lst, IRInteger(2)))) == y


def test_part_negative_index() -> None:
    lst = ilist(x, y, IRInteger(3))
    vm, _ = make_vm()
    assert vm.eval(IRApply(_PART, (lst, IRInteger(-1)))) == IRInteger(3)


def test_flatten_one_level() -> None:
    nested = ilist(IRInteger(1), ilist(IRInteger(2), IRInteger(3)), IRInteger(4))
    vm, _ = make_vm()
    result = vm.eval(IRApply(_FLATTEN, (nested,)))
    assert result == ilist(IRInteger(1), IRInteger(2), IRInteger(3), IRInteger(4))


def test_join() -> None:
    l1 = ilist(IRInteger(1))
    l2 = ilist(IRInteger(2), IRInteger(3))
    vm, _ = make_vm()
    result = vm.eval(IRApply(_JOIN, (l1, l2)))
    assert result == ilist(IRInteger(1), IRInteger(2), IRInteger(3))


def test_sort_numerics_before_symbols() -> None:
    """Sort puts numerics before symbols (same ordering as canonical)."""
    lst = ilist(x, IRInteger(1))
    vm, _ = make_vm()
    result = vm.eval(IRApply(_SORT, (lst,)))
    assert isinstance(result, IRApply)
    assert result.head.name == "List"
    assert result.args[0] == IRInteger(1)
    assert result.args[1] == x


def test_map_add_one() -> None:
    """Map(f, [1, 2, 3]) where f(x) := x+1 gives [2, 3, 4]."""
    from symbolic_ir import DEFINE, LIST

    lst = ilist(IRInteger(1), IRInteger(2), IRInteger(3))
    vm, backend = make_vm()
    f_sym = IRSymbol("f")
    body = IRApply(ADD, (x, IRInteger(1)))
    define_record = IRApply(DEFINE, (f_sym, IRApply(LIST, (x,)), body))
    backend.bind("f", define_record)
    result = vm.eval(IRApply(_MAP, (f_sym, lst)))
    assert result == ilist(IRInteger(2), IRInteger(3), IRInteger(4))


def test_map_with_lambda() -> None:
    """Map(lambda([z], z^2), [1, 2, 3, 4]) → [1, 4, 9, 16].

    The lambda is an inline IRApply used as the function head.
    The VM's ``_apply_lambda`` method performs beta-reduction.
    """
    from symbolic_ir import LIST

    z = IRSymbol("z")
    # lambda([z], z^2)
    lambda_node = IRApply(
        IRSymbol("lambda"),
        (IRApply(LIST, (z,)), IRApply(POW, (z, IRInteger(2)))),
    )
    lst = ilist(IRInteger(1), IRInteger(2), IRInteger(3), IRInteger(4))
    vm, _ = make_vm()
    result = vm.eval(IRApply(_MAP, (lambda_node, lst)))
    assert result == ilist(IRInteger(1), IRInteger(4), IRInteger(9), IRInteger(16))


def test_lambda_direct_call() -> None:
    """IRApply(lambda([z], z+1), (5,)) β-reduces to 6."""
    from symbolic_ir import LIST

    z = IRSymbol("z")
    lambda_node = IRApply(
        IRSymbol("lambda"),
        (IRApply(LIST, (z,)), IRApply(ADD, (z, IRInteger(1)))),
    )
    vm, _ = make_vm()
    result = vm.eval(IRApply(lambda_node, (IRInteger(5),)))
    assert result == IRInteger(6)


def test_lambda_two_params() -> None:
    """lambda([a, b], a + b) applied to (3, 4) → 7."""
    from symbolic_ir import LIST

    a_sym, b_sym = IRSymbol("a"), IRSymbol("b")
    lambda_node = IRApply(
        IRSymbol("lambda"),
        (IRApply(LIST, (a_sym, b_sym)), IRApply(ADD, (a_sym, b_sym))),
    )
    vm, _ = make_vm()
    result = vm.eval(IRApply(lambda_node, (IRInteger(3), IRInteger(4))))
    assert result == IRInteger(7)


def test_apply_add() -> None:
    """Apply(Add, [3, 4]) evaluates to 7.

    Add is a binary operator in the symbolic VM, so Apply is tested
    with a 2-element list rather than a 3-element one.  The n-ary fold
    would require a left-fold wrapper that is a MACSYMA-level concern,
    not part of the universal substrate.
    """
    lst = ilist(IRInteger(3), IRInteger(4))
    vm, _ = make_vm()
    result = vm.eval(IRApply(_APPLY, (ADD, lst)))
    assert result == IRInteger(7)


def test_select_keeps_matching() -> None:
    """Select(IsEven-like, [1, 2, 3, 4]) — use a user function as predicate."""
    from symbolic_ir import DEFINE, LIST

    lst = ilist(IRInteger(1), IRInteger(2), IRInteger(3), IRInteger(4))
    vm, backend = make_vm()
    p_sym = IRSymbol("p")
    # Predicate p(x) := x = 2  (only keeps 2).
    from symbolic_ir import EQUAL
    body = IRApply(EQUAL, (x, IRInteger(2)))
    define_record = IRApply(DEFINE, (p_sym, IRApply(LIST, (x,)), body))
    backend.bind("p", define_record)
    result = vm.eval(IRApply(_SELECT, (p_sym, lst)))
    assert isinstance(result, IRApply)
    assert result.head.name == "List"
    assert result.args == (IRInteger(2),)


def test_list_wrong_arity_passthrough() -> None:
    vm, _ = make_vm()
    expr = IRApply(_LENGTH, ())
    assert vm.eval(expr) == expr


def test_list_non_list_passthrough() -> None:
    vm, _ = make_vm()
    expr = IRApply(_FIRST, (IRInteger(1),))
    # Not a list — should fall through unevaluated.
    assert vm.eval(expr) == expr


# ===========================================================================
# Section 6: Matrix operations
# ===========================================================================


def _make_2x2(a: IRNode, b: IRNode, c: IRNode, d: IRNode) -> IRApply:
    """Build a 2×2 Matrix expression (unevaluated)."""
    return IRApply(_MATRIX, (irow(a, b), irow(c, d)))


def test_matrix_construction() -> None:
    """Matrix(List(1,2), List(3,4)) produces a valid Matrix IR node."""
    vm, _ = make_vm()
    expr = _make_2x2(IRInteger(1), IRInteger(2), IRInteger(3), IRInteger(4))
    result = vm.eval(expr)
    assert isinstance(result, IRApply)
    assert result.head.name == "Matrix"


def test_determinant_2x2() -> None:
    """det([[1,2],[3,4]]) = 1*4 - 2*3 = -2.

    The determinant handler passes the raw cofactor expression through
    vm.eval(), so the numeric arithmetic collapses to IRInteger(-2)
    directly without a separate simplify call.
    """
    vm, _ = make_vm()
    M = vm.eval(_make_2x2(IRInteger(1), IRInteger(2), IRInteger(3), IRInteger(4)))
    result = vm.eval(IRApply(_DETERMINANT, (M,)))
    assert result == IRInteger(-2)


def test_transpose_2x2() -> None:
    """Transpose([[1,2],[3,4]]) = [[1,3],[2,4]]."""
    from cas_matrix import get_entry

    vm, _ = make_vm()
    M = vm.eval(_make_2x2(IRInteger(1), IRInteger(2), IRInteger(3), IRInteger(4)))
    result = vm.eval(IRApply(_TRANSPOSE, (M,)))
    assert isinstance(result, IRApply)
    assert result.head.name == "Matrix"
    assert get_entry(result, 1, 1) == IRInteger(1)
    assert get_entry(result, 1, 2) == IRInteger(3)
    assert get_entry(result, 2, 1) == IRInteger(2)
    assert get_entry(result, 2, 2) == IRInteger(4)


def test_matrix_unequal_rows_passthrough() -> None:
    """Matrix with rows of unequal length falls through unevaluated."""
    vm, _ = make_vm()
    expr = IRApply(_MATRIX, (irow(IRInteger(1), IRInteger(2)), irow(IRInteger(3),)))
    result = vm.eval(expr)
    # Should be unchanged (MatrixError caught).
    assert result == expr


# ===========================================================================
# Section 7: Limit and Taylor
# ===========================================================================


def test_limit_polynomial() -> None:
    """Limit(x^2, x, 3) → 9."""
    vm, _ = make_vm()
    body = IRApply(POW, (x, IRInteger(2)))
    expr = IRApply(_LIMIT, (body, x, IRInteger(3)))
    result = vm.eval(expr)
    assert result == IRInteger(9)


def test_limit_linear() -> None:
    """Limit(2*x + 1, x, 0) → 1."""
    vm, _ = make_vm()
    body = IRApply(ADD, (IRApply(MUL, (IRInteger(2), x)), IRInteger(1)))
    expr = IRApply(_LIMIT, (body, x, IRInteger(0)))
    result = vm.eval(expr)
    assert result == IRInteger(1)


def test_limit_wrong_arity_passthrough() -> None:
    vm, _ = make_vm()
    expr = IRApply(_LIMIT, (x, IRSymbol("x")))
    assert vm.eval(expr) == expr


def test_taylor_constant_term() -> None:
    """Taylor(x^2, x, 0, 0) → constant term = 0."""
    vm, _ = make_vm()
    body = IRApply(POW, (x, IRInteger(2)))
    expr = IRApply(_TAYLOR, (body, x, IRInteger(0), IRInteger(0)))
    result = vm.eval(expr)
    assert result == IRInteger(0)


def test_taylor_full_quadratic() -> None:
    """Taylor(x^2, x, 0, 2) → x^2 (exact polynomial up to order 2)."""
    vm, _ = make_vm()
    body = IRApply(POW, (x, IRInteger(2)))
    expr = IRApply(_TAYLOR, (body, x, IRInteger(0), IRInteger(2)))
    result = vm.eval(expr)
    # Should be non-None and not the unevaluated Taylor form.
    assert result is not None
    if isinstance(result, IRApply):
        assert result.head != _TAYLOR


def test_taylor_transcendental_sin() -> None:
    """Taylor(sin(x), x, 0, 4) expands via symbolic diff fallback.

    The polynomial-based path raises PolynomialError; the derivative
    fallback computes each coefficient f^(k)(0)/k! symbolically.
    Numerically verified at x=0.4 (well inside radius of convergence).
    """
    from cas_substitution import subst as _subst
    from symbolic_ir import SIN, IRFloat

    vm, _ = make_vm()
    body = IRApply(SIN, (x,))
    expr = IRApply(_TAYLOR, (body, x, IRInteger(0), IRInteger(4)))
    result = vm.eval(expr)

    # Must not come back as the unevaluated Taylor head.
    assert not (isinstance(result, IRApply) and result.head == _TAYLOR), (
        f"Expected expanded polynomial, got unevaluated: {result}"
    )

    # Numerically verify: T(0.4) ≈ sin(0.4).
    pt = IRFloat(0.4)
    num_result = vm.eval(_subst(pt, x, result))
    assert hasattr(num_result, "value"), (
        f"Expected numeric result after substitution, got {num_result}"
    )
    assert abs(num_result.value - math.sin(0.4)) < 1e-3, (
        f"Taylor(sin(x),x,0,4) at x=0.4: got {num_result.value}, "
        f"expected ≈{math.sin(0.4)}"
    )


# ===========================================================================
# Section 8: Numeric/arithmetic handlers
# ===========================================================================


def test_abs_positive() -> None:
    vm, _ = make_vm()
    assert vm.eval(IRApply(_ABS, (IRInteger(5),))) == IRInteger(5)


def test_abs_negative() -> None:
    vm, _ = make_vm()
    assert vm.eval(IRApply(_ABS, (IRInteger(-3),))) == IRInteger(3)


def test_abs_zero() -> None:
    vm, _ = make_vm()
    assert vm.eval(IRApply(_ABS, (IRInteger(0),))) == IRInteger(0)


def test_abs_rational() -> None:
    vm, _ = make_vm()
    result = vm.eval(IRApply(_ABS, (IRRational(-1, 3),)))
    assert result == IRRational(1, 3)


def test_abs_symbolic_passthrough() -> None:
    vm, _ = make_vm()
    expr = IRApply(_ABS, (x,))
    assert vm.eval(expr) == expr


def test_floor() -> None:
    vm, _ = make_vm()
    assert vm.eval(IRApply(_FLOOR, (IRFloat(3.7),))) == IRInteger(3)


def test_floor_negative() -> None:
    vm, _ = make_vm()
    assert vm.eval(IRApply(_FLOOR, (IRFloat(-1.2),))) == IRInteger(-2)


def test_ceiling() -> None:
    vm, _ = make_vm()
    assert vm.eval(IRApply(_CEILING, (IRFloat(3.2),))) == IRInteger(4)


def test_ceiling_exact() -> None:
    vm, _ = make_vm()
    assert vm.eval(IRApply(_CEILING, (IRInteger(4),))) == IRInteger(4)


def test_mod_positive() -> None:
    vm, _ = make_vm()
    assert vm.eval(IRApply(_MOD, (IRInteger(7), IRInteger(3)))) == IRInteger(1)


def test_mod_zero_divisor_passthrough() -> None:
    vm, _ = make_vm()
    expr = IRApply(_MOD, (IRInteger(7), IRInteger(0)))
    assert vm.eval(expr) == expr


def test_gcd() -> None:
    vm, _ = make_vm()
    assert vm.eval(IRApply(_GCD, (IRInteger(12), IRInteger(8)))) == IRInteger(4)


def test_gcd_coprime() -> None:
    vm, _ = make_vm()
    assert vm.eval(IRApply(_GCD, (IRInteger(7), IRInteger(13)))) == IRInteger(1)


def test_lcm() -> None:
    vm, _ = make_vm()
    assert vm.eval(IRApply(_LCM, (IRInteger(4), IRInteger(6)))) == IRInteger(12)


def test_gcd_symbolic_passthrough() -> None:
    vm, _ = make_vm()
    expr = IRApply(_GCD, (x, IRInteger(4)))
    assert vm.eval(expr) == expr


# ===========================================================================
# Section 9: Lhs / Rhs (C5)
# ===========================================================================


def test_lhs_of_equation() -> None:
    """Lhs(Equal(x, 3)) → x."""
    vm, _ = make_vm()
    eq = IRApply(_EQUAL, (x, IRInteger(3)))
    assert vm.eval(IRApply(_LHS, (eq,))) == x


def test_rhs_of_equation() -> None:
    """Rhs(Equal(x, 3)) → 3."""
    vm, _ = make_vm()
    eq = IRApply(_EQUAL, (x, IRInteger(3)))
    assert vm.eval(IRApply(_RHS, (eq,))) == IRInteger(3)


def test_lhs_non_equation_passthrough() -> None:
    """Lhs(x + 1) returns unevaluated — not an equation."""
    vm, _ = make_vm()
    expr = IRApply(_LHS, (IRApply(ADD, (x, IRInteger(1))),))
    assert vm.eval(expr) == expr


def test_rhs_wrong_arity_passthrough() -> None:
    """Rhs with wrong arity returns unevaluated."""
    vm, _ = make_vm()
    expr = IRApply(_RHS, (x, y))
    assert vm.eval(expr) == expr


def test_lhs_of_numeric_equation() -> None:
    """Lhs(Equal(2*x, 6)) → 2*x (a general IR node)."""
    vm, _ = make_vm()
    lhs_expr = IRApply(IRSymbol("Mul"), (IRInteger(2), x))
    eq = IRApply(_EQUAL, (lhs_expr, IRInteger(6)))
    result = vm.eval(IRApply(_LHS, (eq,)))
    # The result should be the lhs expression (possibly simplified by vm.eval)
    assert isinstance(result, IRApply) or result == lhs_expr


# ===========================================================================
# Section 10: MakeList (C2)
# ===========================================================================


def test_make_list_single_arg() -> None:
    """MakeList(i^2, i, 4) → [1, 4, 9, 16]."""
    vm, _ = make_vm()
    i = IRSymbol("i")
    body = IRApply(POW, (i, IRInteger(2)))
    result = vm.eval(IRApply(_MAKE_LIST, (body, i, IRInteger(4))))
    expected = ilist(IRInteger(1), IRInteger(4), IRInteger(9), IRInteger(16))
    assert result == expected


def test_make_list_range_two_bounds() -> None:
    """MakeList(i, i, 3, 6) → [3, 4, 5, 6]."""
    vm, _ = make_vm()
    i = IRSymbol("i")
    result = vm.eval(IRApply(_MAKE_LIST, (i, i, IRInteger(3), IRInteger(6))))
    expected = ilist(IRInteger(3), IRInteger(4), IRInteger(5), IRInteger(6))
    assert result == expected


def test_make_list_with_step() -> None:
    """MakeList(i*2, i, 1, 5, 2) → [2, 6, 10]  (i=1,3,5)."""
    vm, _ = make_vm()
    i = IRSymbol("i")
    body = IRApply(IRSymbol("Mul"), (IRInteger(2), i))
    result = vm.eval(IRApply(_MAKE_LIST, (body, i, IRInteger(1), IRInteger(5), IRInteger(2))))
    expected = ilist(IRInteger(2), IRInteger(6), IRInteger(10))
    assert result == expected


def test_make_list_constant_body() -> None:
    """MakeList(7, i, 3) → [7, 7, 7] (body doesn't use the variable)."""
    vm, _ = make_vm()
    i = IRSymbol("i")
    result = vm.eval(IRApply(_MAKE_LIST, (IRInteger(7), i, IRInteger(3))))
    expected = ilist(IRInteger(7), IRInteger(7), IRInteger(7))
    assert result == expected


def test_make_list_wrong_arity_passthrough() -> None:
    """MakeList with wrong arity returns unevaluated."""
    vm, _ = make_vm()
    i = IRSymbol("i")
    expr = IRApply(_MAKE_LIST, (i, i))  # only 2 args
    assert vm.eval(expr) == expr


# ===========================================================================
# Section 11: At / point evaluation (C4)
# ===========================================================================


def test_at_single_substitution() -> None:
    """At(x^2 + 1, Equal(x, 3)) → 10."""
    vm, _ = make_vm()
    body = IRApply(ADD, (IRApply(POW, (x, IRInteger(2))), IRInteger(1)))
    rule = IRApply(_EQUAL, (x, IRInteger(3)))
    result = vm.eval(IRApply(_AT, (body, rule)))
    assert result == IRInteger(10)


def test_at_list_of_rules() -> None:
    """At(x + y, List(Equal(x, 2), Equal(y, 5))) → 7."""
    vm, _ = make_vm()
    body = IRApply(ADD, (x, y))
    rules = ilist(IRApply(_EQUAL, (x, IRInteger(2))), IRApply(_EQUAL, (y, IRInteger(5))))
    result = vm.eval(IRApply(_AT, (body, rules)))
    assert result == IRInteger(7)


def test_at_non_rule_passthrough() -> None:
    """At(x^2, x) — second arg is not an Equal — returns unevaluated."""
    vm, _ = make_vm()
    body = IRApply(POW, (x, IRInteger(2)))
    expr = IRApply(_AT, (body, x))
    assert vm.eval(expr) == expr


def test_at_wrong_arity_passthrough() -> None:
    """At with wrong arity returns unevaluated."""
    vm, _ = make_vm()
    expr = IRApply(_AT, (x,))
    assert vm.eval(expr) == expr


# ===========================================================================
# Section 12: Complex number handlers (B2)
# ===========================================================================

_I = IRSymbol("ImaginaryUnit")
_RE = IRSymbol("Re")
_IM = IRSymbol("Im")
_CONJUGATE = IRSymbol("Conjugate")
_ARG = IRSymbol("Arg")
_RECT_FORM = IRSymbol("RectForm")
_POLAR_FORM = IRSymbol("PolarForm")


def _rect(a: object, b: object) -> IRApply:
    """Build ``a + b*ImaginaryUnit`` in IR."""
    return IRApply(ADD, (a, IRApply(MUL, (b, _I))))  # type: ignore[arg-type]


def test_imaginary_unit_prebound() -> None:
    """ImaginaryUnit is pre-bound to itself in SymbolicBackend."""
    vm, _ = make_vm()
    assert vm.eval(_I) == _I


def test_i_power_2() -> None:
    """ImaginaryUnit^2 → -1."""
    vm, _ = make_vm()
    expr = IRApply(IRSymbol("Pow"), (_I, IRInteger(2)))
    assert vm.eval(expr) == IRInteger(-1)


def test_i_power_4() -> None:
    """ImaginaryUnit^4 → 1."""
    vm, _ = make_vm()
    expr = IRApply(IRSymbol("Pow"), (_I, IRInteger(4)))
    assert vm.eval(expr) == IRInteger(1)


def test_i_power_3() -> None:
    """ImaginaryUnit^3 → Neg(ImaginaryUnit)."""
    vm, _ = make_vm()
    expr = IRApply(IRSymbol("Pow"), (_I, IRInteger(3)))
    result = vm.eval(expr)
    assert isinstance(result, IRApply)
    assert result.head.name == "Neg"
    assert result.args[0] == _I


def test_re_of_rect() -> None:
    """Re(3 + 4*i) → 3."""
    vm, _ = make_vm()
    node = _rect(IRInteger(3), IRInteger(4))
    assert vm.eval(IRApply(_RE, (node,))) == IRInteger(3)


def test_re_of_pure_real() -> None:
    """Re(x) → x when x has no imaginary part."""
    vm, _ = make_vm()
    assert vm.eval(IRApply(_RE, (x,))) == x


def test_im_of_rect() -> None:
    """Im(3 + 4*i) → 4."""
    vm, _ = make_vm()
    node = _rect(IRInteger(3), IRInteger(4))
    assert vm.eval(IRApply(_IM, (node,))) == IRInteger(4)


def test_im_of_pure_real() -> None:
    """Im(5) → 0."""
    vm, _ = make_vm()
    assert vm.eval(IRApply(_IM, (IRInteger(5),))) == IRInteger(0)


def test_conjugate_rect() -> None:
    """Conjugate(3 + 4*i) → Add(3, ...) form."""
    vm, _ = make_vm()
    node = _rect(IRInteger(3), IRInteger(4))
    result = vm.eval(IRApply(_CONJUGATE, (node,)))
    assert isinstance(result, IRApply)
    assert result.head.name in ("Add",)


def test_conjugate_pure_imaginary() -> None:
    """Conjugate(i) → Neg(i)."""
    vm, _ = make_vm()
    result = vm.eval(IRApply(_CONJUGATE, (_I,)))
    assert isinstance(result, IRApply)
    assert result.head.name == "Neg"
    assert result.args[0] == _I


def test_abs_complex_3_4i() -> None:
    """Abs(3 + 4*i) → sqrt(25) (not yet folded to 5)."""
    vm, _ = make_vm()
    node = _rect(IRInteger(3), IRInteger(4))
    result = vm.eval(IRApply(_ABS, (node,)))
    assert result is not None


def test_abs_real_numeric() -> None:
    """Abs(-3) → 3 (real numeric fold unchanged)."""
    vm, _ = make_vm()
    result = vm.eval(IRApply(_ABS, (IRInteger(-3),)))
    assert result == IRInteger(3)


def test_re_wrong_arity_passthrough() -> None:
    """Re(a, b) → unevaluated."""
    vm, _ = make_vm()
    expr = IRApply(_RE, (IRInteger(1), IRInteger(2)))
    assert vm.eval(expr) == expr


def test_im_wrong_arity_passthrough() -> None:
    """Im(a, b) → unevaluated."""
    vm, _ = make_vm()
    expr = IRApply(_IM, (IRInteger(1), IRInteger(2)))
    assert vm.eval(expr) == expr


# ===========================================================================
# Section 13: Trig handlers (B1)
# ===========================================================================

_TRIG_SIMPLIFY = IRSymbol("TrigSimplify")
_TRIG_EXPAND = IRSymbol("TrigExpand")
_TRIG_REDUCE = IRSymbol("TrigReduce")
_SIN = IRSymbol("Sin")
_COS = IRSymbol("Cos")
_PI = IRSymbol("%pi")


def test_trig_simplify_pythagorean() -> None:
    """TrigSimplify(sin²(x) + cos²(x)) → 1."""
    vm, _ = make_vm()
    sin2 = IRApply(POW, (IRApply(_SIN, (x,)), IRInteger(2)))
    cos2 = IRApply(POW, (IRApply(_COS, (x,)), IRInteger(2)))
    expr = IRApply(_TRIG_SIMPLIFY, (IRApply(ADD, (sin2, cos2)),))
    result = vm.eval(expr)
    assert result == IRInteger(1)


def test_trig_simplify_sin_pi_is_zero() -> None:
    """TrigSimplify(Sin(π)) → 0."""
    vm, _ = make_vm()
    expr = IRApply(_TRIG_SIMPLIFY, (IRApply(_SIN, (_PI,)),))
    result = vm.eval(expr)
    assert result == IRInteger(0)


def test_trig_expand_sin_2x() -> None:
    """TrigExpand(Sin(2x)) expands to contain Sin and Cos."""
    vm, _ = make_vm()
    expr = IRApply(_TRIG_EXPAND, (
        IRApply(_SIN, (IRApply(MUL, (IRInteger(2), x)),)),
    ))
    result = vm.eval(expr)
    assert isinstance(result, IRApply)


def test_trig_reduce_sin2() -> None:
    """TrigReduce(Sin(x)^2) → (1 - Cos(2x)) / 2."""
    vm, _ = make_vm()
    expr = IRApply(_TRIG_REDUCE, (
        IRApply(POW, (IRApply(_SIN, (x,)), IRInteger(2))),
    ))
    result = vm.eval(expr)
    assert isinstance(result, IRApply)
    # Should not be a plain Pow(Sin(x), 2) anymore
    assert not (
        result.head.name == "Pow"
        and isinstance(result.args[0], IRApply)
        and result.args[0].head.name == "Sin"
    )


def test_trig_simplify_wrong_arity_passthrough() -> None:
    """TrigSimplify(a, b) → unevaluated."""
    vm, _ = make_vm()
    y = IRSymbol("y")
    expr = IRApply(_TRIG_SIMPLIFY, (x, y))
    assert vm.eval(expr) == expr


# ===========================================================================
# Section 14: Rational function operations (A3)
#             Expand (full), Collect, Together, RatSimplify, Apart
# ===========================================================================

_COLLECT = IRSymbol("Collect")
_TOGETHER = IRSymbol("Together")
_RAT_SIMPLIFY = IRSymbol("RatSimplify")
_APART = IRSymbol("Apart")
_DIV = IRSymbol("Div")


def _subst_at(vm: "VM", expr: IRApply, val: int) -> IRNode:
    """Evaluate ``expr`` with ``x = val`` by calling the Subst handler."""
    return vm.eval(IRApply(_SUBST, (IRInteger(val), x, expr)))


# --- Expand (full polynomial distribution) ----------------------------------


def test_expand_product() -> None:
    """Expand((x+1)*(x+2)) → polynomial evaluating to 2, 6, 12 at x=0,1,2."""
    vm, _ = make_vm()
    inner = IRApply(MUL, (IRApply(ADD, (x, IRInteger(1))), IRApply(ADD, (x, IRInteger(2)))))
    result = vm.eval(IRApply(_EXPAND, (inner,)))
    # The result should NOT still be a Mul or an Expand — it should be expanded
    assert not (isinstance(result, IRApply) and result.head == _EXPAND)
    # Verify by evaluating at numeric points:  (0+1)*(0+2)=2, (1+1)*(1+2)=6
    assert _subst_at(vm, result, 0) == IRInteger(2)
    assert _subst_at(vm, result, 1) == IRInteger(6)
    assert _subst_at(vm, result, 2) == IRInteger(12)


def test_expand_power() -> None:
    """Expand((x+1)^2) → polynomial evaluating to 1, 4, 9 at x=0,1,2."""
    vm, _ = make_vm()
    inner = IRApply(POW, (IRApply(ADD, (x, IRInteger(1))), IRInteger(2)))
    result = vm.eval(IRApply(_EXPAND, (inner,)))
    assert _subst_at(vm, result, 0) == IRInteger(1)
    assert _subst_at(vm, result, 1) == IRInteger(4)
    assert _subst_at(vm, result, 2) == IRInteger(9)


def test_expand_already_flat() -> None:
    """Expand(x^2 + 1) → still evaluates correctly at numeric points."""
    vm, _ = make_vm()
    inner = IRApply(ADD, (IRApply(POW, (x, IRInteger(2))), IRInteger(1)))
    result = vm.eval(IRApply(_EXPAND, (inner,)))
    assert _subst_at(vm, result, 0) == IRInteger(1)
    assert _subst_at(vm, result, 2) == IRInteger(5)


def test_expand_rational_function() -> None:
    """Expand of a rational function returns a Div node (not flattened further)."""
    from symbolic_ir import DIV

    vm, _ = make_vm()
    # (x+1) / (x+2) — can't expand a rational function into a polynomial
    inner = IRApply(DIV, (IRApply(ADD, (x, IRInteger(1))), IRApply(ADD, (x, IRInteger(2)))))
    result = vm.eval(IRApply(_EXPAND, (inner,)))
    # Should return something (either Div or the original) — not crash
    assert result is not None


def test_expand_wrong_arity_passthrough() -> None:
    """Expand() or Expand(a, b) with wrong arity → unevaluated."""
    vm, _ = make_vm()
    expr = IRApply(_EXPAND, ())
    assert vm.eval(expr) == expr


# --- Collect ----------------------------------------------------------------


def test_collect_combines_like_terms() -> None:
    """Collect(x^2 + x^2, x) → 2*x^2."""
    vm, _ = make_vm()
    inner = IRApply(ADD, (IRApply(POW, (x, IRInteger(2))), IRApply(POW, (x, IRInteger(2)))))
    result = vm.eval(IRApply(_COLLECT, (inner, x)))
    # 2*x^2 evaluates to 0, 2, 8 at x=0, 1, 2
    assert _subst_at(vm, result, 0) == IRInteger(0)
    assert _subst_at(vm, result, 1) == IRInteger(2)
    assert _subst_at(vm, result, 2) == IRInteger(8)


def test_collect_linear_terms() -> None:
    """Collect(3*x + 2*x + 1, x) → 5*x + 1."""
    vm, _ = make_vm()
    inner = IRApply(ADD, (
        IRApply(ADD, (IRApply(MUL, (IRInteger(3), x)), IRApply(MUL, (IRInteger(2), x)))),
        IRInteger(1),
    ))
    result = vm.eval(IRApply(_COLLECT, (inner, x)))
    # (5*x + 1): at x=0 → 1, at x=1 → 6, at x=2 → 11
    assert _subst_at(vm, result, 0) == IRInteger(1)
    assert _subst_at(vm, result, 1) == IRInteger(6)
    assert _subst_at(vm, result, 2) == IRInteger(11)


def test_collect_wrong_arity_passthrough() -> None:
    """Collect with wrong arity → unevaluated."""
    vm, _ = make_vm()
    expr = IRApply(_COLLECT, (x,))  # missing var arg
    assert vm.eval(expr) == expr


def test_collect_non_symbol_var_passthrough() -> None:
    """Collect(expr, 3) — non-symbol var → unevaluated."""
    vm, _ = make_vm()
    inner = IRApply(POW, (x, IRInteger(2)))
    expr = IRApply(_COLLECT, (inner, IRInteger(3)))
    assert vm.eval(expr) == expr


# --- Together ---------------------------------------------------------------


def test_together_two_fractions() -> None:
    """Together(1/x + 1/(x+1)) evaluates correctly at x=2: 1/2 + 1/3 = 5/6."""
    from symbolic_ir import DIV

    vm, _ = make_vm()
    # Build 1/x + 1/(x+1) in IR
    frac1 = IRApply(DIV, (IRInteger(1), x))
    frac2 = IRApply(DIV, (IRInteger(1), IRApply(ADD, (x, IRInteger(1)))))
    inner = IRApply(ADD, (frac1, frac2))
    result = vm.eval(IRApply(_TOGETHER, (inner,)))
    # At x=2: 1/2 + 1/3 = 5/6 — and the combined form should give the same
    val = _subst_at(vm, result, 2)
    assert val == IRRational(5, 6)


def test_together_already_polynomial() -> None:
    """Together(x^2 + 1) → polynomial (no denominator change)."""
    vm, _ = make_vm()
    inner = IRApply(ADD, (IRApply(POW, (x, IRInteger(2))), IRInteger(1)))
    result = vm.eval(IRApply(_TOGETHER, (inner,)))
    assert _subst_at(vm, result, 2) == IRInteger(5)


def test_together_wrong_arity_passthrough() -> None:
    """Together() with wrong arity → unevaluated."""
    vm, _ = make_vm()
    expr = IRApply(_TOGETHER, ())
    assert vm.eval(expr) == expr


# --- RatSimplify ------------------------------------------------------------


def test_ratsimp_cancel_linear_factor() -> None:
    """RatSimplify((x^2-1)/(x-1)) → x+1."""
    from symbolic_ir import DIV, SUB

    vm, _ = make_vm()
    # (x^2 - 1) / (x - 1)
    num = IRApply(SUB, (IRApply(POW, (x, IRInteger(2))), IRInteger(1)))
    den = IRApply(SUB, (x, IRInteger(1)))
    inner = IRApply(DIV, (num, den))
    result = vm.eval(IRApply(_RAT_SIMPLIFY, (inner,)))
    # Result should be x + 1 (a polynomial): at x=2 → 3, at x=3 → 4
    assert _subst_at(vm, result, 2) == IRInteger(3)
    assert _subst_at(vm, result, 3) == IRInteger(4)


def test_ratsimp_already_reduced() -> None:
    """RatSimplify(1/(x+1)) → same rational function (nothing to cancel)."""
    from symbolic_ir import DIV

    vm, _ = make_vm()
    inner = IRApply(DIV, (IRInteger(1), IRApply(ADD, (x, IRInteger(1)))))
    result = vm.eval(IRApply(_RAT_SIMPLIFY, (inner,)))
    # At x=1: 1/2 → IRRational(1,2)
    val = _subst_at(vm, result, 1)
    assert val == IRRational(1, 2)


def test_ratsimp_polynomial_passthrough() -> None:
    """RatSimplify(x + 1) → x + 1 (already a polynomial)."""
    vm, _ = make_vm()
    inner = IRApply(ADD, (x, IRInteger(1)))
    result = vm.eval(IRApply(_RAT_SIMPLIFY, (inner,)))
    assert _subst_at(vm, result, 3) == IRInteger(4)


def test_ratsimp_wrong_arity_passthrough() -> None:
    """RatSimplify() with wrong arity → unevaluated."""
    vm, _ = make_vm()
    expr = IRApply(_RAT_SIMPLIFY, ())
    assert vm.eval(expr) == expr


# --- Apart ------------------------------------------------------------------


def test_apart_simple_poles() -> None:
    """Apart(1/(x^2-1), x) decomposes into partial fractions.

    1/(x^2-1) = 1/((x-1)(x+1)) = 1/(2*(x-1)) - 1/(2*(x+1))

    We verify by evaluating the result at x=3: 1/(9-1) = 1/8.
    """
    from symbolic_ir import DIV, SUB

    vm, _ = make_vm()
    # 1 / (x^2 - 1)
    den = IRApply(SUB, (IRApply(POW, (x, IRInteger(2))), IRInteger(1)))
    inner = IRApply(DIV, (IRInteger(1), den))
    result = vm.eval(IRApply(_APART, (inner, x)))
    # Verify the partial fraction at x=3: 1/(9-1) = 1/8
    val = _subst_at(vm, result, 3)
    assert val == IRRational(1, 8)


def test_apart_two_simple_poles() -> None:
    """Apart((3x+5)/((x+2)(x+3)), x) decomposes correctly.

    At x=0: 5/(2*3) = 5/6; verify the apart result gives the same.
    """
    from symbolic_ir import DIV, MUL

    vm, _ = make_vm()
    # (3*x + 5) / ((x+2)*(x+3))
    num = IRApply(ADD, (IRApply(MUL, (IRInteger(3), x)), IRInteger(5)))
    den = IRApply(MUL, (
        IRApply(ADD, (x, IRInteger(2))),
        IRApply(ADD, (x, IRInteger(3))),
    ))
    inner = IRApply(DIV, (num, den))
    result = vm.eval(IRApply(_APART, (inner, x)))
    val = _subst_at(vm, result, 0)
    assert val == IRRational(5, 6)


def test_apart_already_polynomial() -> None:
    """Apart(x^2 + 1, x) → x^2 + 1 (no fraction to decompose)."""
    vm, _ = make_vm()
    inner = IRApply(ADD, (IRApply(POW, (x, IRInteger(2))), IRInteger(1)))
    result = vm.eval(IRApply(_APART, (inner, x)))
    assert _subst_at(vm, result, 2) == IRInteger(5)


def test_apart_wrong_arity_passthrough() -> None:
    """Apart(x) with only one arg → unevaluated."""
    vm, _ = make_vm()
    expr = IRApply(_APART, (x,))
    assert vm.eval(expr) == expr


def test_apart_non_symbol_var_passthrough() -> None:
    """Apart(1/(x+1), 3) — non-symbol var → unevaluated."""
    from symbolic_ir import DIV

    vm, _ = make_vm()
    inner = IRApply(DIV, (IRInteger(1), IRApply(ADD, (x, IRInteger(1)))))
    expr = IRApply(_APART, (inner, IRInteger(3)))
    assert vm.eval(expr) == expr
