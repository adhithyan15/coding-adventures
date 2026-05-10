"""Eigenvalues, eigenvectors, and characteristic polynomial.

This module implements three closely related operations for square
matrices with rational entries.

Characteristic polynomial
-------------------------
The characteristic polynomial of an n×n matrix A is:

    charpoly(A, λ) = det(λI − A)

It is a monic polynomial of degree n whose roots are exactly the
eigenvalues of A (with multiplicity).

Algorithm
~~~~~~~~~
Each entry of the matrix (λI − A) is a *polynomial in λ*:

- Diagonal position (i, i): ``λ − aᵢᵢ``  →  coefficients ``[−aᵢᵢ, 1]``
- Off-diagonal (i, j):       ``−aᵢⱼ``    →  coefficients ``[−aᵢⱼ]``

Polynomials are represented as ``list[Fraction]`` in ascending power
order: ``p[k]`` is the coefficient of ``λ^k``.

The determinant is computed by the same cofactor expansion used by
:mod:`cas_matrix.determinant`, but now operating on polynomial-valued
entries rather than IR nodes.  The result is a list of Fraction
coefficients.

Eigenvalues
-----------
Once the characteristic polynomial ``c₀ + c₁λ + … + cₙλⁿ = 0`` is in
hand, its roots give the eigenvalues.  We delegate root-finding to
``cas_solve``:

- n = 1: ``solve_linear``
- n = 2: ``solve_quadratic``
- n = 3: ``solve_cubic``
- n = 4: ``solve_quartic``
- n > 4: return ``None`` (unevaluated; numerical root-finding not in scope)

The return format matches MACSYMA's ``eigenvalues`` output:
``List(List(λ₁, m₁), List(λ₂, m₂), …)`` where ``mᵢ`` is the
algebraic multiplicity.

Multiplicities are computed by comparing eigenvalue IR nodes numerically
at a single test point (same strategy used in cas-ode's exactness check).
Two eigenvalues are considered equal when their floating-point
evaluations agree to within ``1e-9``.

Eigenvectors
------------
For each eigenvalue λᵢ:

1. If λᵢ can be evaluated to a Fraction (i.e., it is ``IRInteger`` or
   ``IRRational``): form the matrix B = A − λᵢI, run RREF, extract the
   null-space basis.
2. Otherwise (irrational / complex eigenvalue): return ``List()`` for
   that eigenvalue's vector list — symbolic null-space computation is
   out of scope for Phase 19.

The null-space basis is computed from the RREF:

- Free columns = non-pivot columns.
- For each free column j: build a vector v where v[j]=1, v[free_k]=0
  (k≠j), and v[pivot_col_c] = −RREF[pivot_row_c, j].

Each eigenvector is returned as a column-vector ``Matrix`` IR node.

Literate reading order
-----------------------
1. ``_poly_*``  — polynomial-coefficient arithmetic helpers
2. ``_det_poly`` — cofactor determinant on polynomial entries
3. ``char_poly_coeffs`` — build characteristic polynomial coefficients
4. ``charpoly`` — IR form for the user (Add/Mul/Pow tree)
5. ``_eval_eigenvalue_float`` — numeric evaluation for multiplicity
6. ``eigenvalues`` — characteristic polynomial → roots → grouped list
7. ``_ir_to_fraction`` — convert IRInteger/IRRational to Fraction
8. ``_null_space_fractions`` — RREF-based null-space computation
9. ``eigenvectors`` — per-eigenvalue null-space basis
"""

from __future__ import annotations

from fractions import Fraction

from cas_solve.cubic import solve_cubic
from cas_solve.linear import solve_linear
from cas_solve.quadratic import solve_quadratic
from symbolic_ir import (
    ADD,
    DIV,
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

from cas_matrix.matrix import MatrixError, matrix, num_cols, num_rows
from cas_matrix.rowreduce import _frac_to_ir, _matrix_to_fractions

# ---------------------------------------------------------------------------
# Section 1 — Polynomial arithmetic helpers
# ---------------------------------------------------------------------------
#
# A polynomial p(λ) is stored as a list of Fraction values where p[k] is the
# coefficient of λ^k.  The list is "dense" — every power from 0 to deg(p) is
# present (some coefficients may be zero).
#
# Why lists of Fractions instead of dicts or symbolic IR?  Because we only
# ever evaluate characteristic polynomials of *rational* matrices (the only
# kind our row-reduction routines support).  Keeping everything in exact
# Fraction arithmetic avoids any interaction with the VM evaluator and makes
# the code self-contained.


def _poly_add(p: list[Fraction], q: list[Fraction]) -> list[Fraction]:
    """Add two polynomials coefficient-by-coefficient.

    If the lists differ in length the shorter one is treated as padded
    with zeros on the right.

    Parameters
    ----------
    p, q:
        Coefficient lists in ascending power order.

    Returns
    -------
    Coefficient list of p + q.

    Examples
    --------
    ::

        _poly_add([1, 2], [3])        # [4, 2]  (1+3 + 2*λ)
        _poly_add([1], [0, 1])        # [1, 1]  (1 + λ)
    """
    n = max(len(p), len(q))
    return [
        (p[i] if i < len(p) else Fraction(0))
        + (q[i] if i < len(q) else Fraction(0))
        for i in range(n)
    ]


def _poly_neg(p: list[Fraction]) -> list[Fraction]:
    """Negate every coefficient.

    Parameters
    ----------
    p:
        Coefficient list.

    Returns
    -------
    ``[−p[0], −p[1], …]``.
    """
    return [-c for c in p]


def _poly_sub(p: list[Fraction], q: list[Fraction]) -> list[Fraction]:
    """Subtract q from p (coefficient-by-coefficient).

    Equivalent to ``_poly_add(p, _poly_neg(q))``.

    Parameters
    ----------
    p, q:
        Coefficient lists in ascending power order.

    Returns
    -------
    Coefficient list of p − q.
    """
    return _poly_add(p, _poly_neg(q))


def _poly_mul(p: list[Fraction], q: list[Fraction]) -> list[Fraction]:
    """Multiply two polynomials.

    Uses the standard O(n·m) convolution.  Returns ``[0]`` if either
    input is empty.

    Parameters
    ----------
    p, q:
        Coefficient lists in ascending power order.

    Returns
    -------
    Coefficient list of p · q with degree deg(p) + deg(q).

    Examples
    --------
    ::

        # (λ − 1)(λ − 2) = λ² − 3λ + 2
        _poly_mul([-1, 1], [-2, 1])   # [2, -3, 1]
    """
    if not p or not q:
        return [Fraction(0)]
    result = [Fraction(0)] * (len(p) + len(q) - 1)
    for i, a in enumerate(p):
        for j, b in enumerate(q):
            result[i + j] += a * b
    return result


# ---------------------------------------------------------------------------
# Section 2 — Cofactor determinant on polynomial-valued entries
# ---------------------------------------------------------------------------
#
# This mirrors the cofactor expansion in ``cas_matrix.determinant._det`` but
# operates on polynomial lists rather than IR nodes.  The recursion is:
#
#   det(M) = sum_j  (−1)^j * M[0,j] * det(minor(M,0,j))
#
# where M[0,j] is now a polynomial and det(minor) is also a polynomial.
# Multiplying two polynomials gives the correct convolution of coefficients.


def _det_poly(
    rows: list[list[list[Fraction]]],
) -> list[Fraction]:
    """Compute det of a polynomial-valued matrix via cofactor expansion.

    Each entry ``rows[i][j]`` is a coefficient list (ascending powers of λ).
    Returns the coefficient list of the resulting determinant polynomial.

    The algorithm is identical to :func:`cas_matrix.determinant._det` but
    uses ``_poly_mul``, ``_poly_add``, ``_poly_sub`` instead of IR operations.

    Complexity: O(n!) — suitable for n ≤ 6.

    Parameters
    ----------
    rows:
        Square list-of-lists; each inner element is a polynomial (list of
        Fraction).

    Returns
    -------
    Coefficient list of ``det(rows)``.
    """
    n = len(rows)
    if n == 0:
        return [Fraction(1)]  # det of 0×0 matrix is 1 by convention
    if n == 1:
        return list(rows[0][0])
    if n == 2:
        # det([[a,b],[c,d]]) = a*d - b*c
        a, b = rows[0][0], rows[0][1]
        c, d = rows[1][0], rows[1][1]
        return _poly_sub(_poly_mul(a, d), _poly_mul(b, c))
    result: list[Fraction] = [Fraction(0)]
    for j, entry in enumerate(rows[0]):
        # Build the minor: delete row 0 and column j.
        minor = [
            [rows[r][col] for col in range(n) if col != j]
            for r in range(1, n)
        ]
        sub = _det_poly(minor)
        product = _poly_mul(entry, sub)
        if j % 2 == 0:
            result = _poly_add(result, product)
        else:
            result = _poly_sub(result, product)
    return result


# ---------------------------------------------------------------------------
# Section 3 — Characteristic polynomial computation
# ---------------------------------------------------------------------------


def char_poly_coeffs(M: IRNode) -> list[Fraction]:
    """Return the coefficients of the characteristic polynomial det(λI − A).

    The characteristic polynomial is:

        p(λ) = det(λI − A) = λⁿ + cₙ₋₁λⁿ⁻¹ + … + c₁λ + c₀

    Note that the convention here is ``det(λI − A)`` (not ``det(A − λI)``),
    which gives a **monic** polynomial (leading coefficient = +1).

    Parameters
    ----------
    M:
        Square matrix IR node with ``IRInteger`` / ``IRRational`` entries.

    Returns
    -------
    Ascending-power coefficient list ``[c₀, c₁, …, cₙ]`` where
    ``p(λ) = Σ cₖ λᵏ``.

    Raises
    ------
    MatrixError
        If ``M`` is not square or has symbolic entries.

    Examples
    --------
    ::

        A = matrix([[IRInteger(1), IRInteger(2)],
                    [IRInteger(2), IRInteger(1)]])
        char_poly_coeffs(A)   # [Fraction(-3), Fraction(-2), Fraction(1)]
        # i.e. λ² − 2λ − 3 = (λ−3)(λ+1)
    """
    n = num_rows(M)
    if n != num_cols(M):
        raise MatrixError(
            f"char_poly_coeffs: matrix must be square, got "
            f"{n}×{num_cols(M)}"
        )
    frows = _matrix_to_fractions(M)

    # Build polynomial-valued entries of (λI − A):
    #   diagonal:    λ − aᵢᵢ  →  [−aᵢᵢ, 1]
    #   off-diagonal: −aᵢⱼ    →  [−aᵢⱼ]
    poly_rows: list[list[list[Fraction]]] = []
    for i in range(n):
        row: list[list[Fraction]] = []
        for j in range(n):
            if i == j:
                # λ - aᵢᵢ
                row.append([-frows[i][j], Fraction(1)])
            else:
                # -aᵢⱼ
                row.append([-frows[i][j]])
        poly_rows.append(row)

    return _det_poly(poly_rows)


# ---------------------------------------------------------------------------
# Section 4 — IR form of characteristic polynomial
# ---------------------------------------------------------------------------


def _frac_to_ir_local(f: Fraction) -> IRNode:
    """Convert Fraction to canonical IR literal (local helper)."""
    if f.denominator == 1:
        return IRInteger(f.numerator)
    return IRRational(f.numerator, f.denominator)


def charpoly(M: IRNode, lam: IRSymbol) -> IRNode:
    """Return the characteristic polynomial ``det(λI − A)`` as an IR tree.

    The result is an ``Add``/``Mul``/``Pow`` expression in the symbol
    ``lam``.  Coefficients are ``IRInteger`` or ``IRRational``.

    Parameters
    ----------
    M:
        Square matrix with rational entries.
    lam:
        The symbolic variable for the polynomial (e.g. ``IRSymbol("lambda")``).

    Returns
    -------
    An IR expression representing the characteristic polynomial.

    Raises
    ------
    MatrixError
        If ``M`` is not square or has symbolic entries.

    Examples
    --------
    ::

        A = matrix([[IRInteger(4), IRInteger(2)],
                    [IRInteger(1), IRInteger(3)]])
        lam = IRSymbol("lambda")
        charpoly(A, lam)
        # (lambda^2 - 7*lambda + 10)
        # i.e. (λ−5)(λ−2)
    """
    coeffs = char_poly_coeffs(M)
    # Build the polynomial as an IR Add tree:
    #   c₀ + c₁·λ + c₂·λ² + … + cₙ·λⁿ
    terms: list[IRNode] = []
    for k, c in enumerate(coeffs):
        if c == 0:
            continue
        if k == 0:
            # constant term
            terms.append(_frac_to_ir_local(c))
        elif k == 1:
            # c·λ
            c_ir = _frac_to_ir_local(c)
            if c == Fraction(1):
                terms.append(lam)
            else:
                terms.append(IRApply(MUL, (c_ir, lam)))
        else:
            # c·λ^k
            power = IRApply(POW, (lam, IRInteger(k)))
            c_ir = _frac_to_ir_local(c)
            if c == Fraction(1):
                terms.append(power)
            else:
                terms.append(IRApply(MUL, (c_ir, power)))
    if not terms:
        return IRInteger(0)
    if len(terms) == 1:
        return terms[0]
    # Fold into a left-associative Add tree.
    result = terms[0]
    for t in terms[1:]:
        result = IRApply(ADD, (result, t))
    return result


# ---------------------------------------------------------------------------
# Section 5 — Numeric eigenvalue evaluation for multiplicity grouping
# ---------------------------------------------------------------------------
#
# After the polynomial solver returns a list of IR roots, we need to group
# equal roots and count multiplicity.  The solvers *deduplicate* repeated
# roots: ``solve_quadratic(1, -4, 4)`` returns ``[2]`` (one root), not
# ``[2, 2]``.  So we cannot rely on the solver's list length alone.
#
# Strategy:
# 1. Group returned roots by float proximity (tol 1e-9).
# 2. For each distinct group, use the **derivative test** on the char poly
#    to determine the true algebraic multiplicity: the multiplicity of root
#    r is the order of vanishing of p at r (the smallest k such that
#    p^(k)(r) ≠ 0).
# 3. Assign the derivative-test multiplicity to each group.
#
# This correctly handles ``solve_quadratic`` returning one root for disc=0
# (multiplicity 2) and the trivial case of all distinct roots (all mult=1).


def _multiplicity_float(coeffs: list[Fraction], root_float: float) -> int:
    """Determine the algebraic multiplicity of a root using the derivative test.

    The algebraic multiplicity of a root r in polynomial p(λ) is the largest
    integer m such that (λ − r)^m divides p(λ).  Equivalently, it is the
    order of vanishing: the smallest k for which p^(k)(r) ≠ 0.

    We evaluate p and its successive derivatives numerically at ``root_float``.

    Parameters
    ----------
    coeffs:
        Ascending-power coefficient list [c₀, c₁, …, cₙ] of the characteristic
        polynomial (Fraction values).
    root_float:
        Float approximation of the root.

    Returns
    -------
    The algebraic multiplicity (≥ 1 if root_float is actually a root).
    """
    tol = 1e-4  # looser than grouping tolerance — numerical derivatives drift

    def eval_poly(c: list[float], x: float) -> float:
        return sum(c[k] * x**k for k in range(len(c)))

    def deriv_coeffs(c: list[float]) -> list[float]:
        """Return derivative coefficient list (descending one degree)."""
        if len(c) <= 1:
            return [0.0]
        return [(k + 1) * c[k + 1] for k in range(len(c) - 1)]

    current = [float(c) for c in coeffs]
    mult = 0
    # Count how many successive derivatives vanish at root_float.
    while len(current) > 1 and abs(eval_poly(current, root_float)) < tol:
        mult += 1
        current = deriv_coeffs(current)
    return max(mult, 1)  # at least 1 (it's a root)


_TEST_POINT = 1.7


def _eval_eigenvalue_complex(root: IRNode) -> complex:
    """Evaluate an eigenvalue IR node to a complex number for grouping.

    Handles all output formats of cas_solve: IRInteger, IRRational, complex
    nodes with ``%i``, roots with ``Sqrt``, and Add/Sub/Mul/Div/Neg trees.

    Parameters
    ----------
    root:
        An eigenvalue IR node as returned by ``solve_linear``,
        ``solve_quadratic``, ``solve_cubic``, or ``solve_quartic``.

    Returns
    -------
    A complex approximation.  Complex eigenvalues like ``0 + 1j`` and
    ``0 - 1j`` receive distinct values so they are *not* grouped together.
    """
    # Using complex arithmetic lets us distinguish conjugate pairs (+i vs -i)
    # which both evaluate to 0.0 if only the real part is kept.
    I_UNIT = IRSymbol("%i")
    SQRT_SYM = IRSymbol("Sqrt")
    from symbolic_ir import SQRT  # standard head

    def ev(node: IRNode) -> complex:
        if isinstance(node, IRInteger):
            return complex(node.value)
        if isinstance(node, IRRational):
            return complex(node.numer / node.denom)
        if isinstance(node, IRSymbol):
            if node == I_UNIT:
                return 1j
            return 0.0 + 0j  # unknown symbol
        if not isinstance(node, IRApply):
            return 0.0 + 0j
        h = node.head
        args = node.args
        if h == ADD:
            return sum(ev(a) for a in args)
        if h == SUB:
            return ev(args[0]) - ev(args[1])
        if h == MUL:
            r: complex = 1.0 + 0j
            for a in args:
                r *= ev(a)
            return r
        if h == DIV:
            d = ev(args[1])
            return ev(args[0]) / d if d != 0 else 1e18 + 0j
        if h == NEG:
            return -ev(args[0])
        if h in (SQRT, SQRT_SYM):
            val = ev(args[0])
            if val.imag == 0 and val.real >= 0:
                return complex(val.real**0.5)
            return 0.0 + 0j  # complex sqrt — rare; map to 0 to avoid grouping
        if h == POW:
            base = ev(args[0])
            exp_val = ev(args[1])
            try:
                return base**exp_val
            except (ValueError, ZeroDivisionError):
                return 0.0 + 0j
        return 0.0 + 0j  # unknown head

    return ev(root)


# ---------------------------------------------------------------------------
# Section 6 — Eigenvalues
# ---------------------------------------------------------------------------


def eigenvalues(M: IRNode) -> IRApply:
    """Compute the eigenvalues of a square rational matrix.

    Computes the characteristic polynomial, finds its roots via the
    appropriate polynomial solver (linear through quartic), groups equal
    roots by multiplicity, and returns them in MACSYMA's format:

        ``List(List(λ₁, m₁), List(λ₂, m₂), …)``

    where ``mᵢ`` is the algebraic multiplicity (how many times ``λᵢ``
    appears as a root).

    Parameters
    ----------
    M:
        Square matrix with ``IRInteger`` / ``IRRational`` entries.

    Returns
    -------
    An IR ``List`` of ``List`` pairs, each pair being
    ``[eigenvalue_IR, multiplicity_IRInteger]``.

    Raises
    ------
    MatrixError
        If ``M`` is not square or has symbolic entries.
    ValueError
        If the matrix is larger than 4×4 (solvers only go up to quartic).

    Examples
    --------
    ::

        eigenvalues(matrix([[IRInteger(1), IRInteger(2)],
                             [IRInteger(2), IRInteger(1)]]))
        # List(List(-1, 1), List(3, 1))

        eigenvalues(matrix([[IRInteger(2), IRInteger(0)],
                             [IRInteger(0), IRInteger(2)]]))
        # List(List(2, 2))
    """
    n = num_rows(M)
    if n != num_cols(M):
        raise MatrixError(f"eigenvalues: non-square matrix {n}×{num_cols(M)}")
    if n > 4:
        raise MatrixError(
            "eigenvalues: matrix larger than 4×4 not supported "
            "(eigenvalue computation requires degree >4 polynomial solver)"
        )

    coeffs = char_poly_coeffs(M)
    # coeffs = [c₀, c₁, …, cₙ] with cₙ = 1 (monic)
    # The polynomial is c₀ + c₁λ + … + cₙλⁿ
    # The solvers expect DESCENDING order: solve_quadratic(a, b, c) for aλ²+bλ+c

    LIST_HEAD = IRSymbol("List")

    # Dispatch to the correct solver based on n.
    if n == 1:
        # cₙ λ + c₀ = 0  →  c₁ λ + c₀ = 0
        roots_or_str = solve_linear(coeffs[1], coeffs[0])
    elif n == 2:
        # c₂λ² + c₁λ + c₀ = 0
        roots_or_str = solve_quadratic(coeffs[2], coeffs[1], coeffs[0])
    elif n == 3:
        # c₃λ³ + c₂λ² + c₁λ + c₀ = 0
        roots_or_str = solve_cubic(coeffs[3], coeffs[2], coeffs[1], coeffs[0])
    else:
        # n == 4: c₄λ⁴ + c₃λ³ + c₂λ² + c₁λ + c₀ = 0
        from cas_solve.quartic import solve_quartic as _solve_quartic
        roots_or_str = _solve_quartic(
            coeffs[4], coeffs[3], coeffs[2], coeffs[1], coeffs[0]
        )

    # solve_* return either a list of IR nodes or the string "all"
    # (the "all" case means 0=0, impossible for an n≥1 char poly).
    if isinstance(roots_or_str, str) or not roots_or_str:
        # Degenerate: empty or "all" — return unevaluated.
        return IRApply(IRSymbol("Eigenvalues"), (M,))

    roots: list[IRNode] = list(roots_or_str)

    # Group equal roots by complex proximity.  Using complex values (not just
    # real parts) correctly distinguishes conjugate pairs like +i and −i that
    # both have real part 0.  The solvers deduplicate repeated roots
    # (solve_quadratic returns one root for disc=0), so we use the derivative
    # test on the char poly to determine true algebraic multiplicity.
    tol = 1e-9
    cvals = [_eval_eigenvalue_complex(r) for r in roots]
    used = [False] * len(roots)
    groups: list[tuple[IRNode, float]] = []   # (representative_root, real_val)
    for i, r in enumerate(roots):
        if used[i]:
            continue
        used[i] = True
        for j in range(i + 1, len(roots)):
            if not used[j] and abs(cvals[i] - cvals[j]) < tol:
                used[j] = True
        groups.append((r, cvals[i].real))

    # Compute algebraic multiplicity via derivative test on the char poly.
    final_groups: list[tuple[IRNode, int]] = []
    for root_ir, root_float in groups:
        mult = _multiplicity_float(coeffs, root_float)
        final_groups.append((root_ir, mult))

    # Build List(List(λ₁, m₁), …)
    pair_nodes = tuple(
        IRApply(LIST_HEAD, (lam, IRInteger(mult)))
        for lam, mult in final_groups
    )
    return IRApply(LIST_HEAD, pair_nodes)


# ---------------------------------------------------------------------------
# Section 7 — Helper: convert IR leaf to Fraction (or None)
# ---------------------------------------------------------------------------


def _ir_to_fraction(node: IRNode) -> Fraction | None:
    """Return the Fraction value of an IRInteger or IRRational, else None.

    Used to detect whether an eigenvalue can be substituted exactly into
    the matrix for eigenvector computation.

    Parameters
    ----------
    node:
        An IR node.

    Returns
    -------
    ``Fraction`` if the node is a pure rational literal, ``None`` otherwise.
    """
    if isinstance(node, IRInteger):
        return Fraction(node.value)
    if isinstance(node, IRRational):
        return Fraction(node.numer, node.denom)
    # Negation of a rational
    if (
        isinstance(node, IRApply)
        and node.head == NEG
        and len(node.args) == 1
    ):
        inner = _ir_to_fraction(node.args[0])
        return -inner if inner is not None else None
    return None


# ---------------------------------------------------------------------------
# Section 8 — Null-space computation from RREF
# ---------------------------------------------------------------------------


def _null_space_fractions(A: list[list[Fraction]]) -> list[list[Fraction]]:
    """Compute the null space of a rational matrix via RREF.

    Parameters
    ----------
    A:
        Row-major list of lists of Fraction, shape m×n.

    Returns
    -------
    A list of column vectors (each a ``list[Fraction]`` of length n).
    If A has full column rank, returns ``[]`` (trivial null space).

    Algorithm
    ---------
    1. Perform RREF on A (Gauss-Jordan over Q).
    2. Identify pivot column indices.
    3. Free columns = non-pivot columns.
    4. For each free column j build basis vector v (length n):
       - ``v[j] = 1``
       - ``v[free_k] = 0`` for every other free column k
       - ``v[pivot_col_for_row_r] = −RREF[r, j]`` for each pivot row r

    Examples
    --------
    ::

        # A = [[1,2,3],[4,5,6]] → RREF = [[1,0,-1],[0,1,2]]
        # pivot cols 0, 1; free col 2
        # basis vector: v[2]=1, v[1]=-2, v[0]=1 → [1,-2,1]
        _null_space_fractions([[...], [...]])
        # [[Fraction(1), Fraction(-2), Fraction(1)]]
    """
    if not A or not A[0]:
        return []

    m = len(A)
    n = len(A[0])
    # Work on a copy so we don't mutate the caller's data.
    rref = [row[:] for row in A]

    pivot_cols: list[int] = []  # pivot_cols[r] = column index of pivot in row r
    pivot_row = 0
    for col in range(n):
        # Find a non-zero entry in this column at or below pivot_row.
        pivot_pos: int | None = None
        for r in range(pivot_row, m):
            if rref[r][col] != 0:
                pivot_pos = r
                break
        if pivot_pos is None:
            continue
        # Swap pivot row up.
        if pivot_pos != pivot_row:
            rref[pivot_row], rref[pivot_pos] = rref[pivot_pos], rref[pivot_row]
        # Normalise pivot to 1.
        pv = rref[pivot_row][col]
        rref[pivot_row] = [x / pv for x in rref[pivot_row]]
        # Eliminate all other entries in this column.
        for r in range(m):
            if r != pivot_row:
                factor = rref[r][col]
                if factor != 0:
                    rref[r] = [rref[r][c] - factor * rref[pivot_row][c]
                               for c in range(n)]
        pivot_cols.append(col)
        pivot_row += 1

    pivot_col_set = set(pivot_cols)
    free_cols = [c for c in range(n) if c not in pivot_col_set]

    if not free_cols:
        return []  # trivial null space

    # Build a dict: pivot column → the row it lives in.
    pivot_col_to_row: dict[int, int] = {col: r for r, col in enumerate(pivot_cols)}

    basis: list[list[Fraction]] = []
    for j in free_cols:
        # Basis vector for free column j.
        v = [Fraction(0)] * n
        v[j] = Fraction(1)
        for pcol, prow in pivot_col_to_row.items():
            v[pcol] = -rref[prow][j]
        basis.append(v)

    return basis


# ---------------------------------------------------------------------------
# Section 9 — Eigenvectors
# ---------------------------------------------------------------------------


def eigenvectors(M: IRNode) -> IRApply:
    """Compute eigenvalues and eigenvectors of a square rational matrix.

    Returns a list of triples, one per distinct eigenvalue:

        ``List(List(λ₁, m₁, List(v₁, v₂, …)), …)``

    where ``vᵢ`` are column-vector ``Matrix`` IR nodes spanning the
    eigenspace for ``λ₁``.

    For eigenvalues that are irrational or complex (i.e., not plain
    ``IRInteger`` / ``IRRational``), the eigenvector list for that
    eigenvalue is ``List()`` — symbolic null-space computation is out
    of scope for Phase 19.

    Parameters
    ----------
    M:
        Square matrix with ``IRInteger`` / ``IRRational`` entries.

    Returns
    -------
    IR ``List`` of triples as described above.

    Raises
    ------
    MatrixError
        If M is not square, has symbolic entries, or is larger than 4×4.

    Examples
    --------
    ::

        eigenvectors(matrix([[IRInteger(1), IRInteger(2)],
                              [IRInteger(2), IRInteger(1)]]))
        # List(
        #   List(-1, 1, List(Matrix([[-1],[1]]))),
        #   List( 3, 1, List(Matrix([[ 1],[1]])))
        # )
    """
    n = num_rows(M)
    LIST_HEAD = IRSymbol("List")

    # Reuse eigenvalues to get the grouping.
    eigs = eigenvalues(M)
    if eigs.head != LIST_HEAD:
        # eigenvalues returned unevaluated — propagate
        return IRApply(IRSymbol("Eigenvectors"), (M,))

    frows = _matrix_to_fractions(M)
    triples: list[IRNode] = []

    for pair in eigs.args:
        lam_ir, mult_ir = pair.args[0], pair.args[1]

        lam_frac = _ir_to_fraction(lam_ir)
        if lam_frac is None:
            # Irrational / complex — can't do exact null space.
            triple = IRApply(LIST_HEAD, (lam_ir, mult_ir, IRApply(LIST_HEAD, ())))
            triples.append(triple)
            continue

        # Build B = A − λI with Fraction entries.
        B = [
            [frows[i][j] - (lam_frac if i == j else Fraction(0)) for j in range(n)]
            for i in range(n)
        ]

        basis = _null_space_fractions(B)

        # Convert each basis vector to a column-vector Matrix IR node.
        vec_nodes: list[IRNode] = []
        for vec in basis:
            col_ir = matrix([[_frac_to_ir(v)] for v in vec])
            vec_nodes.append(col_ir)

        vec_list = IRApply(LIST_HEAD, tuple(vec_nodes))
        triple = IRApply(LIST_HEAD, (lam_ir, mult_ir, vec_list))
        triples.append(triple)

    return IRApply(LIST_HEAD, tuple(triples))
