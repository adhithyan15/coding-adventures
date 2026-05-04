"""CAS substrate handlers for :class:`MacsymaBackend`.

Each handler dispatches a MACSYMA IR head to the appropriate CAS
substrate package.  They all follow the standard
``symbolic_vm.backend.Handler`` signature::

    def handler(vm: VM, expr: IRApply) -> IRNode

Organised in sections that mirror the substrate packages:

- **simplify / expand** — :mod:`cas_simplify`
- **substitution** — :mod:`cas_substitution`
- **factor** — :mod:`cas_factor`
- **solve** — :mod:`cas_solve`
- **list operations** — :mod:`cas_list_operations`
- **matrix** — :mod:`cas_matrix`
- **limit / taylor** — :mod:`cas_limit_series`
- **numeric / arithmetic** — builtin Python :mod:`math`

All handlers follow the same defensive contract:

1. Validate arity. Wrong-arity calls return the expression unevaluated
   so the user sees e.g. ``Factor(x, y)`` instead of a Python traceback.
2. Catch the substrate's public exception types and return the expression
   unevaluated.  This keeps the REPL alive on partial inputs.

:func:`build_cas_handler_table` returns the complete ``dict[str, Handler]``
that :class:`MacsymaBackend` merges into its dispatcher at startup.
"""

from __future__ import annotations

import math
from fractions import Fraction
from typing import TYPE_CHECKING

from cas_factor import factor_integer_polynomial
from cas_limit_series import PolynomialError, limit_direct, taylor_polynomial
from cas_list_operations import (
    LIST,
    ListOperationError,
    append,
    apply_,
    first,
    flatten,
    join,
    last,
    length,
    map_,
    part,
    range_,
    rest,
    reverse,
    select,
    sort_,
)
from cas_matrix import MatrixError, determinant, inverse, is_matrix, matrix, transpose
from cas_simplify import canonical, simplify
from cas_solve import ALL, solve_cubic, solve_linear, solve_linear_system, solve_quadratic, solve_quartic
from cas_substitution import subst
from symbolic_ir import (
    ADD,
    MUL,
    NEG,
    POW,
    IRApply,
    IRFloat,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)
from symbolic_vm.backend import Handler
from symbolic_vm.numeric import from_number, to_number
from symbolic_vm.polynomial_bridge import from_polynomial, to_rational

if TYPE_CHECKING:
    from symbolic_vm import VM

# ---------------------------------------------------------------------------
# Constants we never want to treat as variables during factor/solve
# ---------------------------------------------------------------------------

_CONSTANTS: frozenset[str] = frozenset({"Pi", "E", "%pi", "%e", "True", "False", "i"})


# ---------------------------------------------------------------------------
# Simplify / expand
# ---------------------------------------------------------------------------


def simplify_handler(_vm: VM, expr: IRApply) -> IRNode:
    """``Simplify(expr)`` — apply the fixed-point simplifier."""
    if len(expr.args) != 1:
        return expr
    return simplify(expr.args[0])


def expand_handler(_vm: VM, expr: IRApply) -> IRNode:
    """``Expand(expr)`` — fully distribute products and powers.

    Uses the polynomial bridge to expand ``(x+1)*(x+2)`` → ``x^2+3x+2``
    and ``(x+1)^2`` → ``x^2+2x+1``.  Requires a single-variable polynomial
    expression; falls back to structural :func:`canonical` otherwise.
    """
    if len(expr.args) != 1:
        return expr
    inner = expr.args[0]
    # Try real polynomial expansion via the bridge (single-variable only).
    x = _find_variable(inner)
    if x is not None:
        result = to_rational(inner, x)
        if result is not None:
            num, den = result
            # Only expand pure polynomials (rational functions stay symbolic).
            if den == (Fraction(1),):
                return from_polynomial(num, x)
    # Fallback: structural canonicalization for zero-variable or
    # multi-variable expressions.
    return canonical(inner)


# ---------------------------------------------------------------------------
# Substitution
# ---------------------------------------------------------------------------


def subst_handler(vm: VM, expr: IRApply) -> IRNode:
    """``Subst(value, var, target)`` — replace ``var`` with ``value`` in ``target``.

    After substitution the result is re-evaluated through the VM so that
    ``subst(2, x, x^2 + 1)`` produces ``5`` rather than ``Pow(2, 2) + 1``.
    """
    if len(expr.args) != 3:
        return expr
    value, var, target = expr.args
    substituted = subst(value, var, target)
    return vm.eval(substituted)


# ---------------------------------------------------------------------------
# Factor
# ---------------------------------------------------------------------------


def factor_handler(_vm: VM, expr: IRApply) -> IRNode:
    """``Factor(poly_expr)`` — factor a univariate integer polynomial.

    Identifies the single free variable in ``poly_expr``, converts to
    an integer coefficient list, calls :func:`cas_factor.factor_integer_polynomial`,
    and reassembles the IR as ``Mul(content, Pow(factor_1, mult_1), …)``.

    Returns the expression unevaluated if:

    - the expression has more than one free variable,
    - it cannot be represented as an integer polynomial over the free variable,
    - the polynomial is constant (degree 0).
    """
    if len(expr.args) != 1:
        return expr
    inner = expr.args[0]

    # Pick the single free variable.
    x = _find_variable(inner)
    if x is None:
        return expr  # no variable — constant already

    # Convert to integer coefficient list via the polynomial bridge.
    coeffs = _ir_to_integer_poly(inner, x)
    if coeffs is None:
        return expr

    # Factor.
    content_val, factors = factor_integer_polynomial(coeffs)

    if not factors:
        # The polynomial was just the content (constant).
        return inner

    # Degree-1 polynomials (linear: [b, a]) are already in "factored form"
    # by definition — return the polynomial directly rather than unevaluated.
    # Only for degree ≥ 2 do we distinguish "irreducible" from "factored".
    if len(coeffs) <= 2:
        return _factor_result_to_ir(content_val, factors, x)

    # For degree ≥ 2: if there is exactly one factor with multiplicity 1
    # and the content is ±1, the polynomial is irreducible over Z — no
    # non-trivial factoring was possible.  Return the expression
    # *unevaluated* (head stays ``Factor``) so the user can see that it
    # cannot be simplified further.
    if len(factors) == 1 and factors[0][1] == 1 and abs(content_val) == 1:
        return expr

    return _factor_result_to_ir(content_val, factors, x)


def _find_variable(node: IRNode) -> IRSymbol | None:
    """Return the first :class:`IRSymbol` that is not a known constant.

    Recurses into :class:`IRApply` nodes.  Returns ``None`` if no free
    variable is found or if more than one distinct variable is found
    (multi-variate — not supported in Phase 1).
    """
    found: set[str] = set()
    _collect_variables(node, found)
    if len(found) == 1:
        return IRSymbol(next(iter(found)))
    return None


def _collect_variables(node: IRNode, found: set[str]) -> None:
    if isinstance(node, IRSymbol):
        if node.name not in _CONSTANTS:
            found.add(node.name)
    elif isinstance(node, IRApply):
        for arg in node.args:
            _collect_variables(arg, found)


def _ir_to_integer_poly(inner: IRNode, x: IRSymbol) -> list[int] | None:
    """Convert IR to ``list[int]`` coefficient list (low-degree first).

    Returns ``None`` if ``inner`` is not a pure polynomial in ``x`` with
    rational coefficients, or if the denominators don't clear to integers.
    """
    result = to_rational(inner, x)
    if result is None:
        return None
    num, den = result
    # Must be a polynomial, not a genuine rational function.
    if den != (Fraction(1),):
        return None

    # Clear denominators: find LCM of all coefficient denominators,
    # then scale up to get integer coefficients.
    denom_lcm = 1
    for c in num:
        denom_lcm = _lcm(denom_lcm, c.denominator)

    int_coeffs = [int(c * denom_lcm) for c in num]

    # Only include denom_lcm in the content if it's > 1 and we'd lose
    # info. For integer polynomials this is a no-op; for rational inputs
    # we scale up — that changes the polynomial, so bail out instead.
    if denom_lcm != 1:
        return None  # rational polynomial, not integer — leave unevaluated

    return int_coeffs


def _gcd(a: int, b: int) -> int:
    while b:
        a, b = b, a % b
    return abs(a)


def _lcm(a: int, b: int) -> int:
    return abs(a * b) // _gcd(a, b) if a and b else 0


def _factor_result_to_ir(
    content_val: int,
    factors: list[tuple[list[int], int]],
    x: IRSymbol,
) -> IRNode:
    """Assemble ``Mul(content, Pow(f1, m1), Pow(f2, m2), …)`` as IR."""
    parts: list[IRNode] = []

    if content_val != 1:
        parts.append(IRInteger(content_val))

    for poly_coeffs, mult in factors:
        factor_ir = _linear_poly_to_ir(poly_coeffs, x)
        if mult == 1:
            parts.append(factor_ir)
        else:
            parts.append(IRApply(POW, (factor_ir, IRInteger(mult))))

    if not parts:
        return IRInteger(content_val)
    if len(parts) == 1:
        return parts[0]
    # Fold into left-associative binary Mul chain.
    acc: IRNode = parts[0]
    for p in parts[1:]:
        acc = IRApply(MUL, (acc, p))
    return acc


def _linear_poly_to_ir(coeffs: list[int], x: IRSymbol) -> IRNode:
    """Convert a small coefficient list to IR.

    Covers the linear case ``[b, a]`` → ``a*x + b``.  Higher degrees
    fall through to a generic builder.
    """
    if len(coeffs) == 1:
        return IRInteger(coeffs[0])
    if len(coeffs) == 2:
        b, a = coeffs
        # Build ``a*x``
        if a == 1:
            ax: IRNode = x
        elif a == -1:
            ax = IRApply(NEG, (x,))
        else:
            ax = IRApply(MUL, (IRInteger(a), x))
        if b == 0:
            return ax
        if b > 0:
            return IRApply(ADD, (ax, IRInteger(b)))
        # b < 0: emit Sub(a*x, |b|)
        from symbolic_ir import SUB

        return IRApply(SUB, (ax, IRInteger(-b)))
    # Generic: build Add chain for each term.
    terms: list[IRNode] = []
    for i, c in enumerate(coeffs):
        if c == 0:
            continue
        if i == 0:
            terms.append(IRInteger(c))
        elif i == 1:
            if c == 1:
                terms.append(x)
            elif c == -1:
                terms.append(IRApply(NEG, (x,)))
            else:
                terms.append(IRApply(MUL, (IRInteger(c), x)))
        else:
            power: IRNode = IRApply(POW, (x, IRInteger(i)))
            if c == 1:
                terms.append(power)
            elif c == -1:
                terms.append(IRApply(NEG, (power,)))
            else:
                terms.append(IRApply(MUL, (IRInteger(c), power)))
    if not terms:
        return IRInteger(0)
    from symbolic_ir import ADD as _ADD

    acc2: IRNode = terms[0]
    for t in terms[1:]:
        acc2 = IRApply(_ADD, (acc2, t))
    return acc2


# ---------------------------------------------------------------------------
# Solve
# ---------------------------------------------------------------------------


def solve_handler(_vm: VM, expr: IRApply) -> IRNode:
    """``Solve(equation, var)`` — solve polynomial equations and linear systems.

    Two call forms are handled:

    1. ``Solve(equation, var)`` — single-variable polynomial, up to degree 4.
       ``equation`` may be any IR expression treated as ``expr = 0``, or an
       ``Equal(lhs, rhs)`` node rewritten to ``Sub(lhs, rhs) = 0``.

    2. ``Solve(List(eq1, eq2, …), List(x, y, …))`` — linear system of
       equations solved by Gaussian elimination (MACSYMA's ``linsolve``
       also compiles to this form).  Returns ``List(Rule(x, val), …)``.

    Returns ``List(solution, …)`` or unevaluated on failure.
    """
    if len(expr.args) != 2:
        return expr
    eq_ir, var_ir = expr.args

    # ------------------------------------------------------------------
    # Branch 1 — linear system: both args are List nodes.
    # ------------------------------------------------------------------
    if (
        isinstance(eq_ir, IRApply)
        and isinstance(eq_ir.head, IRSymbol)
        and eq_ir.head.name == "List"
        and isinstance(var_ir, IRApply)
        and isinstance(var_ir.head, IRSymbol)
        and var_ir.head.name == "List"
    ):
        equations = list(eq_ir.args)
        variables: list[IRSymbol] = []
        for v in var_ir.args:
            if not isinstance(v, IRSymbol):
                return expr
            variables.append(v)
        if not equations or not variables:
            return expr
        sol = solve_linear_system(equations, variables)
        if sol is None:
            return expr
        return IRApply(LIST, tuple(sol))

    # ------------------------------------------------------------------
    # Branch 2 — single-variable polynomial equation.
    # ------------------------------------------------------------------
    if not isinstance(var_ir, IRSymbol):
        return expr

    poly_ir = _unwrap_equation(eq_ir)
    result = to_rational(poly_ir, var_ir)
    if result is None:
        return expr
    num, den = result
    # Require a pure polynomial (denominator = constant 1).
    if den != (Fraction(1),):
        return expr

    coeffs = list(num)
    deg = len(coeffs) - 1

    if deg < 0:
        return expr  # zero polynomial — degenerate
    if deg == 0:
        # Constant equation: 0 solutions.
        return IRApply(LIST, ())

    if deg == 1:
        solutions: list[IRNode] | str = solve_linear(
            Fraction(coeffs[1]), Fraction(coeffs[0])
        )
    elif deg == 2:
        solutions = solve_quadratic(
            Fraction(coeffs[2]),
            Fraction(coeffs[1]),
            Fraction(coeffs[0]),
        )
    elif deg == 3:
        solutions = solve_cubic(
            Fraction(coeffs[3]),
            Fraction(coeffs[2]),
            Fraction(coeffs[1]),
            Fraction(coeffs[0]),
        )
    elif deg == 4:
        solutions = solve_quartic(
            Fraction(coeffs[4]),
            Fraction(coeffs[3]),
            Fraction(coeffs[2]),
            Fraction(coeffs[1]),
            Fraction(coeffs[0]),
        )
    else:
        # Degree > 4 — unevaluated.
        return expr

    if solutions == ALL:
        return IRApply(LIST, ())  # all reals — represent as empty list for now
    if isinstance(solutions, list) and not solutions:
        # Empty list means the solver couldn't find closed-form roots
        # (e.g. casus irreducibilis for cubics) — return unevaluated.
        return expr
    assert isinstance(solutions, list)
    return IRApply(LIST, tuple(solutions))


def _unwrap_equation(eq_ir: IRNode) -> IRNode:
    """If ``eq_ir`` is ``Equal(lhs, rhs)``, return ``Sub(lhs, rhs)``.

    Otherwise return ``eq_ir`` unchanged (treat as ``= 0`` expression).
    """
    if (
        isinstance(eq_ir, IRApply)
        and isinstance(eq_ir.head, IRSymbol)
        and eq_ir.head.name == "Equal"
        and len(eq_ir.args) == 2
    ):
        from symbolic_ir import SUB

        return IRApply(SUB, eq_ir.args)
    return eq_ir


# ---------------------------------------------------------------------------
# List operations
# ---------------------------------------------------------------------------


def length_handler(_vm: VM, expr: IRApply) -> IRNode:
    """``Length(list)`` — number of elements."""
    if len(expr.args) != 1:
        return expr
    try:
        return length(expr.args[0])
    except ListOperationError:
        return expr


def first_handler(_vm: VM, expr: IRApply) -> IRNode:
    """``First(list)`` — first element."""
    if len(expr.args) != 1:
        return expr
    try:
        return first(expr.args[0])
    except ListOperationError:
        return expr


def rest_handler(_vm: VM, expr: IRApply) -> IRNode:
    """``Rest(list)`` — all but the first element."""
    if len(expr.args) != 1:
        return expr
    try:
        return rest(expr.args[0])
    except ListOperationError:
        return expr


def last_handler(_vm: VM, expr: IRApply) -> IRNode:
    """``Last(list)`` — last element."""
    if len(expr.args) != 1:
        return expr
    try:
        return last(expr.args[0])
    except ListOperationError:
        return expr


def append_handler(_vm: VM, expr: IRApply) -> IRNode:
    """``Append(list1, list2, …)`` — concatenate lists."""
    if len(expr.args) < 2:
        return expr
    try:
        return append(*expr.args)
    except ListOperationError:
        return expr


def reverse_handler(_vm: VM, expr: IRApply) -> IRNode:
    """``Reverse(list)`` — reverse the list."""
    if len(expr.args) != 1:
        return expr
    try:
        return reverse(expr.args[0])
    except ListOperationError:
        return expr


def range_handler(_vm: VM, expr: IRApply) -> IRNode:
    """``Range(n)`` or ``Range(start, stop)`` or ``Range(start, stop, step)``.

    Mirrors MACSYMA's ``makelist`` convention: single-arg form generates
    ``[1, 2, …, n]``.
    """
    try:
        if len(expr.args) == 1:
            arg = expr.args[0]
            if not isinstance(arg, IRInteger):
                return expr
            return range_(arg.value)
        if len(expr.args) == 2:
            a, b = expr.args
            if not isinstance(a, IRInteger) or not isinstance(b, IRInteger):
                return expr
            return range_(a.value, b.value)
        if len(expr.args) == 3:
            a, b, s = expr.args
            if (
                not isinstance(a, IRInteger)
                or not isinstance(b, IRInteger)
                or not isinstance(s, IRInteger)
            ):
                return expr
            return range_(a.value, b.value, s.value)
        return expr
    except ListOperationError:
        return expr


def map_handler(vm: VM, expr: IRApply) -> IRNode:
    """``Map(f, list)`` — apply ``f`` to every element through the VM.

    Each application ``f(elem)`` is evaluated by the VM so that
    ``Map(sin, [0])`` gives ``[0.0]`` rather than ``[Sin(0)]``.
    """
    if len(expr.args) != 2:
        return expr
    f, lst = expr.args
    try:
        mapped = map_(f, lst)
        # Evaluate each element.
        evaluated = tuple(vm.eval(e) for e in mapped.args)
        return IRApply(LIST, evaluated)
    except ListOperationError:
        return expr


def apply_handler(vm: VM, expr: IRApply) -> IRNode:
    """``Apply(f, list)`` — call ``f`` with the list's elements as args."""
    if len(expr.args) != 2:
        return expr
    f, lst = expr.args
    try:
        applied = apply_(f, lst)
        return vm.eval(applied)
    except ListOperationError:
        return expr


def select_handler(vm: VM, expr: IRApply) -> IRNode:
    """``Select(pred, list)`` — keep elements where ``pred(elem)`` is truthy.

    The predicate is applied through the VM; an element is kept when the
    result is the symbol ``True``.
    """
    if len(expr.args) != 2:
        return expr
    pred, lst = expr.args
    try:

        def _pred(elem: IRNode) -> bool:
            result = vm.eval(IRApply(pred, (elem,)))
            return isinstance(result, IRSymbol) and result.name == "True"

        return select(lst, _pred)
    except ListOperationError:
        return expr


def sort_handler(_vm: VM, expr: IRApply) -> IRNode:
    """``Sort(list)`` — sort by canonical repr ordering."""
    if len(expr.args) != 1:
        return expr
    try:
        return sort_(expr.args[0])
    except ListOperationError:
        return expr


def part_handler(_vm: VM, expr: IRApply) -> IRNode:
    """``Part(list, index)`` — 1-based element access."""
    if len(expr.args) != 2:
        return expr
    lst, idx = expr.args
    if not isinstance(idx, IRInteger):
        return expr
    try:
        return part(lst, idx.value)
    except ListOperationError:
        return expr


def flatten_handler(_vm: VM, expr: IRApply) -> IRNode:
    """``Flatten(list)`` — one level of nested-list expansion."""
    if len(expr.args) != 1:
        return expr
    try:
        return flatten(expr.args[0])
    except ListOperationError:
        return expr


def join_handler(_vm: VM, expr: IRApply) -> IRNode:
    """``Join(list1, list2, …)`` — Mathematica-spelling append."""
    if len(expr.args) < 2:
        return expr
    try:
        return join(*expr.args)
    except ListOperationError:
        return expr


# ---------------------------------------------------------------------------
# Matrix operations
# ---------------------------------------------------------------------------


def _as_row_args(row: IRNode) -> list[IRNode] | None:
    """Return the elements of a ``List(…)`` row, or ``None``."""
    if (
        isinstance(row, IRApply)
        and isinstance(row.head, IRSymbol)
        and row.head.name == "List"
    ):
        return list(row.args)
    return None


def matrix_handler(_vm: VM, expr: IRApply) -> IRNode:
    """``Matrix(List(…), List(…), …)`` — validate shape and return as-is.

    The IR representation *is* the matrix; this handler just validates
    that every row has the same width and the input is non-empty.
    """
    try:
        rows = []
        for arg in expr.args:
            row = _as_row_args(arg)
            if row is None:
                return expr
            rows.append(row)
        return matrix(rows)
    except MatrixError:
        return expr


def transpose_handler(_vm: VM, expr: IRApply) -> IRNode:
    """``Transpose(matrix)`` — transpose the matrix."""
    if len(expr.args) != 1:
        return expr
    try:
        return transpose(expr.args[0])
    except (MatrixError, ValueError):
        return expr


def determinant_handler(vm: VM, expr: IRApply) -> IRNode:
    """``Determinant(matrix)`` — compute the determinant (exact, symbolic).

    The :mod:`cas_matrix` substrate returns the determinant as a symbolic
    IR expression (e.g. ``Sub(Mul(1, 4), Mul(2, 3))``).  We pass it
    through the VM so numeric entries fold to a concrete integer/rational
    (e.g. ``IRInteger(-2)``).
    """
    if len(expr.args) != 1:
        return expr
    try:
        raw = determinant(expr.args[0])
        return vm.eval(raw)
    except (MatrixError, ValueError):
        return expr


def inverse_handler(_vm: VM, expr: IRApply) -> IRNode:
    """``Inverse(matrix)`` — compute the matrix inverse."""
    if len(expr.args) != 1:
        return expr
    try:
        return inverse(expr.args[0])
    except (MatrixError, ValueError):
        return expr


# ---------------------------------------------------------------------------
# Limit and Taylor
# ---------------------------------------------------------------------------


def limit_handler(vm: VM, expr: IRApply) -> IRNode:
    """``Limit(body, var, point)`` — direct-substitution limit.

    The substituted result is simplified and re-evaluated so that
    ``Limit(x^2, x, 3)`` collapses to ``9`` rather than ``Add(9, 0)``.
    """
    if len(expr.args) != 3:
        return expr
    body, var, point = expr.args
    if not isinstance(var, IRSymbol):
        return expr
    result = limit_direct(body, var, point)
    simplified = simplify(vm.eval(result))
    return simplified


def taylor_handler(_vm: VM, expr: IRApply) -> IRNode:
    """``Taylor(body, var, point, order)`` — polynomial Taylor expansion."""
    if len(expr.args) != 4:
        return expr
    body, var, point, order_ir = expr.args
    if not isinstance(var, IRSymbol):
        return expr
    if not isinstance(order_ir, IRInteger):
        return expr
    try:
        return taylor_polynomial(body, var, point, order_ir.value)
    except PolynomialError:
        return expr


# ---------------------------------------------------------------------------
# Numeric / arithmetic
# ---------------------------------------------------------------------------


def abs_handler(_vm: VM, expr: IRApply) -> IRNode:
    """``Abs(x)`` — absolute value."""
    if len(expr.args) != 1:
        return expr
    n = to_number(expr.args[0])
    if n is None:
        return expr
    return from_number(abs(n))


def floor_handler(_vm: VM, expr: IRApply) -> IRNode:
    """``Floor(x)`` — floor function."""
    if len(expr.args) != 1:
        return expr
    n = to_number(expr.args[0])
    if n is None:
        return expr
    return IRInteger(math.floor(float(n)))


def ceiling_handler(_vm: VM, expr: IRApply) -> IRNode:
    """``Ceiling(x)`` — ceiling function."""
    if len(expr.args) != 1:
        return expr
    n = to_number(expr.args[0])
    if n is None:
        return expr
    return IRInteger(math.ceil(float(n)))


def mod_handler(_vm: VM, expr: IRApply) -> IRNode:
    """``Mod(a, b)`` — modulo."""
    if len(expr.args) != 2:
        return expr
    na = to_number(expr.args[0])
    nb = to_number(expr.args[1])
    if na is None or nb is None or nb == 0:
        return expr
    result = Fraction(na) % Fraction(nb)
    return from_number(result)


def gcd_handler(_vm: VM, expr: IRApply) -> IRNode:
    """``Gcd(a, b)`` — greatest common divisor (integer args only)."""
    if len(expr.args) != 2:
        return expr
    a, b = expr.args
    if not isinstance(a, IRInteger) or not isinstance(b, IRInteger):
        return expr
    return IRInteger(math.gcd(a.value, b.value))


def lcm_handler(_vm: VM, expr: IRApply) -> IRNode:
    """``Lcm(a, b)`` — least common multiple (integer args only)."""
    if len(expr.args) != 2:
        return expr
    a, b = expr.args
    if not isinstance(a, IRInteger) or not isinstance(b, IRInteger):
        return expr
    return IRInteger(math.lcm(a.value, b.value))


# ---------------------------------------------------------------------------
# Handler-table builder
# ---------------------------------------------------------------------------


def build_cas_handler_table() -> dict[str, Handler]:
    """Return the complete CAS handler dispatch table.

    Keys are the canonical IR head names (string).  Values are the
    handler functions defined in this module.  Merge this into the
    backend's ``_handlers`` dict on startup.
    """
    return {
        # Simplify / expand
        "Simplify": simplify_handler,
        "Expand": expand_handler,
        # Substitution
        "Subst": subst_handler,
        # Factor
        "Factor": factor_handler,
        # Solve
        "Solve": solve_handler,
        # List operations
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
        # Matrix
        "Matrix": matrix_handler,
        "Transpose": transpose_handler,
        "Determinant": determinant_handler,
        "Inverse": inverse_handler,
        # Limit / Taylor
        "Limit": limit_handler,
        "Taylor": taylor_handler,
        # Numeric
        "Abs": abs_handler,
        "Floor": floor_handler,
        "Ceiling": ceiling_handler,
        "Mod": mod_handler,
        "Gcd": gcd_handler,
        "Lcm": lcm_handler,
    }
