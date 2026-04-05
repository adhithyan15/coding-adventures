"""
reed_solomon — Reed-Solomon error-correcting codes over GF(256).

Reed-Solomon (RS) is a block error-correcting code invented by Irving Reed and
Gustave Solomon in 1960.  The idea is simple: add redundancy to a message so
that even after some bytes are corrupted the original can be reconstructed.

Where RS codes appear:
  - QR codes: up to 30 % of the symbol can be scratched and still decoded.
  - CDs / DVDs: CIRC two-level RS corrects scratches and burst errors.
  - Hard drives: firmware sector-level error correction.
  - Voyager probes: images sent across 20 + billion kilometres.
  - RAID-6: the two parity drives ARE an (n, n-2) RS code over GF(256).

How it fits in the MA series:

    MA00 polynomial   — coefficient-array polynomial arithmetic
    MA01 gf256        — GF(2^8) field arithmetic  (add=XOR, mul=table lookup)
    MA02 reed-solomon — RS encoding / decoding (THIS PACKAGE)

An RS encoder is just polynomial multiplication over GF(256).
An RS decoder is Berlekamp-Massey + Chien search + Forney — all polynomial
operations over GF(256), composed into a 5-step pipeline.

Polynomial conventions
----------------------
Inside encode / decode, codeword bytes are treated as a **big-endian** polynomial:

    codeword[0] · x^{n-1}  +  codeword[1] · x^{n-2}  +  …  +  codeword[n-1]

The systematic layout is:
    [ message bytes (k) | check bytes (n_check) ]
      degree n-1 … n_check   degree n_check-1 … 0

This is the standard RS / QR code convention.  It is the *opposite* of MA00's
little-endian convention (index = degree).  The transition point is well-marked
in every function below.

For error position ``p`` in a big-endian codeword of length ``n``, the **locator
number** is ``X_p = α^{n-1-p}`` and its inverse is
``X_p⁻¹ = α^{(p + 256 - n) mod 255}``.
"""

from __future__ import annotations

from typing import Sequence

from gf256 import add, divide, multiply, power

VERSION = "0.1.0"


# =============================================================================
# Error Classes
# =============================================================================


class TooManyErrorsError(Exception):
    """Raised when decoding fails because there are more errors than t = n_check/2.

    The code can correct at most t byte errors.  If more are present the
    codeword is unrecoverable and this exception is raised rather than silently
    returning wrong data.
    """

    def __init__(self) -> None:
        super().__init__(
            "reed-solomon: too many errors — codeword is unrecoverable"
        )


class InvalidInputError(Exception):
    """Raised when encode / decode receives invalid parameters.

    Common causes:
    - n_check is 0 or odd (must be a positive even number)
    - total codeword length exceeds 255 (the GF(256) block size limit)
    - received codeword is shorter than n_check
    """

    def __init__(self, reason: str) -> None:
        super().__init__(f"reed-solomon: invalid input — {reason}")


# =============================================================================
# Generator Polynomial
# =============================================================================


def build_generator(n_check: int) -> list[int]:
    """Build the RS generator polynomial for a given number of check bytes.

    The generator is the product of ``n_check`` linear factors:

        g(x) = (x + α¹)(x + α²) … (x + α^{n_check})

    where ``α = 2`` is the primitive element of GF(256).

    Return value
    ------------
    A **little-endian** coefficient list (index = degree), length ``n_check + 1``.
    The last element is always ``1`` (the leading / monic coefficient of x^{n_check}).

    Algorithm
    ---------
    Start with g = [1].  At each step multiply in the next linear factor (αⁱ + x):

        for j in range(len(g)):
            new_g[j]   ^= gf256.multiply(α^i, g[j])   ← coefficient · α^i
            new_g[j+1] ^= g[j]                         ← coefficient · x

    Example: n_check = 2
    ~~~~~~~~~~~~~~~~~~~~

        Start: g = [1]
        i=1: α¹ = 2
          new_g = [mul(1,2), 1] = [2, 1]
        i=2: α² = 4
          j=0: new_g[0] ^= mul(2,4)=8 → 8;  new_g[1] ^= 2
          j=1: new_g[1] ^= mul(1,4)=4 → 2^4=6;  new_g[2] ^= 1
          g = [8, 6, 1]

    Verify α¹=2 is a root:
        g(2) = 8 + 6·2 + 1·4  (all GF(256))
             = 8 ^ mul(6,2) ^ mul(1,4)
             = 8 ^ 12 ^ 4
             = 0  ✓   (all XOR to zero)

    Raises
    ------
    InvalidInputError
        If ``n_check`` is 0 or odd.
    """
    if n_check == 0 or n_check % 2 != 0:
        raise InvalidInputError(
            f"n_check must be a positive even number, got {n_check}"
        )

    g: list[int] = [1]

    for i in range(1, n_check + 1):
        alpha_i = power(2, i)                  # α^i  (GF(256) element)
        new_g = [0] * (len(g) + 1)
        for j, coeff in enumerate(g):
            new_g[j] ^= multiply(coeff, alpha_i)   # coeff · α^i  (low end)
            new_g[j + 1] ^= coeff                  # coeff · x    (shift up)
        g = new_g

    return g


# =============================================================================
# Internal Polynomial Helpers
# =============================================================================


def _poly_eval_be(p: Sequence[int], x: int) -> int:
    """Evaluate a **big-endian** GF(256) polynomial at ``x`` using Horner's method.

    ``p[0]`` is the highest-degree coefficient.  Iteration goes left to right:

        acc = 0
        for each byte b in p (highest degree first):
            acc = acc · x  +  b          (all GF(256) arithmetic)

    This is used for syndrome evaluation:  S_j = codeword(α^j).

    An error at position ``p_err`` contributes
    ``e · (α^j)^{n-1-p_err} = e · X_{p_err}^j`` to S_j, where
    ``X_{p_err} = α^{n-1-p_err}`` is the error locator number.
    """
    acc = 0
    for b in p:
        acc = add(multiply(acc, x), b)
    return acc


def _poly_eval_le(p: Sequence[int], x: int) -> int:
    """Evaluate a **little-endian** GF(256) polynomial at ``x`` using Horner's method.

    ``p[i]`` is the coefficient of ``x^i``.  We iterate from the highest degree
    down to degree 0:

        acc = 0
        for i from len(p)-1 down to 0:
            acc = acc · x  +  p[i]

    Used for evaluating Λ(x), Ω(x), and Λ'(x) in the Chien / Forney steps.
    """
    acc = 0
    for coeff in reversed(p):
        acc = add(multiply(acc, x), coeff)
    return acc


def _poly_mul_le(a: Sequence[int], b: Sequence[int]) -> list[int]:
    """Multiply two **little-endian** GF(256) polynomials (convolution).

    The result has degree deg(a) + deg(b), so length len(a)+len(b)-1.

    Schoolbook multiplication:

        result[i + j] ^= a[i] · b[j]    for all i, j

    In GF(256), addition is XOR, so ^= is the right operator.

    Used in the Forney step to compute  Ω(x) = S(x) · Λ(x)  mod x^{2t}.
    """
    if not a or not b:
        return []
    result = [0] * (len(a) + len(b) - 1)
    for i, ai in enumerate(a):
        for j, bj in enumerate(b):
            result[i + j] ^= multiply(ai, bj)
    return result


def _poly_mod_be(dividend: Sequence[int], divisor: Sequence[int]) -> list[int]:
    """Remainder of **big-endian** GF(256) polynomial long division.

    Both ``dividend`` and ``divisor`` are big-endian (first element = highest degree).
    The divisor **must be monic** (leading coefficient = 1); this is guaranteed
    because the generator polynomial g(x) is monic by construction.

    Algorithm — schoolbook long division
    ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

        rem = list(dividend)
        for i in range(len(dividend) - len(divisor) + 1):
            coeff = rem[i]                # this is rem[i] / 1 since monic
            if coeff == 0:
                continue
            for j in range(len(divisor)):
                rem[i + j] ^= coeff · divisor[j]

    After the loop, the remainder sits in the last (len(divisor)-1) elements.

    Why this works
    ~~~~~~~~~~~~~~
    At each step we zero out ``rem[i]`` by subtracting ``coeff · g(x) · x^{...}``.
    In characteristic 2, subtraction equals addition, so ^= does both.

    Returns
    -------
    A list of length ``len(divisor) - 1`` (the remainder coefficients, big-endian).
    If ``dividend`` is shorter than ``divisor``, returns a copy of dividend.
    """
    rem: list[int] = list(dividend)
    div_len = len(divisor)

    if len(rem) < div_len:
        return rem

    steps = len(rem) - div_len + 1
    for i in range(steps):
        coeff = rem[i]
        if coeff == 0:
            continue
        for j, d in enumerate(divisor):
            rem[i + j] ^= multiply(coeff, d)

    return rem[len(rem) - (div_len - 1):]


def _inv_locator(p: int, n: int) -> int:
    """Inverse locator X_p⁻¹ for byte position ``p`` in a codeword of length ``n``.

    Big-endian convention: position ``p`` corresponds to the term with degree ``n-1-p``.

        X_p  = α^{n-1-p}
        X_p⁻¹ = α^{(p + 256 - n) mod 255}

    The +256 before % 255 keeps the exponent non-negative when ``p < n - 256``,
    which cannot happen for valid inputs (n ≤ 255, 0 ≤ p < n) but the formula is
    written this way for symmetry with the TypeScript / Rust reference.

    Special cases:
    - p = 0 (first byte, degree n-1):  exp = (256-n) mod 255
    - p = n-1 (last byte, degree 0):   exp = (n-1+256-n) mod 255 = 255 mod 255 = 0
                                        → X⁻¹ = α⁰ = 1
    """
    exp = (p + 256 - n) % 255
    return power(2, exp)


# =============================================================================
# Encoding
# =============================================================================


def encode(message: bytes | bytearray, n_check: int) -> bytes:
    """Encode a message with Reed-Solomon, producing a systematic codeword.

    **Systematic** encoding means the original message bytes appear unchanged
    at the start of the output, followed by the computed check bytes:

        output = [ message bytes (k) | check bytes (n_check) ]

    Algorithm
    ---------
    1. Build the generator polynomial g (little-endian LE).
    2. Reverse g to big-endian BE  (g_LE[-1]=1 becomes g_BE[0]=1 — monic).
    3. Form the shifted message:
           shifted = message  ||  [0] * n_check
       This represents M(x) · x^{n_check} in big-endian form.
    4. Compute the remainder:  R = shifted mod g_BE.
    5. Output:  message  ||  R   (R padded to exactly n_check bytes on the left).

    Why it works
    ~~~~~~~~~~~~
    The codeword polynomial is:
        C(x) = M(x)·x^{n_check}  XOR  R(x)
             = Q(x)·g(x)          (by the division algorithm)

    So C(αⁱ) = Q(αⁱ)·g(αⁱ) = 0 for i = 1 … n_check, because αⁱ is a root
    of g(x).  This is the fundamental property the decoder exploits.

    Parameters
    ----------
    message:
        Raw data bytes (arbitrary content, arbitrary length k).
    n_check:
        Number of check bytes to append.  Must be a positive even integer.
        Controls the error-correction capacity: t = n_check // 2 byte errors.

    Returns
    -------
    bytes of length k + n_check.

    Raises
    ------
    InvalidInputError
        If n_check is 0 or odd, or if k + n_check > 255.
    """
    if n_check == 0 or n_check % 2 != 0:
        raise InvalidInputError(
            f"n_check must be a positive even number, got {n_check}"
        )
    n = len(message) + n_check
    if n > 255:
        raise InvalidInputError(
            f"total codeword length {n} exceeds GF(256) block size limit of 255"
        )

    # Build generator in LE, then reverse to BE for division.
    g_le = build_generator(n_check)
    g_be = list(reversed(g_le))   # g_be[0] = 1  (monic)

    # shifted = message || zeros  (big-endian representation of M(x)·x^{n_check})
    shifted: list[int] = list(message) + [0] * n_check

    # Remainder of big-endian division by monic g_be.
    remainder = _poly_mod_be(shifted, g_be)   # len == n_check  (usually)

    # Codeword = message || check_bytes.
    # If remainder is shorter than n_check (leading coefficients were 0),
    # pad with leading zeros.
    check = bytes(n_check - len(remainder)) + bytes(remainder)

    return bytes(message) + check


# =============================================================================
# Syndromes
# =============================================================================


def syndromes(received: bytes | bytearray, n_check: int) -> list[int]:
    """Compute the ``n_check`` syndrome values of a received codeword.

    S_j = received(α^j)   for  j = 1, 2, …, n_check.

    A valid (uncorrupted) codeword satisfies C(αⁱ) = 0 for all i = 1 … n_check,
    because C(x) is divisible by g(x) = ∏(x + αⁱ).

    An error at position ``p_err`` changes syndrome S_j by
    ``e · X_{p_err}^j``  where  ``X_{p_err} = α^{n-1-p_err}``.

    If every syndrome is zero the codeword has no errors; any non-zero syndrome
    reveals corruption.

    Parameters
    ----------
    received:
        Codeword bytes (possibly corrupted).
    n_check:
        Number of check bytes in the codeword.

    Returns
    -------
    List of n_check ints in [0, 255].  All-zero means no errors detected.
    """
    return [_poly_eval_be(received, power(2, j)) for j in range(1, n_check + 1)]


# =============================================================================
# Berlekamp-Massey Algorithm
# =============================================================================


def _berlekamp_massey(synds: Sequence[int]) -> tuple[list[int], int]:
    """Find the shortest LFSR that generates the syndrome sequence.

    The LFSR **connection polynomial** Λ(x) is the **error locator polynomial**.
    Its roots (evaluated at Λ(x)=0) are the inverses of the error locators X_k⁻¹.
    Finding those roots via Chien search reveals the error positions.

    If errors occurred at positions with locators X₁, X₂, …, X_v, then:

        Λ(x) = ∏_{k=1}^{v} (1 - X_k · x)    Λ(0) = 1

    Algorithm  (adapted from Massey 1969 / classic text-book presentation)
    ~~~~~~~~~~~

    Inputs:  syndromes S[0..2t-1]   (0-based indexing)
    Output:  (Λ, L)  where Λ is LE and L is the number of errors.

        C = [1],  B = [1],  L = 0,  x_shift = 1,  b_scale = 1

        for n from 0 to 2t-1:

            # Compute discrepancy
            d = S[n]  XOR  ∑_{j=1}^{L}  Λ[j] · S[n-j]

            if d == 0:
                x_shift += 1                             # no update
            elif 2·L ≤ n:
                # More errors than modelled — extend Λ
                T          = copy(C)
                scale      = d / b_scale                 # GF(256) division
                C          = C  XOR  (scale · x^{x_shift} · B)
                L          = n + 1 - L
                B, b_scale = T, d
                x_shift    = 1
            else:
                # Consistent update — adjust without growing
                scale  = d / b_scale
                C      = C  XOR  (scale · x^{x_shift} · B)
                x_shift += 1

        return C, L

    The inner operation ``C XOR (scale · x^{x_shift} · B)`` shifts B left by
    x_shift positions (prepends x_shift zeros), multiplies every coefficient by
    scale, then XORs element-wise with C.

    Returns
    -------
    (lambda_poly, num_errors)
        lambda_poly : list[int], little-endian, Λ[0] = 1.
        num_errors  : int, degree of Λ (= number of errors found).
    """
    two_t = len(synds)

    c: list[int] = [1]          # current error locator Λ (LE)
    b: list[int] = [1]          # previous Λ (LE)
    big_l = 0                   # current number of errors
    x_shift = 1                 # iterations since last update
    b_scale = 1                 # discrepancy at the last update step

    for n in range(two_t):

        # -----------------------------------------------------------------
        # Compute discrepancy  d = S[n]  +  Σ_{j=1}^{L}  Λ[j] · S[n-j]
        # -----------------------------------------------------------------
        d = synds[n]
        for j in range(1, big_l + 1):
            if j < len(c) and n >= j:
                d ^= multiply(c[j], synds[n - j])

        # -----------------------------------------------------------------
        # Update rule
        # -----------------------------------------------------------------
        if d == 0:
            x_shift += 1

        elif 2 * big_l <= n:
            # Found more errors than currently modelled — grow Λ.
            t_save = c[:]                          # save current Λ

            scale = divide(d, b_scale)
            # Extend c if needed to hold x_shift + len(b) coefficients.
            target_len = x_shift + len(b)
            if len(c) < target_len:
                c = c + [0] * (target_len - len(c))
            for k, bk in enumerate(b):
                c[x_shift + k] ^= multiply(scale, bk)

            big_l = n + 1 - big_l
            b = t_save
            b_scale = d
            x_shift = 1

        else:
            # Consistent update — adjust Λ without growing the degree.
            scale = divide(d, b_scale)
            target_len = x_shift + len(b)
            if len(c) < target_len:
                c = c + [0] * (target_len - len(c))
            for k, bk in enumerate(b):
                c[x_shift + k] ^= multiply(scale, bk)
            x_shift += 1

    return c, big_l


# =============================================================================
# Chien Search
# =============================================================================


def _chien_search(lam: Sequence[int], n: int) -> list[int]:
    """Find which byte positions are error locations via exhaustive search.

    Position ``p`` is an error location if and only if Λ(X_p⁻¹) = 0, where:

        X_p⁻¹ = α^{(p + 256 - n) mod 255}   (``_inv_locator(p, n)``)

    We test all n positions p = 0, 1, …, n-1 and collect the matches.

    Correctness
    ~~~~~~~~~~~
    Λ(x) = ∏_{k} (1 - X_k·x).  This polynomial evaluates to zero when
    ``x = X_k⁻¹`` for each error locator X_k.  So the Chien search is just
    "evaluate Λ at every candidate inverse locator and pick the zeros."

    Parameters
    ----------
    lam:
        Error locator polynomial in LE form (Λ[0] = 1).
    n:
        Total codeword length.

    Returns
    -------
    Sorted list of error positions (0-indexed, big-endian order).
    """
    positions: list[int] = []
    for p in range(n):
        xi_inv = _inv_locator(p, n)
        if _poly_eval_le(lam, xi_inv) == 0:
            positions.append(p)
    return positions


# =============================================================================
# Forney Algorithm
# =============================================================================


def _forney(
    lam: Sequence[int],
    synds: Sequence[int],
    positions: list[int],
    n: int,
) -> list[int]:
    """Compute error magnitudes from known error positions.

    For each error at position ``p``:

        e_p = Ω(X_p⁻¹)  /  Λ'(X_p⁻¹)

    where:
    - ``Ω(x) = (S(x) · Λ(x)) mod x^{2t}``  — error evaluator polynomial
    - ``S(x) = S₁ + S₂x + … + S_{2t}x^{2t-1}``  — syndrome polynomial (LE)
    - ``Λ'(x)``  — formal derivative of Λ in GF(2^8)

    Background: why this formula works
    ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    The received polynomial is r(x) = c(x) + e(x) where c(x) is the true
    codeword and e(x) = ∑_k e_k · x^{n-1-p_k} is the sparse error polynomial.

    The syndromes satisfy S_j = r(α^j) = e(α^j) (since c(α^j) = 0).

    The **key identity** is:

        S(x) · Λ(x) ≡ Ω(x)  (mod x^{2t})

    This encodes the error magnitudes in Ω.  Forney's formula recovers them by
    differentiating Λ and dividing.

    Formal derivative in characteristic 2
    ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    In GF(2^8), 2 = 0, so the usual derivative rule ``d/dx(ax^n) = n·a·x^{n-1}``
    kills every even-degree term (their coefficients have a factor of 2 = 0):

        Λ'(x) = Λ₁ + 0·x + Λ₃x² + 0·x³ + Λ₅x⁴ + …

    In code: keep only the coefficients at **odd** indices; reduce their index by 1.

    Parameters
    ----------
    lam:
        Error locator polynomial (LE).
    synds:
        Syndrome list of length 2t.
    positions:
        Error positions found by Chien search.
    n:
        Total codeword length.

    Returns
    -------
    List of error magnitudes, one per position (same order as ``positions``).

    Raises
    ------
    TooManyErrorsError
        If the formal derivative evaluates to zero at any error locator inverse
        (indicates the codeword is too badly corrupted to correct).
    """
    two_t = len(synds)

    # -----------------------------------------------------------------
    # Ω(x) = S(x) · Λ(x)  mod  x^{2t}
    # S is already in LE form: S[0] = S₁, S[1] = S₂, …
    # -----------------------------------------------------------------
    omega_full = _poly_mul_le(list(synds), list(lam))
    omega = omega_full[:two_t]   # truncate to degree < 2t

    # -----------------------------------------------------------------
    # Formal derivative Λ'(x): keep odd-indexed Λ coefficients,
    # shift each down by 1.
    # Λ'[j-1] = Λ[j]  for j odd (j = 1, 3, 5, …)
    # -----------------------------------------------------------------
    lambda_prime = [0] * max(0, len(lam) - 1)
    for j in range(1, len(lam)):
        if j % 2 == 1:                          # odd index — survives derivative
            lambda_prime[j - 1] ^= lam[j]

    # -----------------------------------------------------------------
    # Error magnitude for each position
    # -----------------------------------------------------------------
    magnitudes: list[int] = []
    for pos in positions:
        xi_inv = _inv_locator(pos, n)
        omega_val = _poly_eval_le(omega, xi_inv)
        lp_val    = _poly_eval_le(lambda_prime, xi_inv)
        if lp_val == 0:
            raise TooManyErrorsError()
        magnitudes.append(divide(omega_val, lp_val))

    return magnitudes


# =============================================================================
# Public API
# =============================================================================


def error_locator(synds: Sequence[int]) -> list[int]:
    """Compute the error locator polynomial Λ(x) from a syndrome array.

    Runs the Berlekamp-Massey algorithm and returns Λ in **little-endian** form
    with Λ[0] = 1.

    Exposed for advanced use cases (QR decoders, diagnostics) where a caller
    may want to run BM without a full decode.

    Parameters
    ----------
    synds:
        Syndrome sequence of length 2t.

    Returns
    -------
    list[int] in little-endian form, length = number-of-errors + 1.
    """
    lam, _ = _berlekamp_massey(synds)
    return lam


def decode(received: bytes | bytearray, n_check: int) -> bytes:
    """Decode a received codeword, correcting up to t = n_check // 2 byte errors.

    Five-step pipeline
    ------------------

        received bytes
             │
             ▼ Step 1: Syndromes S₁ … S_{n_check}
             │         all zero → return message directly (no errors)
             │
             ▼ Step 2: Berlekamp-Massey → Λ(x), error count L
             │         L > t → TooManyErrorsError
             │
             ▼ Step 3: Chien search → error positions {p₁ … pᵥ}
             │         |positions| ≠ L → TooManyErrorsError
             │
             ▼ Step 4: Forney → error magnitudes {e₁ … eᵥ}
             │
             ▼ Step 5: received[p_k] ^= e_k  for each k
             │
             ▼ Return first k = len(received) - n_check bytes

    Parameters
    ----------
    received:
        Possibly corrupted codeword bytes.
    n_check:
        Number of check bytes (must be even ≥ 2).

    Returns
    -------
    bytes — the recovered message (length = len(received) - n_check).

    Raises
    ------
    InvalidInputError
        If n_check is 0 / odd, or received is shorter than n_check.
    TooManyErrorsError
        If more than t errors are present.
    """
    if n_check == 0 or n_check % 2 != 0:
        raise InvalidInputError(
            f"n_check must be a positive even number, got {n_check}"
        )
    if len(received) < n_check:
        raise InvalidInputError(
            f"received length {len(received)} < n_check {n_check}"
        )

    t = n_check // 2
    n = len(received)
    k = n - n_check

    # ------------------------------------------------------------------
    # Step 1: Syndromes
    # ------------------------------------------------------------------
    synds = syndromes(received, n_check)
    if all(s == 0 for s in synds):
        return bytes(received[:k])

    # ------------------------------------------------------------------
    # Step 2: Berlekamp-Massey → error locator Λ and error count L
    # ------------------------------------------------------------------
    lam, num_errors = _berlekamp_massey(synds)
    if num_errors > t:
        raise TooManyErrorsError()

    # ------------------------------------------------------------------
    # Step 3: Chien search → error positions
    # ------------------------------------------------------------------
    positions = _chien_search(lam, n)
    if len(positions) != num_errors:
        raise TooManyErrorsError()

    # ------------------------------------------------------------------
    # Step 4: Forney → error magnitudes
    # ------------------------------------------------------------------
    magnitudes = _forney(lam, synds, positions, n)

    # ------------------------------------------------------------------
    # Step 5: Apply corrections
    # ------------------------------------------------------------------
    corrected = bytearray(received)
    for pos, mag in zip(positions, magnitudes):
        corrected[pos] ^= mag

    return bytes(corrected[:k])
