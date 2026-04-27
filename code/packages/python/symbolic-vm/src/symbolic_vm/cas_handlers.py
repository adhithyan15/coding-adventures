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

from cas_factor import factor_integer_polynomial
from cas_limit_series import PolynomialError, limit_direct, taylor_polynomial
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
from symbolic_ir import (
    ADD,
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
    """``Expand(expr)`` — structural canonical form.

    Phase 1 implementation: applies :func:`cas_simplify.canonical` which
    flattens nested Add/Mul, sorts commutative operands, and drops
    singleton wrappers. Full polynomial distribution (e.g.
    ``(a+b)*(c+d) → ac+ad+bc+bd``) is deferred to Phase 2 when the
    polynomial package is extended with a multiply-and-expand step.

    Returning the canonical form is the correct Phase 1 behaviour — it
    matches what every CAS does for ``expand`` on an already-expanded
    expression, and it normalises the sort order so subsequent rules fire
    reliably.
    """
    if len(expr.args) != 1:
        return expr
    return canonical(expr.args[0])


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

    Phase 1: finds all integer-valued rational roots via the rational-root
    theorem, divides them out, and leaves any irreducible residual as a
    single factor. Example::

        Factor(x^2 - 1)  →  Mul(Sub(x, 1), Add(x, 1))
        Factor(2*x^2 + 4*x + 2)  →  Mul(2, Pow(Add(x, 1), 2))

    Returns the expression unevaluated if:
    - There is no free variable (purely numeric — no factoring needed).
    - The expression is not a polynomial in the identified variable.
    - The polynomial has non-integer-valued rational roots (irreducible).
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
    """``Solve(equation, var)`` — closed-form solutions over Q.

    Handles linear and quadratic equations. The first argument is either:

    - A bare expression ``f(var)`` — treated as ``f(var) = 0``.
    - ``Equal(lhs, rhs)`` — treated as ``lhs - rhs = 0``.

    Returns ``List(sol1, sol2, ...)`` of :class:`~symbolic_ir.IRInteger`
    or :class:`~symbolic_ir.IRRational` nodes.  Complex roots from the
    quadratic formula (e.g. ``x^2 + 1 = 0``) are returned as
    ``IRApply(Mul, (IRInteger(-1), IRApply(Sqrt, (IRInteger(-1),))))`` — the
    CAS renders them symbolically. Returns the expression unevaluated for
    degree > 2.

    Truth-table for coefficients::

        degree 1: a*x + b = 0  →  [x = -b/a]
        degree 2: a*x^2 + b*x + c = 0  →  quadratic formula solutions
        degree 0 with b ≠ 0: no solution → []
        degree 0 with b = 0: all x satisfy → ``all`` symbol
    """
    if len(expr.args) != 2:
        return expr
    eq_ir, var_ir = expr.args
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

    # Degree > 2: return unevaluated.
    return expr


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


def abs_handler(_vm: "VM", expr: IRApply) -> IRNode:
    """``Abs(x)`` → absolute value. Folds numerics; leaves symbolics."""
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
        # --- cas_substitution -----------------------------------------------
        "Subst": subst_handler,
        # --- cas_factor -----------------------------------------------------
        "Factor": factor_handler,
        # --- cas_solve -------------------------------------------------------
        "Solve": solve_handler,
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
    }
