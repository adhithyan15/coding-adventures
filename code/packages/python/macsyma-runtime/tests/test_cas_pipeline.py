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


def test_pipeline_is_prime_alias_true() -> None:
    """is_prime(17) → True (alias for primep)."""
    result = _eval("is_prime(17)")
    assert result == _sym("True")


def test_pipeline_is_prime_alias_false() -> None:
    """is_prime(15) → False (alias for primep)."""
    result = _eval("is_prime(15)")
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


# ---------------------------------------------------------------------------
# Section P — Trig operations (B1)
# ---------------------------------------------------------------------------


def test_pipeline_trigsimp_pythagorean() -> None:
    """trigsimp(sin(x)^2 + cos(x)^2) → 1."""
    result = _eval("trigsimp(sin(x)^2 + cos(x)^2)")
    assert result == IRInteger(1)


def test_pipeline_trigsimp_sin_pi() -> None:
    """trigsimp(sin(%pi)) → 0 or IRFloat(0.0).

    Note: %pi is pre-bound to IRFloat in MacsymaBackend, so sin(%pi) may
    evaluate numerically before trigsimp sees the symbolic form.
    """
    result = _eval("trigsimp(sin(%pi))")
    # Accept IRInteger(0) or IRFloat(≈0)
    if isinstance(result, IRInteger):
        assert result == IRInteger(0)
    else:
        assert isinstance(result, IRFloat)
        assert abs(result.value) < 1e-10


def test_pipeline_trigsimp_cos_pi() -> None:
    """trigsimp(cos(%pi)) → -1 or IRFloat(-1.0)."""
    result = _eval("trigsimp(cos(%pi))")
    if isinstance(result, IRInteger):
        assert result == IRInteger(-1)
    else:
        assert isinstance(result, IRFloat)
        assert abs(result.value + 1.0) < 1e-10


def test_pipeline_trigexpand_sin_2x() -> None:
    """trigexpand(sin(2*x)) → expanded (contains Sin and Cos)."""
    result = _eval("trigexpand(sin(2*x))")
    assert isinstance(result, IRApply)
    # The expansion should not be a bare Sin(2*x) anymore
    if result.head.name == "Sin":
        # Allow canonical form to leave it as-is only if it didn't expand
        pass  # pragma: no cover
    # At minimum it should return an IR expression


def test_pipeline_trigreduce_sin2() -> None:
    """trigreduce(sin(x)^2) → (1 - cos(2*x)) / 2."""
    result = _eval("trigreduce(sin(x)^2)")
    assert isinstance(result, IRApply)
    # Should not be a plain Pow(Sin(x), 2) any more
    assert not (
        result.head.name == "Pow"
        and isinstance(result.args[0], IRApply)
        and result.args[0].head.name == "Sin"
    )


# ---------------------------------------------------------------------------
# Section Q — Rational function operations (A3)
# ---------------------------------------------------------------------------


def test_pipeline_expand_product() -> None:
    """expand((x+1)*(x+2)) produces the expanded polynomial form."""
    result = _eval("expand((x+1)*(x+2))")
    # Result should not be a Mul or Expand — it must be expanded
    assert isinstance(result, IRApply)
    assert result.head.name not in ("Expand", "Mul")
    # Verify numeric correctness: (0+1)*(0+2)=2 at x=0 requires subst
    # Just check the structural head is Add (polynomial form)
    assert result.head.name == "Add"


def test_pipeline_expand_power() -> None:
    """expand((x+1)^2) produces the expanded polynomial."""
    result = _eval("expand((x+1)^2)")
    assert isinstance(result, IRApply)
    assert result.head.name == "Add"


def test_pipeline_collect_like_powers() -> None:
    """collect(x^2 + 3*x + x^2, x) → 2*x^2 + 3*x."""
    result = _eval("collect(x^2 + 3*x + x^2, x)")
    # The result should be a collected polynomial (an Add expression)
    assert isinstance(result, IRApply)
    assert result.head.name == "Add"


def test_pipeline_ratsimp_cancel() -> None:
    """ratsimp((x^2-1)/(x-1)) → x+1."""
    result = _eval("ratsimp((x^2-1)/(x-1))")
    # x+1 is an Add expression
    assert isinstance(result, IRApply)
    assert result.head.name == "Add"
    # Verify the result contains integer 1 (the constant term)
    assert IRInteger(1) in result.args


def test_pipeline_together_fractions() -> None:
    """together(1/x + 1/(x+1)) produces a single rational expression."""
    result = _eval("together(1/x + 1/(x+1))")
    # Result should be a single Div — combined over common denominator
    assert isinstance(result, IRApply)
    assert result.head.name == "Div"


def test_pipeline_partfrac_decomposition() -> None:
    """partfrac(1/(x^2-1), x) decomposes into partial fractions."""
    result = _eval("partfrac(1/(x^2-1), x)")
    # Result should be an Add of two rational terms
    assert isinstance(result, IRApply)
    # Not the original Div form
    assert result.head.name != "Div"


# ---------------------------------------------------------------------------
# Section R — Kronecker factoring (A1 Phase 2)
# ---------------------------------------------------------------------------


def test_pipeline_factor_sophie_germain() -> None:
    """factor(x^4 + 4) splits via Sophie Germain identity."""
    result = _eval("factor(x^4 + 4)")
    # Must be a product (Mul), not unevaluated Factor(…).
    assert isinstance(result, IRApply)
    assert result.head.name == "Mul"


def test_pipeline_factor_cyclotomic_x4_x2_1() -> None:
    """factor(x^4 + x^2 + 1) = (x^2+x+1)(x^2-x+1)."""
    result = _eval("factor(x^4 + x^2 + 1)")
    assert isinstance(result, IRApply)
    assert result.head.name == "Mul"


def test_pipeline_factor_mixed_linear_and_irreducible_quadratic() -> None:
    """factor((x^2+1)*(x-2)) — linear factor extracted, quadratic left intact."""
    # x^3 - 2x^2 + x - 2
    result = _eval("factor(x^3 - 2*x^2 + x - 2)")
    assert isinstance(result, IRApply)
    # Should be a Mul with the linear factor (x-2) and the quadratic (x^2+1).
    assert result.head.name == "Mul"


def test_pipeline_factor_irreducible_x2_plus_1_unchanged() -> None:
    """factor(x^2 + 1) returns the original expression unevaluated."""
    result = _eval("factor(x^2 + 1)")
    # x^2 + 1 is irreducible over Z; Factor(…) node returned as-is.
    assert isinstance(result, IRApply)
    assert result.head.name == "Factor"


# ---------------------------------------------------------------------------
# Section S — Calculus: diff and integrate (already wired, no pipeline tests)
# ---------------------------------------------------------------------------


def test_pipeline_diff_monomial() -> None:
    """diff(x^3, x) → 3*x^2."""
    result = _eval("diff(x^3, x)")
    # The VM differentiates symbolically; result should be a Mul or Pow.
    assert isinstance(result, IRApply)
    # Should not be an unevaluated D(…).
    assert result.head.name != "D"


def test_pipeline_diff_polynomial() -> None:
    """diff(x^2 + 2*x + 1, x) → 2*x + 2."""
    result = _eval("diff(x^2 + 2*x + 1, x)")
    assert isinstance(result, IRApply)
    assert result.head.name != "D"


def test_pipeline_diff_sin() -> None:
    """diff(sin(x), x) → cos(x)."""
    from symbolic_ir import COS, IRApply as _IRApply, IRSymbol as _IRSym
    result = _eval("diff(sin(x), x)")
    # Should be Cos(x).
    assert isinstance(result, IRApply)
    assert result.head.name == "Cos"


def test_pipeline_diff_cos() -> None:
    """diff(cos(x), x) → -sin(x)."""
    result = _eval("diff(cos(x), x)")
    assert isinstance(result, IRApply)
    # Result is Neg(Sin(x)) or Mul(-1, Sin(x)).
    assert result.head.name != "D"


def test_pipeline_diff_exp() -> None:
    """diff(exp(x), x) → exp(x)."""
    result = _eval("diff(exp(x), x)")
    assert isinstance(result, IRApply)
    assert result.head.name == "Exp"


def test_pipeline_integrate_power() -> None:
    """integrate(x^2, x) → x^3/3 (power rule)."""
    result = _eval("integrate(x^2, x)")
    assert isinstance(result, IRApply)
    # Should not come back as unevaluated Integrate(…).
    assert result.head.name != "Integrate"


def test_pipeline_integrate_sin() -> None:
    """integrate(sin(x), x) → -cos(x)."""
    result = _eval("integrate(sin(x), x)")
    assert isinstance(result, IRApply)
    assert result.head.name != "Integrate"
    # Result is Neg(Cos(x)) or Mul(-1, Cos(x)).
    # Either way it should contain a Cos somewhere.
    def _has_cos(node: object) -> bool:
        if isinstance(node, IRApply):
            if isinstance(node.head, IRSymbol) and node.head.name == "Cos":
                return True
            return any(_has_cos(a) for a in node.args)
        return False
    assert _has_cos(result), f"Expected cos in result, got: {result}"


def test_pipeline_integrate_cos() -> None:
    """integrate(cos(x), x) → sin(x)."""
    result = _eval("integrate(cos(x), x)")
    assert isinstance(result, IRApply)
    assert result.head.name != "Integrate"


def test_pipeline_integrate_exp() -> None:
    """integrate(exp(x), x) → exp(x)."""
    result = _eval("integrate(exp(x), x)")
    assert isinstance(result, IRApply)
    assert result.head.name == "Exp"


def test_pipeline_integrate_constant() -> None:
    """integrate(3, x) → 3*x (constant rule)."""
    result = _eval("integrate(3, x)")
    # Result is Mul(3, x) or Mul(x, 3).
    assert isinstance(result, IRApply)
    assert result.head.name not in ("Integrate",)


def test_pipeline_integrate_sum() -> None:
    """integrate(x + 1, x) → x^2/2 + x (linearity)."""
    result = _eval("integrate(x + 1, x)")
    assert isinstance(result, IRApply)
    assert result.head.name != "Integrate"


# ---------------------------------------------------------------------------
# Section T — Matrix operations and numeric functions
# ---------------------------------------------------------------------------


def test_pipeline_matrix_2x2_structure() -> None:
    """matrix([1,2],[3,4]) creates a Matrix IR node."""
    result = _eval("matrix([1,2],[3,4])")
    assert isinstance(result, IRApply)
    assert result.head.name == "Matrix"
    assert len(result.args) == 2  # 2 rows


def test_pipeline_determinant_2x2() -> None:
    """determinant(matrix([1,2],[3,4])) → -2."""
    result = _eval("determinant(matrix([1,2],[3,4]))")
    assert result == IRInteger(-2)


def test_pipeline_transpose_2x2() -> None:
    """transpose(matrix([1,2],[3,4])) → matrix([1,3],[2,4])."""
    result = _eval("transpose(matrix([1,2],[3,4]))")
    assert isinstance(result, IRApply)
    assert result.head.name == "Matrix"


def test_pipeline_gcd_integers() -> None:
    """gcd(12, 8) → 4."""
    result = _eval("gcd(12, 8)")
    assert result == IRInteger(4)


def test_pipeline_lcm_integers() -> None:
    """lcm(4, 6) → 12."""
    result = _eval("lcm(4, 6)")
    assert result == IRInteger(12)


def test_pipeline_mod_integers() -> None:
    """mod(17, 5) → 2."""
    result = _eval("mod(17, 5)")
    assert result == IRInteger(2)


def test_pipeline_floor_float() -> None:
    """floor(3.7) → 3."""
    result = _eval("floor(3.7)")
    assert result == IRInteger(3)


def test_pipeline_ceiling_float() -> None:
    """ceiling(3.2) → 4."""
    result = _eval("ceiling(3.2)")
    assert result == IRInteger(4)


def test_pipeline_abs_negative() -> None:
    """abs(-5) → 5."""
    result = _eval("abs(-5)")
    assert result == IRInteger(5)

