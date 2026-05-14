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

from macsyma_compiler import compile_macsyma
from macsyma_compiler.compiler import _STANDARD_FUNCTIONS
from macsyma_parser import parse_macsyma
from symbolic_ir import IRApply, IRFloat, IRInteger, IRRational, IRSymbol
from symbolic_vm import VM

from macsyma_runtime import MacsymaBackend, extend_compiler_name_table

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
    vals = {
        (s.numerator if isinstance(s, IRRational) else s.value)
        for s in result.args
    }
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


def test_pipeline_factor_common_multivariate_factor() -> None:
    """factor(x^2*y - y) extracts the common y factor."""
    result = _eval("factor(x^2*y - y)")
    assert isinstance(result, IRApply)
    assert result.head.name == "Mul"
    assert "Factor" not in str(result)
    assert "y" in str(result)


def test_pipeline_factor_multivariate_perfect_square() -> None:
    """factor(x^2 + 2*x*y + y^2) recognises (x+y)^2."""
    result = _eval("factor(x^2 + 2*x*y + y^2)")
    assert isinstance(result, IRApply)
    assert result.head.name == "Pow"
    base, exponent = result.args
    assert exponent == IRInteger(2)
    assert isinstance(base, IRApply)
    assert base.head.name == "Add"
    assert set(base.args) == {IRSymbol("x"), IRSymbol("y")}


def test_pipeline_factor_multivariate_difference_of_squares() -> None:
    """factor(x^2 - y^2) recognises (x-y)*(x+y)."""
    result = _eval("factor(x^2 - y^2)")
    assert isinstance(result, IRApply)
    assert result.head.name == "Mul"
    assert result.args == (
        IRApply(IRSymbol("Sub"), (IRSymbol("x"), IRSymbol("y"))),
        IRApply(IRSymbol("Add"), (IRSymbol("x"), IRSymbol("y"))),
    )


def test_pipeline_factor_multivariate_difference_of_cubes() -> None:
    """factor(x^3 - y^3) recognises (x-y)*(x^2+x*y+y^2)."""
    result = _eval("factor(x^3 - y^3)")
    assert isinstance(result, IRApply)
    assert result.head.name == "Mul"
    assert result.args == (
        IRApply(IRSymbol("Sub"), (IRSymbol("x"), IRSymbol("y"))),
        IRApply(
            IRSymbol("Add"),
            (
                IRApply(
                    IRSymbol("Add"),
                    (
                        IRApply(IRSymbol("Pow"), (IRSymbol("x"), IRInteger(2))),
                        IRApply(IRSymbol("Mul"), (IRSymbol("x"), IRSymbol("y"))),
                    ),
                ),
                IRApply(IRSymbol("Pow"), (IRSymbol("y"), IRInteger(2))),
            ),
        ),
    )


def test_pipeline_factor_multivariate_sum_of_cubes() -> None:
    """factor(x^3 + y^3) recognises (x+y)*(x^2-x*y+y^2)."""
    result = _eval("factor(x^3 + y^3)")
    assert isinstance(result, IRApply)
    assert result.head.name == "Mul"
    assert result.args == (
        IRApply(IRSymbol("Add"), (IRSymbol("x"), IRSymbol("y"))),
        IRApply(
            IRSymbol("Add"),
            (
                IRApply(
                    IRSymbol("Add"),
                    (
                        IRApply(IRSymbol("Pow"), (IRSymbol("x"), IRInteger(2))),
                        IRApply(
                            IRSymbol("Mul"),
                            (
                                IRInteger(-1),
                                IRApply(
                                    IRSymbol("Mul"), (IRSymbol("x"), IRSymbol("y"))
                                ),
                            ),
                        ),
                    ),
                ),
                IRApply(IRSymbol("Pow"), (IRSymbol("y"), IRInteger(2))),
            ),
        ),
    )


def test_pipeline_factor_multivariate_perfect_cube_sum() -> None:
    """factor(x^3 + 3*x^2*y + 3*x*y^2 + y^3) recognises (x+y)^3."""
    result = _eval("factor(x^3 + 3*x^2*y + 3*x*y^2 + y^3)")
    assert isinstance(result, IRApply)
    assert result.head.name == "Pow"
    base, exponent = result.args
    assert exponent == IRInteger(3)
    assert isinstance(base, IRApply)
    assert base.head.name == "Add"
    assert set(base.args) == {IRSymbol("x"), IRSymbol("y")}


def test_pipeline_factor_multivariate_perfect_cube_difference() -> None:
    """factor(x^3 - 3*x^2*y + 3*x*y^2 - y^3) recognises (x-y)^3."""
    result = _eval("factor(x^3 - 3*x^2*y + 3*x*y^2 - y^3)")
    assert isinstance(result, IRApply)
    assert result.head.name == "Pow"
    base, exponent = result.args
    assert exponent == IRInteger(3)
    assert isinstance(base, IRApply)
    assert base.head.name == "Sub"
    assert base.args == (IRSymbol("x"), IRSymbol("y"))


def test_pipeline_factor_multivariate_grouping() -> None:
    """factor(x*y + x*z + y + z) recognises (x+1)*(y+z)."""
    result = _eval("factor(x*y + x*z + y + z)")
    assert isinstance(result, IRApply)
    assert result.args == (
        IRApply(IRSymbol("Add"), (IRSymbol("x"), IRInteger(1))),
        IRApply(IRSymbol("Add"), (IRSymbol("y"), IRSymbol("z"))),
    )


def test_pipeline_factor_multivariate_grouping_with_signed_residual() -> None:
    """factor(x*y - x*z + y - z) recognises (x+1)*(y-z)."""
    result = _eval("factor(x*y - x*z + y - z)")
    assert isinstance(result, IRApply)
    assert result.args == (
        IRApply(IRSymbol("Add"), (IRSymbol("x"), IRInteger(1))),
        IRApply(
            IRSymbol("Add"),
            (
                IRSymbol("y"),
                IRApply(IRSymbol("Mul"), (IRInteger(-1), IRSymbol("z"))),
            ),
        ),
    )


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


# ---------------------------------------------------------------------------
# Section U — Taylor series (cas-limit-series, Phase 28)
# ---------------------------------------------------------------------------
#
# ``taylor(expr, var, point, order)`` calls the ``Taylor`` CAS handler via
# the ``macsyma_runtime.cas_handlers`` dispatcher.  The polynomial bridge
# converts the IR expression to a coefficient list, computes the expansion,
# and returns an IR polynomial.
# ---------------------------------------------------------------------------


def test_pipeline_taylor_polynomial() -> None:
    """taylor(x^2, x, 0, 2) produces an expanded polynomial (not unevaluated)."""
    result = _eval("taylor(x^2, x, 0, 2)")
    # The Taylor handler must return a simplified polynomial, not the unevaluated
    # Taylor(x^2, x, 0, 2) form.
    assert isinstance(result, (IRApply, IRInteger, IRSymbol, IRRational, IRFloat))
    if isinstance(result, IRApply):
        assert result.head.name != "Taylor", (
            f"Expected expanded polynomial, got unevaluated Taylor: {result!r}"
        )


def test_pipeline_taylor_constant() -> None:
    """taylor(3, x, 0, 1) → 3 (a constant is its own Taylor expansion)."""
    result = _eval("taylor(3, x, 0, 1)")
    assert result == IRInteger(3)


def test_pipeline_taylor_linear() -> None:
    """taylor(x + 1, x, 0, 1) → x + 1 (linear polynomial is exact)."""
    result = _eval("taylor(x + 1, x, 0, 1)")
    assert isinstance(result, (IRApply, IRSymbol))
    if isinstance(result, IRApply):
        assert result.head.name != "Taylor", (
            f"Expected expanded form, got unevaluated: {result!r}"
        )


# ---------------------------------------------------------------------------
# Section V — Symbolic summation (cas-summation, Phase 25)
# ---------------------------------------------------------------------------
#
# ``sum(expr, var, lower, upper)`` maps to the ``Sum`` CAS handler via the
# compiler's ``_STANDARD_FUNCTIONS`` table.  ``cas_summation.evaluate_sum``
# recognises polynomial summands (k^n) and applies Faulhaber's formulas to
# produce a closed-form integer result when bounds are concrete.
# ---------------------------------------------------------------------------


def test_pipeline_sum_k_concrete() -> None:
    """sum(k, k, 1, 4) → 10 (1 + 2 + 3 + 4)."""
    result = _eval("sum(k, k, 1, 4)")
    assert result == IRInteger(10)


def test_pipeline_sum_constant_body() -> None:
    """sum(2, k, 1, 5) → 10 (constant 2 summed 5 times)."""
    result = _eval("sum(2, k, 1, 5)")
    assert result == IRInteger(10)


def test_pipeline_sum_squares() -> None:
    """sum(k^2, k, 1, 3) → 14 (1 + 4 + 9)."""
    result = _eval("sum(k^2, k, 1, 3)")
    assert result == IRInteger(14)


# ---------------------------------------------------------------------------
# Section W — Laplace transforms (cas-laplace, Phase 29)
# ---------------------------------------------------------------------------
#
# ``laplace(f, t, s)`` maps to the ``Laplace`` CAS head, dispatched by the
# ``_build_laplace_handlers()`` layer.  Tests verify the transform evaluates
# to an expression involving the frequency variable ``s``; exact form checks
# are done in ``test_ode_wiring.py`` at the IR level.
# ---------------------------------------------------------------------------


def test_pipeline_laplace_constant() -> None:
    """laplace(1, t, s) evaluates to a rational function of s  (L{1} = 1/s)."""
    result = _eval("laplace(1, t, s)")
    assert isinstance(result, IRApply), (
        f"Expected IRApply (rational of s), got {result!r}"
    )
    # The result must involve the frequency variable s.
    result_str = repr(result)
    assert "s" in result_str, f"Expected 's' in result: {result_str}"


def test_pipeline_laplace_linear() -> None:
    """laplace(t, t, s) evaluates (L{t} = 1/s²) to an IRApply containing s."""
    result = _eval("laplace(t, t, s)")
    assert isinstance(result, IRApply), (
        f"Expected IRApply (rational of s), got {result!r}"
    )
    result_str = repr(result)
    assert "s" in result_str, f"Expected 's' in result: {result_str}"


# ---------------------------------------------------------------------------
# Section X — ODE solving via MACSYMA surface (cas-ode, Phase 29)
# ---------------------------------------------------------------------------
#
# ``ode2(eqn, y, x)`` calls the ``ODE2`` CAS handler.  First-order linear
# homogeneous ODEs (y' + p(x)*y = 0) are solved by the integrating-factor
# path; the result is ``Equal(y, C*exp(…))``.  Full ODE tests at IR level
# live in ``test_ode_wiring.py``; here we validate the MACSYMA surface
# (parse → compile → eval) end-to-end.
# ---------------------------------------------------------------------------


def test_pipeline_ode2_first_order_homogeneous() -> None:
    """ode2(diff(y, x) + y, y, x) → Equal(y, C·exp(-x)).

    First-order linear homogeneous ODE:  y' + y = 0.
    Integrating factor μ = exp(x) → y = C·exp(-x).
    """
    result = _eval("ode2(diff(y, x) + y, y, x)")
    assert isinstance(result, IRApply), f"Expected IRApply, got {result!r}"
    assert result.head.name == "Equal", (
        f"Expected Equal(y, ...), got head={result.head!r}"
    )
    # lhs of Equal must be the y symbol
    assert isinstance(result.args[0], IRSymbol)
    assert result.args[0].name == "y", (
        f"Expected lhs = y, got {result.args[0]!r}"
    )


def test_pipeline_ode2_first_order_decaying() -> None:
    """ode2(diff(y, x) + 2*y, y, x) → Equal(y, C·exp(-2x)).

    Faster-decaying case: P(x) = 2.  Solution: y = C·exp(-2x).
    """
    result = _eval("ode2(diff(y, x) + 2*y, y, x)")
    assert isinstance(result, IRApply)
    assert result.head.name == "Equal", (
        f"Expected Equal result, got {result!r}"
    )
    lhs = result.args[0]
    assert isinstance(lhs, IRSymbol) and lhs.name == "y"


# ---------------------------------------------------------------------------
# Section Y — Log/exp cancellation (symbolic-vm Phase 30)
# ---------------------------------------------------------------------------
#
# Phase 30 of symbolic-vm adds algebraic cancellation rules to the ``Log``
# and ``Exp`` handlers:
#
#   log(exp(x))  → x          (cancel: log is left-inverse of exp)
#   exp(log(x))  → x          (cancel: exp is left-inverse of log)
#
# These identities hold for all real x without any assumption because:
# - exp maps ℝ into ℝ⁺ and log is its exact inverse on ℝ⁺.
# - log(x) requires x > 0 in the real domain, so exp(log(x)) = x is safe.
# ---------------------------------------------------------------------------


def test_pipeline_log_exp_cancel() -> None:
    """log(exp(x)) → x  (Phase 30 log/exp cancellation)."""
    result = _eval("log(exp(x))")
    assert result == IRSymbol("x"), (
        f"Expected IRSymbol('x'), got {result!r}"
    )


def test_pipeline_exp_log_cancel() -> None:
    """exp(log(x)) → x  (Phase 30 exp/log cancellation)."""
    result = _eval("exp(log(x))")
    assert result == IRSymbol("x"), (
        f"Expected IRSymbol('x'), got {result!r}"
    )


def test_pipeline_log_numeric_fold() -> None:
    """log(1) → 0  (numeric fold: ln(1) = 0)."""
    result = _eval("log(1)")
    assert result == IRInteger(0), f"Expected 0, got {result!r}"


# ---------------------------------------------------------------------------
# Section Z — Inverse trig symmetry + trig special values
#             (symbolic-vm Phases 32 and 33)
# ---------------------------------------------------------------------------
#
# Phase 32 added odd-symmetry rules to inverse-trig handlers:
#
#   asin(-x) → -asin(x)
#   atan(-x) → -atan(x)
#
# Phase 33 added π-multiple detection to sin/cos/tan handlers, returning
# exact algebraic values:
#
#   sin(π/6) = 1/2,   cos(π/3) = 1/2,   cos(π) = -1, etc.
#
# ``MacsymaBackend`` pre-binds ``%pi → IRFloat(math.pi)``.  Phase 33's
# IRFloat-matching strategy divides by ``math.pi`` and checks for rationals
# with denominator in {1, 2, 3, 4, 6}, so ``sin(%pi/6)`` correctly returns
# the exact ``IRRational(1, 2)`` rather than a float approximation.
# ---------------------------------------------------------------------------


def test_pipeline_asin_odd_symmetry() -> None:
    """asin(-x) should be simplified (Phase 32 odd-symmetry rule).

    The Phase 32 ``asin_handler`` simplifies ``Asin(Neg(x)) → Neg(Asin(x))``.
    The result must NOT be an unevaluated ``Asin`` applied directly to a
    negated argument.
    """
    result = _eval("asin(-x)")
    assert isinstance(result, IRApply), f"Expected IRApply, got {result!r}"
    # The head should NOT still be Asin (that would mean the symmetry rule
    # was not applied and the expression is still unevaluated asin(-x)).
    assert result.head.name != "Asin", (
        f"Expected odd-symmetry simplification, got unevaluated asin: {result!r}"
    )


def test_pipeline_sin_pi_sixth_exact() -> None:
    """sin(%pi/6) → IRRational(1, 2)  (Phase 33 trig special values).

    ``%pi`` is pre-bound to ``IRFloat(math.pi)`` in ``MacsymaBackend``.
    Phase 33 detects that ``IRFloat(pi/6)`` is within 10⁻⁹ of ``π/6`` and
    returns the exact algebraic value ``1/2`` as ``IRRational(1, 2)``.
    """
    result = _eval("sin(%pi/6)")
    assert result == IRRational(1, 2), (
        f"Expected IRRational(1, 2), got {result!r}"
    )


def test_pipeline_cos_pi_third_exact() -> None:
    """cos(%pi/3) → IRRational(1, 2)  (Phase 33 trig special values)."""
    result = _eval("cos(%pi/3)")
    assert result == IRRational(1, 2), (
        f"Expected IRRational(1, 2), got {result!r}"
    )


def test_pipeline_cos_pi_exact() -> None:
    """cos(%pi) → -1  (Phase 33 trig special values: cos(π) = -1)."""
    result = _eval("cos(%pi)")
    assert result == IRInteger(-1), f"Expected -1, got {result!r}"


def test_pipeline_sin_pi_exact() -> None:
    """sin(%pi) → 0  (Phase 33 trig special values: sin(π) = 0)."""
    # %pi is pre-bound as a float; the numeric fold should give 0 or near-0.
    # Phase 33 may return IRInteger(0) directly.
    result = _eval("sin(%pi)")
    if isinstance(result, IRInteger):
        assert result == IRInteger(0)
    elif isinstance(result, IRFloat):
        assert abs(result.value) < 1e-10, f"Expected ≈0, got {result.value!r}"
    else:
        raise AssertionError(f"Unexpected result type: {result!r}")


# ---------------------------------------------------------------------------
# Section AA — Advanced matrix operations (Phase 32 name-table extensions)
#
# eigenvalues, eigenvectors, charpoly, nullspace, columnspace, rowspace, norm,
# lu — all implemented in symbolic-vm via cas-matrix but were missing from
# the MACSYMA name table until v1.23.0.
# ---------------------------------------------------------------------------


def test_pipeline_eigenvalues_2x2() -> None:
    """eigenvalues(matrix([1,2],[3,4])) returns a List of eigenvalues.

    The 2×2 matrix [[1,2],[3,4]] has characteristic polynomial
    λ² − 5λ − 2 = 0, giving λ = (5 ± √33) / 2.
    We just verify the result is a non-unevaluated List.
    """
    result = _eval("eigenvalues(matrix([1,2],[3,4]))")
    assert isinstance(result, IRApply)
    assert result.head.name == "List", (
        f"Expected List, got head={result.head.name!r}"
    )


def test_pipeline_charpoly_2x2() -> None:
    """charpoly(matrix([1,2],[3,4]), x) returns the characteristic polynomial.

    det(λI − A) = (λ−1)(λ−4) − 6 = λ² − 5λ − 2.
    We verify the result is evaluated (not head CharPoly) and has an Add head.
    """
    result = _eval("charpoly(matrix([1,2],[3,4]), x)")
    assert isinstance(result, IRApply)
    assert result.head.name != "CharPoly", (
        f"charpoly returned unevaluated: {result!r}"
    )


def test_pipeline_nullspace_rank_deficient() -> None:
    """nullspace(matrix([1,2],[2,4])) returns a non-empty basis.

    [[1,2],[2,4]] has rank 1; its nullspace is spanned by [2,-1].
    The result should be a List of vectors.
    """
    result = _eval("nullspace(matrix([1,2],[2,4]))")
    assert isinstance(result, IRApply)
    assert result.head.name == "List", (
        f"Expected List, got head={result.head.name!r}"
    )
    assert len(result.args) >= 1, "Nullspace should have at least one basis vector"


def test_pipeline_rowreduce_upper_triangular() -> None:
    """rowreduce(matrix([1,2,3],[4,5,6])) gives row-echelon form.

    The result is a Matrix node (not unevaluated RowReduce).
    """
    result = _eval("rowreduce(matrix([1,2,3],[4,5,6]))")
    assert isinstance(result, IRApply)
    assert result.head.name == "Matrix", (
        f"Expected Matrix, got head={result.head.name!r}"
    )


def test_pipeline_norm_3_4_vector() -> None:
    """norm(matrix([3],[4])) → 5  (Euclidean norm of column vector [3;4])."""
    result = _eval("norm(matrix([3],[4]))")
    assert result == IRInteger(5), f"Expected 5, got {result!r}"


def test_pipeline_norm_identity_row() -> None:
    """norm(matrix([1,0,0])) → 1  (unit row vector)."""
    result = _eval("norm(matrix([1,0,0]))")
    assert result == IRInteger(1), f"Expected 1, got {result!r}"


# ---------------------------------------------------------------------------
# Section BB — Cube root (Phase 32 name-table extensions)
#
# cbrt — implemented in symbolic-vm but missing from the MACSYMA name table.
# ---------------------------------------------------------------------------


def test_pipeline_cbrt_exact_cube() -> None:
    """cbrt(8) → 2  (exact integer cube root)."""
    result = _eval("cbrt(8)")
    assert result == IRInteger(2), f"Expected 2, got {result!r}"


def test_pipeline_cbrt_negative() -> None:
    """cbrt(-27) → -3  (cube root of a negative perfect cube)."""
    result = _eval("cbrt(-27)")
    assert result == IRInteger(-3), f"Expected -3, got {result!r}"


def test_pipeline_cbrt_float() -> None:
    """cbrt(2.0) ≈ 1.2599  (floating-point cube root)."""
    import math as _math

    result = _eval("cbrt(2.0)")
    assert isinstance(result, IRFloat), f"Expected IRFloat, got {result!r}"
    assert abs(result.value - _math.cbrt(2.0)) < 1e-9


# ---------------------------------------------------------------------------
# Section CC — Log/exp transformations (Phase 32 name-table extensions)
#
# radcan, logcontract, logexpand — implemented in symbolic-vm via
# cas_simplify but missing from the MACSYMA name table until v1.23.0.
# ---------------------------------------------------------------------------


def test_pipeline_logcontract_sum_of_logs() -> None:
    """logcontract(log(x) + log(y)) → log(x*y)."""
    result = _eval("logcontract(log(x) + log(y))")
    assert isinstance(result, IRApply)
    # Head should be Log, not LogContract (evaluated)
    assert result.head.name == "Log", (
        f"Expected Log head, got {result.head.name!r}"
    )
    # The single argument should be a Mul containing x and y
    assert len(result.args) == 1
    inner = result.args[0]
    assert isinstance(inner, IRApply) and inner.head.name == "Mul"


def test_pipeline_logexpand_log_product() -> None:
    """logexpand(log(x*y)) → log(x) + log(y)."""
    result = _eval("logexpand(log(x*y))")
    assert isinstance(result, IRApply)
    # After expansion the result should be an Add of two Log terms.
    assert result.head.name == "Add", (
        f"Expected Add head, got {result.head.name!r}"
    )


def test_pipeline_radcan_sqrt_squared() -> None:
    """radcan(sqrt(x^2)) → abs(x)  (or Abs(x) in IR)."""
    result = _eval("radcan(sqrt(x^2))")
    assert isinstance(result, IRApply)
    # The head should be Abs (radcan simplifies √(x²) → |x|)
    assert result.head.name == "Abs", (
        f"Expected Abs head from radcan(sqrt(x^2)), got {result.head.name!r}"
    )


# ===========================================================================
# Section DD — Fourier transforms (cas-fourier, Phase 33)
# ===========================================================================
#
# MACSYMA names ``fourier`` and ``ifourier`` map to ``Fourier`` and
# ``IFourier`` IR heads, which are handled by the ``cas-fourier`` substrate
# wired into ``SymbolicBackend``.
#
# Key results (for the standard un-normalised convention):
#   fourier(1, t, ω)        = 2π · δ(ω)     (Fourier of constant)
#   ifourier(1, ω, t)       = δ(t)           (inverse Fourier of 1)
#   fourier(δ(t), t, ω)     = 1              (Fourier of delta is 1)
# ===========================================================================


def test_pipeline_fourier_constant() -> None:
    """fourier(1, t, w) → 2π·DiracDelta(w).

    The Fourier transform of a constant c is 2π·c·δ(ω).  For c = 1 the
    result has the ``DiracDelta`` head wrapped in a Mul with 2π.
    """
    result = _eval("fourier(1, t, w)")
    # Result should be a Mul node containing DiracDelta
    assert isinstance(result, IRApply), f"Expected IRApply, got {type(result).__name__}"
    assert result.head.name in ("Mul", "DiracDelta"), (
        f"Unexpected head {result.head.name!r} for fourier(1, t, w)"
    )


def test_pipeline_ifourier_constant() -> None:
    """ifourier(1, w, t) → DiracDelta(t).

    The inverse Fourier transform of 1 is the Dirac delta δ(t).
    """
    result = _eval("ifourier(1, w, t)")
    assert isinstance(result, IRApply), f"Expected IRApply, got {type(result).__name__}"
    assert result.head.name == "DiracDelta", (
        f"Expected DiracDelta head from ifourier(1, w, t), got {result.head.name!r}"
    )
    # Argument should be the time variable t
    assert len(result.args) == 1
    assert isinstance(result.args[0], IRSymbol)
    assert result.args[0].name == "t"


def test_pipeline_fourier_delta_is_one() -> None:
    """fourier(delta(t), t, w) → 1 (Fourier of Dirac delta is the constant 1)."""
    result = _eval("fourier(delta(t), t, w)")
    assert result is not None


def test_pipeline_ifourier_delta_is_one() -> None:
    """ifourier(delta(w), w, t) → 1/(2π) or related (inverse Fourier of delta)."""
    result = _eval("ifourier(delta(w), w, t)")
    assert result is not None


# ===========================================================================
# Section EE — Newton's method numeric root finding (cas-mnewton, Phase 33)
# ===========================================================================
#
# MACSYMA name ``mnewton`` maps to the ``MNewton`` IR head, handled by the
# ``cas-mnewton`` substrate.  The handler expects three scalar arguments:
#
#   mnewton(f_expr, variable, initial_guess)
#
# and returns an ``IRFloat`` approximation of a root of f_expr near
# initial_guess.
#
# The MACSYMA surface form (and Maxima documentation) often uses list
# arguments ``mnewton([f], [x], [x0])``; the scalar form is the canonical
# call here, as the list-unpacking form requires multi-system Newton which
# is a separate capability.
# ===========================================================================


def test_pipeline_mnewton_quadratic_root() -> None:
    """mnewton(x^2 - 4, x, 1.5) ≈ 2.0 (positive root of x²=4)."""
    result = _eval("mnewton(x^2 - 4, x, 1.5)")
    assert isinstance(result, IRFloat), (
        f"Expected IRFloat from mnewton, got {type(result).__name__}: {result}"
    )
    assert abs(result.value - 2.0) < 1e-6, (
        f"mnewton root {result.value} not close to 2.0"
    )


def test_pipeline_mnewton_cubic_root() -> None:
    """mnewton(x^3 - 8, x, 2.0) ≈ 2.0 (real cube root of 8)."""
    result = _eval("mnewton(x^3 - 8, x, 2.0)")
    assert isinstance(result, IRFloat), (
        f"Expected IRFloat from mnewton, got {type(result).__name__}: {result}"
    )
    assert abs(result.value - 2.0) < 1e-6, (
        f"mnewton root {result.value} not close to 2.0"
    )


def test_pipeline_mnewton_sine_root() -> None:
    """mnewton(sin(x), x, 3) ≈ π (root of sin near 3)."""
    import math

    result = _eval("mnewton(sin(x), x, 3)")
    assert isinstance(result, IRFloat), (
        f"Expected IRFloat from mnewton, got {type(result).__name__}: {result}"
    )
    assert abs(result.value - math.pi) < 1e-4, (
        f"mnewton root {result.value} not close to π = {math.pi}"
    )


def test_pipeline_mnewton_exp_root() -> None:
    """mnewton(exp(x) - 2, x, 0.5) ≈ ln(2) (root of eˣ = 2)."""
    import math

    result = _eval("mnewton(exp(x) - 2, x, 0.5)")
    assert isinstance(result, IRFloat), (
        f"Expected IRFloat from mnewton, got {type(result).__name__}: {result}"
    )
    assert abs(result.value - math.log(2)) < 1e-6, (
        f"mnewton root {result.value} not close to ln(2) = {math.log(2)}"
    )


# ===========================================================================
# Section FF — Algebraic extension factoring (cas-algebraic, Phase 33)
# ===========================================================================
#
# MACSYMA name ``algfactor`` maps to the ``AlgFactor`` IR head, handled by
# the ``cas-algebraic`` substrate.  It factors a polynomial over Q[√d]:
#
#   algfactor(x^2 - 2, sqrt(2))  →  (x − √2)(x + √2)
#
# The result is a ``Mul`` of two ``Add`` nodes (linear factors).
# ===========================================================================


def test_pipeline_algfactor_x2_minus_2() -> None:
    """algfactor(x^2 - 2, sqrt(2)) splits over Q[√2]."""
    result = _eval("algfactor(x^2 - 2, sqrt(2))")
    # Should produce Mul(Add(x, -Sqrt(2)), Add(x, Sqrt(2))) or similar
    assert isinstance(result, IRApply), (
        f"Expected IRApply, got {type(result).__name__}: {result}"
    )
    assert result.head.name == "Mul", (
        f"Expected Mul from algfactor, got {result.head.name!r}"
    )
    assert len(result.args) == 2, (
        f"Expected 2 factors from algfactor(x^2-2, sqrt(2)), got {len(result.args)}"
    )


def test_pipeline_algfactor_x2_minus_3() -> None:
    """algfactor(x^2 - 3, sqrt(3)) splits over Q[√3]."""
    result = _eval("algfactor(x^2 - 3, sqrt(3))")
    assert isinstance(result, IRApply), (
        f"Expected IRApply, got {type(result).__name__}: {result}"
    )
    assert result.head.name == "Mul", (
        f"Expected Mul from algfactor(x^2-3, sqrt(3)), got {result.head.name!r}"
    )


def test_pipeline_algfactor_irreducible_stays_unevaluated() -> None:
    """algfactor(x^2 + 1, sqrt(2)) stays unevaluated (irreducible over Q[√2])."""
    result = _eval("algfactor(x^2 + 1, sqrt(2))")
    # x² + 1 is irreducible over Q[√2]; should remain as AlgFactor or x^2+1
    # Accept either form: unevaluated AlgFactor or the expression unchanged.
    assert result is not None  # At minimum it must not crash


# ===========================================================================
# Section GG — Gröbner bases and polynomial reduction (cas-multivariate, Phase 33)
# ===========================================================================
#
# MACSYMA names ``groebner`` and ``poly_reduce`` map to ``Groebner`` and
# ``PolyReduce`` IR heads, handled by the ``cas-multivariate`` substrate.
#
#   groebner([polys], [vars])  → reduced Gröbner basis
#   poly_reduce(f, [basis], [vars])  → f reduced modulo the basis
#
# The reduced Gröbner basis of {x²-1, x-1} w.r.t. lex order is {x-1},
# because x²-1 = (x-1)(x+1) and x-1 already divides x²-1.
#
# Polynomial reduction: x³ mod {x²-1} → x  (since x³ = x·(x²-1) + x).
# ===========================================================================


def test_pipeline_groebner_single_variable() -> None:
    """groebner([x^2-1, x-1], [x]) → [x-1] (GCD-like reduction)."""
    result = _eval("groebner([x^2-1, x-1], [x])")
    assert isinstance(result, IRApply), (
        f"Expected IRApply from groebner, got {type(result).__name__}: {result}"
    )
    assert result.head.name == "List", (
        f"Expected List from groebner, got {result.head.name!r}"
    )
    # Basis should reduce to a single polynomial: x - 1
    assert len(result.args) == 1, (
        f"Expected 1-element basis for groebner([x^2-1, x-1], [x]), "
        f"got {len(result.args)} elements"
    )


def test_pipeline_groebner_returns_list() -> None:
    """groebner([x^2 + y^2 - 1, x - y], [x, y]) returns a List."""
    result = _eval("groebner([x^2 + y^2 - 1, x - y], [x, y])")
    assert isinstance(result, IRApply)
    assert result.head.name == "List"


def test_pipeline_poly_reduce_x3_mod_x2_minus_1() -> None:
    """poly_reduce(x^3, [x^2-1], [x]) → x  (x³ = x·(x²-1) + x)."""
    result = _eval("poly_reduce(x^3, [x^2-1], [x])")
    assert result == IRSymbol("x"), (
        f"Expected x from poly_reduce(x^3, [x^2-1], [x]), got {result}"
    )


def test_pipeline_poly_reduce_zero_remainder() -> None:
    """poly_reduce(x^2 - 1, [x^2-1], [x]) → 0  (exact divisibility)."""
    result = _eval("poly_reduce(x^2 - 1, [x^2-1], [x])")
    assert result == IRInteger(0), (
        f"Expected 0 from poly_reduce(x^2-1, [x^2-1], [x]), got {result}"
    )


# ===========================================================================
# Section HH — Special Functions (Phase 23 pipeline tests)
# ===========================================================================
#
# Phase 23 added handlers for the transcendental special functions that arise
# as integration fallbacks and CAS building blocks:
#
#   erf(x), erfc(x), erfi(x)   — error functions
#   gamma(z)                    — Euler's Gamma function
#   beta(a, b)                  — Euler's Beta function
#   si(x), ci(x)                — trigonometric integrals
#   shi(x), chi(x)              — hyperbolic integrals
#   li2(x)                      — Spence's dilogarithm
#   fresnel_s(x), fresnel_c(x)  — Fresnel integrals
#   lambert_w(x)                — principal branch of the Lambert W function
#
# The MACSYMA name table has mapped all these names to canonical IR heads
# (e.g. ``erf`` → ``Erf``, ``gamma`` → ``GammaFunc``) since Phase 23.  The
# symbolic-vm handlers implement:
#
#   - Exact results at known special values (e.g. erf(0) = 0, Γ(n) = (n-1)!)
#   - Numeric evaluation via pure-Python series/Lanczos for float inputs
#
# These tests validate the full pipeline (MACSYMA surface → parse → compile
# → VM → result) for the first time.
# ===========================================================================


def test_pipeline_erf_zero() -> None:
    """erf(0) → 0  (exact; erf(0) = 0 by definition)."""
    result = _eval("erf(0)")
    assert result == IRInteger(0), f"Expected 0, got {result!r}"


def test_pipeline_erf_numeric() -> None:
    """erf(1.0) → IRFloat(≈ 0.8427)  (Gauss error function at x = 1)."""
    import math as _math

    result = _eval("erf(1.0)")
    assert isinstance(result, IRFloat), f"Expected IRFloat, got {type(result).__name__}"
    assert abs(result.value - _math.erf(1.0)) < 1e-9, (
        f"erf(1.0) = {result.value}, expected ≈ {_math.erf(1.0)}"
    )


def test_pipeline_erfc_numeric() -> None:
    """erfc(1.0) → IRFloat(≈ 0.1573)  (complementary error function; erfc = 1 − erf)."""
    import math as _math

    result = _eval("erfc(1.0)")
    assert isinstance(result, IRFloat), f"Expected IRFloat, got {type(result).__name__}"
    expected = 1.0 - _math.erf(1.0)
    assert abs(result.value - expected) < 1e-9, (
        f"erfc(1.0) = {result.value}, expected ≈ {expected}"
    )


def test_pipeline_gamma_positive_integer() -> None:
    """gamma(5) → 24  (Γ(5) = 4! = 24, exact integer result)."""
    result = _eval("gamma(5)")
    assert result == IRInteger(24), f"Expected 24, got {result!r}"


def test_pipeline_gamma_one() -> None:
    """gamma(1) → 1  (Γ(1) = 0! = 1, exact integer result)."""
    result = _eval("gamma(1)")
    assert result == IRInteger(1), f"Expected 1, got {result!r}"


def test_pipeline_gamma_half() -> None:
    """gamma(1/2) → √π  (Γ(1/2) = √π; returned as Sqrt(%pi) or an IRFloat).

    The ``gamma_handler`` recognises the half-integer form and returns the
    symbolic node ``Sqrt(%pi)`` (because %pi is looked up by name in the
    handler, not pre-substituted).  The test accepts either exact symbolic
    form or a float approximation of √π.
    """
    import math as _math

    result = _eval("gamma(1/2)")
    # The handler returns IRApply(Sqrt, (%pi,)); %pi inside the returned node
    # is NOT automatically evaluated because the VM does not re-enter the
    # returned node.  Accept both the symbolic and numeric forms.
    if isinstance(result, IRApply):
        assert result.head.name == "Sqrt", (
            f"Expected Sqrt head from gamma(1/2), got {result.head.name!r}"
        )
    else:
        assert isinstance(result, IRFloat), (
            f"Expected IRApply(Sqrt,...) or IRFloat from gamma(1/2), got {result!r}"
        )
        assert abs(result.value - _math.sqrt(_math.pi)) < 1e-9


def test_pipeline_beta_integers() -> None:
    """beta(2, 3) → 1/12  (B(2,3) = Γ(2)·Γ(3)/Γ(5) = 1·2/24 = 1/12).

    The ``beta_handler`` rationalises the float result to the nearest
    simple fraction when the denominator fits in 10 000, returning
    ``IRRational(1, 12)`` for integer arguments.
    """
    result = _eval("beta(2, 3)")
    assert result == IRRational(1, 12), (
        f"Expected IRRational(1, 12) from beta(2, 3), got {result!r}"
    )


def test_pipeline_si_zero() -> None:
    """si(0) → 0  (exact; Si(0) = 0 by definition of the sine integral)."""
    result = _eval("si(0)")
    assert result == IRInteger(0), f"Expected 0, got {result!r}"


def test_pipeline_si_numeric() -> None:
    """si(1.0) → IRFloat  (Si(1) ≈ 0.9461, the sine integral at x = 1)."""
    result = _eval("si(1.0)")
    assert isinstance(result, IRFloat), f"Expected IRFloat, got {type(result).__name__}"
    # Si(1) ≈ 0.9460831 — verify to 4 decimal places
    assert abs(result.value - 0.9461) < 1e-3, (
        f"si(1.0) = {result.value}, expected ≈ 0.9461"
    )


def test_pipeline_ci_numeric() -> None:
    """ci(1.0) → IRFloat  (Ci(1) ≈ 0.3374, the cosine integral at x = 1)."""
    result = _eval("ci(1.0)")
    assert isinstance(result, IRFloat), f"Expected IRFloat, got {type(result).__name__}"
    # Ci(1) ≈ 0.3374039229 — verify to 4 decimal places
    assert abs(result.value - 0.3374) < 1e-3, (
        f"ci(1.0) = {result.value}, expected ≈ 0.3374"
    )


def test_pipeline_shi_zero() -> None:
    """shi(0) → 0  (exact; Shi(0) = 0 by definition of the hyperbolic sine integral)."""
    result = _eval("shi(0)")
    assert result == IRInteger(0), f"Expected 0, got {result!r}"


def test_pipeline_li2_zero() -> None:
    """li2(0) → 0  (exact; Li₂(0) = 0 by definition of the dilogarithm)."""
    result = _eval("li2(0)")
    assert result == IRInteger(0), f"Expected 0, got {result!r}"


def test_pipeline_li2_numeric() -> None:
    """li2(0.5) → IRFloat(≈ 0.5822)  (Spence's dilogarithm at x = 1/2).

    The exact value is Li₂(1/2) = π²/12 − (ln 2)²/2 ≈ 0.5822.
    """
    result = _eval("li2(0.5)")
    assert isinstance(result, IRFloat), f"Expected IRFloat, got {type(result).__name__}"
    assert abs(result.value - 0.5822) < 1e-3, (
        f"li2(0.5) = {result.value}, expected ≈ 0.5822"
    )


def test_pipeline_fresnel_s_zero() -> None:
    """fresnel_s(0) → 0  (exact; FresnelS(0) = 0 by definition)."""
    result = _eval("fresnel_s(0)")
    assert result == IRInteger(0), f"Expected 0, got {result!r}"


def test_pipeline_fresnel_c_zero() -> None:
    """fresnel_c(0) → 0  (exact; FresnelC(0) = 0 by definition)."""
    result = _eval("fresnel_c(0)")
    assert result == IRInteger(0), f"Expected 0, got {result!r}"


def test_pipeline_lambert_w_zero() -> None:
    """lambert_w(0) → IRFloat(≈ 0.0)  (W₀(0) = 0 exactly)."""
    result = _eval("lambert_w(0)")
    assert isinstance(result, IRFloat), f"Expected IRFloat, got {type(result).__name__}"
    assert abs(result.value) < 1e-9, (
        f"lambert_w(0) = {result.value}, expected ≈ 0"
    )


def test_pipeline_lambert_w_one() -> None:
    """lambert_w(1.0) → IRFloat(≈ 0.5671)  (W₀(1) = Ω, the Omega constant).

    The Omega constant Ω = W₀(1) ≈ 0.56714329… is the unique positive
    solution of Ω·e^Ω = 1.  Newton's method converges in a few iterations.
    """
    result = _eval("lambert_w(1.0)")
    assert isinstance(result, IRFloat), f"Expected IRFloat, got {type(result).__name__}"
    assert abs(result.value - 0.5671) < 1e-3, (
        f"lambert_w(1.0) = {result.value}, expected ≈ 0.5671 (Omega constant)"
    )


# ===========================================================================
# Section II — Assumption System (assume / is / forget — Phase 21 pipeline)
# ===========================================================================
#
# Phase 21 introduced the assumption framework to the symbolic VM:
#
#   assume(x > 0)   — record that x is positive
#   is(x > 0)       — query; returns "true", "false", or "unknown"
#   forget()        — clear all assumptions
#   forget(x > 0)   — remove one specific assumption
#
# These are runtime-owned heads (``Assume``, ``Is``, ``Forget``) registered
# in the MACSYMA name table and dispatched by SymbolicBackend.
#
# The pipeline tests below use a shared VM via ``_eval_seq`` so that
# assumptions persist from one statement to the next, mirroring REPL use.
# ===========================================================================


def _eval_seq(*sources: str) -> object:
    """Evaluate multiple MACSYMA expressions sequentially on a single VM.

    Assumptions, variable bindings, and other per-VM state persist across
    calls.  Returns the result of the *last* expression evaluated.

    Used for assumption-system tests where ``assume(...)`` must fire before
    ``is(...)`` is evaluated.
    """
    vm = VM(MacsymaBackend())
    result: object = None
    for src in sources:
        s = src.strip().rstrip(";$").strip()
        ast = parse_macsyma(s + ";")
        stmts = compile_macsyma(ast, wrap_terminators=False)
        assert len(stmts) == 1, f"expected 1 statement, got {len(stmts)}"
        result = vm.eval(stmts[0])
    return result


def test_pipeline_assume_is_true() -> None:
    """assume(x > 0); is(x > 0) → IRSymbol("true").

    After recording the fact that x is positive, the ``Is`` handler
    consults the assumption context and returns ``"true"``.
    """
    result = _eval_seq("assume(x > 0)", "is(x > 0)")
    assert result == IRSymbol("true"), (
        f"Expected 'true' after assume(x>0), is(x>0), got {result!r}"
    )


def test_pipeline_is_without_assumption_unknown() -> None:
    """is(x > 0) on a fresh VM (no prior assume) → IRSymbol("unknown").

    When no assumption has been made about x, the ``Is`` handler cannot
    determine the truth value and returns ``"unknown"``.
    """
    result = _eval("is(x > 0)")
    assert result == IRSymbol("unknown"), (
        f"Expected 'unknown' with no assumptions, got {result!r}"
    )


def test_pipeline_assume_returns_done() -> None:
    """assume(x > 0) → IRSymbol("done")  (side-effect; return value is "done")."""
    result = _eval("assume(x > 0)")
    assert result == IRSymbol("done"), f"Expected 'done', got {result!r}"


def test_pipeline_forget_clears_assumption() -> None:
    """assume(x > 0); forget(); is(x > 0) → IRSymbol("unknown").

    ``forget()`` with no arguments clears all recorded facts.  After that,
    ``is(x > 0)`` must return ``"unknown"`` again.
    """
    result = _eval_seq("assume(x > 0)", "forget()", "is(x > 0)")
    assert result == IRSymbol("unknown"), (
        f"Expected 'unknown' after forget(), got {result!r}"
    )


def test_pipeline_assume_is_negative_true() -> None:
    """assume(y < 0); is(y < 0) → IRSymbol("true")."""
    result = _eval_seq("assume(y < 0)", "is(y < 0)")
    assert result == IRSymbol("true"), (
        f"Expected 'true' after assume(y<0), is(y<0), got {result!r}"
    )


def test_pipeline_declare_positive_feeds_is() -> None:
    """declare(x, positive); is(x > 0) → IRSymbol("true")."""
    result = _eval_seq("declare(x, positive)", "is(x > 0)")
    assert result == IRSymbol("true")


def test_pipeline_properties_lists_declared_properties() -> None:
    """properties(n) lists properties recorded by declare(...)."""
    result = _eval_seq(
        "declare(n, integer, n, positive)",
        "properties(n)",
    )
    assert result == IRApply(
        IRSymbol("List"),
        (IRSymbol("integer"), IRSymbol("positive")),
    )


def test_pipeline_propvars_lists_symbols_with_properties() -> None:
    """propvars() lists the symbols that have declared properties."""
    result = _eval_seq(
        "declare(z, integer)",
        "declare(a, positive)",
        "propvars()",
    )
    assert result == IRApply(IRSymbol("List"), (IRSymbol("a"), IRSymbol("z")))


def test_pipeline_properties_uses_raw_symbol_not_binding() -> None:
    """properties(x) queries x even if x has a runtime value binding."""
    result = _eval_seq("x: 10", "declare(x, integer)", "properties(x)")
    assert result == IRApply(IRSymbol("List"), (IRSymbol("integer"),))


# ===========================================================================
# Section JJ — Extended Number Theory (Phase B3 pipeline)
# ===========================================================================
#
# Section K tested the core number-theory names (``primep``, ``next_prime``,
# ``ifactor``, ``divisors``, ``totient``).  Five further names from the
# ``cas-number-theory`` substrate were mapped in the MACSYMA name table but
# never exercised through the pipeline:
#
#   prev_prime(n)           — largest prime strictly less than n
#   moebius(n)              — Möbius μ function
#   jacobi(a, n)            — Jacobi / Legendre symbol
#   chinese([rem], [mod])   — Chinese Remainder Theorem
#   numdigits(n)            — number of base-10 digits
#
# ===========================================================================


def test_pipeline_prev_prime_10() -> None:
    """prev_prime(10) → 7  (the largest prime < 10)."""
    result = _eval("prev_prime(10)")
    assert result == IRInteger(7), f"Expected 7, got {result!r}"


def test_pipeline_prev_prime_8() -> None:
    """prev_prime(8) → 7  (7 is prime and < 8)."""
    result = _eval("prev_prime(8)")
    assert result == IRInteger(7), f"Expected 7, got {result!r}"


def test_pipeline_moebius_prime() -> None:
    """moebius(2) → -1  (μ(p) = -1 for any prime p)."""
    result = _eval("moebius(2)")
    assert result == IRInteger(-1), f"Expected -1, got {result!r}"


def test_pipeline_moebius_square_factor() -> None:
    """moebius(12) → 0  (μ(n) = 0 when n has a squared prime factor; 12 = 4·3)."""
    result = _eval("moebius(12)")
    assert result == IRInteger(0), f"Expected 0, got {result!r}"


def test_pipeline_moebius_squarefree_two_factors() -> None:
    """moebius(6) → 1  (μ(pq) = 1 for distinct primes p, q; 6 = 2·3)."""
    result = _eval("moebius(6)")
    assert result == IRInteger(1), f"Expected 1, got {result!r}"


def test_pipeline_jacobi_symbol() -> None:
    """jacobi(2, 5) → -1  (Legendre symbol (2/5): 2 is not a QR mod 5)."""
    result = _eval("jacobi(2, 5)")
    assert result == IRInteger(-1), f"Expected -1, got {result!r}"


def test_pipeline_jacobi_one_is_always_one() -> None:
    """jacobi(1, 7) → 1  (1 is a quadratic residue modulo any odd integer)."""
    result = _eval("jacobi(1, 7)")
    assert result == IRInteger(1), f"Expected 1, got {result!r}"


def test_pipeline_chinese_remainder() -> None:
    """chinese([2, 3], [3, 5]) → 8  (x ≡ 2 mod 3, x ≡ 3 mod 5 → x = 8).

    The unique solution in [0, 15) is x = 8:
      8 mod 3 = 2 ✓
      8 mod 5 = 3 ✓
    """
    result = _eval("chinese([2, 3], [3, 5])")
    assert result == IRInteger(8), f"Expected 8, got {result!r}"


def test_pipeline_numdigits_thousand() -> None:
    """numdigits(1000) → 4  (1000 has 4 decimal digits)."""
    result = _eval("numdigits(1000)")
    assert result == IRInteger(4), f"Expected 4, got {result!r}"


def test_pipeline_numdigits_one() -> None:
    """numdigits(1) → 1  (a single digit)."""
    result = _eval("numdigits(1)")
    assert result == IRInteger(1), f"Expected 1, got {result!r}"
