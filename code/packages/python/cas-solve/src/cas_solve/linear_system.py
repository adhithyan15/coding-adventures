"""Linear-system solver: Gaussian elimination with Fraction coefficients.

Solves ``n`` linear equations in ``n`` unknowns, returning exact rational
solutions as a list of ``Rule(var, value)`` IR nodes.

Algorithm
---------
Standard Gaussian elimination with partial pivoting on an augmented
matrix of :class:`fractions.Fraction` entries (exact arithmetic).

Input normalisation
-------------------
Each element of ``equations`` may be:
- ``Equal(lhs, rhs)`` — treated as ``lhs - rhs = 0``.
- Any other IR node — treated as ``expr = 0``.

Each normalised expression is linearised by :func:`_linear_eval`, which
walks the IR tree and extracts rational coefficients for each variable.
Non-linear terms (products of two variables, ``Pow(x, 2)``, etc.) cause
the solver to return ``None``.
"""

from __future__ import annotations

from fractions import Fraction

from symbolic_ir import IRApply, IRInteger, IRNode, IRRational, IRSymbol

LIST_SYMBOL = IRSymbol("List")
RULE_SYMBOL = IRSymbol("Rule")


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def solve_linear_system(
    equations: list[IRNode],
    variables: list[IRSymbol],
) -> list[IRNode] | None:
    """Solve a system of linear equations by Gaussian elimination.

    Parameters
    ----------
    equations:
        A list of IR nodes (``Equal(lhs, rhs)`` or ``expr = 0`` form).
    variables:
        The variables to solve for, in column order.

    Returns
    -------
    A list of ``Rule(var, value)`` IR nodes when there is a unique
    rational solution; ``None`` otherwise (non-linear, singular, or
    under/over-determined system).
    """
    n = len(variables)
    if len(equations) != n or n == 0:
        return None

    var_names = {v.name: i for i, v in enumerate(variables)}

    # Build augmented matrix [A | b]: row i is [c0, c1, ..., cn-1, rhs]
    mat: list[list[Fraction]] = []
    for eq in equations:
        row = _equation_to_row(eq, var_names, n)
        if row is None:
            return None
        mat.append(row)

    # Forward elimination with partial pivoting
    for col in range(n):
        # Find the pivot row (largest absolute value in column >= col)
        pivot_row = max(range(col, n), key=lambda r: abs(mat[r][col]))
        if mat[pivot_row][col] == 0:
            return None  # Singular matrix → no unique solution
        mat[col], mat[pivot_row] = mat[pivot_row], mat[col]

        pivot = mat[col][col]
        for row in range(col + 1, n):
            if mat[row][col] == 0:
                continue
            factor = mat[row][col] / pivot
            for j in range(col, n + 1):
                mat[row][j] -= factor * mat[col][j]

    # Back substitution
    solution: list[Fraction] = [Fraction(0)] * n
    for i in range(n - 1, -1, -1):
        if mat[i][i] == 0:
            return None
        rhs = mat[i][n]
        for j in range(i + 1, n):
            rhs -= mat[i][j] * solution[j]
        solution[i] = rhs / mat[i][i]

    # Return List(Rule(var, val), ...)
    return [
        IRApply(RULE_SYMBOL, (var, _frac_to_ir(val)))
        for var, val in zip(variables, solution)
    ]


# ---------------------------------------------------------------------------
# Internal helpers
# ---------------------------------------------------------------------------


def _equation_to_row(
    eq: IRNode, var_names: dict[str, int], n: int
) -> list[Fraction] | None:
    """Normalise ``eq`` and extract the augmented-matrix row.

    Returns ``[c0, c1, ..., cn-1, rhs]`` (rhs on the right) or ``None``
    if the expression is not linear in the named variables.
    """
    # Equal(lhs, rhs) → lhs - rhs
    if (
        isinstance(eq, IRApply)
        and isinstance(eq.head, IRSymbol)
        and eq.head.name == "Equal"
        and len(eq.args) == 2
    ):
        SUB = IRSymbol("Sub")
        expr: IRNode = IRApply(SUB, (eq.args[0], eq.args[1]))
    else:
        expr = eq

    result = _linear_eval(expr, var_names, n)
    if result is None:
        return None
    var_coeffs, const = result
    # Row: coefficients in columns 0..n-1, RHS = -const
    return list(var_coeffs) + [Fraction(-const)]


def _node_to_fraction(node: IRNode) -> Fraction | None:
    """Return the Fraction value of a constant IR node, or ``None``."""
    if isinstance(node, IRInteger):
        return Fraction(node.value)
    if isinstance(node, IRRational):
        return Fraction(node.numer, node.denom)
    return None


def _linear_eval(
    node: IRNode, var_names: dict[str, int], n: int
) -> tuple[list[Fraction], Fraction] | None:
    """Parse ``node`` as a linear polynomial in the named variables.

    Returns ``(coefficients, constant)`` where ``coefficients[i]`` is the
    coefficient of variable ``i``.  Returns ``None`` for non-linear or
    unrecognised expressions.
    """

    def zero() -> list[Fraction]:
        return [Fraction(0)] * n

    # Numeric constants
    fval = _node_to_fraction(node)
    if fval is not None:
        return zero(), fval

    # Variables
    if isinstance(node, IRSymbol):
        if node.name in var_names:
            c = zero()
            c[var_names[node.name]] = Fraction(1)
            return c, Fraction(0)
        return None  # Unknown symbol (e.g. %pi) → treat as non-linear

    if not isinstance(node, IRApply):
        return None

    head = node.head.name if isinstance(node.head, IRSymbol) else ""

    # Add: sum all children
    if head == "Add":
        total_c = zero()
        total_k = Fraction(0)
        for arg in node.args:
            r = _linear_eval(arg, var_names, n)
            if r is None:
                return None
            c, k = r
            for i in range(n):
                total_c[i] += c[i]
            total_k += k
        return total_c, total_k

    # Sub: first − second
    if head == "Sub" and len(node.args) == 2:
        r0 = _linear_eval(node.args[0], var_names, n)
        r1 = _linear_eval(node.args[1], var_names, n)
        if r0 is None or r1 is None:
            return None
        c0, k0 = r0
        c1, k1 = r1
        return [c0[i] - c1[i] for i in range(n)], k0 - k1

    # Neg: negate
    if head == "Neg" and len(node.args) == 1:
        r = _linear_eval(node.args[0], var_names, n)
        if r is None:
            return None
        c, k = r
        return [-c[i] for i in range(n)], -k

    # Mul: product — at most one non-constant factor allowed
    if head == "Mul":
        scalar = Fraction(1)
        linear_part: tuple[list[Fraction], Fraction] | None = None
        for arg in node.args:
            fv = _node_to_fraction(arg)
            if fv is not None:
                scalar *= fv
            else:
                r = _linear_eval(arg, var_names, n)
                if r is None:
                    return None
                c, k = r
                if all(ci == Fraction(0) for ci in c):
                    # Purely constant
                    scalar *= k
                else:
                    if linear_part is not None:
                        return None  # x*y — non-linear
                    linear_part = (c, k)
        if linear_part is None:
            return zero(), scalar
        lc, lk = linear_part
        return [lc[i] * scalar for i in range(n)], lk * scalar

    # Pow: only Pow(expr, 0) and Pow(expr, 1) are linear
    if head == "Pow" and len(node.args) == 2:
        exp = node.args[1]
        if isinstance(exp, IRInteger):
            if exp.value == 0:
                return zero(), Fraction(1)
            if exp.value == 1:
                return _linear_eval(node.args[0], var_names, n)
        return None  # x^2, x^(1/2), etc. — non-linear

    # Anything else (Sin, Cos, ...) is non-linear in variables
    return None


def _frac_to_ir(f: Fraction) -> IRNode:
    """Convert a :class:`~fractions.Fraction` to an ``IRInteger`` or
    ``IRRational``."""
    if f.denominator == 1:
        return IRInteger(f.numerator)
    return IRRational(f.numerator, f.denominator)
