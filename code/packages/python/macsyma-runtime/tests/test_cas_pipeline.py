"""End-to-end pipeline tests: MACSYMA source → VM result.

Each test sends a MACSYMA string through the complete pipeline:

    parse_macsyma  →  compile_macsyma  →  VM(MacsymaBackend).eval

This validates that the name-table extension (``extend_compiler_name_table``)
wires MACSYMA user-visible names (``factor``, ``solve``, ``length``, …) to
the canonical IR heads (``Factor``, ``Solve``, ``Length``, …) that the
``SymbolicBackend`` CAS handlers dispatch on.

These tests belong in ``macsyma-runtime`` rather than ``symbolic-vm`` because
they require the full MACSYMA compiler + name table, which is a
``macsyma-runtime`` concern.  Pure handler unit tests live in
``symbolic-vm/tests/test_cas_handlers.py`` and use IR directly.
"""

from __future__ import annotations

import pytest
from macsyma_compiler import compile_macsyma
from macsyma_compiler.compiler import _STANDARD_FUNCTIONS
from macsyma_parser import parse_macsyma
from symbolic_ir import IRApply, IRFloat, IRInteger, IRRational, IRSymbol

from macsyma_runtime import MacsymaBackend, extend_compiler_name_table
from symbolic_vm import VM

# Extend the compiler name table so MACSYMA names compile to canonical IR.
# This call is idempotent — subsequent calls are no-ops.
extend_compiler_name_table(_STANDARD_FUNCTIONS)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _eval(source: str) -> object:
    """Parse + compile + eval ``source`` (no terminator needed).

    Returns the evaluated IR node.  The Display/Suppress wrapper added by
    ``wrap_terminators=False`` is absent here; we evaluate the raw IR.
    """
    # Normalise: strip trailing ``;`` or ``$`` so our pipeline is clean.
    src = source.strip().rstrip(";$").strip()
    ast = parse_macsyma(src + ";")
    stmts = compile_macsyma(ast, wrap_terminators=False)
    assert len(stmts) == 1, f"expected 1 statement, got {len(stmts)}: {stmts}"
    vm = VM(MacsymaBackend())
    return vm.eval(stmts[0])


def _int(n: int) -> IRInteger:
    return IRInteger(n)


def _sym(name: str) -> IRSymbol:
    return IRSymbol(name)


# ---------------------------------------------------------------------------
# Section A — symbolic simplification
# ---------------------------------------------------------------------------


def test_pipeline_simplify_add_zero() -> None:
    """simplify(x + 0) → x (identity-rule fires)."""
    result = _eval("simplify(x + 0)")
    assert result == IRSymbol("x")


def test_pipeline_simplify_mul_one() -> None:
    """simplify(x * 1) → x."""
    result = _eval("simplify(x * 1)")
    assert result == IRSymbol("x")


def test_pipeline_expand_is_callable() -> None:
    """expand(x + 0) returns a non-null IR node (canonical form)."""
    result = _eval("expand(x + 0)")
    assert result is not None


# ---------------------------------------------------------------------------
# Section B — substitution
# ---------------------------------------------------------------------------


def test_pipeline_subst_numeric() -> None:
    """subst(2, x, x^2 + 1) → 5."""
    result = _eval("subst(2, x, x^2 + 1)")
    assert result == _int(5)


def test_pipeline_subst_symbolic() -> None:
    """subst(y, x, x + x) → 2*y in some form (addition still present)."""
    result = _eval("subst(y, x, x + x)")
    # After substitution x→y: y+y; after arithmetic: may fold or remain Add.
    # The substitution must have replaced x with y: result should not be x.
    assert result != IRSymbol("x")


# ---------------------------------------------------------------------------
# Section C — factoring
# ---------------------------------------------------------------------------


def test_pipeline_factor_difference_of_squares() -> None:
    """factor(x^2 - 1) returns a factored expression, not Factor(...)."""
    result = _eval("factor(x^2 - 1)")
    # The Factor handler returns something other than Factor(Sub(Pow(x,2),1)).
    assert not (
        isinstance(result, IRApply)
        and isinstance(result.head, IRSymbol)
        and result.head.name == "Factor"
    ), f"Expected factored result, got unevaluated: {result}"


def test_pipeline_factor_irreducible_stays_unevaluated() -> None:
    """factor(x^2 + 1) is irreducible over Z → stays as Factor(...)."""
    result = _eval("factor(x^2 + 1)")
    assert isinstance(result, IRApply)
    assert isinstance(result.head, IRSymbol)
    assert result.head.name == "Factor"


# ---------------------------------------------------------------------------
# Section D — solving
# ---------------------------------------------------------------------------


def test_pipeline_solve_linear() -> None:
    """solve(2*x - 4, x) → [2]."""
    result = _eval("solve(2*x - 4, x)")
    assert isinstance(result, IRApply)
    assert result.head.name == "List"
    assert len(result.args) == 1
    # Solution may be IRRational(2,1) or IRInteger(2).
    sol = result.args[0]
    assert sol in (_int(2), IRRational(2, 1))


def test_pipeline_solve_quadratic() -> None:
    """solve(x^2 - 5*x + 6, x) → [2, 3] in some order."""
    result = _eval("solve(x^2 - 5*x + 6, x)")
    assert isinstance(result, IRApply)
    assert result.head.name == "List"
    assert len(result.args) == 2
    # Solutions may be IRRational or IRInteger.
    vals = {(s.numerator if isinstance(s, IRRational) else s.value) for s in result.args}
    assert vals == {2, 3}


# ---------------------------------------------------------------------------
# Section E — list operations
# ---------------------------------------------------------------------------


def test_pipeline_length() -> None:
    """length([a, b, c]) → 3."""
    result = _eval("length([a, b, c])")
    assert result == _int(3)


def test_pipeline_first() -> None:
    """first([a, b, c]) → a."""
    result = _eval("first([a, b, c])")
    assert result == _sym("a")


def test_pipeline_rest() -> None:
    """rest([a, b, c]) → [b, c]."""
    result = _eval("rest([a, b, c])")
    assert isinstance(result, IRApply)
    assert result.head.name == "List"
    assert result.args == (_sym("b"), _sym("c"))


def test_pipeline_append() -> None:
    """append([1], [2, 3]) → [1, 2, 3]."""
    result = _eval("append([1], [2, 3])")
    assert isinstance(result, IRApply)
    assert result.head.name == "List"
    assert len(result.args) == 3


def test_pipeline_reverse() -> None:
    """reverse([1, 2, 3]) → [3, 2, 1]."""
    result = _eval("reverse([1, 2, 3])")
    assert isinstance(result, IRApply)
    assert result.head.name == "List"
    assert result.args == (_int(3), _int(2), _int(1))


# ---------------------------------------------------------------------------
# Section F — constants
# ---------------------------------------------------------------------------


def test_pipeline_pi_resolves() -> None:
    """%pi evaluates to an IRFloat close to math.pi."""
    import math

    result = _eval("%pi")
    assert isinstance(result, IRFloat)
    assert abs(result.value - math.pi) < 1e-9


def test_pipeline_e_resolves() -> None:
    """%e evaluates to an IRFloat close to math.e."""
    import math

    result = _eval("%e")
    assert isinstance(result, IRFloat)
    assert abs(result.value - math.e) < 1e-9


# ---------------------------------------------------------------------------
# Section G — limit
# ---------------------------------------------------------------------------


def test_pipeline_limit_polynomial() -> None:
    """limit(x^2, x, 3) → 9."""
    result = _eval("limit(x^2, x, 3)")
    # Result may be IRInteger(9) or IRRational(9,1) after numeric fold.
    if isinstance(result, IRRational):
        assert result.numerator == 9
    else:
        assert result == _int(9)


# ---------------------------------------------------------------------------
# Section H — lhs / rhs (C5)
# ---------------------------------------------------------------------------


def test_pipeline_lhs_of_equation() -> None:
    """lhs(x = 3) → x."""
    result = _eval("lhs(x = 3)")
    assert result == _sym("x")


def test_pipeline_rhs_of_equation() -> None:
    """rhs(x = 3) → 3."""
    result = _eval("rhs(x = 3)")
    assert result == _int(3)


def test_pipeline_lhs_of_complex_equation() -> None:
    """lhs(x^2 - 1 = 0) → x^2 - 1 (in some IR form)."""
    result = _eval("lhs(x^2 - 1 = 0)")
    # Should not be the integer 0 (that's the rhs) or the full equation.
    assert result != _int(0)
    assert not (
        isinstance(result, IRApply)
        and isinstance(result.head, IRSymbol)
        and result.head.name == "Equal"
    ), "lhs should strip the Equal wrapper"


def test_pipeline_rhs_of_complex_equation() -> None:
    """rhs(x^2 - 1 = 0) → 0."""
    result = _eval("rhs(x^2 - 1 = 0)")
    assert result == _int(0)


# ---------------------------------------------------------------------------
# Section I — makelist (C2)
# ---------------------------------------------------------------------------


def test_pipeline_makelist_squares() -> None:
    """makelist(i^2, i, 4) → [1, 4, 9, 16]."""
    result = _eval("makelist(i^2, i, 4)")
    assert isinstance(result, IRApply)
    assert result.head.name == "List"
    assert len(result.args) == 4
    assert result.args[0] == _int(1)
    assert result.args[1] == _int(4)
    assert result.args[2] == _int(9)
    assert result.args[3] == _int(16)


def test_pipeline_makelist_range() -> None:
    """makelist(i, i, 3, 6) → [3, 4, 5, 6]."""
    result = _eval("makelist(i, i, 3, 6)")
    assert isinstance(result, IRApply)
    assert result.head.name == "List"
    assert len(result.args) == 4
    assert result.args == (_int(3), _int(4), _int(5), _int(6))


def test_pipeline_makelist_step() -> None:
    """makelist(i, i, 1, 9, 2) → [1, 3, 5, 7, 9]."""
    result = _eval("makelist(i, i, 1, 9, 2)")
    assert isinstance(result, IRApply)
    assert result.head.name == "List"
    assert len(result.args) == 5
    assert result.args == (_int(1), _int(3), _int(5), _int(7), _int(9))


# ---------------------------------------------------------------------------
# Section J — at / point evaluation (C4)
# ---------------------------------------------------------------------------


def test_pipeline_at_single_point() -> None:
    """at(x^2 + 1, x = 3) → 10."""
    result = _eval("at(x^2 + 1, x = 3)")
    assert result == _int(10)


def test_pipeline_at_linear() -> None:
    """at(2*x - 1, x = 5) → 9."""
    result = _eval("at(2*x - 1, x = 5)")
    assert result == _int(9)


# ---------------------------------------------------------------------------
# Section K — number theory (B3)
# ---------------------------------------------------------------------------


def test_pipeline_primep_true() -> None:
    """primep(97) → True."""
    result = _eval("primep(97)")
    assert result == _sym("True")


def test_pipeline_primep_false() -> None:
    """primep(100) → False."""
    result = _eval("primep(100)")
    assert result == _sym("False")


def test_pipeline_next_prime() -> None:
    """next_prime(10) → 11."""
    result = _eval("next_prime(10)")
    assert result == _int(11)


def test_pipeline_ifactor() -> None:
    """ifactor(12) returns a list of [prime, exp] pairs."""
    result = _eval("ifactor(12)")
    assert isinstance(result, IRApply)
    assert result.head.name == "List"
    # Should have 2 pairs: [[2,2],[3,1]]
    assert len(result.args) == 2


def test_pipeline_divisors() -> None:
    """divisors(12) → [1, 2, 3, 4, 6, 12]."""
    result = _eval("divisors(12)")
    assert isinstance(result, IRApply)
    assert result.head.name == "List"
    values = [a.value for a in result.args]  # type: ignore[attr-defined]
    assert values == [1, 2, 3, 4, 6, 12]


def test_pipeline_totient() -> None:
    """totient(12) → 4."""
    result = _eval("totient(12)")
    assert result == _int(4)


# ===========================================================================
# Section L: Complex number operations (B2)
# ===========================================================================


def test_pipeline_imaginary_unit() -> None:
    """%i evaluates to the ImaginaryUnit symbol."""
    result = _eval("%i")
    assert isinstance(result, IRSymbol)
    assert result.name == "ImaginaryUnit"


def test_pipeline_realpart_of_rect() -> None:
    """realpart(3 + 4*%i) → 3."""
    result = _eval("realpart(3 + 4*%i)")
    assert result == _int(3)


def test_pipeline_imagpart_of_rect() -> None:
    """imagpart(3 + 4*%i) → 4."""
    result = _eval("imagpart(3 + 4*%i)")
    assert result == _int(4)


def test_pipeline_realpart_pure_real() -> None:
    """realpart(7) → 7 (no imaginary component)."""
    result = _eval("realpart(7)")
    assert result == _int(7)


def test_pipeline_imagpart_pure_real() -> None:
    """imagpart(7) → 0."""
    result = _eval("imagpart(7)")
    assert result == _int(0)


def test_pipeline_conjugate_rect() -> None:
    """conjugate(3 + 4*%i) returns an Add expression (3 - 4*%i)."""
    result = _eval("conjugate(3 + 4*%i)")
    assert isinstance(result, IRApply)
    assert result.head.name == "Add"


def test_pipeline_i_power_2() -> None:
    """%i^2 → -1."""
    result = _eval("%i^2")
    assert result == _int(-1)


def test_pipeline_i_power_4() -> None:
    """%i^4 → 1."""
    result = _eval("%i^4")
    assert result == _int(1)


def test_pipeline_i_power_3() -> None:
    """%i^3 → -i (a Neg expression)."""
    result = _eval("%i^3")
    assert isinstance(result, IRApply)
    assert result.head.name == "Neg"


# ---------------------------------------------------------------------------
# Section M — cubic and quartic equation solving (A2a / A2b)
# ---------------------------------------------------------------------------


def test_pipeline_solve_cubic_three_rational() -> None:
    """solve(x^3 - 6*x^2 + 11*x - 6, x) → [1, 2, 3]."""
    result = _eval("solve(x^3 - 6*x^2 + 11*x - 6, x)")
    assert isinstance(result, IRApply)
    assert result.head.name == "List"
    roots = set(result.args)
    assert IRInteger(1) in roots
    assert IRInteger(2) in roots
    assert IRInteger(3) in roots


def test_pipeline_solve_cubic_one_rational_two_complex() -> None:
    """solve(x^3 + 1, x) → 3 roots including -1."""
    result = _eval("solve(x^3 + 1, x)")
    assert isinstance(result, IRApply)
    assert result.head.name == "List"
    assert len(result.args) == 3
    assert IRInteger(-1) in result.args


def test_pipeline_solve_quartic_four_rational() -> None:
    """solve(x^4 - 10*x^2 + 9, x) → {±1, ±3}."""
    result = _eval("solve(x^4 - 10*x^2 + 9, x)")
    assert isinstance(result, IRApply)
    assert result.head.name == "List"
    roots = set(result.args)
    assert IRInteger(1) in roots
    assert IRInteger(-1) in roots
    assert IRInteger(3) in roots
    assert IRInteger(-3) in roots


def test_pipeline_solve_quartic_biquadratic() -> None:
    """solve(x^4 - 5*x^2 + 4, x) → {±1, ±2}."""
    result = _eval("solve(x^4 - 5*x^2 + 4, x)")
    assert isinstance(result, IRApply)
    assert result.head.name == "List"
    roots = set(result.args)
    assert IRInteger(1) in roots
    assert IRInteger(-1) in roots
    assert IRInteger(2) in roots
    assert IRInteger(-2) in roots


def test_pipeline_solve_quartic_all_positive_roots() -> None:
    """solve(x^4 - 10*x^3 + 35*x^2 - 50*x + 24, x) → {1, 2, 3, 4}."""
    result = _eval("solve(x^4 - 10*x^3 + 35*x^2 - 50*x + 24, x)")
    assert isinstance(result, IRApply)
    assert result.head.name == "List"
    roots = set(result.args)
    assert IRInteger(1) in roots
    assert IRInteger(2) in roots
    assert IRInteger(3) in roots
    assert IRInteger(4) in roots


# ---------------------------------------------------------------------------
# Section N — NSolve numeric root-finding (A2c)
# ---------------------------------------------------------------------------


def test_pipeline_nsolve_cubic() -> None:
    """nsolve(x^3 - 6*x^2 + 11*x - 6, x) → 3 numeric roots near 1, 2, 3."""
    result = _eval("nsolve(x^3 - 6*x^2 + 11*x - 6, x)")
    assert isinstance(result, IRApply)
    assert result.head.name == "List"
    assert len(result.args) == 3
    vals = sorted(r.value for r in result.args if isinstance(r, IRFloat))
    assert len(vals) == 3
    assert abs(vals[0] - 1.0) < 1e-6
    assert abs(vals[1] - 2.0) < 1e-6
    assert abs(vals[2] - 3.0) < 1e-6


def test_pipeline_nsolve_quintic() -> None:
    """nsolve(x^5 - 1, x) → 5 roots."""
    result = _eval("nsolve(x^5 - 1, x)")
    assert isinstance(result, IRApply)
    assert result.head.name == "List"
    assert len(result.args) == 5


# ---------------------------------------------------------------------------
# Section O — Linear system solving (A2d)
# ---------------------------------------------------------------------------


def test_pipeline_linsolve_2x2() -> None:
    """solve([x + y = 3, x - y = 1], [x, y]) → [Rule(x,2), Rule(y,1)]."""
    # MACSYMA's linsolve routes to Solve with list args
    result = _eval("linsolve([x + y = 3, x - y = 1], [x, y])")
    assert isinstance(result, IRApply)
    assert result.head.name == "List"
    rules = {
        r.args[0].name: r.args[1]
        for r in result.args
        if isinstance(r, IRApply) and r.head.name == "Rule"
    }
    assert rules["x"] == IRInteger(2)
    assert rules["y"] == IRInteger(1)


def test_pipeline_linsolve_3x3() -> None:
    """solve([x+y+z=6, 2*x+y=5, z=3], [x,y,z]) → [x=2, y=1, z=3]."""
    result = _eval("linsolve([x + y + z = 6, 2*x + y = 5, z = 3], [x, y, z])")
    assert isinstance(result, IRApply)
    assert result.head.name == "List"
    rules = {
        r.args[0].name: r.args[1]
        for r in result.args
        if isinstance(r, IRApply) and r.head.name == "Rule"
    }
    assert rules["x"] == IRInteger(2)
    assert rules["y"] == IRInteger(1)
    assert rules["z"] == IRInteger(3)
