"""Berlekamp-Zassenhaus-Hensel (BZH) factoring for monic integer polynomials.

This module extends ``cas_factor`` with a modular arithmetic approach that
correctly handles cases the Kronecker algorithm misses — most notably
high-degree cyclotomic polynomials and products whose value at few integer
points yields divisor sets too large to enumerate.

Algorithm overview
------------------
Given a *monic* primitive polynomial ``f ∈ Z[x]`` of degree ``n``:

1. **Choose a good prime** ``p``.
   We need ``f mod p`` to be *squarefree* and the leading coefficient to
   survive (i.e. ``lc(f) mod p ≠ 0``).  Squarefreeness is tested by
   computing ``gcd(f mod p, f' mod p)`` over ``GF(p)`` — if the GCD has
   degree 0 the reduction is squarefree.  We try primes 2, 3, 5, 7, 11, …
   up to ``MAX_PRIME``.

2. **Factor mod p using Berlekamp's algorithm**.
   Berlekamp constructs the *Q-matrix* (also called the Frobenius matrix or
   power-reduction matrix).  Entry ``Q[j][i]`` is the coefficient of ``x^i``
   in ``x^(j·p) mod f``, reduced mod p.  The null space of ``(Q − I) mod p``
   has dimension equal to the number of irreducible factors.  Each null
   vector ``v(x)`` generates factors via ``gcd(f, v(x) − s)`` for
   ``s = 0, 1, …, p−1``.

3. **Zassenhaus bound and Hensel lifting**.
   Mignotte's bound ensures that any true factor of ``f`` has coefficients
   bounded by::

       B = ||f||_2 · 2^n · sqrt(n+1)     (n = deg f)

   We lift the mod-``p`` factors to mod ``p^k`` where ``p^k > 2·B``.
   Each lift step uses the Newton/Hensel identity: given
   ``f ≡ g·h (mod p^k)`` and ``s·g + t·h ≡ 1 (mod p)`` (the initial Bézout
   relation), we solve for ``δg, δh`` s.t.
   ``f ≡ (g + p^k·δg)·(h + p^k·δh) (mod p^{k+1})``.
   The strategy is *linear Hensel lift* — one factor of ``p`` per iteration.

4. **Factor combination** (Zassenhaus recombination).
   We try all subsets of size 1, 2, … of the lifted factors.  A candidate
   subset is genuine if the product of its elements (normalised by the
   leading coefficient of ``f``) divides ``f`` exactly in ``Z[x]``.  We
   stop at subsets of size ``⌊r/2⌋`` where ``r`` is the total number of
   modular factors.

Restriction to monic polynomials
---------------------------------
This implementation requires ``f`` to be monic (leading coefficient = 1).
The caller (:func:`cas_factor.factor.factor_integer_polynomial`) ensures this
by extracting content and checking; when the primitive polynomial is
non-monic, it is left for the Kronecker algorithm.

This restriction vastly simplifies the lifting: all Berlekamp factors of a
monic polynomial are themselves monic mod ``p``, and the Hensel lift works
directly with integer-coefficient polynomials.

Public API
----------
::

    bzh_factor(coeffs: list[int]) -> list[list[int]] | None

Returns a list of factor coefficient lists (each primitive, positive leading
coefficient) or ``None`` if the polynomial is detected as irreducible, the
input is non-monic, the degree exceeds ``MAX_DEGREE``, or no suitable prime
was found.

Limitations
-----------
- **Monic only**: non-monic primitives return ``None``.
- **Degree cap**: ``MAX_DEGREE = 20``.
- **Prime cap**: primes up to ``MAX_PRIME = 200``.
- **Factor combination**: subset search up to size ``⌊r/2⌋``; for polynomials
  with many modular factors (> ~20) this can be slow but is correct.

Examples
--------
::

    bzh_factor([-1, 0, 0, 0, 0, 1])   # x^5 − 1
    # [[-1, 1], [1, 1, 1, 1, 1]]       # (x−1)(x^4+x^3+x^2+x+1)

    bzh_factor([1, 0, 0, 0, 1])       # x^4 + 1 — irreducible over Q
    # None

    bzh_factor([-1, 0, 0, 0, 0, 0, 0, 0, 1])   # x^8 − 1
    # [[-1, 1], [1, 1], [1, 0, 1], [1, 0, 0, 0, 1]]
"""

from __future__ import annotations

import math
from itertools import combinations

from cas_factor.polynomial import Poly, degree, normalize, primitive_part

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

MAX_DEGREE: int = 20
"""Maximum polynomial degree we attempt.  Degrees above this return None."""

MAX_PRIME: int = 200
"""We try primes up to this value when searching for a good modulus."""

# Pre-compute small primes up to MAX_PRIME via a Sieve of Eratosthenes.
# The sieve is the simplest primality test: mark all composites as False.
_SMALL_PRIMES: list[int] = []
_sieve = [True] * (MAX_PRIME + 1)
_sieve[0] = _sieve[1] = False
for _si in range(2, MAX_PRIME + 1):
    if _sieve[_si]:
        _SMALL_PRIMES.append(_si)
        for _sj in range(_si * _si, MAX_PRIME + 1, _si):
            _sieve[_sj] = False
del _sieve, _si, _sj


# ---------------------------------------------------------------------------
# Polynomial arithmetic over GF(p)
# ---------------------------------------------------------------------------
# Convention: polynomials are lists of integers in ascending-degree order,
# coefficients already in [0, p-1].  The zero polynomial is the empty list.
# Trailing zeros (high-degree zeros) are always stripped.


def _pmod(coeffs: list[int], p: int) -> list[int]:
    """Reduce every coefficient into ``[0, p-1]`` and strip trailing zeros."""
    out = [c % p for c in coeffs]
    while out and out[-1] == 0:
        out.pop()
    return out


def _pdeg(coeffs: list[int]) -> int:
    """Degree of a coefficient list.  Empty list has degree -1."""
    return len(coeffs) - 1


def _pneg(coeffs: list[int], p: int) -> list[int]:
    """Negate a polynomial in GF(p)[x]."""
    return [(p - c) % p for c in coeffs]


def _padd(a: list[int], b: list[int], p: int) -> list[int]:
    """Add two polynomials in GF(p)[x]."""
    n = max(len(a), len(b))
    result = [0] * n
    for i, c in enumerate(a):
        result[i] = (result[i] + c) % p
    for i, c in enumerate(b):
        result[i] = (result[i] + c) % p
    while result and result[-1] == 0:
        result.pop()
    return result


def _psub(a: list[int], b: list[int], p: int) -> list[int]:
    """Subtract b from a in GF(p)[x]."""
    return _padd(a, _pneg(b, p), p)


def _pmul(a: list[int], b: list[int], p: int) -> list[int]:
    """Multiply two polynomials in GF(p)[x]."""
    if not a or not b:
        return []
    n = len(a) + len(b) - 1
    result = [0] * n
    for i, ca in enumerate(a):
        for j, cb in enumerate(b):
            result[i + j] = (result[i + j] + ca * cb) % p
    while result and result[-1] == 0:
        result.pop()
    return result


def _pscale(poly: list[int], s: int, p: int) -> list[int]:
    """Multiply all coefficients by scalar ``s`` mod p."""
    return [(c * s) % p for c in poly]


def _pmod_poly(a: list[int], b: list[int], p: int) -> list[int]:
    """Compute ``a mod b`` in GF(p)[x] — the remainder of polynomial division.

    Caller must ensure ``b`` is non-zero.  Uses standard long division,
    eliminating the leading term of ``a`` by subtracting scalar multiples of
    ``b`` shifted by the appropriate power of ``x``.

    The invariant is that we reduce the *degree* of ``a`` strictly below that
    of ``b``.  Each iteration reduces the degree by at least 1, so the loop
    runs at most ``deg(a) - deg(b) + 1`` times.
    """
    a = list(a)
    db = _pdeg(b)
    if db < 0:
        raise ZeroDivisionError("division by zero polynomial")
    lead_b_inv = pow(b[-1], p - 2, p)  # modular inverse (p prime, b[-1] ≠ 0)
    while _pdeg(a) >= db:
        if not a:
            break
        shift = _pdeg(a) - db
        factor = (a[-1] * lead_b_inv) % p
        for k, c in enumerate(b):
            a[shift + k] = (a[shift + k] - factor * c) % p
        while a and a[-1] == 0:
            a.pop()
    return a


def _pdiv_quotient(a: list[int], b: list[int], p: int) -> list[int]:
    """Return the quotient of ``a / b`` in GF(p)[x]."""
    a = list(a)
    db = _pdeg(b)
    if db < 0:
        raise ZeroDivisionError("division by zero polynomial")
    lead_b_inv = pow(b[-1], p - 2, p)
    quot: list[int] = []
    while _pdeg(a) >= db:
        if not a:
            break
        shift = _pdeg(a) - db
        factor = (a[-1] * lead_b_inv) % p
        while len(quot) <= shift:
            quot.append(0)
        quot[shift] = (quot[shift] + factor) % p
        for k, c in enumerate(b):
            a[shift + k] = (a[shift + k] - factor * c) % p
        while a and a[-1] == 0:
            a.pop()
    while quot and quot[-1] == 0:
        quot.pop()
    return quot


def _pgcd(a: list[int], b: list[int], p: int) -> list[int]:
    """Monic GCD of polynomials in GF(p)[x] via Euclidean algorithm.

    Returns a *monic* GCD or ``[]`` if both inputs are zero.  The Euclidean
    algorithm terminates because each remainder has strictly smaller degree
    than the previous divisor.  Making the result monic gives a canonical
    representative.
    """
    while b:
        a, b = b, _pmod_poly(a, b, p)
    if a and a[-1] != 1:
        inv = pow(a[-1], p - 2, p)
        a = _pscale(a, inv, p)
    return a


def _pgcd_extended(
    a: list[int], b: list[int], p: int
) -> tuple[list[int], list[int], list[int]]:
    """Extended Euclidean algorithm in GF(p)[x].

    Returns ``(g, s, t)`` such that ``s·a + t·b ≡ g (mod p)`` where ``g``
    is the monic GCD of ``a`` and ``b``.

    The extended GCD maintains the invariant at each step::

        old_r = old_s · a + old_t · b
        r     =    s  · a +    t  · b

    by updating ``(old_s, s)`` and ``(old_t, t)`` in parallel with
    ``(old_r, r)`` during the Euclidean loop.

    This Bézout relation is used to seed the Hensel lifting: for coprime
    ``g1, g2`` with ``s·g1 + t·g2 ≡ 1 (mod p)``, the Diophantine equation
    ``u·g1 + v·g2 ≡ e (mod p)`` is solved in the lifting steps.
    """
    old_r, r = list(a), list(b)
    old_s: list[int] = [1]
    s: list[int] = []
    old_t: list[int] = []
    t: list[int] = [1]

    while r:
        q = _pdiv_quotient(old_r, r, p)
        old_r, r = r, _psub(old_r, _pmul(q, r, p), p)
        old_s, s = s, _psub(old_s, _pmul(q, s, p), p)
        old_t, t = t, _psub(old_t, _pmul(q, t, p), p)

    g = old_r
    s_out = old_s
    t_out = old_t

    # Normalise so g is monic.
    if g and g[-1] != 1:
        inv = pow(g[-1], p - 2, p)
        g = _pscale(g, inv, p)
        s_out = _pscale(s_out, inv, p)
        t_out = _pscale(t_out, inv, p)

    return g, s_out, t_out


def _pderiv(poly: list[int], p: int) -> list[int]:
    """Formal derivative of a polynomial in GF(p)[x].

    The derivative of ``c_k · x^k`` is ``k · c_k · x^{k-1}``, reduced mod p.
    For any term whose exponent is a multiple of ``p``, the derivative
    coefficient is zero — this correctly captures the Frobenius endomorphism
    behaviour that makes ``x^p`` a perfect p-th power in char p.
    """
    if len(poly) <= 1:
        return []
    result = [(i * poly[i]) % p for i in range(1, len(poly))]
    while result and result[-1] == 0:
        result.pop()
    return result


def _is_squarefree_mod_p(poly: list[int], p: int) -> bool:
    """Test whether ``poly`` is squarefree in GF(p)[x].

    A polynomial is squarefree iff it shares no factor with its derivative,
    i.e. ``gcd(f, f') = 1`` (a non-zero constant).

    Edge cases:

    - If ``f' = 0`` mod p (happens for polynomials like ``x^p``) the
      polynomial is NOT squarefree — return ``False``.
    - If ``f`` itself is zero mod p, return ``False``.
    """
    f_mod = _pmod(poly, p)
    if not f_mod:
        return False
    df = _pderiv(f_mod, p)
    if not df:
        return False  # f' = 0 mod p → not squarefree (or char-p artifact)
    g = _pgcd(f_mod, df, p)
    return _pdeg(g) == 0  # GCD is a non-zero constant iff squarefree


# ---------------------------------------------------------------------------
# Berlekamp's factoring algorithm over GF(p)
# ---------------------------------------------------------------------------


def _poly_powmod(exp: int, mod_poly: list[int], p: int) -> list[int]:
    """Compute ``x^exp mod mod_poly`` in GF(p)[x] via repeated squaring.

    Represents ``x`` as the polynomial ``[0, 1]`` (degree-1, constant 0,
    leading coefficient 1) and raises it to the power ``exp`` using the
    fast exponentiation identity::

        x^(2k)   = (x^k)^2   mod f
        x^(2k+1) = (x^k)^2 · x  mod f

    This runs in ``O(log exp)`` polynomial multiplications mod ``mod_poly``.
    """
    result = [1]  # start with 1 = x^0
    cur = _pmod_poly([0, 1], mod_poly, p)  # x mod f
    while exp > 0:
        if exp & 1:
            result = _pmod_poly(_pmul(result, cur, p), mod_poly, p)
        cur = _pmod_poly(_pmul(cur, cur, p), mod_poly, p)
        exp >>= 1
    return result


def _null_space_mod_p(M: list[list[int]], n: int, p: int) -> list[list[int]]:
    """Compute the null space of an n×n matrix over GF(p).

    Uses row-reduction (Gaussian elimination over GF(p)) to find the
    reduced row-echelon form of ``M``.  From the RREF, free columns
    correspond directly to null-space basis vectors: set the free variable
    to 1 and back-substitute to find the other components.

    A matrix of rank ``r`` has an (n−r)-dimensional null space.

    Returns a list of basis vectors, each represented as a list of ``n``
    integers in ``[0, p−1]``.  At minimum one vector is returned (the
    polynomial ``1`` in the degree-0 direction, corresponding to the
    trivial factor equal to ``f`` itself).
    """
    A = [list(row) for row in M]
    pivot_cols: list[int] = []
    row_idx = 0

    for col in range(n):
        # Find pivot.
        pivot = -1
        for r_i in range(row_idx, n):
            if A[r_i][col] != 0:
                pivot = r_i
                break
        if pivot == -1:
            continue  # free column

        A[row_idx], A[pivot] = A[pivot], A[row_idx]
        inv = pow(A[row_idx][col], p - 2, p)
        A[row_idx] = [(x * inv) % p for x in A[row_idx]]
        for r_i in range(n):
            if r_i != row_idx and A[r_i][col] != 0:
                factor = A[r_i][col]
                A[r_i] = [(A[r_i][j] - factor * A[row_idx][j]) % p for j in range(n)]
        pivot_cols.append(col)
        row_idx += 1

    free_cols = [c for c in range(n) if c not in pivot_cols]
    pivot_row: dict[int, int] = {col: i for i, col in enumerate(pivot_cols)}

    basis: list[list[int]] = []
    for fc in free_cols:
        vec = [0] * n
        vec[fc] = 1
        for pc in pivot_cols:
            r_i = pivot_row[pc]
            vec[pc] = (p - A[r_i][fc]) % p
        basis.append(vec)

    if not basis:
        # f is irreducible mod p → only the trivial null vector.
        basis.append([1] + [0] * (n - 1))

    return basis


def _berlekamp_factor_mod_p(f_coeffs: list[int], p: int) -> list[list[int]]:
    """Factor ``f`` (monic, squarefree) over GF(p) using Berlekamp's algorithm.

    The algorithm exploits the Frobenius endomorphism ``φ: x ↦ x^p``.
    Any element of the null space of ``(Q − I)`` corresponds to a
    polynomial ``v(x)`` satisfying ``v(x)^p ≡ v(x) (mod f)`` over GF(p).
    For each such ``v`` and each ``s ∈ GF(p)`` the GCD ``gcd(f, v − s)`` is
    a (possibly trivial) factor of ``f``.

    The Q-matrix
    ~~~~~~~~~~~~
    Row ``j`` of ``Q`` is the coefficient vector of ``x^(j·p) mod f`` —
    more precisely, ``Q[j][i] = [x^i] (x^{j·p} mod f)``.  We build
    these rows iteratively: start from ``x^0 = 1``, multiply each time by
    ``x^p mod f`` (computed once using fast exponentiation).

    Null space of (Q − I)
    ~~~~~~~~~~~~~~~~~~~~~
    The null space has dimension ``r`` equal to the number of irreducible
    factors.  We build the matrix ``M = (Q^T − I)`` and row-reduce over
    GF(p) using :func:`_null_space_mod_p`.

    Splitting
    ~~~~~~~~~
    Starting from ``[f]``, for each null-space basis vector ``v`` (skipping
    the trivial all-ones-in-first-entry vector) we compute
    ``gcd(factor, v − s)`` for ``s = 0, …, p−1``.  Any non-trivial GCD
    splits a factor.  We stop when we have exactly ``r`` factors.

    Returns
    -------
    list[list[int]]
        Monic irreducible factors of ``f`` in GF(p)[x], product equal to ``f``.
    """
    n = _pdeg(f_coeffs)
    if n <= 0:
        return [f_coeffs] if f_coeffs else []
    if n == 1:
        return [f_coeffs]

    # --- Build Q-matrix. ---
    # Q[j] = coefficient vector of x^(jp) mod f, length n.
    xp_mod_f = _poly_powmod(p, f_coeffs, p)  # x^p mod f

    Q: list[list[int]] = []
    current = [1]  # x^0 = 1
    for _j in range(n):
        row = list(current) + [0] * (n - len(current))
        Q.append(row)
        current = _pmod_poly(_pmul(current, xp_mod_f, p), f_coeffs, p)

    # --- Build M = (Q^T − I) and find null space. ---
    M: list[list[int]] = [[0] * n for _ in range(n)]
    for i in range(n):
        for j in range(n):
            M[i][j] = (Q[j][i] - (1 if i == j else 0)) % p

    null_basis = _null_space_mod_p(M, n, p)
    r = len(null_basis)

    if r == 1:
        return [f_coeffs]  # irreducible mod p

    # --- Split using null-space vectors. ---
    factors: list[list[int]] = [list(f_coeffs)]

    for v_coeffs in null_basis[1:]:
        if len(factors) == r:
            break
        new_factors: list[list[int]] = []
        for g in factors:
            if _pdeg(g) <= 0:
                new_factors.append(g)
                continue
            split_found = False
            for s in range(p):
                v_minus_s = list(v_coeffs)
                if v_minus_s:
                    v_minus_s[0] = (v_minus_s[0] - s) % p
                else:
                    v_minus_s = [(p - s) % p]
                while v_minus_s and v_minus_s[-1] == 0:
                    v_minus_s.pop()
                if not v_minus_s:
                    v_minus_s = [0]
                h = _pgcd(g, v_minus_s, p)
                if 0 < _pdeg(h) < _pdeg(g):
                    complement = _pdiv_quotient(g, h, p)
                    new_factors.append(h)
                    new_factors.append(complement)
                    split_found = True
                    break
            if not split_found:
                new_factors.append(g)
        factors = new_factors

    # Ensure all factors are monic.
    result = []
    for g in factors:
        if g and g[-1] != 1:
            inv = pow(g[-1], p - 2, p)
            result.append(_pscale(g, inv, p))
        elif g:
            result.append(g)
    return result


# ---------------------------------------------------------------------------
# Zassenhaus bound
# ---------------------------------------------------------------------------


def _zassenhaus_bound(f: Poly) -> float:
    """Mignotte's bound on the coefficient magnitude of any factor of ``f``.

    Any factor ``g`` of ``f = a_n x^n + … + a_0`` of degree ≤ n/2 has all
    coefficients bounded (in absolute value) by::

        B = 2^n · sqrt(n+1) · ||f||_2

    where ``||f||_2 = sqrt(Σ aᵢ²)`` is the Euclidean coefficient norm.

    This is a conservative but proven bound (Cohen, §3.5.1).  We use it to
    determine the minimum lifting precision: we need the modulus ``p^k``
    to exceed ``2·B`` so that the centered reduction of any lifted factor
    equals the true factor.

    For the linear Hensel lift (one factor of p per step) we need at most
    ``log_p(2B+1)`` lift iterations.
    """
    n = degree(f)
    if n < 0:
        return 0.0
    norm2 = math.sqrt(sum(c * c for c in f))
    return (2**n) * math.sqrt(n + 1) * norm2


# ---------------------------------------------------------------------------
# Polynomial arithmetic over Z (exact, no modular reduction)
# ---------------------------------------------------------------------------


def _iz_add(a: list[int], b: list[int]) -> list[int]:
    """Add two integer polynomials (exact)."""
    n = max(len(a), len(b))
    result = [0] * n
    for i, c in enumerate(a):
        result[i] += c
    for i, c in enumerate(b):
        result[i] += c
    while result and result[-1] == 0:
        result.pop()
    return result


def _iz_sub(a: list[int], b: list[int]) -> list[int]:
    """Subtract b from a (exact integer polynomials)."""
    n = max(len(a), len(b))
    result = [0] * n
    for i, c in enumerate(a):
        result[i] += c
    for i, c in enumerate(b):
        result[i] -= c
    while result and result[-1] == 0:
        result.pop()
    return result


def _iz_mul(a: list[int], b: list[int]) -> list[int]:
    """Multiply two integer polynomials (exact)."""
    if not a or not b:
        return []
    n = len(a) + len(b) - 1
    result = [0] * n
    for i, ca in enumerate(a):
        for j, cb in enumerate(b):
            result[i + j] += ca * cb
    while result and result[-1] == 0:
        result.pop()
    return result


def _center_mod(coeffs: list[int], m: int) -> list[int]:
    """Reduce coefficients to the symmetric range ``(-m/2, m/2]``."""
    half = m // 2
    result = []
    for c in coeffs:
        r = c % m
        if r > half:
            r -= m
        result.append(r)
    while result and result[-1] == 0:
        result.pop()
    return result


# ---------------------------------------------------------------------------
# Diophantine equation solver in GF(p)[x]
# ---------------------------------------------------------------------------


def _diophantine_mod_p(
    a: list[int], b: list[int], c: list[int], p: int
) -> tuple[list[int], list[int]]:
    """Solve ``u·a + v·b ≡ c (mod p)`` in GF(p)[x].

    Requires ``gcd(a, b) = 1 mod p`` and ``deg(c) < deg(a) + deg(b)``.
    Returns ``(u, v)`` with ``deg(u) < deg(b)`` and ``deg(v) < deg(a)``.

    Derivation
    ----------
    From the Bézout relation ``s·a + t·b ≡ 1 (mod p)`` (computed by the
    extended GCD), multiply through by ``c``::

        s·c·a + t·c·b ≡ c (mod p)

    Set ``u = s·c mod b`` (reduce mod b to enforce ``deg(u) < deg(b)``),
    letting ``q = (s·c) div b`` so that ``s·c = q·b + u``.
    Then ``v = t·c + q·a mod a`` (reduce mod a to enforce ``deg(v) < deg(a)``).

    Verification: ``u·a + v·b ≡ (s·c - q·b)·a + (t·c + q·a)·b``
                              ``= s·c·a + t·c·b ≡ c``. ✓
    """
    _, s, t = _pgcd_extended(a, b, p)
    sc = _pmul(s, c, p)
    u = _pmod_poly(sc, b, p)          # deg u < deg b
    q = _pdiv_quotient(sc, b, p)
    tc = _pmul(t, c, p)
    v_raw = _padd(tc, _pmul(q, a, p), p)
    v = _pmod_poly(v_raw, a, p)       # deg v < deg a
    return u, v


# ---------------------------------------------------------------------------
# Linear Hensel lifting
# ---------------------------------------------------------------------------


def _to_z_centered(poly: list[int], p: int) -> list[int]:
    """Convert GF(p) coefficients to the centered Z range ``(-p/2, p/2]``."""
    result = [c if c <= p // 2 else c - p for c in poly]
    while result and result[-1] == 0:
        result.pop()
    return result


def _linear_hensel_lift(
    f: Poly,
    g_init: list[int],
    h_init: list[int],
    p: int,
    target_mod: int,
) -> tuple[list[int], list[int]] | None:
    """Lift a two-factor split ``f ≡ g·h (mod p)`` to ``f ≡ g*·h* (mod target_mod)``.

    Uses the *linear Hensel lift* — one factor of ``p`` per iteration — to
    avoid the quadratic blow-up in coefficient size that the Newton step can
    introduce when the initial approximation is only ``mod p``.

    Algorithm (Cohen 3.5.6 adapted to two factors)
    -----------------------------------------------
    At step ``k`` we have ``g_k, h_k`` with ``f ≡ g_k · h_k (mod p^k)``.

    1. Compute the error: ``e = (f − g_k · h_k) / p^k`` (exact integer poly).
    2. Reduce ``e`` mod p to get ``e_mod``.
    3. Solve the Diophantine equation over GF(p):
       ``δ_h · g + δ_g · h ≡ e_mod (mod p)``
       using :func:`_diophantine_mod_p` with the fixed ``g, h = g_mod, h_mod``
       (the original mod-p reductions; they stay constant throughout).
    4. Update: ``g_{k+1} = g_k + p^k · δ_g``  and  ``h_{k+1} = h_k + p^k · δ_h``.

    Note on variable naming:
    :func:`_diophantine_mod_p` solves ``u·a + v·b = c`` and returns ``(u, v)``.
    With ``a = g_mod``, ``b = h_mod``, ``c = e_mod``:
    - ``u`` plays the role of ``δ_h`` (updates ``h``),
    - ``v`` plays the role of ``δ_g`` (updates ``g``).

    Parameters
    ----------
    f : Poly
        The target polynomial (monic, primitive).
    g_init : list[int]
        First factor of ``f mod p`` (monic in GF(p)[x]).
    h_init : list[int]
        Second factor of ``f mod p`` (monic in GF(p)[x]).
    p : int
        The prime modulus (squarefree prime for ``f mod p``).
    target_mod : int
        The desired precision: we lift until the modulus reaches this.

    Returns
    -------
    tuple[list[int], list[int]] | None
        ``(g_lifted, h_lifted)`` as centered Z-coefficient lists, or
        ``None`` if the initial factors are not coprime mod p.
    """
    g_mod = _pmod(list(g_init), p)
    h_mod = _pmod(list(h_init), p)

    # Verify coprimality.
    gcd_check = _pgcd(g_mod, h_mod, p)
    if _pdeg(gcd_check) != 0:
        return None  # not coprime mod p

    # Initialise g, h as Z-coefficient lists (centered around 0).
    g = _to_z_centered(g_mod, p)
    h = _to_z_centered(h_mod, p)

    pk = p        # current modulus factor p^k
    mod = p       # current modulus

    while mod < target_mod:
        e_diff = _iz_sub(f, _iz_mul(g, h))
        if not e_diff:
            break  # exact factorization already found
        # Check all error coefficients are divisible by pk.
        if any(c % pk != 0 for c in e_diff):
            return None  # lifting failed (shouldn't happen with correct setup)
        e = [c // pk for c in e_diff]
        while e and e[-1] == 0:
            e.pop()

        e_mod = _pmod(e, p)

        # Solve u·g_mod + v·h_mod ≡ e_mod (mod p).
        # u = δ_h (correction to h), v = δ_g (correction to g).
        u_mod, v_mod = _diophantine_mod_p(g_mod, h_mod, e_mod, p)

        u_z = _to_z_centered(u_mod, p)
        v_z = _to_z_centered(v_mod, p)

        # g_new = g + pk * v_z.
        g_new = list(g)
        for i, c in enumerate(v_z):
            while len(g_new) <= i:
                g_new.append(0)
            g_new[i] += pk * c
        while g_new and g_new[-1] == 0:
            g_new.pop()

        # h_new = h + pk * u_z.
        h_new = list(h)
        for i, c in enumerate(u_z):
            while len(h_new) <= i:
                h_new.append(0)
            h_new[i] += pk * c
        while h_new and h_new[-1] == 0:
            h_new.pop()

        g, h = g_new, h_new
        pk *= p
        mod *= p  # advance by one factor of p

    return _center_mod(g, target_mod), _center_mod(h, target_mod)


# ---------------------------------------------------------------------------
# Multi-factor Hensel lift (divide-and-conquer)
# ---------------------------------------------------------------------------


def _multi_hensel_lift(
    f: Poly,
    factors_mod_p: list[list[int]],
    p: int,
    target: float,
) -> list[list[int]] | None:
    """Lift all modular factors of ``f`` to integer precision > ``target``.

    Uses divide-and-conquer Hensel lifting:

    1. **Base cases**: zero factors → ``[]``; one factor → ``[f]`` (the
       polynomial is irreducible for this sub-problem).
    2. **Two factors**: delegate directly to :func:`_linear_hensel_lift`.
    3. **More than two factors**: split the factor list in half.  Compute the
       GF(p)-product of each half (to get two "aggregate" factors).  Lift
       the pair ``(left_product, right_product)`` for ``f``.  Then
       recursively lift within each half using the lifted products as the
       new polynomials.

    This tree structure ensures that each call to the two-factor lift works
    on polynomials that factor exactly into the given sub-list, preserving
    correctness at each level.

    Returns a flat list of all lifted factors (one per modular factor in
    ``factors_mod_p``), in the same order.
    """
    r = len(factors_mod_p)
    if r == 0:
        return []
    if r == 1:
        return [list(f)]

    mod = p
    while mod <= target:
        mod *= p

    if r == 2:
        result = _linear_hensel_lift(
            f, factors_mod_p[0], factors_mod_p[1], p, mod
        )
        return list(result) if result else None

    mid = r // 2
    left_facs = factors_mod_p[:mid]
    right_facs = factors_mod_p[mid:]

    # Products of each half mod p (for the intermediate Hensel step).
    from functools import reduce

    left_prod = reduce(lambda a, b: _pmul(a, b, p), left_facs)
    right_prod = reduce(lambda a, b: _pmul(a, b, p), right_facs)

    # Ensure monic (they should be monic if all input factors are monic).
    for prod in (left_prod, right_prod):
        if prod and prod[-1] != 1:
            inv = pow(prod[-1], p - 2, p)
            prod[:] = [(c * inv) % p for c in prod]
            while prod and prod[-1] == 0:
                prod.pop()

    pair = _linear_hensel_lift(f, left_prod, right_prod, p, mod)
    if pair is None:
        return None
    left_lifted, right_lifted = pair

    left_result = _multi_hensel_lift(left_lifted, left_facs, p, target)
    right_result = _multi_hensel_lift(right_lifted, right_facs, p, target)

    if left_result is None or right_result is None:
        return None
    return left_result + right_result


# ---------------------------------------------------------------------------
# Divisibility test over Z
# ---------------------------------------------------------------------------


def _poly_divides_z(f: Poly, g: Poly) -> Poly | None:
    """Check whether ``g`` divides ``f`` exactly in Z[x].

    Returns the integer quotient if ``g | f``, otherwise ``None``.
    Uses polynomial long division over Z: at each step, the leading
    coefficient of ``g`` must divide the current leading coefficient of
    the remainder — if it doesn't, ``g`` does not divide ``f``.
    """
    f = normalize(f)
    g = normalize(g)
    if not g:
        return None
    if degree(g) > degree(f):
        return None
    if degree(g) == 0:
        c = g[0]
        if c != 0 and all(coef % c == 0 for coef in f):
            return [coef // c for coef in f]
        return None

    rem = list(f)
    dg = degree(g)
    quot_coeffs: list[int] = []

    while len(rem) - 1 >= dg:
        dr = len(rem) - 1
        shift = dr - dg
        lc_g = g[-1]
        lc_r = rem[-1]
        if lc_r % lc_g != 0:
            return None
        c = lc_r // lc_g
        while len(quot_coeffs) <= shift:
            quot_coeffs.append(0)
        quot_coeffs[shift] = c
        for k, gk in enumerate(g):
            rem[shift + k] -= c * gk
        while rem and rem[-1] == 0:
            rem.pop()

    if rem:
        return None
    result = list(reversed(quot_coeffs))
    while result and result[-1] == 0:
        result.pop()
    return result if result else [0]


# ---------------------------------------------------------------------------
# Zassenhaus factor combination
# ---------------------------------------------------------------------------


def _combine_factors(
    f: Poly,
    lifted: list[list[int]],
    modulus: int,
) -> list[Poly] | None:
    """Try all subsets of lifted factors to find genuine Z[x] factors.

    This is the Zassenhaus recombination step.  We iterate over subsets of
    sizes 1, 2, …, ``⌊len(lifted)/2⌋``.  For each subset, we:

    1. Multiply the lifted factors together (exact integer arithmetic).
    2. Center the result modulo ``modulus`` (to recover the small-coefficient
       representative in ``(-modulus/2, modulus/2]``).
    3. Take the primitive part (divide out the integer GCD of coefficients).
    4. Test whether the primitive part divides ``f`` exactly in Z[x].

    Since ``f`` is monic and all lifted factors are monic (lifts of monic
    mod-p factors), the primitive part of any subset product is the same
    as the product itself — no scaling by ``lc(f)`` is needed.

    When a genuine factor is found, we remove its indices from the pool and
    replace ``f`` with the cofactor.  The loop continues with the smaller
    pool.  The final remaining polynomial (if non-trivial) is itself
    irreducible.

    Returns a list of irreducible factor coefficient lists (positive leading
    coefficient), or ``None`` if no non-trivial factors were found (``f``
    appears to be irreducible).
    """
    remaining_f = list(f)
    remaining_lifted = list(lifted)
    factors: list[Poly] = []

    while len(remaining_lifted) > 1:
        found = False
        max_size = len(remaining_lifted) // 2
        for size in range(1, max_size + 1):
            for subset_idx in combinations(range(len(remaining_lifted)), size):
                # Multiply selected lifted factors.
                prod: list[int] = [1]
                for i in subset_idx:
                    prod = _iz_mul(prod, remaining_lifted[i])
                # Center and make primitive.
                prod = _center_mod(prod, modulus)
                pp = primitive_part(prod)
                if not pp:
                    continue
                if pp[-1] < 0:
                    pp = [-c for c in pp]
                # Test divisibility.
                quot = _poly_divides_z(remaining_f, pp)
                if quot is not None:
                    factors.append(pp)
                    remaining_f = primitive_part(quot)
                    if remaining_f and remaining_f[-1] < 0:
                        remaining_f = [-c for c in remaining_f]
                    remaining_lifted = [
                        remaining_lifted[i]
                        for i in range(len(remaining_lifted))
                        if i not in set(subset_idx)
                    ]
                    found = True
                    break
            if found:
                break
        if not found:
            break  # remaining polynomial is irreducible

    # Any remaining non-trivial polynomial is irreducible.
    remaining_f = normalize(remaining_f)
    if remaining_f and remaining_f not in ([1], [-1]) and degree(remaining_f) >= 1:
        if remaining_f[-1] < 0:
            remaining_f = [-c for c in remaining_f]
        factors.append(remaining_f)

    return factors if factors else None


# ---------------------------------------------------------------------------
# Main public entry point
# ---------------------------------------------------------------------------


def bzh_factor(coeffs: list[int]) -> list[list[int]] | None:
    """Factor a **monic** primitive integer polynomial using the BZH algorithm.

    The Berlekamp-Zassenhaus-Hensel pipeline is:

    1. Verify the polynomial is monic and within the degree cap.
    2. Find a squarefree prime ``p`` for ``f mod p``.
    3. Factor ``f mod p`` over GF(p) using Berlekamp's algorithm.
    4. If only one modular factor exists, report irreducible.
    5. Lift the modular factors to precision ``> 2·B`` (Mignotte's bound)
       using the linear Hensel lift in divide-and-conquer mode.
    6. Try all subsets of lifted factors (Zassenhaus combination) and
       collect genuine divisors of ``f``.

    Parameters
    ----------
    coeffs : list[int]
        Integer coefficient list in ascending-degree order.
        The polynomial **must** be primitive (content = 1) and monic
        (leading coefficient = 1).

    Returns
    -------
    list[list[int]] | None
        A list of irreducible factor coefficient lists (each primitive,
        positive leading coefficient), or ``None`` when:

        - The polynomial is non-monic.
        - The degree exceeds ``MAX_DEGREE`` (= 20).
        - No squarefree prime was found (extremely rare in practice).
        - The Berlekamp factorization mod p yields only one factor
          (indicating ``f`` is irreducible over Q).
        - The Zassenhaus combination yields only the trivial factorization
          (confirming irreducibility).

    Examples
    --------
    ::

        bzh_factor([-1, 0, 0, 0, 0, 1])
        # [[-1, 1], [1, 1, 1, 1, 1]]    # x^5−1 = (x−1)(x^4+x^3+x^2+x+1)

        bzh_factor([1, 0, 0, 0, 1])
        # None                            # x^4+1 is irreducible over Q

        bzh_factor([-1, 0, 0, 0, 0, 0, 0, 0, 1])
        # [[-1,1],[1,1],[1,0,1],[1,0,0,0,1]]  # x^8−1 fully factored
    """
    f = normalize(coeffs)
    if not f:
        return None

    d = degree(f)
    if d < 2:
        return None  # degree 0 or 1 — always irreducible or linear (no split)
    if d > MAX_DEGREE:
        return None  # degree cap

    lc = f[-1]
    if lc != 1:
        return None  # restrict to monic polynomials

    # --- Step 1: Find a squarefree prime. ---
    good_prime: int | None = None
    for p in _SMALL_PRIMES:
        # Skip primes that kill the leading coefficient (already handled
        # by the monic check, but defensive for future non-monic extension).
        if lc % p == 0:
            continue
        if _is_squarefree_mod_p(f, p):
            good_prime = p
            break

    if good_prime is None:
        return None  # no suitable prime found

    p = good_prime

    # --- Step 2: Factor f mod p using Berlekamp. ---
    # f is monic, so f mod p is already monic (lc = 1 mod p for p > 1).
    f_mod = _pmod(f, p)
    mod_factors = _berlekamp_factor_mod_p(f_mod, p)

    if len(mod_factors) == 1:
        return None  # irreducible mod p → very likely irreducible over Q

    # --- Step 3: Hensel lift to precision > 2 * Zassenhaus_bound. ---
    B = _zassenhaus_bound(f)
    target = 2.0 * B + 1.0

    lifted = _multi_hensel_lift(f, mod_factors, p, target)
    if lifted is None:
        return None

    # Compute the actual modulus used for centering.
    mod = p
    while mod <= target:
        mod *= p

    # --- Step 4: Zassenhaus combination. ---
    combined = _combine_factors(f, lifted, mod)

    # Return None if the "factoring" gave back only a single factor equal to f.
    if combined is None:
        return None
    if len(combined) == 1:
        single = normalize(combined[0])
        if single[-1] < 0:
            single = [-c for c in single]
        orig = list(f)
        if single == orig:
            return None  # truly irreducible
    if len(combined) < 2:
        return None

    return combined
