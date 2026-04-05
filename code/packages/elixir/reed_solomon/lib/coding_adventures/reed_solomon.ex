# === ReedSolomon — MA02: Reed-Solomon Error-Correcting Codes over GF(256) ===
#
# Reed-Solomon codes are the error-correcting backbone of QR codes, CDs, DVDs,
# deep-space probes (Voyager), and RAID-6. They work by treating a message as a
# polynomial over GF(256) and appending redundancy bytes (check bytes) computed
# from that polynomial. A decoder can then detect and fix up to t = n_check / 2
# corrupted bytes even without knowing where the corruption occurred.
#
# ## The Stack
#
#   MA00 polynomial    — coefficient-array arithmetic over any field
#   MA01 gf256         — GF(2^8) field arithmetic (add=XOR, mul=log-table)
#   MA02 reed_solomon  — this module: encodes and decodes over GF(256)
#
# ## Polynomial Conventions (shared across all language implementations)
#
#   - **Codewords** are big-endian byte arrays: index 0 holds the coefficient of
#     the highest-degree term. This matches the wire/storage format.
#   - **Internal polynomials** (Λ, Ω, generator g) are little-endian lists:
#     index 0 holds the constant term (degree 0 coefficient).
#
#   So a big-endian codeword `[c_0, c_1, ..., c_{n-1}]` represents:
#     c_0·x^{n-1} + c_1·x^{n-2} + ... + c_{n-1}·x^0
#
#   And a little-endian poly `[a_0, a_1, ..., a_m]` represents:
#     a_0 + a_1·x + a_2·x^2 + ... + a_m·x^m
#
# ## Five-Step Decoding Pipeline
#
#   1. Syndromes      S_j = received(α^j)  for j = 1..n_check
#   2. Berlekamp-Massey  → error locator polynomial Λ(x)
#   3. Chien search   → error positions (indices into the codeword)
#   4. Forney formula → error magnitudes (byte values to XOR)
#   5. Correction     → XOR error magnitudes into codeword at found positions
#
# All arithmetic on coefficients is in GF(256): add = XOR, mul/div = log tables.

defmodule CodingAdventures.ReedSolomon do
  alias CodingAdventures.GF256, as: GF

  @moduledoc """
  Reed-Solomon error-correcting codes over GF(256).

  Implements systematic RS encoding (message bytes preserved, check bytes
  appended) and full syndrome decoding (Berlekamp-Massey + Chien + Forney).

  Corrects up to `t = n_check / 2` byte errors anywhere in the codeword.

  ## Quick example

      message = [72, 101, 108, 108, 111]  # "Hello" as bytes
      n_check = 8

      codeword = ReedSolomon.encode(message, n_check)
      # length == 13, first 5 bytes == message

      # Corrupt 4 bytes (= t for n_check=8) — still recoverable
      codeword = List.replace_at(codeword, 0, Bitwise.bxor(Enum.at(codeword, 0), 0xFF))
      codeword = List.replace_at(codeword, 2, Bitwise.bxor(Enum.at(codeword, 2), 0xAA))

      recovered = ReedSolomon.decode(codeword, n_check)
      # recovered == message
  """

  # ── Error types ─────────────────────────────────────────────────────────────

  defmodule TooManyErrors do
    @moduledoc """
    Raised when the decoder cannot correct the received codeword.

    This means more than `t = n_check / 2` byte errors were present —
    beyond the correction capacity of the code.
    """
    defexception [:message]

    @impl true
    def exception(_opts) do
      %TooManyErrors{
        message:
          "reed-solomon: too many errors — correction capacity exceeded; " <>
            "use more check bytes (larger n_check) or assume the codeword is unrecoverable"
      }
    end
  end

  defmodule InvalidInput do
    @moduledoc """
    Raised for invalid inputs: bad `n_check` or oversized codewords.
    """
    defexception [:message]

    @impl true
    def exception(reason) when is_binary(reason) do
      %InvalidInput{message: "reed-solomon: invalid input — #{reason}"}
    end
  end

  # ── Public API ───────────────────────────────────────────────────────────────

  @doc """
  Build the monic generator polynomial `g(x) = ∏(x + αⁱ)` for i = 1..n_check.

  Returns a little-endian list of length `n_check + 1`. The first element is
  the constant term; the last element is always 1 (monic).

  ## Example

      build_generator(2)
      # [8, 6, 1]  — represents x^2 + 6x + 8 = (x+2)(x+4)

  `α = 2`, so α^1 = 2 and α^2 = 4.
  Check: (x+2)(x+4) in GF(256) → constant = GF.multiply(2,4) = 8,
         x-coefficient = GF.add(2, 4) = 6.
  """
  def build_generator(n_check) do
    # Start with g = [1] — the degree-0 polynomial "1".
    # At each step multiply in the factor (x + α^i) = [α^i, 1] in LE form.
    Enum.reduce(1..n_check, [1], fn i, g ->
      # α^i = GF.power(2, i)
      factor = [GF.power(2, i), 1]
      poly_mul_le(g, factor)
    end)
  end

  @doc """
  Encode `message` (list of byte integers) with `n_check` redundancy bytes.

  Returns a list of length `length(message) + n_check`. The first
  `length(message)` bytes are the original message (systematic encoding);
  the remaining `n_check` bytes are check bytes derived from the generator
  polynomial.

  ## Raises

  - `InvalidInput` if `n_check` is 0, odd, or the total codeword would exceed 255 bytes.
  """
  def encode(message, n_check) do
    validate_n_check!(n_check)
    n = length(message) + n_check

    if n > 255 do
      raise InvalidInput,
            "codeword length #{n} exceeds 255 — maximum GF(256) block size"
    end

    gen = build_generator(n_check)

    # Systematic encoding: compute message(x) * x^n_check mod g(x).
    # "Shift" the message up by n_check positions (append n_check zero bytes),
    # then reduce by the generator to get the check bytes.
    padded = message ++ List.duplicate(0, n_check)
    check = poly_mod_be(padded, gen)

    message ++ check
  end

  @doc """
  Compute the `n_check` syndrome values `S_j = received(α^j)` for j = 1..n_check.

  Returns a list of `n_check` GF(256) values. If all are zero, the codeword
  has no detectable errors. Any non-zero value indicates corruption.

  Uses big-endian (Horner) polynomial evaluation on the received codeword.
  """
  def syndromes(received, n_check) do
    Enum.map(1..n_check, fn j ->
      alpha_j = GF.power(2, j)
      poly_eval_be(received, alpha_j)
    end)
  end

  @doc """
  Compute the error locator polynomial `Λ(x)` using the Berlekamp-Massey algorithm.

  Takes a syndrome list (output of `syndromes/2`) and returns `Λ(x)` in
  little-endian form with `Λ[0] = 1`.

  `Λ(x) = 1 + Λ_1·x + Λ_2·x^2 + ... + Λ_t·x^t`

  The roots of `Λ(x)` are the inverses of the error locators X_p = α^p, where
  p is the position of an error in the codeword.
  """
  def error_locator(syndromes_list) do
    berlekamp_massey(syndromes_list)
  end

  @doc """
  Decode a (possibly corrupted) codeword. Returns the recovered message bytes.

  Runs the full 5-step decoding pipeline:
    1. Compute syndromes
    2. Berlekamp-Massey → error locator Λ(x)
    3. Chien search → error positions
    4. Forney formula → error magnitudes
    5. Correct and return message portion

  ## Raises

  - `TooManyErrors` if correction capacity is exceeded
  - `InvalidInput` if `n_check` is invalid or `received` is too short
  """
  def decode(received, n_check) do
    validate_n_check!(n_check)

    if length(received) < n_check do
      raise InvalidInput,
            "received codeword is shorter than n_check=#{n_check}"
    end

    synd = syndromes(received, n_check)

    if Enum.all?(synd, &(&1 == 0)) do
      # No errors — return message portion directly.
      Enum.take(received, length(received) - n_check)
    else
      locator = berlekamp_massey(synd)
      t = div(n_check, 2)
      num_errors = length(locator) - 1

      if num_errors > t do
        raise TooManyErrors
      end

      positions = chien_search(locator, length(received))

      if length(positions) != num_errors do
        raise TooManyErrors
      end

      magnitudes = forney(synd, locator, positions, length(received))

      corrected =
        Enum.zip(positions, magnitudes)
        |> Enum.reduce(received, fn {pos, mag}, acc ->
          current = Enum.at(acc, pos)
          List.replace_at(acc, pos, GF.add(current, mag))
        end)

      Enum.take(corrected, length(corrected) - n_check)
    end
  end

  # ── Private helpers ──────────────────────────────────────────────────────────

  # validate_n_check!/1
  #
  # n_check must be a positive even integer (so t = n_check / 2 is an integer).
  defp validate_n_check!(n_check) do
    cond do
      n_check <= 0 ->
        raise InvalidInput, "n_check must be positive, got #{n_check}"

      rem(n_check, 2) != 0 ->
        raise InvalidInput, "n_check must be even (so t = n_check/2 is an integer), got #{n_check}"

      true ->
        :ok
    end
  end

  # poly_eval_be/2
  #
  # Evaluate a big-endian polynomial at a point `x` using Horner's method.
  #
  # Big-endian: p[0] is the coefficient of the highest-degree term.
  # For codeword [c_0, c_1, ..., c_{n-1}]:
  #   result = c_0·x^{n-1} + c_1·x^{n-2} + ... + c_{n-1}
  #
  # Horner: accumulate left-to-right: acc = acc * x + c_i
  defp poly_eval_be(poly, x) do
    Enum.reduce(poly, 0, fn coef, acc ->
      GF.add(GF.multiply(acc, x), coef)
    end)
  end

  # poly_eval_le/2
  #
  # Evaluate a little-endian polynomial at a point `x` using Horner's method.
  #
  # Little-endian: p[0] is the constant term (degree 0).
  # We evaluate from the highest degree down (reverse the list first).
  defp poly_eval_le(poly, x) do
    poly
    |> Enum.reverse()
    |> Enum.reduce(0, fn coef, acc ->
      GF.add(GF.multiply(acc, x), coef)
    end)
  end

  # poly_mul_le/2
  #
  # Multiply two little-endian GF(256) polynomials.
  #
  # Result degree = deg(a) + deg(b), length = len(a) + len(b) - 1.
  # Coefficient of x^k in the product = Σ a[i] * b[k-i] for all valid i.
  defp poly_mul_le(a, b) do
    len_a = length(a)
    len_b = length(b)
    result_len = len_a + len_b - 1
    result = List.duplicate(0, result_len)

    a_indexed = Enum.with_index(a)
    b_indexed = Enum.with_index(b)

    Enum.reduce(a_indexed, result, fn {ai, i}, acc ->
      Enum.reduce(b_indexed, acc, fn {bj, j}, acc2 ->
        idx = i + j
        current = Enum.at(acc2, idx)
        List.replace_at(acc2, idx, GF.add(current, GF.multiply(ai, bj)))
      end)
    end)
  end

  # poly_mod_be/2
  #
  # Compute big-endian polynomial `dividend mod divisor` over GF(256).
  #
  # `divisor` is in little-endian form (as returned by build_generator).
  # The remainder is returned as a big-endian list of `degree(divisor)` bytes.
  #
  # We use synthetic (polynomial long) division:
  #   For each leading coefficient of the current remainder, multiply the
  #   divisor by that coefficient and XOR (subtract in GF(256)) it in.
  #
  # Since the divisor is monic (leading coefficient = 1), we don't need to
  # divide by the leading coefficient at each step — it cancels directly.
  defp poly_mod_be(dividend, gen_le) do
    # Convert divisor to big-endian for easier alignment.
    gen_be = Enum.reverse(gen_le)
    gen_len = length(gen_be)
    # The degree of the generator = gen_len - 1 = n_check.
    # We work on a mutable working buffer equal to the dividend.
    # After processing, the last n_check bytes are the remainder.
    work =
      Enum.reduce(0..(length(dividend) - gen_len), dividend, fn i, buf ->
        lead = Enum.at(buf, i)

        if lead == 0 do
          buf
        else
          Enum.reduce(0..(gen_len - 1), buf, fn j, buf2 ->
            gj = Enum.at(gen_be, j)
            idx = i + j
            current = Enum.at(buf2, idx)
            List.replace_at(buf2, idx, GF.add(current, GF.multiply(lead, gj)))
          end)
        end
      end)

    # Return the last n_check bytes (the remainder after the message prefix).
    Enum.drop(work, length(dividend) - (gen_len - 1))
  end

  # berlekamp_massey/1
  #
  # The Berlekamp-Massey algorithm finds the shortest LFSR (= shortest polynomial
  # Λ(x)) that generates the given syndrome sequence.
  #
  # For Reed-Solomon, Λ(x) is the error locator polynomial:
  #   Λ(x) = ∏(1 - X_i · x)
  # where X_i = α^{p_i} is the error locator for position p_i.
  #
  # BM iterates over each syndrome, detecting discrepancies and updating Λ.
  # Each update multiplies in a correction term derived from the previous best
  # polynomial that generated a different-length sequence.
  #
  # Reference: Blahut, "Algebraic Codes for Data Transmission", Ch. 7.
  defp berlekamp_massey(syndromes_list) do
    n = length(syndromes_list)

    # Initial state:
    #   c = current best-fit polynomial (starts at [1])
    #   b = previous best-fit polynomial (starts at [1])
    #   l = current length (number of errors found so far)
    #   x = shift counter
    initial = {[1], [1], 0, 1}

    {c_final, _, _, _} =
      Enum.reduce(0..(n - 1), initial, fn i, {c, b, l, x} ->
        # Compute discrepancy d = S_i + Σ Λ_j * S_{i-j}
        # (only the non-trivial terms; c[0] = 1 is excluded from the sum)
        # When c = [1] (no errors found yet), length(c) - 1 = 0, so
        # 1..0 would be a descending range in Elixir. We use //1 to force
        # ascending step, making 1..0//1 an empty range (no iteration).
        d =
          Enum.reduce(1..(length(c) - 1)//1, Enum.at(syndromes_list, i), fn j, acc ->
            if j <= i do
              GF.add(acc, GF.multiply(Enum.at(c, j), Enum.at(syndromes_list, i - j)))
            else
              acc
            end
          end)

        if d == 0 do
          # No discrepancy: Λ unchanged, just advance the shift.
          {c, b, l, x + 1}
        else
          # Discrepancy found: adjust Λ.
          # t = c - d * x^x * b  (in polynomial arithmetic)
          b_shifted = List.duplicate(0, x) ++ b
          t = poly_add_le(c, poly_scale_le(b_shifted, d))

          if 2 * l <= i do
            # Length increases: update l, save old c as new b (reset shift).
            new_l = i + 1 - l
            new_b = poly_scale_le(c, GF.inverse(d))
            {t, new_b, new_l, 1}
          else
            {t, b, l, x + 1}
          end
        end
      end)

    c_final
  end

  # poly_add_le/2
  #
  # Add two little-endian GF(256) polynomials (addition = XOR coefficient-wise).
  # Pads the shorter one with zeros.
  defp poly_add_le(a, b) do
    len = max(length(a), length(b))
    a_padded = a ++ List.duplicate(0, len - length(a))
    b_padded = b ++ List.duplicate(0, len - length(b))

    Enum.zip(a_padded, b_padded)
    |> Enum.map(fn {ai, bi} -> GF.add(ai, bi) end)
  end

  # poly_scale_le/2
  #
  # Multiply every coefficient of a little-endian polynomial by a scalar in GF(256).
  defp poly_scale_le(poly, scalar) do
    Enum.map(poly, fn c -> GF.multiply(c, scalar) end)
  end

  # chien_search/2
  #
  # Find the positions of errors by testing every possible locator X = α^k.
  #
  # An error at codeword position p corresponds to locator X_p = α^p.
  # Λ(X_p^{-1}) = 0 means p is an error position.
  #
  # The inverse locator formula:
  #   X_p^{-1} = α^{(p + 256 - n) mod 255}
  #
  # where n = length(codeword). This converts from "position in codeword"
  # to "power of α" using the locator convention α^0 = position 0 from
  # the right (i.e., position n-1 in 0-indexed codeword coordinates).
  #
  # Returns the list of error positions sorted ascending.
  defp chien_search(locator, n) do
    Enum.reduce(0..(n - 1), [], fn p, acc ->
      inv_loc = GF.power(2, rem(p + 256 - n, 255))

      if poly_eval_le(locator, inv_loc) == 0 do
        [p | acc]
      else
        acc
      end
    end)
    |> Enum.reverse()
  end

  # forney/4
  #
  # Compute error magnitudes using the Forney formula.
  #
  # Given:
  #   - syndromes S (list of n_check values)
  #   - error locator Λ(x) (little-endian)
  #   - error positions [p_0, p_1, ...]
  #   - codeword length n
  #
  # Step 1: Compute the error evaluator polynomial Ω(x):
  #   Ω(x) = S(x) · Λ(x) mod x^{n_check}
  # where S(x) = S_1 + S_2·x + ... + S_{n_check}·x^{n_check-1} (LE syndrome poly).
  #
  # Step 2: Compute the formal derivative Λ'(x):
  #   In GF(2^8) (characteristic 2), the derivative of x^k is:
  #   - 0 if k is even (coefficient vanishes mod 2)
  #   - coefficient * x^{k-1} if k is odd
  #   So only odd-indexed terms survive.
  #
  # Step 3: For each error at position p:
  #   X_p_inv = α^{(p + 256 - n) mod 255}
  #   magnitude = Ω(X_p^{-1}) / Λ'(X_p^{-1})
  #
  # Generator convention: roots at α^1, α^2, ..., α^{n_check}  (b = 1).
  # The standard Forney formula with b=1 simplifies to X_p^{b-1} = X_p^0 = 1,
  # so the X_p scaling factor vanishes entirely. The magnitude is simply:
  #   magnitude = Ω(X_p^{-1}) / Λ'(X_p^{-1})
  #
  # Cross-check: for a single error e at position p, Ω is a constant equal to
  # e * X_p (where X_p = α^{n-1-p}), and Λ'(X_p^{-1}) = X_p, so the ratio = e. ✓
  defp forney(syndromes_list, locator, positions, n) do
    n_check = length(syndromes_list)

    # Build syndrome polynomial S(x) in LE form.
    s_poly = syndromes_list

    # Ω(x) = S(x) * Λ(x) mod x^{n_check}
    # Truncate the product to the first n_check terms (degree < n_check).
    omega_full = poly_mul_le(s_poly, locator)
    omega = Enum.take(omega_full, n_check)

    # Formal derivative of Λ(x) in GF(2^8):
    #
    # For Λ(x) = Σ Λ_k * x^k, the formal derivative is:
    #   Λ'(x) = Σ_{k odd} Λ_k * x^{k-1}   (even terms vanish in char 2)
    #
    # So Λ'(x) = Λ_1 + Λ_3·x^2 + Λ_5·x^4 + ...
    #
    # In little-endian: position i in Λ' gets the coefficient from Λ[i+1] when
    # i+1 is odd (i.e., when i is even). Odd positions are 0.
    #
    # Implementation: drop Λ_0, then each remaining element at original index k
    # goes to position k-1. Odd-k elements (now at even positions) are kept;
    # even-k elements (now at odd positions) are zeroed.
    #
    # Equivalently: drop(1) to remove Λ_0, then with_index(1) restores original
    # index, and we keep only those with odd original index.
    lambda_prime =
      locator
      |> Enum.drop(1)
      |> Enum.with_index(1)
      |> Enum.map(fn {coef, idx} -> if rem(idx, 2) == 1, do: coef, else: 0 end)

    Enum.map(positions, fn p ->
      x_inv = GF.power(2, rem(p + 256 - n, 255))

      omega_val = poly_eval_le(omega, x_inv)
      lambda_prime_val = poly_eval_le(lambda_prime, x_inv)

      # magnitude = Ω(X_p^{-1}) / Λ'(X_p^{-1})
      GF.divide(omega_val, lambda_prime_val)
    end)
  end
end
