"""VM handlers for the CAS substrate packages.

Architecture note
-----------------
These handlers are installed on :class:`~symbolic_vm.backends.SymbolicBackend`,
**not** on any language-specific backend. This means any future CAS frontend
— Maple, Mathematica, REDUCE — that extends ``SymbolicBackend`` inherits all
algebraic operations automatically. Only surface-syntax quirks (MACSYMA's
``Display``/``Suppress``/``Kill``/``Ev``, Mathematica's pattern-holding
attributes, etc.) live in the language backend subclass.

The canonical IR heads handled here are language-neutral. A MACSYMA compiler
maps ``factor(x)`` → ``Factor(x)`` IR; a Mathematica compiler maps
``Factor[x]`` → ``Factor(x)`` IR; both route to the same handler.

Handler signature::

    def handler(vm: VM, expr: IRApply) -> IRNode

Every handler follows the "graceful fall-through" contract: if the input
doesn't match the shape the handler expects (wrong arity, non-polynomial,
etc.) it returns ``expr`` unchanged — the same unevaluated-expression
behaviour every CAS uses for operations it can't resolve.

Packages used
-------------
- ``cas_simplify`` — canonical form and identity-rule simplification
- ``cas_factor``   — integer polynomial factoring (rational-root Phase 1)
- ``cas_solve``    — linear and quadratic equation solving over Q
- ``cas_substitution`` — structural substitution
- ``cas_list_operations`` — length, first, rest, last, append, reverse, …
- ``cas_matrix``   — matrix construction, determinant, transpose, inverse
- ``cas_limit_series`` — direct-substitution limit and Taylor polynomials
"""

from __future__ import annotations

import math
from fractions import Fraction
from typing import TYPE_CHECKING

from cas_complex import IMAGINARY_UNIT as _IMAGINARY_UNIT
from cas_complex import build_complex_handler_table as _build_complex
from cas_complex.handlers import (
    abs_complex_handler as _abs_complex_handler,
    imaginary_power_handler as _imaginary_power_handler,
)
from cas_complex.normalize import contains_imaginary as _contains_imaginary
from cas_factor import factor_integer_polynomial
from cas_limit_series import PolynomialError, limit_direct, taylor_polynomial
from cas_number_theory.handlers import build_number_theory_handler_table as _build_nt
from cas_solve import nsolve_fraction_poly as _nsolve_fraction_poly
from cas_solve import solve_cubic as _solve_cubic
from cas_solve import solve_linear_system as _solve_linear_system
from cas_solve import solve_quartic as _solve_quartic
from cas_list_operations import (
    ListOperationError,
    append,
    first,
    flatten,
    join,
    last,
    length,
    part,
    range_,
    rest,
    reverse,
    sort_,
)
from cas_matrix import (
    MatrixError,
    determinant,
    inverse,
    matrix,
    transpose,
)
from cas_simplify import canonical, simplify
from cas_solve import ALL, solve_linear, solve_quadratic
from cas_substitution import subst
from polynomial import (
    degree as _poly_degree,
    deriv as _poly_deriv,
    divmod_poly as _poly_divmod,
    evaluate as _poly_evaluate,
    gcd as _poly_gcd,
    monic as _poly_monic,
    normalize as _poly_normalize,
    rational_roots as _poly_rational_roots,
)
from symbolic_ir import (
    ADD,
    DIV,
    EQUAL,
    MUL,
    NEG,
    POW,
    SUB,
    IRApply,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)
from symbolic_vm.backend import Handler
from symbolic_vm.numeric import Numeric, from_number, to_number
from symbolic_vm.polynomial_bridge import from_polynomial, to_rational

if TYPE_CHECKING:
    from symbolic_vm.vm import VM

# ---------------------------------------------------------------------------
# Sentinel for Solve(all-real-solutions)
# ---------------------------------------------------------------------------

# Maxima returns ``all`` when every value of x satisfies the equation.
_ALL_SYMBOL = IRSymbol("all")

# Pre-built empty list.
_EMPTY_LIST = IRApply(IRSymbol("List"), ())

# Constants that are NOT free variables (skip them in _find_variable).
_CONSTANT_NAMES = frozenset({"True", "False", "%pi", "%e", "%i"})


# ===========================================================================
# Section 1: cas_simplify handlers
# ===========================================================================


def simplify_handler(_vm: "VM", expr: IRApply) -> IRNode:
    """``Simplify(expr)`` — fixed-point canonical + identity-rule simplifier.

    Applies :func:`cas_simplify.simplify` to the already-evaluated inner
    expression. This runs multiple passes of:

    1. ``canonical`` — flatten n-ary Add/Mul, sort commutative args.
    2. ``numeric_fold`` — collapse numeric sub-expressions.
    3. ``rewrite`` — apply identity rules (``x+0→x``, ``x*1→x``, etc.).

    Terminates when no rule fires (or after at most 50 iterations).
    """
    if len(expr.args) != 1:
        return expr
    return simplify(expr.args[0])


def expand_handler(_vm: "VM", expr: IRApply) -> IRNode:
    """``Expand(expr)`` — full polynomial expansion.

    Distributes ``Mul`` over ``Add`` and expands integer powers of
    polynomials via the polynomial bridge. Works for single-variable
    polynomials with rational (Q) coefficients. For multi-variable or
    transcendental expressions (where :func:`~symbolic_vm.polynomial_bridge.to_rational`
    returns ``None``) the implementation falls back to
    :func:`cas_simplify.canonical`.

    Examples::

        Expand(Mul(Add(x, 1), Add(x, 2)))  →  Add(Add(2, Mul(3, x)), Pow(x, 2))
        Expand(Pow(Add(x, 1), 2))          →  Add(Add(1, Mul(2, x)), Pow(x, 2))
    """
    if len(expr.args) != 1:
        return expr
    inner = expr.args[0]

    x = _find_variable(inner)
    if x is None:
        return canonical(inner)

    rational = to_rational(inner, x)
    if rational is None:
        return canonical(inner)

    num, den = rational
    _ONE_FRAC: tuple[Fraction, ...] = (Fraction(1),)

    if _poly_normalize(den) == _ONE_FRAC:
        # Pure polynomial — emit fully expanded form
        return from_polynomial(num, x)

    # Rational function — expand numerator and denominator separately
    return IRApply(DIV, (from_polynomial(num, x), from_polynomial(den, x)))


# ===========================================================================
# Section 1b: Rational function operations (A3) — Collect, Together,
#             RatSimplify, Apart
# ===========================================================================

# Fraction sentinel for "unit polynomial denominator".
_ONE_FRAC: tuple[Fraction, ...] = (Fraction(1),)


def _frac_to_ir(f: Fraction) -> IRNode:
    """Lift a ``Fraction`` coefficient to its canonical IR literal."""
    if f.denominator == 1:
        return IRInteger(f.numerator)
    return IRRational(f.numerator, f.denominator)


def collect_handler(_vm: "VM", expr: IRApply) -> IRNode:
    """``Collect(expr, var)`` — collect terms by powers of ``var``.

    Groups all terms in ``expr`` by their degree in ``var`` and returns the
    collected polynomial form. Works for single-variable polynomials with
    rational (Q) coefficients; returns unevaluated for symbolic coefficients
    or transcendental sub-expressions.

    MACSYMA syntax: ``collect(x^2 + 2*x + x^2, x)`` → ``2*x^2 + 2*x``.
    """
    if len(expr.args) != 2:
        return expr
    inner, var = expr.args
    if not isinstance(var, IRSymbol):
        return expr

    rational = to_rational(inner, var)
    if rational is None:
        return expr  # Symbolic coefficients or transcendentals — can't collect

    num, den = rational
    if _poly_normalize(den) == _ONE_FRAC:
        return from_polynomial(num, var)

    # Rational function — collecting makes no sense; return unevaluated
    return expr


def together_handler(_vm: "VM", expr: IRApply) -> IRNode:
    """``Together(expr)`` — combine rational sub-expressions over a common denominator.

    Converts a sum of rational functions into a single fraction
    ``P(x)/Q(x)`` with common denominator. The inverse of
    :func:`apart_handler`.

    MACSYMA syntax: ``together(1/x + 1/(x+1))`` → ``(2*x+1)/(x^2+x)``.
    """
    if len(expr.args) != 1:
        return expr
    inner = expr.args[0]
    x = _find_variable(inner)
    if x is None:
        return canonical(inner)

    rational = to_rational(inner, x)
    if rational is None:
        return canonical(inner)

    num, den = rational
    normalized_den = _poly_normalize(den)

    if normalized_den == _ONE_FRAC:
        return from_polynomial(num, x)

    # Normalise denominator to monic for canonical form
    lead = normalized_den[-1]
    if lead != Fraction(1):
        num_n = tuple(c / lead for c in num)
        den_n = tuple(c / lead for c in normalized_den)
    else:
        num_n = num
        den_n = normalized_den

    return IRApply(DIV, (from_polynomial(num_n, x), from_polynomial(den_n, x)))


def rat_simplify_handler(_vm: "VM", expr: IRApply) -> IRNode:
    """``RatSimplify(expr)`` — cancel common polynomial factors.

    Computes the GCD of numerator and denominator and cancels it, reducing
    the rational expression to lowest terms.

    MACSYMA syntax: ``ratsimp((x^2-1)/(x-1))`` → ``x+1``.
    """
    if len(expr.args) != 1:
        return expr
    inner = expr.args[0]
    x = _find_variable(inner)
    if x is None:
        return simplify(inner)

    rational = to_rational(inner, x)
    if rational is None:
        return canonical(inner)

    num, den = rational
    normalized_den = _poly_normalize(den)

    if normalized_den == _ONE_FRAC:
        return from_polynomial(num, x)

    # Cancel common GCD between numerator and denominator
    common = _poly_gcd(num, den)
    common_monic = _poly_monic(common)

    if _poly_degree(common_monic) >= 1:
        # Non-trivial common factor — divide it out
        num_red, _ = _poly_divmod(num, common_monic)
        den_red, _ = _poly_divmod(den, common_monic)
        den_norm = _poly_normalize(den_red)
    else:
        num_red = num
        den_norm = normalized_den

    if den_norm == _ONE_FRAC:
        return from_polynomial(num_red, x)

    # Emit as Div(P, Q) with monic denominator
    lead = den_norm[-1]
    if lead != Fraction(1):
        num_final: tuple[Fraction, ...] = tuple(c / lead for c in num_red)
        den_final: tuple[Fraction, ...] = tuple(c / lead for c in den_norm)
    else:
        num_final = num_red  # type: ignore[assignment]
        den_final = den_norm  # type: ignore[assignment]

    return IRApply(DIV, (from_polynomial(num_final, x), from_polynomial(den_final, x)))


def _apart_proper(
    num: tuple[Fraction, ...],
    den: tuple[Fraction, ...],
    x: IRSymbol,
) -> IRNode | None:
    """Partial-fraction decompose a *proper* rational function (deg num < deg den).

    Phase 1: handles only denominators whose roots are all distinct rational
    numbers (= all roots are from ``polynomial.rational_roots``). Returns
    ``None`` if the denominator has irreducible quadratic factors or
    repeated roots.

    Uses the residue formula ``A_i = P(r_i) / Q'(r_i)`` for each simple
    pole ``r_i``.
    """
    roots = _poly_rational_roots(den)
    if len(roots) != _poly_degree(den):
        return None  # Irreducible quadratic or repeated factors

    den_deriv = _poly_deriv(den)
    terms: list[IRNode] = []

    for r in roots:
        num_val = _poly_evaluate(num, r)
        den_d_val = _poly_evaluate(den_deriv, r)
        if den_d_val == 0:
            return None  # Repeated root (shouldn't happen for distinct roots)

        A = Fraction(num_val) / Fraction(den_d_val)

        # Linear factor: (x − r) as IR
        neg_r = Fraction(-1) * (r if isinstance(r, Fraction) else Fraction(r))
        factor_ir = from_polynomial((neg_r, Fraction(1)), x)

        # Emit A / (x − r) — drop explicit coefficient of ±1
        if A == 1:
            terms.append(IRApply(DIV, (IRInteger(1), factor_ir)))
        elif A == -1:
            terms.append(IRApply(NEG, (IRApply(DIV, (IRInteger(1), factor_ir)),)))
        else:
            terms.append(IRApply(DIV, (_frac_to_ir(A), factor_ir)))

    if not terms:
        return IRInteger(0)
    if len(terms) == 1:
        return terms[0]

    acc: IRNode = terms[0]
    for t in terms[1:]:
        acc = IRApply(ADD, (acc, t))
    return acc


def apart_handler(_vm: "VM", expr: IRApply) -> IRNode:
    """``Apart(expr, var)`` — partial fraction decomposition.

    Decomposes a rational function ``P(x)/Q(x)`` into a sum of simpler
    partial fractions. Phase 1 implementation handles denominators with
    only simple (non-repeated) rational roots; returns unevaluated when
    the denominator has irreducible quadratic factors or repeated roots.

    If ``deg(P) ≥ deg(Q)`` (improper fraction) the polynomial part is
    separated first: ``P/Q = poly_part + proper_fraction``.

    MACSYMA syntax: ``partfrac(1/(x^2-1), x)``
    → ``-1/(2*(1+x)) + 1/(2*(-1+x))``.
    """
    if len(expr.args) != 2:
        return expr
    inner, var = expr.args
    if not isinstance(var, IRSymbol):
        return expr

    rational = to_rational(inner, var)
    if rational is None:
        return expr

    num, den = rational
    if _poly_normalize(den) == _ONE_FRAC:
        return from_polynomial(num, var)  # Already a polynomial

    num_deg = _poly_degree(num)
    den_deg = _poly_degree(den)

    if num_deg >= den_deg:
        # Improper fraction — polynomial division first
        q, r = _poly_divmod(num, den)
        if not _poly_normalize(r):
            return from_polynomial(q, var)  # Exact division

        proper_result = _apart_proper(r, den, var)
        if proper_result is None:
            return expr  # Denominator not fully factorable

        poly_part = from_polynomial(q, var)
        return IRApply(ADD, (poly_part, proper_result))

    result = _apart_proper(num, den, var)
    if result is None:
        return expr
    return result


# ===========================================================================
# Section 2: cas_substitution handler
# ===========================================================================


def subst_handler(vm: "VM", expr: IRApply) -> IRNode:
    """``Subst(value, var, target)`` — structural substitution then re-eval.

    Replaces every occurrence of ``var`` in ``target`` with ``value`` (the
    same structural-equality matching that ``cas_substitution.subst`` uses),
    then re-evaluates the result through the VM so arithmetic collapses.

    MACSYMA syntax: ``subst(2, x, x^2 + 1)`` → 5.

    Argument order follows MACSYMA convention: value first, variable
    second, expression third. The compiler maps ``subst(val, var, expr)``
    to ``Subst(val, var, expr)`` IR exactly.
    """
    if len(expr.args) != 3:
        return expr
    value, var, target = expr.args
    return vm.eval(subst(value, var, target))


# ===========================================================================
# Section 3: cas_factor handler + helpers
# ===========================================================================


def _find_variable(node: IRNode) -> IRSymbol | None:
    """Return the first free ``IRSymbol`` in ``node``, depth-first.

    Skips the pre-bound constants (``%pi``, ``%e``, etc.). Returns
    ``None`` if the expression contains no free symbols — i.e. it is
    entirely numeric, in which case ``factor`` is a no-op.
    """
    if isinstance(node, IRSymbol):
        if node.name not in _CONSTANT_NAMES:
            return node
        return None
    if isinstance(node, IRApply):
        # Search args left-to-right only. The head of an IRApply is
        # always an operator symbol (Add, Sub, Mul, Pow, …) — never a
        # free variable. Searching the head would incorrectly return
        # "Sub" as the variable for Sub(Pow(x,2), 1).
        for arg in node.args:
            found = _find_variable(arg)
            if found is not None:
                return found
    return None


def _rational_to_integer_poly(num_frac: tuple[Fraction, ...]) -> list[int] | None:
    """Convert a tuple of ``Fraction`` polynomial coefficients to ``list[int]``.

    Clears denominators by multiplying through by the LCM of all
    denominators. Returns ``None`` if any coefficient isn't a finite
    ``Fraction`` (this shouldn't happen after ``to_rational``, but
    defensive coding costs nothing).

    The result is suitable for :func:`cas_factor.factor_integer_polynomial`.
    """
    denoms = [c.denominator for c in num_frac]
    lcm = denoms[0]
    for d in denoms[1:]:
        lcm = lcm * d // math.gcd(lcm, d)
    int_coeffs = [int(c * lcm) for c in num_frac]
    return int_coeffs


def _poly_to_ir(coeffs: list[int], x: IRSymbol) -> IRNode:
    """Build IR for a polynomial from its integer coefficient list.

    The polynomial bridge's ``from_polynomial`` expects ``Fraction``
    coefficients; we lift the ints first.
    """
    frac_coeffs = tuple(Fraction(c) for c in coeffs)
    return from_polynomial(frac_coeffs, x)


def _factor_result_to_ir(
    content: int,
    factors: list[tuple[list[int], int]],
    x: IRSymbol,
) -> IRNode:
    """Convert ``factor_integer_polynomial`` output to a ``Mul(…)`` IR tree.

    Each factor is ``(poly_coeffs, multiplicity)``. The content is the
    integer GCD pulled out front. The result is::

        Mul(content, Pow(f1, m1), Pow(f2, m2), ...)

    Multiplicity 1 is rendered as a bare factor (no ``Pow`` wrapper).
    Content 1 is dropped from the product.
    """
    terms: list[IRNode] = []
    if content != 1:
        terms.append(IRInteger(content))
    for poly_coeffs, mult in factors:
        factor_ir = _poly_to_ir(poly_coeffs, x)
        if mult == 1:
            terms.append(factor_ir)
        else:
            terms.append(IRApply(POW, (factor_ir, IRInteger(mult))))
    if not terms:
        return IRInteger(1)
    if len(terms) == 1:
        return terms[0]
    # Left-associative binary Mul chain to match what the compiler emits.
    acc = terms[0]
    for t in terms[1:]:
        acc = IRApply(MUL, (acc, t))
    return acc


def factor_handler(_vm: "VM", expr: IRApply) -> IRNode:
    """``Factor(expr)`` — factor a univariate integer polynomial over Z.

    Uses rational-root extraction (Phase 1) followed by Kronecker's
    algorithm (Phase 2) to find all irreducible factors.  Examples::

        Factor(x^2 - 1)      →  Mul(Sub(x, 1), Add(x, 1))
        Factor(2*x^2 + 4*x + 2)  →  Mul(2, Pow(Add(x, 1), 2))
        Factor(x^4 + 4)      →  Mul(x^2+2x+2, x^2-2x+2)  [Sophie Germain]
        Factor(x^4+x^2+1)    →  Mul(x^2+x+1, x^2-x+1)    [cyclotomic]

    Returns the expression unevaluated if:
    - There is no free variable (purely numeric — no factoring needed).
    - The expression is not a polynomial in the identified variable.
    - The polynomial is irreducible over Z (e.g. ``x^2 + 1``).
    """
    if len(expr.args) != 1:
        return expr
    inner = expr.args[0]

    # Identify the lone variable.
    x = _find_variable(inner)
    if x is None:
        # No variable: the polynomial is a constant; factor is trivial.
        return inner

    # Try to lift to rational function.
    rational = to_rational(inner, x)
    if rational is None:
        return expr  # transcendental or multi-variable
    num_frac, den_frac = rational

    # We only factor pure polynomials, not rational functions.
    _ONE_FRAC = (Fraction(1),)
    if den_frac != _ONE_FRAC:
        return expr

    # Convert Fraction coefficients → ints.
    int_coeffs = _rational_to_integer_poly(num_frac)
    if int_coeffs is None:
        return expr

    content, factors = factor_integer_polynomial(int_coeffs)

    # If factoring was a no-op — content 1, single factor, same
    # coefficients as the input — the polynomial is irreducible over Z.
    # Return the whole Factor(…) node unevaluated so the user sees
    # ``Factor(x^2 + 1)`` rather than silently stripping the wrapper.
    if (
        content == 1
        and len(factors) == 1
        and factors[0][1] == 1
        and factors[0][0] == int_coeffs
    ):
        return expr

    return _factor_result_to_ir(content, factors, x)


# ===========================================================================
# Section 4: cas_solve handler
# ===========================================================================


def _unwrap_equation(eq_ir: IRNode) -> IRNode:
    """Convert ``Equal(lhs, rhs)`` to ``Sub(lhs, rhs)``.

    For a bare expression (not wrapped in ``Equal``), return it unchanged
    — the convention is ``expr = 0``.
    """
    if (
        isinstance(eq_ir, IRApply)
        and isinstance(eq_ir.head, IRSymbol)
        and eq_ir.head.name == "Equal"
        and len(eq_ir.args) == 2
    ):
        lhs, rhs = eq_ir.args
        return IRApply(SUB, (lhs, rhs))
    return eq_ir


def _ir_to_fraction_poly(
    expr: IRNode, x: IRSymbol
) -> tuple[Fraction, ...] | None:
    """Try to extract ``Fraction`` polynomial coefficients from ``expr``.

    Returns ``(c_0, c_1, ..., c_n)`` — coefficient list in ascending degree
    — or ``None`` if ``expr`` is not a polynomial in ``x`` over Q.
    Uses the polynomial bridge's ``to_rational`` and verifies the
    denominator is the unit polynomial.
    """
    rational = to_rational(expr, x)
    if rational is None:
        return None
    num_frac, den_frac = rational
    _ONE_FRAC = (Fraction(1),)
    if den_frac != _ONE_FRAC:
        return None  # rational function, not polynomial
    return num_frac  # (c_0, c_1, ...) as Fraction


def solve_handler(_vm: "VM", expr: IRApply) -> IRNode:
    """``Solve(equation, var)`` or ``Solve(List(eqs...), List(vars...))``.

    Single-equation form
    --------------------
    The first argument is either:

    - A bare expression ``f(var)`` — treated as ``f(var) = 0``.
    - ``Equal(lhs, rhs)`` — treated as ``lhs - rhs = 0``.

    Returns ``List(sol1, sol2, ...)`` of IR nodes.  Complex roots use
    ``%i`` (``IRSymbol("%i")``) as the imaginary unit.  Returns the
    expression unevaluated for degree > 4 or polynomials that
    Cardano/Ferrari cannot resolve.

    System form (linear systems only)
    ----------------------------------
    ``Solve(List(eq1, eq2, ...), List(x, y, ...))`` solves a linear system
    by Gaussian elimination.  Returns ``List(Rule(x, val), Rule(y, val),
    ...)`` or the expression unevaluated if the system is non-linear,
    singular, or under/over-determined.
    """
    if len(expr.args) != 2:
        return expr
    eq_ir, var_ir = expr.args

    # -----------------------------------------------------------------
    # System form: Solve(List(eqs...), List(vars...))
    # -----------------------------------------------------------------
    if (
        isinstance(eq_ir, IRApply)
        and isinstance(eq_ir.head, IRSymbol)
        and eq_ir.head.name == "List"
        and isinstance(var_ir, IRApply)
        and isinstance(var_ir.head, IRSymbol)
        and var_ir.head.name == "List"
    ):
        equations = list(eq_ir.args)
        variables = [v for v in var_ir.args if isinstance(v, IRSymbol)]
        if len(variables) != len(var_ir.args):
            return expr  # Non-symbol in variable list
        result = _solve_linear_system(equations, variables)
        if result is None:
            return expr  # Non-linear, singular, or wrong size
        return IRApply(IRSymbol("List"), tuple(result))

    # -----------------------------------------------------------------
    # Single-equation form: Solve(eq, var)
    # -----------------------------------------------------------------
    if not isinstance(var_ir, IRSymbol):
        return expr

    poly_ir = _unwrap_equation(eq_ir)
    coeffs = _ir_to_fraction_poly(poly_ir, var_ir)
    if coeffs is None:
        return expr  # not a polynomial in var

    deg = len(coeffs) - 1

    if deg < 0 or (deg == 0 and coeffs[0] == 0):
        # Every value of var satisfies: 0 = 0.
        return _ALL_SYMBOL
    if deg == 0:
        # Non-zero constant: no solution.
        return _EMPTY_LIST
    if deg == 1:
        # a*x + b = 0  (coeffs[0] = b, coeffs[1] = a).
        a_coeff = coeffs[1]
        b_coeff = coeffs[0]
        solutions = solve_linear(a_coeff, b_coeff)
        if solutions == ALL:
            return _ALL_SYMBOL
        return IRApply(IRSymbol("List"), tuple(solutions))
    if deg == 2:
        # a*x^2 + b*x + c = 0  (coeffs[0]=c, coeffs[1]=b, coeffs[2]=a).
        a_coeff = coeffs[2]
        b_coeff = coeffs[1]
        c_coeff = coeffs[0]
        solutions = solve_quadratic(a_coeff, b_coeff, c_coeff)
        return IRApply(IRSymbol("List"), tuple(solutions))
    if deg == 3:
        # a*x^3 + b*x^2 + c*x + d = 0
        # coeffs order: (c_0=d, c_1=c, c_2=b, c_3=a)
        a_coeff = coeffs[3]
        b_coeff = coeffs[2]
        c_coeff = coeffs[1]
        d_coeff = coeffs[0]
        solutions = _solve_cubic(a_coeff, b_coeff, c_coeff, d_coeff)
        if isinstance(solutions, str) or not solutions:
            return expr  # unevaluated (casus irreducibilis or no sol)
        return IRApply(IRSymbol("List"), tuple(solutions))
    if deg == 4:
        # a*x^4 + b*x^3 + c*x^2 + d*x + e = 0
        a_coeff = coeffs[4]
        b_coeff = coeffs[3]
        c_coeff = coeffs[2]
        d_coeff = coeffs[1]
        e_coeff = coeffs[0]
        solutions = _solve_quartic(a_coeff, b_coeff, c_coeff, d_coeff, e_coeff)
        if isinstance(solutions, str) or not solutions:
            return expr  # unevaluated
        return IRApply(IRSymbol("List"), tuple(solutions))

    # Degree > 4: return unevaluated (use NSolve for numeric roots).
    return expr


def nsolve_handler(_vm: "VM", expr: IRApply) -> IRNode:
    """``NSolve(polynomial, var)`` — numeric root-finding via Durand–Kerner.

    Finds all roots of a univariate polynomial numerically using the
    Durand-Kerner (Weierstrass) method.  Returns
    ``List(root1, root2, ...)`` where each root is an ``IRFloat`` (real
    roots) or an ``IRApply(Add, (IRFloat(re), Mul(IRFloat(im), %i)))``
    (complex roots).

    Accepts any degree ≥ 1 polynomial.  Coefficients must be rational
    (``IRInteger`` or ``IRRational``).  Returns the expression unevaluated
    if the input is not a rational polynomial.
    """
    if len(expr.args) != 2:
        return expr
    eq_ir, var_ir = expr.args
    if not isinstance(var_ir, IRSymbol):
        return expr

    poly_ir = _unwrap_equation(eq_ir)
    coeffs = _ir_to_fraction_poly(poly_ir, var_ir)
    if coeffs is None:
        return expr

    deg = len(coeffs) - 1
    if deg < 1:
        return expr  # constant — no numeric roots

    # coeffs is (c_0, c_1, ..., c_n) ascending degree;
    # nsolve_fraction_poly expects descending degree.
    coeffs_desc = list(reversed(coeffs))
    ir_roots = _nsolve_fraction_poly(coeffs_desc)
    return IRApply(IRSymbol("List"), tuple(ir_roots))


# ===========================================================================
# Section 5: cas_list_operations handlers
# ===========================================================================


def _as_list_args(node: IRNode) -> tuple[IRNode, ...]:
    """Return the elements of a ``List(…)`` node or raise ``ListOperationError``."""
    if (
        isinstance(node, IRApply)
        and isinstance(node.head, IRSymbol)
        and node.head.name == "List"
    ):
        return node.args
    raise ListOperationError(f"expected a List, got {node!r}")


def length_handler(_vm: "VM", expr: IRApply) -> IRNode:
    """``Length(list)`` → number of elements as ``IRInteger``."""
    if len(expr.args) != 1:
        return expr
    try:
        return length(expr.args[0])
    except ListOperationError:
        return expr


def first_handler(_vm: "VM", expr: IRApply) -> IRNode:
    """``First(list)`` → the first element."""
    if len(expr.args) != 1:
        return expr
    try:
        return first(expr.args[0])
    except ListOperationError:
        return expr


def rest_handler(_vm: "VM", expr: IRApply) -> IRNode:
    """``Rest(list)`` → all elements except the first."""
    if len(expr.args) != 1:
        return expr
    try:
        return rest(expr.args[0])
    except ListOperationError:
        return expr


def last_handler(_vm: "VM", expr: IRApply) -> IRNode:
    """``Last(list)`` → the last element."""
    if len(expr.args) != 1:
        return expr
    try:
        return last(expr.args[0])
    except ListOperationError:
        return expr


def append_handler(_vm: "VM", expr: IRApply) -> IRNode:
    """``Append(list1, list2, …)`` → concatenated list.

    Accepts two or more lists. MACSYMA's ``append`` also accepts multiple
    args; we follow suit.
    """
    if len(expr.args) < 2:
        return expr
    try:
        return append(*expr.args)
    except ListOperationError:
        return expr


def reverse_handler(_vm: "VM", expr: IRApply) -> IRNode:
    """``Reverse(list)`` → elements in reverse order."""
    if len(expr.args) != 1:
        return expr
    try:
        return reverse(expr.args[0])
    except ListOperationError:
        return expr


def range_handler(_vm: "VM", expr: IRApply) -> IRNode:
    """``Range(n)`` / ``Range(start, stop)`` / ``Range(start, stop, step)``.

    Single-argument form ``Range(n)`` produces ``[1, 2, …, n]`` (the
    MACSYMA ``makelist`` convention). All arguments must be ``IRInteger``.
    """
    try:
        if len(expr.args) == 1:
            n = expr.args[0]
            if not isinstance(n, IRInteger):
                return expr
            return range_(n.value)
        if len(expr.args) == 2:
            start, stop = expr.args
            if not isinstance(start, IRInteger) or not isinstance(stop, IRInteger):
                return expr
            return range_(start.value, stop.value)
        if len(expr.args) == 3:
            start, stop, step = expr.args
            if (
                not isinstance(start, IRInteger)
                or not isinstance(stop, IRInteger)
                or not isinstance(step, IRInteger)
            ):
                return expr
            return range_(start.value, stop.value, step.value)
        return expr
    except ListOperationError:
        return expr


def map_handler(vm: "VM", expr: IRApply) -> IRNode:
    """``Map(f, list)`` → ``[f(a), f(b), f(c), …]`` evaluated through the VM.

    ``f`` is any IR node that can appear as a head (typically an
    ``IRSymbol`` naming a function or a ``Define`` record). Each element
    of ``list`` is passed as the sole argument and the result is fully
    evaluated via ``vm.eval``.

    Design note: we build and evaluate each application ourselves rather
    than using :func:`cas_list_operations.map_` (which builds unevaluated
    IR applies) because the VM would need a second pass to evaluate them.
    Evaluating inline keeps the semantics clean.
    """
    if len(expr.args) != 2:
        return expr
    f, lst = expr.args
    try:
        elems = _as_list_args(lst)
    except ListOperationError:
        return expr
    results = tuple(vm.eval(IRApply(f, (e,))) for e in elems)
    return IRApply(IRSymbol("List"), results)


def apply_handler(vm: "VM", expr: IRApply) -> IRNode:
    """``Apply(f, list)`` → ``f(a, b, c, …)`` evaluated through the VM.

    Replaces the ``List`` head with ``f`` and evaluates the result.
    Example: ``Apply(Add, [1, 2, 3])`` evaluates to ``6``.
    """
    if len(expr.args) != 2:
        return expr
    f, lst = expr.args
    try:
        elems = _as_list_args(lst)
    except ListOperationError:
        return expr
    return vm.eval(IRApply(f, elems))


def select_handler(vm: "VM", expr: IRApply) -> IRNode:
    """``Select(pred, list)`` → elements ``e`` for which ``pred(e)`` is ``True``.

    ``pred`` must be a function-head IR node. It is called with each list
    element through ``vm.eval``; elements whose result is the symbol
    ``True`` (the VM's canonical boolean true) are kept.
    """
    if len(expr.args) != 2:
        return expr
    pred, lst = expr.args
    try:
        elems = _as_list_args(lst)
    except ListOperationError:
        return expr
    kept: list[IRNode] = []
    _TRUE = IRSymbol("True")
    for e in elems:
        result = vm.eval(IRApply(pred, (e,)))
        if result == _TRUE:
            kept.append(e)
    return IRApply(IRSymbol("List"), tuple(kept))


def sort_handler(_vm: "VM", expr: IRApply) -> IRNode:
    """``Sort(list)`` → stable sort by canonical ``repr`` key.

    Uses the same ordering as :func:`cas_simplify.canonical` — numerics
    first, then symbols, then compound expressions — so the result is
    consistent with the canonical form the simplifier produces.
    """
    if len(expr.args) != 1:
        return expr
    try:
        return sort_(expr.args[0])
    except ListOperationError:
        return expr


def part_handler(_vm: "VM", expr: IRApply) -> IRNode:
    """``Part(list, index)`` → 1-based element access.

    ``Part(lst, 1)`` is the first element; ``Part(lst, -1)`` is the last.
    Returns the expression unevaluated for out-of-range or non-integer
    index.
    """
    if len(expr.args) != 2:
        return expr
    lst, idx = expr.args
    if not isinstance(idx, IRInteger):
        return expr
    try:
        return part(lst, idx.value)
    except ListOperationError:
        return expr


def flatten_handler(_vm: "VM", expr: IRApply) -> IRNode:
    """``Flatten(list)`` / ``Flatten(list, depth)`` → flattened list.

    Default depth is 1 (one level of nesting). Pass an ``IRInteger``
    depth as the optional second argument for deeper flattening.
    """
    if not expr.args:
        return expr
    try:
        if len(expr.args) == 1:
            return flatten(expr.args[0])
        if len(expr.args) == 2 and isinstance(expr.args[1], IRInteger):
            return flatten(expr.args[0], expr.args[1].value)
        return expr
    except ListOperationError:
        return expr


def join_handler(_vm: "VM", expr: IRApply) -> IRNode:
    """``Join(list1, list2, …)`` → concatenation (alias for ``Append``)."""
    if len(expr.args) < 2:
        return expr
    try:
        return join(*expr.args)
    except ListOperationError:
        return expr


# ===========================================================================
# Section 6: cas_matrix handlers
# ===========================================================================


def matrix_handler(_vm: "VM", expr: IRApply) -> IRNode:
    """``Matrix(List(…), List(…), …)`` → validated ``Matrix`` IR node.

    Each argument must be a ``List`` of equal length (the rows). The
    result is the canonical ``IRApply(Matrix, (row1, row2, …))`` shape
    that every other matrix handler expects. Rows of unequal length or
    non-List arguments cause a fall-through to the unevaluated form.
    """
    if not expr.args:
        return expr
    try:
        rows = [list(_as_list_args(r)) for r in expr.args]
        return matrix(rows)
    except (MatrixError, ListOperationError):
        return expr


def transpose_handler(_vm: "VM", expr: IRApply) -> IRNode:
    """``Transpose(M)`` → matrix with rows and columns swapped."""
    if len(expr.args) != 1:
        return expr
    try:
        return transpose(expr.args[0])
    except MatrixError:
        return expr


def determinant_handler(vm: "VM", expr: IRApply) -> IRNode:
    """``Determinant(M)`` → scalar.

    Computes the symbolic determinant via cofactor expansion, then
    passes the result through ``vm.eval()`` so that any pure-numeric
    sub-expressions collapse to ``IRInteger`` / ``IRRational`` values.

    For a 2×2 matrix ``[[a,b],[c,d]]`` the raw result is ``Sub(Mul(a,d),
    Mul(b,c))``, which folds to an integer when all entries are numeric.
    """
    if len(expr.args) != 1:
        return expr
    try:
        return vm.eval(determinant(expr.args[0]))
    except MatrixError:
        return expr


def inverse_handler(_vm: "VM", expr: IRApply) -> IRNode:
    """``Inverse(M)`` → matrix whose entries are IR rational expressions."""
    if len(expr.args) != 1:
        return expr
    try:
        return inverse(expr.args[0])
    except MatrixError:
        return expr


# ===========================================================================
# Section 7: cas_limit_series handlers
# ===========================================================================


def limit_handler(vm: "VM", expr: IRApply) -> IRNode:
    """``Limit(expr, var, point)`` — direct-substitution limit.

    Phase 1: substitutes ``point`` for ``var`` in ``expr`` via
    :func:`cas_limit_series.limit_direct`. If the result is obviously
    indeterminate (a literal ``0/0``), the unevaluated ``Limit(…)`` is
    returned instead. The result is simplified and then re-evaluated
    through the VM so arithmetic collapses.

    L'Hôpital and limits at ±∞ are deferred to a later phase.
    """
    if len(expr.args) != 3:
        return expr
    body, var, point = expr.args
    if not isinstance(var, IRSymbol):
        return expr
    result = limit_direct(body, var, point)
    # Simplify then re-evaluate; limit_direct returns raw substituted IR.
    return vm.eval(simplify(result))


def taylor_handler(_vm: "VM", expr: IRApply) -> IRNode:
    """``Taylor(expr, var, point, order)`` — truncated Taylor polynomial.

    Expands the polynomial ``expr`` around ``point`` to the given
    ``order``. All four arguments are required:

    - ``expr``  — polynomial IR expression in ``var``
    - ``var``   — the expansion variable (``IRSymbol``)
    - ``point`` — the expansion point (numeric IR literal)
    - ``order`` — non-negative ``IRInteger`` truncation order

    Non-polynomial inputs (transcendental functions, multiple variables)
    raise :class:`cas_limit_series.PolynomialError` internally; the
    handler catches this and returns the expression unevaluated.
    """
    if len(expr.args) != 4:
        return expr
    body, var, point, order_ir = expr.args
    if not isinstance(var, IRSymbol):
        return expr
    if not isinstance(order_ir, IRInteger):
        return expr
    try:
        return taylor_polynomial(body, var, point, order_ir.value)
    except (PolynomialError, ValueError):
        return expr


# ===========================================================================
# Section 8: numeric / arithmetic handlers
# ===========================================================================


def _numeric_unary(
    expr: IRApply, fn
) -> IRNode:
    """Apply a Python callable ``fn`` to a single numeric IR arg."""
    if len(expr.args) != 1:
        return expr
    n = to_number(expr.args[0])
    if n is None:
        return expr
    return from_number(fn(n))


def _numeric_binary(
    expr: IRApply, fn
) -> IRNode:
    """Apply a Python callable ``fn(a, b)`` to two numeric IR args."""
    if len(expr.args) != 2:
        return expr
    a = to_number(expr.args[0])
    b = to_number(expr.args[1])
    if a is None or b is None:
        return expr
    return from_number(fn(a, b))


def abs_handler(vm: "VM", expr: IRApply) -> IRNode:
    """``Abs(x)`` → absolute value.

    For complex inputs (containing ``ImaginaryUnit``), delegates to
    :func:`cas_complex.handlers.abs_complex_handler` which returns
    ``sqrt(re^2 + im^2)``.  For real numeric inputs, folds directly.
    Leaves symbolic real inputs unevaluated.
    """
    if len(expr.args) == 1 and _contains_imaginary(expr.args[0]):
        return _abs_complex_handler(vm, expr)
    return _numeric_unary(expr, abs)


def floor_handler(_vm: "VM", expr: IRApply) -> IRNode:
    """``Floor(x)`` → greatest integer ≤ x."""
    return _numeric_unary(expr, lambda n: Fraction(math.floor(n)))


def ceiling_handler(_vm: "VM", expr: IRApply) -> IRNode:
    """``Ceiling(x)`` → smallest integer ≥ x."""
    return _numeric_unary(expr, lambda n: Fraction(math.ceil(n)))


def mod_handler(_vm: "VM", expr: IRApply) -> IRNode:
    """``Mod(a, b)`` → ``a mod b``. Both arguments must be numeric."""
    if len(expr.args) != 2:
        return expr
    a = to_number(expr.args[0])
    b = to_number(expr.args[1])
    if a is None or b is None:
        return expr
    if b == 0:
        return expr  # undefined — leave unevaluated
    if isinstance(a, Fraction) and isinstance(b, Fraction):
        return from_number(a % b)
    return from_number(float(a) % float(b))


def gcd_handler(_vm: "VM", expr: IRApply) -> IRNode:
    """``Gcd(a, b)`` → greatest common divisor. Both must be integers."""
    if len(expr.args) != 2:
        return expr
    a = to_number(expr.args[0])
    b = to_number(expr.args[1])
    if not isinstance(a, Fraction) or not isinstance(b, Fraction):
        return expr
    if a.denominator != 1 or b.denominator != 1:
        return expr
    return IRInteger(math.gcd(a.numerator, b.numerator))


def lcm_handler(_vm: "VM", expr: IRApply) -> IRNode:
    """``Lcm(a, b)`` → least common multiple. Both must be integers."""
    if len(expr.args) != 2:
        return expr
    a = to_number(expr.args[0])
    b = to_number(expr.args[1])
    if not isinstance(a, Fraction) or not isinstance(b, Fraction):
        return expr
    if a.denominator != 1 or b.denominator != 1:
        return expr
    return IRInteger(math.lcm(a.numerator, b.numerator))


# ===========================================================================
# Section 9: equation-side handlers (C5)
# ===========================================================================

_EQUAL_HEAD = "Equal"


def lhs_handler(_vm: "VM", expr: IRApply) -> IRNode:
    """``Lhs(eq)`` → left-hand side of equation ``Equal(a, b)`` → ``a``.

    MACSYMA syntax: ``lhs(x = 3)`` → ``x``.

    If the argument is not an ``Equal`` expression, returns unevaluated.
    """
    if len(expr.args) != 1:
        return expr
    eq = expr.args[0]
    if (
        isinstance(eq, IRApply)
        and isinstance(eq.head, IRSymbol)
        and eq.head.name == _EQUAL_HEAD
        and len(eq.args) == 2
    ):
        return eq.args[0]
    return expr


def rhs_handler(_vm: "VM", expr: IRApply) -> IRNode:
    """``Rhs(eq)`` → right-hand side of equation ``Equal(a, b)`` → ``b``.

    MACSYMA syntax: ``rhs(x = 3)`` → ``3``.

    If the argument is not an ``Equal`` expression, returns unevaluated.
    """
    if len(expr.args) != 1:
        return expr
    eq = expr.args[0]
    if (
        isinstance(eq, IRApply)
        and isinstance(eq.head, IRSymbol)
        and eq.head.name == _EQUAL_HEAD
        and len(eq.args) == 2
    ):
        return eq.args[1]
    return expr


# ===========================================================================
# Section 10: MakeList handler (C2)
# ===========================================================================

_LIST_HEAD = IRSymbol("List")


def make_list_handler(vm: "VM", expr: IRApply) -> IRNode:
    """``MakeList(expr, var, n)`` or ``MakeList(expr, var, from, to[, step])``.

    Evaluates *expr* for *var* = each integer in the specified range and
    collects the results into a ``List``.

    Supported arities
    -----------------
    - 3 args: ``MakeList(expr, var, n)``  → range ``1..n`` (step 1).
    - 4 args: ``MakeList(expr, var, from, to)`` → range ``from..to``.
    - 5 args: ``MakeList(expr, var, from, to, step)`` → with given step.

    MACSYMA examples::

        makelist(i^2, i, 4)        → [1, 4, 9, 16]
        makelist(i*2, i, 2, 6, 2)  → [4, 8, 12]
    """
    nargs = len(expr.args)
    if nargs not in (3, 4, 5):
        return expr

    body = expr.args[0]
    var = expr.args[1]
    if not isinstance(var, IRSymbol):
        return expr

    # Resolve range bounds — they must evaluate to integers.
    def _to_int(node: IRNode) -> int | None:
        evaled = vm.eval(node)
        if isinstance(evaled, IRInteger):
            return evaled.value
        return None

    if nargs == 3:
        stop = _to_int(expr.args[2])
        if stop is None:
            return expr
        start, step = 1, 1
    elif nargs == 4:
        start = _to_int(expr.args[2])
        stop = _to_int(expr.args[3])
        if start is None or stop is None:
            return expr
        step = 1
    else:
        start = _to_int(expr.args[2])
        stop = _to_int(expr.args[3])
        step = _to_int(expr.args[4])
        if start is None or stop is None or step is None or step == 0:
            return expr

    results: list[IRNode] = []
    for i in range(start, stop + 1, step):
        substituted = subst(IRInteger(i), var, body)
        results.append(vm.eval(substituted))

    return IRApply(_LIST_HEAD, tuple(results))


# ===========================================================================
# Section 11: At handler (C4)
# ===========================================================================


def at_handler(vm: "VM", expr: IRApply) -> IRNode:
    """``At(expr, Equal(var, val))`` → evaluate *expr* at *var* = *val*.

    MACSYMA syntax: ``at(x^2 + 1, x = 3)`` → ``10``.

    This is syntactic sugar over :func:`subst_handler`. The ``Equal``-as-
    substitution-rule convention is MACSYMA-specific (Mathematica uses
    ``Rule`` instead), so this handler lives in the common substrate but
    the name-table binding lives only in ``macsyma-runtime``.

    Supported forms
    ---------------
    - ``At(expr, Equal(var, val))`` — single substitution.
    - ``At(expr, List(Equal(v1, a1), Equal(v2, a2), …))`` — simultaneous
      substitution of multiple variables.
    """
    if len(expr.args) != 2:
        return expr
    body, rule_or_list = expr.args

    def _apply_rule(current: IRNode, rule: IRNode) -> IRNode | None:
        """Return *current* with the rule applied, or None if not a rule."""
        if (
            isinstance(rule, IRApply)
            and isinstance(rule.head, IRSymbol)
            and rule.head.name == _EQUAL_HEAD
            and len(rule.args) == 2
        ):
            var, val = rule.args
            return vm.eval(subst(val, var, current))
        return None

    # Single rule.
    result = _apply_rule(body, rule_or_list)
    if result is not None:
        return result

    # List of rules — apply each in sequence.
    if (
        isinstance(rule_or_list, IRApply)
        and isinstance(rule_or_list.head, IRSymbol)
        and rule_or_list.head.name == "List"
    ):
        current = body
        for rule in rule_or_list.args:
            applied = _apply_rule(current, rule)
            if applied is None:
                return expr  # malformed rule — bail out
            current = applied
        return current

    return expr


# ===========================================================================
# Public entry point
# ===========================================================================


def build_cas_handler_table() -> dict[str, Handler]:
    """Return the full CAS handler table for :class:`SymbolicBackend`.

    The keys are the canonical IR head names. The values are handler
    callables conforming to the ``(VM, IRApply) -> IRNode`` signature.

    This table is merged into ``SymbolicBackend._handlers`` at
    construction time so every CAS frontend (MACSYMA, Maple, Mathematica,
    …) inherits these operations automatically.
    """
    return {
        # --- cas_simplify ---------------------------------------------------
        "Simplify": simplify_handler,
        "Expand": expand_handler,
        # --- rational function operations (A3) ------------------------------
        "Collect": collect_handler,
        "Together": together_handler,
        "RatSimplify": rat_simplify_handler,
        "Apart": apart_handler,
        # --- cas_substitution -----------------------------------------------
        "Subst": subst_handler,
        # --- cas_factor -----------------------------------------------------
        "Factor": factor_handler,
        # --- cas_solve -------------------------------------------------------
        "Solve": solve_handler,
        "NSolve": nsolve_handler,
        # --- cas_list_operations --------------------------------------------
        "Length": length_handler,
        "First": first_handler,
        "Rest": rest_handler,
        "Last": last_handler,
        "Append": append_handler,
        "Reverse": reverse_handler,
        "Range": range_handler,
        "Map": map_handler,
        "Apply": apply_handler,
        "Select": select_handler,
        "Sort": sort_handler,
        "Part": part_handler,
        "Flatten": flatten_handler,
        "Join": join_handler,
        # --- cas_matrix -----------------------------------------------------
        "Matrix": matrix_handler,
        "Transpose": transpose_handler,
        "Determinant": determinant_handler,
        "Inverse": inverse_handler,
        # --- cas_limit_series -----------------------------------------------
        "Limit": limit_handler,
        "Taylor": taylor_handler,
        # --- numeric/arithmetic ---------------------------------------------
        "Abs": abs_handler,
        "Floor": floor_handler,
        "Ceiling": ceiling_handler,
        "Mod": mod_handler,
        "Gcd": gcd_handler,
        "Lcm": lcm_handler,
        # --- equation sides (C5) --------------------------------------------
        "Lhs": lhs_handler,
        "Rhs": rhs_handler,
        # --- MakeList (C2) --------------------------------------------------
        "MakeList": make_list_handler,
        # --- At / point evaluation (C4) -------------------------------------
        "At": at_handler,
        # --- cas_number_theory (B3) -----------------------------------------
        **_build_nt(),
        # --- cas_complex (B2) -----------------------------------------------
        # Re, Im, Conjugate, Arg, RectForm, PolarForm — Re/Im/Conjugate
        # operate on ImaginaryUnit-containing IR expressions and extract
        # the a and b of a + b*i form.  AbsComplex is registered under
        # its own key; the main Abs dispatcher (above) routes to it for
        # complex inputs.  Imaginary power reduction is wired separately
        # into the Pow handler by SymbolicBackend.__init__.
        **_build_complex(),
    }


# ---------------------------------------------------------------------------
# B2 helpers exposed to SymbolicBackend
# ---------------------------------------------------------------------------

#: The ``ImaginaryUnit`` symbol, pre-bound to itself in ``SymbolicBackend``.
IMAGINARY_UNIT_SYMBOL: IRSymbol = _IMAGINARY_UNIT  # type: ignore[assignment]

#: Imaginary-power handler — install on ``Pow`` in ``SymbolicBackend``.
IMAGINARY_POWER_HOOK = _imaginary_power_handler
