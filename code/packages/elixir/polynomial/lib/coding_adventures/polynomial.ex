# === Polynomial — MA00: Coefficient-Array Polynomial Arithmetic ===
#
# A polynomial is a mathematical expression like 3 + 2x + x² built from a
# variable x and constant coefficients. We represent it as a list of floats
# where the index equals the degree:
#
#   [3.0, 2.0, 1.0]   →   3 + 2x + x²
#    ^    ^    ^
#    |    |    └── coefficient of x²  (degree 2)
#    |    └─────── coefficient of x¹  (degree 1)
#    └──────────── coefficient of x⁰  (degree 0, the constant)
#
# This "index = degree" convention is sometimes called "little-endian" because
# the lowest-degree (smallest-power) term comes first. It makes addition
# trivially position-aligned and keeps Horner's method natural to read.
#
# Why does this package exist?
# ──────────────────────────────
# Polynomial arithmetic is the foundation of three important layers:
#
#   1. GF(2^8) arithmetic (MA01) — The Galois Field used by Reed-Solomon and AES
#      is defined by arithmetic in a polynomial ring modulo an irreducible polynomial.
#   2. Reed-Solomon error correction (MA02) — A codeword is a polynomial evaluated
#      at specific points. Encoding is multiplication; decoding uses GCD.
#   3. Checksums and CRCs — A CRC is the remainder after polynomial division
#      modulo a generator polynomial over GF(2).

defmodule CodingAdventures.Polynomial do
  @moduledoc """
  Polynomial arithmetic over real numbers (float coefficients).

  A polynomial is represented as a list of floats where the index equals the
  degree of that term. For example, `[3.0, 0.0, 1.0]` represents `3 + x²`.

  All functions return **normalized** polynomials — trailing near-zero
  coefficients (absolute value < 1.0e-10) are stripped. So `[1.0, 0.0, 0.0]`
  and `[1.0]` both represent the constant polynomial 1.

  The zero polynomial is represented as `[0.0]` (after normalization from
  an empty result). The degree of the zero polynomial is 0 (internally treated
  as -1 by `degree/1` for the long division loop termination condition).
  """

  # ---------------------------------------------------------------------------
  # Fundamentals
  # ---------------------------------------------------------------------------

  @doc """
  Remove trailing near-zero coefficients from a polynomial.

  Trailing zeros represent zero-coefficient high-degree terms. They do not
  change the mathematical value but affect degree comparisons and the stopping
  condition in polynomial long division.

  A coefficient is considered "zero" if its absolute value is less than
  `1.0e-10` — this threshold handles floating-point rounding that accumulates
  during repeated arithmetic operations.

  Returns `[0.0]` for the zero polynomial (never returns an empty list from
  the public API).

  Examples:
      iex> CodingAdventures.Polynomial.normalize([1.0, 0.0, 0.0])
      [1.0]
      iex> CodingAdventures.Polynomial.normalize([0.0])
      [0.0]
      iex> CodingAdventures.Polynomial.normalize([1.0, 2.0, 3.0])
      [1.0, 2.0, 3.0]
  """
  def normalize(poly) do
    # Drop trailing near-zero entries by walking backward from the high end.
    # We use List.foldr to build the result from right to left, keeping a
    # "found_nonzero" flag so we stop dropping once we find a real coefficient.
    result =
      poly
      |> Enum.reverse()
      |> Enum.reduce({[], false}, fn coeff, {acc, found} ->
        if found or abs(coeff) > 1.0e-10 do
          {[coeff | acc], true}
        else
          {acc, false}
        end
      end)
      |> elem(0)

    # Never return an empty list — the zero polynomial is [0.0].
    if result == [], do: [0.0], else: result
  end

  @doc """
  Return the degree of a polynomial.

  The degree is the index of the highest non-zero coefficient.

  Convention for the zero polynomial:
  - Publicly (caller-facing), `degree([0.0])` returns 0 — the zero polynomial
    is treated as a constant for user-facing operations.
  - Internally, the long division loop uses a raw length-based degree of -1
    for the empty internal representation to terminate correctly.

  Examples:
      iex> CodingAdventures.Polynomial.degree([3.0, 0.0, 2.0])
      2
      iex> CodingAdventures.Polynomial.degree([7.0])
      0
      iex> CodingAdventures.Polynomial.degree([0.0])
      0
  """
  def degree(poly) do
    # Use the internal helper that returns -1 for zero.
    d = internal_degree(poly)
    max(d, 0)
  end

  # internal_degree/1 returns -1 for the zero polynomial.
  # This is used inside divmod_poly/2 to drive the termination condition:
  #   "continue while degree(remainder) >= degree(divisor)"
  # When the remainder is zero, internal_degree returns -1, which is less than
  # any non-negative divisor degree, so the loop stops.
  defp internal_degree(poly) do
    n = normalize(poly)
    if n == [0.0], do: -1, else: length(n) - 1
  end

  @doc """
  Return the zero polynomial `[0.0]`.

  Zero is the additive identity: `add(zero(), p) = p` for any polynomial `p`.
  """
  def zero(), do: [0.0]

  @doc """
  Return the multiplicative identity polynomial `[1.0]`.

  Multiplying any polynomial by `one()` returns that polynomial unchanged:
  `multiply(p, one()) = p`.
  """
  def one(), do: [1.0]

  # ---------------------------------------------------------------------------
  # Addition and Subtraction
  # ---------------------------------------------------------------------------

  @doc """
  Add two polynomials term-by-term.

  Addition is the simplest operation: add matching coefficients, padding the
  shorter polynomial with implicit zeros.

  Visual example:
      [1.0, 2.0, 3.0]   =  1 + 2x + 3x²
    + [4.0, 5.0]         =  4 + 5x
    ──────────────────
      [5.0, 7.0, 3.0]   =  5 + 7x + 3x²

  The degree-2 term had no partner in b, so it carried through unchanged.
  """
  def add(a, b) do
    len = max(length(a), length(b))

    # Pad both lists to the same length with zeros before zipping.
    a_padded = pad_right(a, len)
    b_padded = pad_right(b, len)

    result = Enum.zip_with(a_padded, b_padded, fn ai, bi -> ai + bi end)
    normalize(result)
  end

  @doc """
  Subtract polynomial b from polynomial a term-by-term.

  Equivalent to adding a and the negation of b. Implemented directly to avoid
  creating an intermediate negated copy.

  Visual example:
      [5.0, 7.0, 3.0]   =  5 + 7x + 3x²
    - [1.0, 2.0, 3.0]   =  1 + 2x + 3x²
    ──────────────────
      [4.0, 5.0, 0.0]   →  normalize  →  [4.0, 5.0]   =  4 + 5x

  Note: 3x² - 3x² = 0; normalize strips the trailing zero.
  """
  def subtract(a, b) do
    len = max(length(a), length(b))
    a_padded = pad_right(a, len)
    b_padded = pad_right(b, len)

    result = Enum.zip_with(a_padded, b_padded, fn ai, bi -> ai - bi end)
    normalize(result)
  end

  # ---------------------------------------------------------------------------
  # Multiplication
  # ---------------------------------------------------------------------------

  @doc """
  Multiply two polynomials using polynomial convolution.

  Each term a[i]·xⁱ of a multiplies each term b[j]·xʲ of b, contributing
  `a[i] * b[j]` to the result's coefficient at index `i + j`.

  If a has degree m and b has degree n, the result has degree m + n, so the
  result list has length `m + n + 1`.

  Visual example:
      [1.0, 2.0]  =  1 + 2x
    × [3.0, 4.0]  =  3 + 4x
    ────────────────────────────────
    result = [0.0, 0.0, 0.0]   (length = 2 + 2 - 1 = 3)
      i=0, j=0: result[0] += 1·3 = 3   → [3.0, 0.0, 0.0]
      i=0, j=1: result[1] += 1·4 = 4   → [3.0, 4.0, 0.0]
      i=1, j=0: result[1] += 2·3 = 6   → [3.0, 10.0, 0.0]
      i=1, j=1: result[2] += 2·4 = 8   → [3.0, 10.0, 8.0]

    Result: [3.0, 10.0, 8.0]  =  3 + 10x + 8x²
    Verify: (1+2x)(3+4x) = 3+4x+6x+8x² = 3+10x+8x²  ✓
  """
  def multiply(a, b) do
    # Multiplying by zero yields zero.
    if internal_degree(a) < 0 or internal_degree(b) < 0 do
      [0.0]
    else
      result_len = length(a) + length(b) - 1
      result = List.duplicate(0.0, result_len)

      # For each pair (i, j), accumulate a[i] * b[j] into result[i+j].
      # We use Enum.with_index and a mutable accumulator pattern.
      result =
        a
        |> Enum.with_index()
        |> Enum.reduce(result, fn {ai, i}, acc ->
          b
          |> Enum.with_index()
          |> Enum.reduce(acc, fn {bj, j}, inner_acc ->
            List.update_at(inner_acc, i + j, fn v -> v + ai * bj end)
          end)
        end)

      normalize(result)
    end
  end

  # ---------------------------------------------------------------------------
  # Division
  # ---------------------------------------------------------------------------

  @doc """
  Perform polynomial long division, returning `{quotient, remainder}`.

  Given polynomials a and b (b ≠ zero), finds q and r such that:
      a = b × q + r   and   degree(r) < degree(b)

  The algorithm is the polynomial analog of school long division:
  1. Find the leading term of the current remainder.
  2. Divide it by the leading term of b to get the next quotient term.
  3. Subtract (quotient term) × b from the remainder.
  4. Repeat until degree(remainder) < degree(b).

  Detailed example: divide [5.0, 1.0, 3.0, 2.0] = 5 + x + 3x² + 2x³
  by [2.0, 1.0] = 2 + x:

      Step 1: remainder = [5,1,3,2], deg=3. Leading = 2x³, divisor leading = x.
              Quotient term: 2x³/x = 2x² → q[2] = 2.0
              Subtract 2x² × (2+x) = 4x²+2x³ = [0,0,4,2] from remainder:
              [5,1,-1,0] → normalize → [5,1,-1]

      Step 2: remainder = [5,1,-1], deg=2. Leading = -x², divisor leading = x.
              Quotient term: -x²/x = -x → q[1] = -1.0
              Subtract -x × (2+x) = -2x-x² = [0,-2,-1] from [5,1,-1]:
              [5,3,0] → [5,3]

      Step 3: remainder = [5,3], deg=1. Leading = 3x, divisor leading = x.
              Quotient term: 3x/x = 3 → q[0] = 3.0
              Subtract 3 × (2+x) = 6+3x = [6,3] from [5,3]:
              [-1,0] → [-1]

      Step 4: degree([-1]) = 0 < 1 = degree(b). STOP.
      Result: q = [3.0, -1.0, 2.0],  r = [-1.0]
      Verify: (x+2)(3-x+2x²) + (-1) = 3x-x²+2x³+6-2x+4x² - 1 = 5+x+3x²+2x³  ✓

  Raises `ArgumentError` if b is the zero polynomial.
  """
  def divmod_poly(dividend, divisor) do
    nb = normalize(divisor)

    if nb == [0.0] do
      raise ArgumentError, "polynomial division by zero"
    end

    na = normalize(dividend)
    deg_a = internal_degree(na)
    deg_b = internal_degree(nb)

    # If a has lower degree than b, quotient is zero, remainder is a.
    if deg_a < deg_b do
      {[0.0], na}
    else
      # Work on a mutable-style copy of the remainder as a tuple-indexed structure.
      # We represent it as a list and use indexed updates.
      rem = na |> pad_right(deg_a + 1)
      quot = List.duplicate(0.0, deg_a - deg_b + 1)

      # The leading coefficient of the divisor — used to compute each quotient term.
      lead_b = Enum.at(nb, deg_b)

      # Run the long-division loop.
      {quot_final, rem_final} = long_division_loop(rem, quot, nb, deg_b, lead_b, deg_a)

      {normalize(quot_final), normalize(rem_final)}
    end
  end

  # long_division_loop/6 performs the iterative polynomial long division.
  # It processes the remainder from the highest degree down to degree(b),
  # subtracting multiples of the divisor at each step.
  defp long_division_loop(rem, quot, nb, deg_b, lead_b, deg_rem) when deg_rem >= deg_b do
    lead_rem = Enum.at(rem, deg_rem)
    coeff = lead_rem / lead_b
    power = deg_rem - deg_b

    # Update the quotient at position `power`.
    quot = List.replace_at(quot, power, coeff)

    # Subtract coeff * x^power * b from rem.
    # For each coefficient of b at index j, subtract coeff * nb[j] from rem[power + j].
    rem =
      nb
      |> Enum.with_index()
      |> Enum.reduce(rem, fn {bj, j}, acc ->
        List.update_at(acc, power + j, fn v -> v - coeff * bj end)
      end)

    # Find the new effective degree of the remainder by skipping trailing near-zeros.
    new_deg_rem = find_effective_degree(rem, deg_rem - 1)

    long_division_loop(rem, quot, nb, deg_b, lead_b, new_deg_rem)
  end

  defp long_division_loop(rem, quot, _nb, _deg_b, _lead_b, _deg_rem) do
    {quot, rem}
  end

  # find_effective_degree/2 walks backward from `start` to find the highest
  # index with a non-near-zero entry. Returns -1 if all are zero.
  defp find_effective_degree(_list, idx) when idx < 0, do: -1

  defp find_effective_degree(list, idx) do
    val = Enum.at(list, idx)

    if abs(val) > 1.0e-10 do
      idx
    else
      find_effective_degree(list, idx - 1)
    end
  end

  @doc """
  Return the quotient of dividing a by b.

  Delegates to `divmod_poly/2` and returns only the first element.

  Raises `ArgumentError` if b is the zero polynomial.
  """
  def divide(a, b) do
    {q, _r} = divmod_poly(a, b)
    q
  end

  @doc """
  Return the remainder of dividing a by b (polynomial modulo).

  In GF(2^8) construction, we reduce a high-degree polynomial modulo the
  primitive polynomial using this operation.

  Raises `ArgumentError` if b is the zero polynomial.
  """
  def modulo(a, b) do
    {_q, r} = divmod_poly(a, b)
    r
  end

  # ---------------------------------------------------------------------------
  # Evaluation
  # ---------------------------------------------------------------------------

  @doc """
  Evaluate a polynomial at point x using Horner's method.

  **Horner's method** rewrites the polynomial in nested form:
      a₀ + x(a₁ + x(a₂ + ... + x·aₙ))

  This requires only n additions and n multiplications — no exponentiation.

  Algorithm (reading coefficients from high degree down to the constant):
      acc = 0
      for i from n downto 0:
          acc = acc * x + p[i]
      return acc

  Example: evaluate `[3.0, 1.0, 2.0]` = 3 + x + 2x² at x = 4:
      Start: acc = 0
      i=2: acc = 0*4 + 2 = 2
      i=1: acc = 2*4 + 1 = 9
      i=0: acc = 9*4 + 3 = 39
      Verify: 3 + 4 + 2·16 = 3 + 4 + 32 = 39  ✓

  For the zero polynomial, returns 0.
  """
  def evaluate(poly, x) do
    n = normalize(poly)

    if n == [0.0] do
      0.0
    else
      # Horner's method iterates from high-degree coefficient to low.
      # Enum.reduce on the reversed list (high to low) with accumulator.
      n
      |> Enum.reverse()
      |> Enum.reduce(0.0, fn coeff, acc -> acc * x + coeff end)
    end
  end

  # ---------------------------------------------------------------------------
  # Greatest Common Divisor
  # ---------------------------------------------------------------------------

  @doc """
  Compute the greatest common divisor (GCD) of two polynomials.

  Uses the **Euclidean algorithm**: repeatedly replace `(a, b)` with
  `(b, a mod b)` until b is the zero polynomial. The last non-zero remainder
  is the GCD.

  This is identical to the integer GCD algorithm, with polynomial modulo in
  place of integer modulo:

      gcd(a, b):
          while b ≠ zero:
              a, b = b, a mod b
          return normalize(a)

  The result is always returned in **monic** form — scaled so the leading
  coefficient is 1.0.

  Use case: GCD is used in Reed-Solomon decoding (extended Euclidean algorithm)
  to find the error-locator and error-evaluator polynomials.

  Example:
      gcd([6.0, 5.0, 1.0], [2.0]) = [1.0]  (constant gcd → monic = 1)
  """
  def gcd(a, b) do
    u = normalize(a)
    v = normalize(b)
    result = euclidean_gcd(u, v)
    make_monic(result)
  end

  # euclidean_gcd/2 runs the Euclidean algorithm until b is the zero polynomial.
  # We use a guard instead of pattern matching on 0.0 because OTP 27+ treats
  # 0.0 pattern matching as matching only +0.0, not -0.0. Using normalize/1
  # comparison avoids this pitfall entirely.
  defp euclidean_gcd(u, v) do
    if normalize(v) == [0.0] do
      u
    else
      r = modulo(u, v)
      euclidean_gcd(v, r)
    end
  end

  # make_monic/1 scales a polynomial so its leading coefficient is 1.0.
  # A monic polynomial p(x) has leading coefficient 1; this is the canonical
  # form for polynomial GCD results.
  defp make_monic(poly) when poly == [0.0], do: [0.0]

  defp make_monic(poly) do
    lead = List.last(poly)
    Enum.map(poly, fn c -> c / lead end)
  end

  # ---------------------------------------------------------------------------
  # Private Helpers
  # ---------------------------------------------------------------------------

  # pad_right/2 appends 0.0 entries until the list has exactly `len` elements.
  # Used before element-wise addition and subtraction to align coefficients.
  defp pad_right(list, len) do
    current = length(list)

    if current >= len do
      list
    else
      list ++ List.duplicate(0.0, len - current)
    end
  end
end
