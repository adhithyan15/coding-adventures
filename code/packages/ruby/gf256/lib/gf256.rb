# frozen_string_literal: true

# =============================================================================
# gf256 — Galois Field GF(2^8) arithmetic.
# =============================================================================
#
# GF(256) is the finite field with 256 elements (integers 0..255).
# Arithmetic uses the primitive polynomial:
#
#   p(x) = x^8 + x^4 + x^3 + x^2 + 1  =  0x11D  =  285
#
# In characteristic 2: addition = XOR = subtraction.
# Multiplication uses precomputed log/antilog tables for O(1) performance.
#
# Applications: Reed-Solomon error correction, QR codes, AES encryption.
# =============================================================================

module GF256
  VERSION = "0.1.0"

  # Additive identity.
  ZERO = 0

  # Multiplicative identity.
  ONE = 1

  # The primitive (irreducible) polynomial.
  # p(x) = x^8 + x^4 + x^3 + x^2 + 1
  # Binary: 1_0001_1101 = 0x11D = 285
  PRIMITIVE_POLYNOMIAL = 0x11D

  # ---------------------------------------------------------------------------
  # Build Log/Antilog tables at module load time.
  #
  # ALOG[i] = g^i mod p(x)  where g = 2
  # LOG[x]  = i such that g^i = x
  #
  # Algorithm: start with val=1; each step shift left 1 (= multiply by g=2);
  # if val >= 256, XOR with 0x11D to reduce modulo the primitive polynomial.
  # ---------------------------------------------------------------------------

  # ALOG has 256 entries: indices 0..254 are the standard table;
  # ALOG[255] = 1 because g^255 = g^0 = 1 (the group has order 255).
  # This allows inverse(1) = ALOG[255 - LOG[1]] = ALOG[255] = 1 to work.
  _log_table = Array.new(256, 0)
  _alog_table = Array.new(256, 0)
  val = 1
  255.times do |i|
    _alog_table[i] = val
    _log_table[val] = i
    val <<= 1
    val ^= PRIMITIVE_POLYNOMIAL if val >= 256
  end
  _alog_table[255] = 1  # g^255 = g^0 = 1

  LOG = _log_table.freeze
  ALOG = _alog_table.freeze

  private_constant :LOG, :ALOG

  # ---------------------------------------------------------------------------
  # Field Operations
  # ---------------------------------------------------------------------------

  # Add two GF(256) elements: returns a XOR b.
  #
  # In characteristic 2, addition is XOR. No carry, no tables needed.
  # Every element is its own additive inverse: add(x, x) = 0.
  #
  # @param a [Integer] field element (0..255)
  # @param b [Integer] field element (0..255)
  # @return [Integer] a XOR b
  def self.add(a, b)
    a ^ b
  end

  # Subtract two GF(256) elements: returns a XOR b.
  #
  # In characteristic 2, -1 = 1, so subtraction equals addition.
  #
  # @param a [Integer] field element
  # @param b [Integer] field element
  # @return [Integer] a XOR b
  def self.subtract(a, b)
    a ^ b
  end

  # Multiply two GF(256) elements using log/antilog tables.
  #
  # a × b = ALOG[(LOG[a] + LOG[b]) % 255]
  #
  # Special case: if either operand is 0, the result is 0.
  #
  # @param a [Integer] field element (0..255)
  # @param b [Integer] field element (0..255)
  # @return [Integer] product in GF(256)
  def self.multiply(a, b)
    return 0 if a == 0 || b == 0
    ALOG[(LOG[a] + LOG[b]) % 255]
  end

  # Divide a by b in GF(256).
  #
  # a / b = ALOG[(LOG[a] - LOG[b] + 255) % 255]
  #
  # The +255 ensures a non-negative result when LOG[a] < LOG[b].
  #
  # @param a [Integer] dividend (0..255)
  # @param b [Integer] divisor (must not be 0)
  # @return [Integer] quotient in GF(256)
  # @raise [ArgumentError] if b is 0
  def self.divide(a, b)
    raise ArgumentError, "GF256: division by zero" if b == 0
    return 0 if a == 0
    ALOG[(LOG[a] - LOG[b] + 255) % 255]
  end

  # Raise a GF(256) element to a non-negative integer power.
  #
  # base^exp = ALOG[(LOG[base] * exp) % 255]
  #
  # Special cases: 0^0 = 1 by convention; 0^n = 0 for n > 0.
  #
  # @param base [Integer] base element (0..255)
  # @param exp [Integer] non-negative integer exponent
  # @return [Integer] base^exp in GF(256)
  def self.power(base, exp)
    return 1 if base == 0 && exp == 0
    return 0 if base == 0
    return 1 if exp == 0
    ALOG[((LOG[base] * exp) % 255 + 255) % 255]
  end

  # Return the multiplicative inverse of a GF(256) element.
  #
  # a × inverse(a) = 1.
  # inverse(a) = ALOG[255 - LOG[a]]
  #
  # @param a [Integer] field element (must not be 0)
  # @return [Integer] multiplicative inverse
  # @raise [ArgumentError] if a is 0
  def self.inverse(a)
    raise ArgumentError, "GF256: zero has no multiplicative inverse" if a == 0
    ALOG[255 - LOG[a]]
  end

  # Return the additive identity.
  # @return [Integer] 0
  def self.zero
    0
  end

  # Return the multiplicative identity.
  # @return [Integer] 1
  def self.one
    1
  end

  # Expose tables for testing and downstream use.
  def self.log_table
    LOG
  end

  def self.alog_table
    ALOG
  end

  # ===========================================================================
  # GF256Field — parameterizable field factory
  # ===========================================================================
  #
  # The module-level functions are fixed to the Reed-Solomon polynomial 0x11D.
  # AES uses 0x11B. GF256::Field accepts any primitive polynomial and uses
  # Russian peasant multiplication (no log/antilog tables needed).
  #
  # Usage:
  #   aes = GF256::Field.new(0x11B)
  #   aes.multiply(0x53, 0xCA)  # => 1
  #   aes.inverse(0x53)          # => 0xCA

  class Field
    attr_reader :polynomial

    # Build a GF(2^8) field for the given primitive polynomial.
    #
    # Uses Russian peasant (shift-and-XOR) multiplication — no log/antilog tables.
    # This works correctly for any irreducible polynomial without needing a
    # specific primitive generator. (Log/antilog tables require g=2 to be a
    # primitive element, which holds for 0x11D but not for 0x11B — AES uses
    # g=0x03 per FIPS 197 §4.1.)
    #
    # @param polynomial [Integer] the irreducible polynomial (e.g. 0x11B for AES,
    #   0x11D for Reed-Solomon).
    def initialize(polynomial)
      @polynomial = polynomial
      # Low byte of the polynomial used for the reduction step in gf_mul.
      @reduce = polynomial & 0xFF
    end

    # Add two field elements: a XOR b (characteristic-2; polynomial-independent).
    def add(a, b) = a ^ b

    # Subtract two field elements: a XOR b (same as add in GF(2^8)).
    def subtract(a, b) = a ^ b

    # Multiply two field elements using Russian peasant multiplication.
    def multiply(a, b)
      gf_mul(a, b)
    end

    # Divide a by b. Raises ArgumentError if b is 0.
    def divide(a, b)
      raise ArgumentError, "GF256::Field: division by zero" if b == 0
      gf_mul(a, gf_pow(b, 254))
    end

    # Raise base to a non-negative integer power via repeated squaring.
    def power(base, exp)
      raise ArgumentError, "GF256::Field: exponent must be non-negative" if exp < 0
      raise ArgumentError, "GF256::Field: exponent too large (max 2^32 - 1)" if exp > 0xFFFF_FFFF
      gf_pow(base, exp)
    end

    # Return the multiplicative inverse of a. Raises ArgumentError if a is 0.
    # inverse(a) = a^254 since a^255 = 1 (Fermat's little theorem for GF(2^8)).
    def inverse(a)
      raise ArgumentError, "GF256::Field: zero has no multiplicative inverse" if a == 0
      gf_pow(a, 254)
    end

    private

    # Russian peasant multiplication: a * b mod p(x) in GF(2^8).
    # @reduce is the low byte of the primitive polynomial.
    def gf_mul(a, b)
      result = 0
      aa = a
      8.times do
        result ^= aa if b & 1 != 0
        hi = aa & 0x80
        aa = (aa << 1) & 0xFF
        aa ^= @reduce if hi != 0
        b >>= 1
      end
      result
    end

    # Raise base to exp via repeated squaring.
    def gf_pow(base, exp)
      return exp == 0 ? 1 : 0 if base == 0
      return 1 if exp == 0
      result = 1
      b = base
      e = exp
      while e > 0
        result = gf_mul(result, b) if e & 1 != 0
        b = gf_mul(b, b)
        e >>= 1
      end
      result
    end
  end
end
