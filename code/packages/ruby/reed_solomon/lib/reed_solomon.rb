# frozen_string_literal: true

# =============================================================================
# reed_solomon — Reed-Solomon error-correcting codes over GF(256).
# =============================================================================
#
# Reed-Solomon (RS) is a block error-correcting code invented by Irving Reed
# and Gustave Solomon in 1960.  Add nCheck redundancy bytes to a message;
# the decoder can recover the original even when up to t = nCheck/2 bytes
# have been corrupted in transit.
#
# Where RS codes appear:
#   - QR codes: up to 30% of the symbol can be scratched and still decoded.
#   - CDs / DVDs: CIRC two-level RS corrects scratches and burst errors.
#   - Hard drives: firmware sector-level error correction.
#   - Voyager probes: images sent across 20+ billion kilometres.
#   - RAID-6: the two parity drives ARE an (n, n-2) RS code over GF(256).
#
# Building blocks:
#   MA00 polynomial   — coefficient-array polynomial arithmetic
#   MA01 gf256        — GF(2^8) field arithmetic (add=XOR, mul=table lookup)
#   MA02 reed_solomon — RS encoding / decoding (THIS MODULE)
#
# Polynomial conventions
# ----------------------
# Codeword bytes are treated as a big-endian polynomial:
#
#   codeword[0]·x^{n-1} + codeword[1]·x^{n-2} + … + codeword[n-1]
#
# The systematic layout:
#   [ message bytes (k) | check bytes (n_check) ]
#     degree n-1 … n_check   degree n_check-1 … 0
#
# For error position p in a big-endian codeword of length n:
#   X_p     = α^{n-1-p}
#   X_p⁻¹   = α^{(p+256-n) mod 255}
#
# Internal polynomials (generator, Λ, Ω) use little-endian arrays
# (index = degree).  Only codeword bytes use big-endian ordering.
# =============================================================================

require_relative "../../gf256/lib/gf256"

module ReedSolomon
  VERSION = "0.1.0"

  # ===========================================================================
  # Error Classes
  # ===========================================================================

  # Raised when decoding fails because the number of corrupted bytes exceeds
  # the correction capacity t = n_check / 2.
  #
  # The code can correct at most t byte errors.  If more are present the
  # codeword is unrecoverable and this exception is raised rather than
  # silently returning wrong data.
  class TooManyErrors < StandardError
    def initialize
      super("reed-solomon: too many errors — codeword is unrecoverable")
    end
  end

  # Raised when encode / decode receives invalid parameters.
  #
  # Common causes:
  #   - n_check is 0 or odd (must be a positive even integer)
  #   - total codeword length exceeds 255 (GF(256) block size limit)
  #   - received array is shorter than n_check
  class InvalidInput < ArgumentError
    def initialize(reason)
      super("reed-solomon: invalid input — #{reason}")
    end
  end

  # ===========================================================================
  # Generator Polynomial
  # ===========================================================================

  # Build the RS generator polynomial for a given number of check bytes.
  #
  # The generator is the product of n_check linear factors:
  #
  #   g(x) = (x + α¹)(x + α²) … (x + α^{n_check})
  #
  # where α = 2 is the primitive element of GF(256).
  #
  # Returns a little-endian Array of integers (index = degree),
  # length n_check+1.  The last element is always 1 (monic polynomial).
  #
  # Algorithm:
  #   Start with g = [1].  At each step multiply in the next factor (αⁱ + x):
  #
  #     g.each_with_index do |coeff, j|
  #       new_g[j]   ^= GF256.multiply(coeff, alpha_i)   # coeff·α^i
  #       new_g[j+1] ^= coeff                            # coeff·x
  #     end
  #
  # Example: n_check = 2
  #   Start: g = [1]
  #   i=1 (α¹=2): g = [2, 1]
  #   i=2 (α²=4): g = [8, 6, 1]
  #
  #   Verify α¹=2 is a root:
  #   g(2) = 8 ⊕ mul(6,2) ⊕ mul(1,4) = 8 ⊕ 12 ⊕ 4 = 0  ✓
  #
  # @param n_check [Integer] number of check bytes (must be even >= 2)
  # @return [Array<Integer>] little-endian generator polynomial coefficients
  # @raise [InvalidInput] if n_check is 0 or odd
  def self.build_generator(n_check)
    raise InvalidInput.new("n_check must be a positive even number, got #{n_check}") \
      if n_check == 0 || n_check.odd?

    g = [1]

    1.upto(n_check) do |i|
      alpha_i = GF256.power(2, i)
      new_g = Array.new(g.length + 1, 0)
      g.each_with_index do |coeff, j|
        new_g[j]     ^= GF256.multiply(coeff, alpha_i)   # coeff · α^i
        new_g[j + 1] ^= coeff                            # coeff · x
      end
      g = new_g
    end

    g
  end

  # ===========================================================================
  # Internal Polynomial Helpers
  # ===========================================================================

  # Evaluate a big-endian GF(256) polynomial at x using Horner's method.
  #
  # p[0] is the highest-degree coefficient.  Iterate left to right:
  #
  #   acc = 0
  #   p.each { |b| acc = GF256.add(GF256.multiply(acc, x), b) }
  #
  # Used for syndrome evaluation: S_j = poly_eval_be(codeword, α^j).
  #
  # @param p [Array<Integer>] big-endian polynomial coefficients
  # @param x [Integer] evaluation point in GF(256)
  # @return [Integer] p(x) in GF(256)
  def self.poly_eval_be(p, x)
    acc = 0
    p.each { |b| acc = GF256.add(GF256.multiply(acc, x), b) }
    acc
  end

  private_class_method :poly_eval_be

  # Evaluate a little-endian GF(256) polynomial at x using Horner's method.
  #
  # p[i] is the coefficient of x^i.  Iterate from highest to lowest degree:
  #
  #   acc = 0
  #   p.reverse_each { |c| acc = GF256.add(GF256.multiply(acc, x), c) }
  #
  # Used for evaluating Λ(x), Ω(x), and Λ'(x) in Chien search / Forney.
  #
  # @param p [Array<Integer>] little-endian polynomial coefficients
  # @param x [Integer] evaluation point in GF(256)
  # @return [Integer] p(x) in GF(256)
  def self.poly_eval_le(p, x)
    acc = 0
    p.reverse_each { |c| acc = GF256.add(GF256.multiply(acc, x), c) }
    acc
  end

  private_class_method :poly_eval_le

  # Multiply two little-endian GF(256) polynomials (convolution).
  #
  # result[i+j] ^= a[i] · b[j]   for all i, j
  #
  # In GF(256), addition is XOR, so ^= is correct.
  # Used in Forney to compute Ω(x) = S(x)·Λ(x) mod x^{2t}.
  #
  # @param a [Array<Integer>] little-endian polynomial
  # @param b [Array<Integer>] little-endian polynomial
  # @return [Array<Integer>] product polynomial (little-endian)
  def self.poly_mul_le(a, b)
    return [] if a.empty? || b.empty?

    result = Array.new(a.length + b.length - 1, 0)
    a.each_with_index do |ai, i|
      b.each_with_index do |bj, j|
        result[i + j] ^= GF256.multiply(ai, bj)
      end
    end
    result
  end

  private_class_method :poly_mul_le

  # Remainder of big-endian GF(256) polynomial long division.
  #
  # Both dividend and divisor are big-endian (first = highest degree).
  # The divisor must be monic (leading coefficient = 1) — guaranteed
  # because the generator polynomial is always monic.
  #
  # Algorithm: schoolbook long division.
  #
  #   rem = dividend.dup
  #   steps = rem.length - divisor.length + 1
  #   steps.times do |i|
  #     coeff = rem[i]
  #     next if coeff == 0
  #     divisor.each_with_index { |d, j| rem[i+j] ^= GF256.multiply(coeff, d) }
  #   end
  #   rem.last(divisor.length - 1)
  #
  # Returns an array of length divisor.length - 1.
  #
  # @param dividend [Array<Integer>] big-endian coefficients
  # @param divisor  [Array<Integer>] big-endian monic coefficients
  # @return [Array<Integer>] remainder (big-endian)
  def self.poly_mod_be(dividend, divisor)
    rem = dividend.dup
    div_len = divisor.length
    return rem if rem.length < div_len

    steps = rem.length - div_len + 1
    steps.times do |i|
      coeff = rem[i]
      next if coeff == 0

      divisor.each_with_index do |d, j|
        rem[i + j] ^= GF256.multiply(coeff, d)
      end
    end

    rem.last(div_len - 1)
  end

  private_class_method :poly_mod_be

  # Inverse locator X_p⁻¹ for byte position p in a codeword of length n.
  #
  # Big-endian convention: position p has degree n-1-p.
  #   X_p   = α^{n-1-p}
  #   X_p⁻¹ = α^{(p+256-n) mod 255}
  #
  # @param p [Integer] byte position (0-indexed)
  # @param n [Integer] codeword length
  # @return [Integer] X_p⁻¹ as a GF(256) element
  def self.inv_locator(p, n)
    GF256.power(2, (p + 256 - n) % 255)
  end

  private_class_method :inv_locator

  # ===========================================================================
  # Encoding
  # ===========================================================================

  # Encode a message with Reed-Solomon, producing a systematic codeword.
  #
  # Systematic means the message bytes appear unchanged in the output,
  # followed by n_check check bytes:
  #
  #   output = [ message bytes (k) | check bytes (n_check) ]
  #
  # Algorithm:
  #   1. Build the generator polynomial g (little-endian).
  #   2. Reverse g to big-endian gBE (g_le.last=1 becomes gBE[0]=1).
  #   3. Form shifted = message + [0]*n_check  (M(x)·x^{n_check} in BE).
  #   4. Remainder R = shifted mod gBE.
  #   5. Output: message + R  (R left-padded to n_check bytes).
  #
  # Why it works:
  #   C(x) = M(x)·x^{n_check} XOR R(x) = Q(x)·g(x), so C(αⁱ) = 0 for
  #   i=1…n_check.  The decoder exploits this zero-evaluation property.
  #
  # @param message [Array<Integer>] raw data bytes (0..255 each)
  # @param n_check [Integer] number of check bytes (must be even >= 2)
  # @return [Array<Integer>] systematic codeword of length message.length + n_check
  # @raise [InvalidInput] if n_check is 0/odd or total length > 255
  def self.encode(message, n_check)
    raise InvalidInput.new("n_check must be a positive even number, got #{n_check}") \
      if n_check == 0 || n_check.odd?

    n = message.length + n_check
    raise InvalidInput.new("total codeword length #{n} exceeds GF(256) block size limit of 255") \
      if n > 255

    # Build generator in LE, then reverse to BE (gBE[0] = 1, monic).
    g_le = build_generator(n_check)
    g_be = g_le.reverse

    # shifted = message || zeros  (BE representation of M(x)·x^{n_check})
    shifted = message + Array.new(n_check, 0)

    # Remainder of BE division by monic gBE.
    remainder = poly_mod_be(shifted, g_be)   # length == n_check (usually)

    # Check bytes = remainder left-padded to n_check bytes.
    check = Array.new(n_check - remainder.length, 0) + remainder

    message + check
  end

  # ===========================================================================
  # Syndromes
  # ===========================================================================

  # Compute the n_check syndrome values of a received codeword.
  #
  #   S_j = received(α^j)   for j = 1, 2, …, n_check
  #
  # A valid codeword satisfies C(αⁱ) = 0 for all i=1…n_check (divisible by g).
  # All-zero syndromes → no errors; any non-zero → corruption detected.
  #
  # @param received [Array<Integer>] codeword bytes (possibly corrupted)
  # @param n_check  [Integer] number of check bytes
  # @return [Array<Integer>] array of n_check syndrome values
  def self.syndromes(received, n_check)
    (1..n_check).map { |j| poly_eval_be(received, GF256.power(2, j)) }
  end

  # ===========================================================================
  # Berlekamp-Massey Algorithm
  # ===========================================================================

  # Find the shortest LFSR that generates the syndrome sequence.
  #
  # The LFSR connection polynomial Λ(x) is the error locator polynomial.
  # Its roots (where Λ(x)=0) are the inverses of the error locators X_k⁻¹.
  # Chien search finds those roots to reveal the error positions.
  #
  # If errors occurred at positions with locators X₁, X₂, …, Xᵥ:
  #   Λ(x) = ∏_{k=1}^{v} (1 - X_k·x)    with Λ(0) = 1
  #
  # Algorithm (0-based syndrome indexing):
  #   c = [1], b = [1], big_l = 0, x_shift = 1, b_scale = 1
  #
  #   synds.each_with_index do |_, n|
  #     d = synds[n] ^ (1..big_l).reduce(0) { |a, j| a ^ GF256.multiply(c[j], synds[n-j]) }
  #     if d == 0
  #       x_shift += 1
  #     elsif 2*big_l <= n
  #       t = c.dup; scale = GF256.divide(d, b_scale)
  #       c ^= scale·x^{x_shift}·b; big_l = n+1-big_l; b = t; b_scale = d; x_shift = 1
  #     else
  #       scale = GF256.divide(d, b_scale); c ^= scale·x^{x_shift}·b; x_shift += 1
  #     end
  #   end
  #
  # Returns [lambda, num_errors] where lambda is LE and num_errors = degree(Λ).
  #
  # @param synds [Array<Integer>] syndrome sequence (length 2t)
  # @return [Array] [lambda_poly, num_errors]
  def self.berlekamp_massey(synds)
    two_t = synds.length

    c       = [1]   # current Λ (LE)
    b       = [1]   # previous Λ (LE)
    big_l   = 0     # errors found so far
    x_shift = 1     # iterations since last update
    b_scale = 1     # discrepancy at last update

    two_t.times do |n|
      # -----------------------------------------------------------------
      # Discrepancy d = S[n] ^ Σ_{j=1}^{L} Λ[j] · S[n-j]
      # -----------------------------------------------------------------
      d = synds[n]
      1.upto(big_l) do |j|
        d ^= GF256.multiply(c[j] || 0, synds[n - j]) if n >= j
      end

      # -----------------------------------------------------------------
      # Update rule
      # -----------------------------------------------------------------
      if d == 0
        x_shift += 1

      elsif 2 * big_l <= n
        # Found more errors than modelled — grow Λ.
        t_save = c.dup
        scale = GF256.divide(d, b_scale)

        target_len = x_shift + b.length
        c += Array.new([target_len - c.length, 0].max, 0)
        b.each_with_index { |bk, k| c[x_shift + k] ^= GF256.multiply(scale, bk) }

        big_l   = n + 1 - big_l
        b       = t_save
        b_scale = d
        x_shift = 1

      else
        # Consistent update — adjust Λ without growing degree.
        scale = GF256.divide(d, b_scale)
        target_len = x_shift + b.length
        c += Array.new([target_len - c.length, 0].max, 0)
        b.each_with_index { |bk, k| c[x_shift + k] ^= GF256.multiply(scale, bk) }
        x_shift += 1
      end
    end

    [c, big_l]
  end

  private_class_method :berlekamp_massey

  # ===========================================================================
  # Chien Search
  # ===========================================================================

  # Find which byte positions are error locations.
  #
  # Position p is an error location iff Λ(X_p⁻¹) = 0, where
  #   X_p⁻¹ = α^{(p+256-n) mod 255}  (inv_locator(p, n))
  #
  # Test all n positions; collect matches.
  #
  # @param lam [Array<Integer>] error locator polynomial (LE)
  # @param n   [Integer] codeword length
  # @return [Array<Integer>] sorted error positions (0-indexed)
  def self.chien_search(lam, n)
    (0...n).select { |p| poly_eval_le(lam, inv_locator(p, n)) == 0 }
  end

  private_class_method :chien_search

  # ===========================================================================
  # Forney Algorithm
  # ===========================================================================

  # Compute error magnitudes from known error positions.
  #
  # For each error at position p:
  #   e_p = Ω(X_p⁻¹) / Λ'(X_p⁻¹)
  #
  # where:
  #   Ω(x) = (S(x)·Λ(x)) mod x^{2t}      — error evaluator polynomial
  #   S(x) = S₁ + S₂x + … + S_{2t}x^{2t-1}  — syndrome polynomial (LE)
  #   Λ'(x) — formal derivative of Λ in GF(2^8)
  #
  # Formal derivative in characteristic 2:
  #   In GF(2^8), 2=0, so even-degree terms vanish:
  #   Λ'(x) = Λ₁ + Λ₃x² + Λ₅x⁴ + …
  #   (only odd-indexed Λ coefficients survive; their index is reduced by 1)
  #
  # @param lam       [Array<Integer>] error locator polynomial (LE)
  # @param synds     [Array<Integer>] syndrome array (length 2t)
  # @param positions [Array<Integer>] error positions from Chien search
  # @param n         [Integer] codeword length
  # @return [Array<Integer>] error magnitudes, one per position
  # @raise [TooManyErrors] if Λ'(X_p⁻¹) = 0 for any position
  def self.forney(lam, synds, positions, n)
    two_t = synds.length

    # Ω(x) = S(x)·Λ(x) mod x^{2t}: truncate to first 2t terms.
    omega = poly_mul_le(synds, lam).first(two_t)

    # Formal derivative Λ'(x): Λ'[j-1] = Λ[j]  for j odd.
    lambda_prime = Array.new([lam.length - 1, 0].max, 0)
    lam.each_with_index do |coeff, j|
      lambda_prime[j - 1] ^= coeff if j.odd? && j >= 1
    end

    positions.map do |pos|
      xi_inv    = inv_locator(pos, n)
      omega_val = poly_eval_le(omega, xi_inv)
      lp_val    = poly_eval_le(lambda_prime, xi_inv)
      raise TooManyErrors if lp_val == 0

      GF256.divide(omega_val, lp_val)
    end
  end

  private_class_method :forney

  # ===========================================================================
  # Public API
  # ===========================================================================

  # Compute the error locator polynomial Λ(x) from a syndrome array.
  #
  # Runs Berlekamp-Massey and returns Λ in little-endian form with Λ[0]=1.
  # Exposed for advanced use cases (QR decoders, diagnostics).
  #
  # @param synds [Array<Integer>] syndrome array (length 2t)
  # @return [Array<Integer>] Λ(x) in little-endian form
  def self.error_locator(synds)
    lam, = berlekamp_massey(synds)
    lam
  end

  # Decode a received codeword, correcting up to t = n_check/2 byte errors.
  #
  # Five-step pipeline:
  #   1. Syndromes S₁…S_{n_check}. All zero → return message directly.
  #   2. Berlekamp-Massey → Λ(x), error count L.  L > t → TooManyErrors.
  #   3. Chien search → error positions {p₁…pᵥ}. |pos| ≠ L → TooManyErrors.
  #   4. Forney → error magnitudes {e₁…eᵥ}.
  #   5. received[p_k] ^= e_k for each k.
  #
  # Returns the recovered message (length = received.length - n_check).
  #
  # @param received [Array<Integer>] possibly corrupted codeword bytes
  # @param n_check  [Integer] number of check bytes (must be even >= 2)
  # @return [Array<Integer>] recovered message bytes
  # @raise [InvalidInput] if n_check is 0/odd or received is too short
  # @raise [TooManyErrors] if more than t errors are present
  def self.decode(received, n_check)
    raise InvalidInput.new("n_check must be a positive even number, got #{n_check}") \
      if n_check == 0 || n_check.odd?
    raise InvalidInput.new("received length #{received.length} < n_check #{n_check}") \
      if received.length < n_check

    t = n_check / 2
    n = received.length
    k = n - n_check

    # Step 1: Syndromes
    synds = syndromes(received, n_check)
    return received[0, k] if synds.all?(&:zero?)

    # Step 2: Berlekamp-Massey
    lam, num_errors = berlekamp_massey(synds)
    raise TooManyErrors if num_errors > t

    # Step 3: Chien search
    positions = chien_search(lam, n)
    raise TooManyErrors if positions.length != num_errors

    # Step 4: Forney
    magnitudes = forney(lam, synds, positions, n)

    # Step 5: Apply corrections
    corrected = received.dup
    positions.zip(magnitudes) { |pos, mag| corrected[pos] ^= mag }

    corrected[0, k]
  end
end
