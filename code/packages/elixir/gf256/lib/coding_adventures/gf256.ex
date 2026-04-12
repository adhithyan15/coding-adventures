# === GF256 — MA01: Galois Field GF(2^8) Arithmetic ===
#
# GF(2^8) is a **finite field** with exactly 256 elements. The elements are
# the integers 0 through 255, but the arithmetic is completely different from
# ordinary integer arithmetic. Two key properties make it useful:
#
#   1. Addition IS XOR. In characteristic-2, 1 + 1 = 0, so subtraction equals
#      addition. No carry bits, no overflow — just XOR every bit independently.
#
#   2. Multiplication uses logarithm tables. The field has a multiplicative
#      generator g = 2 such that every non-zero element is some power of g.
#      This lets us reduce multiplication to: a × b = g^(log(a) + log(b)).
#
# Applications of GF(256):
#   - Reed-Solomon error correction (QR codes, CDs, hard drives, space probes)
#   - AES encryption (SubBytes and MixColumns steps)
#   - General ECC for storage and communication
#
# The Primitive Polynomial
# ────────────────────────
# The elements of GF(2^8) are polynomials over GF(2) of degree ≤ 7. To keep
# multiplication in-range, we reduce modulo an irreducible degree-8 polynomial.
#
# We use:   p(x) = x^8 + x^4 + x^3 + x^2 + 1 = 0x11D = 285 (decimal)
#
# In binary: bit 8 = x^8, bit 4 = x^4, bit 3 = x^3, bit 2 = x^2, bit 0 = 1.
#             1_0001_1101 = 0x11D.
#
# This polynomial is:
#   - Irreducible over GF(2): cannot be factored into lower-degree polynomials.
#   - Primitive: the element g = 2 (=x) generates all 255 non-zero elements.
#
# Log/Antilog Table Construction
# ───────────────────────────────
# We precompute at compile time:
#   ALOG[i] = 2^i mod p(x)    — antilogarithm table, 256 entries
#   LOG[x]  = i such that 2^i = x — logarithm table, 256 entries
#
# Algorithm:
#   Start with val = 1.  For each step, multiply by 2 (shift left 1 bit).
#   If bit 8 is set (overflow past a byte), XOR with 0x11D to reduce mod p(x).
#
#   Shift-left = multiply by 2 because in GF(2^8), the element "2" is the
#   polynomial x. Multiplying f(x) by x shifts all its coefficients up by one
#   degree. If the degree-8 coefficient becomes 1, reduce mod p(x) by XOR-ing
#   with the polynomial's bit representation.

defmodule CodingAdventures.GF256 do
  import Bitwise

  @moduledoc """
  Galois Field GF(2^8) arithmetic.

  GF(256) is the finite field with 256 elements (integers 0..255). Arithmetic
  uses the primitive polynomial `x^8 + x^4 + x^3 + x^2 + 1 = 0x11D`.

  Key insight: In GF(2^8), **addition is XOR** (characteristic-2 field).
  Subtraction is identical to addition.

  Multiplication and division are performed via **log/antilog tables** built
  at compile time, turning them into O(1) table lookups.

  ## Verification (0x11D polynomial)

      add(0x53, 0xCA)  = 0x53 XOR 0xCA = 0x99
      multiply(0x53, 0x8C) = 0x01   (they are multiplicative inverses)
      power(2, 255) = 1             (g^255 = 1, the generator has order 255)
  """

  # The primitive polynomial x^8 + x^4 + x^3 + x^2 + 1.
  # Binary: 1_0001_1101 = 0x11D = 285.
  # Stored as a module attribute so it is visible in documentation and
  # accessible to callers who need to know which polynomial was used.
  @primitive_polynomial 0x11D
  # Export for callers who need to inspect which polynomial was used.
  def primitive_polynomial(), do: @primitive_polynomial

  # Additive identity: in any field, 0 + x = x.
  @zero 0

  # Multiplicative identity: in any field, 1 * x = x.
  @one 1

  # Build the log and alog tables at compile time using a private function.
  # This is the Elixir idiomatic approach: call the builder in a module
  # attribute expression so it runs during compilation.
  #
  # We need 256-entry arrays; Elixir uses tuples for fast indexed access.
  # The builder returns {alog_tuple, log_tuple}.
  {alog_table, log_table} =
    (fn ->
      # Start the generator sequence. We compute ALOG[0..254] and set
      # ALOG[255] = 1 as a wrap-around sentinel for inverse(1).
      alog = :array.new(256, default: 0)
      log = :array.new(256, default: 0)

      {alog, log} =
        Enum.reduce(0..254, {alog, log}, fn i, {alog_acc, log_acc} ->
          # Determine the current power value.
          # For i=0, it's 1. For i>0, multiply previous by 2.
          val =
            if i == 0 do
              1
            else
              prev = :array.get(i - 1, alog_acc)
              v = prev * 2
              # Reduce modulo p(x) if bit 8 overflows.
              if v >= 256, do: Bitwise.bxor(v, 0x11D), else: v
            end

          alog_acc = :array.set(i, val, alog_acc)
          log_acc = :array.set(val, i, log_acc)
          {alog_acc, log_acc}
        end)

      # ALOG[255] = 1: the multiplicative group has order 255, so g^255 = g^0 = 1.
      # This sentinel is required by inverse(1): 255 - LOG[1] = 255 - 0 = 255
      # → ALOG[255] = 1. Without it, inverse(1) would look up ALOG[255] and
      # get an uninitialized value.
      alog = :array.set(255, 1, alog)

      {:array.to_list(alog), :array.to_list(log)}
    end).()

  @alog_table alog_table
  @log_table log_table

  # ---------------------------------------------------------------------------
  # Field Operations
  # ---------------------------------------------------------------------------

  @doc """
  Add two GF(256) elements.

  In a characteristic-2 field, addition is XOR. Every bit represents a GF(2)
  coefficient, and in GF(2), `1 + 1 = 0 mod 2`. XOR performs GF(2) addition
  on each bit simultaneously.

  No overflow, no carry, no tables needed — just XOR.

      add(0x53, 0xCA) = 0x53 XOR 0xCA = 0x99
      add(x, x) = 0 for all x   (every element is its own additive inverse)
  """
  def add(a, b), do: bxor(a, b)

  @doc """
  Subtract two GF(256) elements.

  In characteristic 2, `−1 = 1`, so subtraction equals addition. This is the
  same as XOR. The hardware simplification is significant: no borrow circuits.

  This design benefits Reed-Solomon decoding: "syndrome" values computed via
  subtraction use the exact same operation as addition.
  """
  def subtract(a, b), do: bxor(a, b)

  @doc """
  Multiply two GF(256) elements using log/antilog tables.

  The mathematical identity: `a × b = g^(log_g(a) + log_g(b))`.

  Where g = 2 is our generator.

  We convert to the log domain (addition), add, then convert back:
      multiply(a, b) = ALOG[(LOG[a] + LOG[b]) mod 255]

  The modulo 255 reflects the cyclic group structure: the non-zero elements
  form a group of order 255, so exponents repeat with period 255.

  Special case: if either operand is 0, the result is 0.
  (Zero has no logarithm — it is not reachable as a power of any generator.)

  Time complexity: O(1) — two list lookups and one addition.
  """
  def multiply(a, b) do
    if a == @zero or b == @zero do
      @zero
    else
      log_a = Enum.at(@log_table, a)
      log_b = Enum.at(@log_table, b)
      Enum.at(@alog_table, rem(log_a + log_b, 255))
    end
  end

  @doc """
  Divide a by b in GF(256).

      a / b = ALOG[(LOG[a] - LOG[b] + 255) mod 255]

  The `+ 255` before the modulo ensures the result is non-negative when
  `LOG[a] < LOG[b]`. Without it, Elixir's `rem/2` can return negative values.

  Special cases:
  - `a = 0` → result is 0 (0 divided by anything is 0)
  - `b = 0` → raises `ArgumentError` (division by zero is undefined in any field)

  Raises `ArgumentError` if b is 0.
  """
  def divide(a, b) do
    if b == @zero do
      raise ArgumentError, "GF256: division by zero"
    end

    if a == @zero do
      @zero
    else
      log_a = Enum.at(@log_table, a)
      log_b = Enum.at(@log_table, b)
      Enum.at(@alog_table, rem(log_a - log_b + 255, 255))
    end
  end

  @doc """
  Raise a GF(256) element to a non-negative integer power.

  Uses the logarithm table:
      base^exp = ALOG[(LOG[base] * exp) mod 255]

  The modulo 255 reflects the order of the multiplicative group:
  every non-zero element satisfies `g^255 = 1` (Fermat's little theorem
  for finite fields — an element's order divides the group order 255).

  Special cases:
  - `exp = 0` → 1 (anything to the zeroth power is 1, including 0^0 = 1)
  - `base = 0` and `exp > 0` → 0
  """
  def power(_base, 0), do: @one

  def power(base, exp) do
    if base == @zero do
      @zero
    else
      log_base = Enum.at(@log_table, base)
      # Use rem on positive result to keep in range.
      idx = rem(rem(log_base * exp, 255) + 255, 255)
      Enum.at(@alog_table, idx)
    end
  end

  @doc """
  Compute the multiplicative inverse of a GF(256) element.

  The inverse satisfies: `a × inverse(a) = 1`.

  By the cyclic group property:
      a × a^(-1) = 1 = g^0 = g^255
      log(a) + log(a^(-1)) ≡ 0 (mod 255)
      log(a^(-1)) = 255 - log(a)
      a^(-1) = ALOG[255 - LOG[a]]

  This is fundamental to Reed-Solomon decoding and AES SubBytes (the S-box
  is computed from the multiplicative inverse in GF(2^8)).

  Raises `ArgumentError` if a is 0 (zero has no multiplicative inverse).
  """
  def inverse(a) do
    if a == @zero do
      raise ArgumentError, "GF256: zero has no multiplicative inverse"
    end

    log_a = Enum.at(@log_table, a)
    Enum.at(@alog_table, 255 - log_a)
  end

  @doc """
  Return the additive identity (zero element).

  In GF(256), 0 is the identity for addition: `add(0, x) = x` for all x.
  """
  def zero(), do: @zero

  @doc """
  Return the multiplicative identity (one element).

  In GF(256), 1 is the identity for multiplication: `multiply(1, x) = x` for all x.
  """
  def one(), do: @one

  @doc """
  Return the antilogarithm table as a list.

  `alog_table()` returns a 256-element list where `Enum.at(alog_table(), i) = 2^i mod p(x)`.
  Index 255 holds 1 (the wrap-around sentinel: g^255 = g^0 = 1).

  Primarily useful for testing and debugging.
  """
  def alog_table(), do: @alog_table

  @doc """
  Return the logarithm table as a list.

  `log_table()` returns a 256-element list where `Enum.at(log_table(), x) = i`
  such that `2^i = x`. `log_table()[0]` is 0 by initialization but is undefined
  mathematically (there is no power of 2 equal to 0 in GF(256)).

  Primarily useful for testing and debugging.
  """
  def log_table(), do: @log_table

  # ---------------------------------------------------------------------------
  # GF256Field — parameterizable field factory
  # ---------------------------------------------------------------------------
  #
  # The functions above are bound to the Reed-Solomon polynomial 0x11D.
  # AES uses 0x11B. `new_field/1` returns a `%GF256Field{}` struct and
  # overloaded functions handle field-first calls.
  #
  # Operations use Russian peasant (shift-and-XOR) multiplication. No log/antilog
  # tables — they require g=2 to be a primitive element, which holds for 0x11D
  # but NOT for 0x11B (AES uses g=0x03 per FIPS 197 §4.1).
  #
  # Usage:
  #   aes = CodingAdventures.GF256.new_field(0x11B)
  #   CodingAdventures.GF256.multiply(aes, 0x53, 0xCA)  # → 1

  @doc """
  Create a new GF(2^8) field with the given primitive polynomial.

  Returns a `%CodingAdventures.GF256Field{}` struct. Pass it as the first
  argument to the field-aware overloads of `multiply/3`, `divide/3`,
  `power/3`, `inverse/2`.

      aes = CodingAdventures.GF256.new_field(0x11B)
      CodingAdventures.GF256.multiply(aes, 0x53, 0xCA)  # → 1
  """
  def new_field(polynomial) do
    %CodingAdventures.GF256Field{polynomial: polynomial}
  end

  # Russian peasant multiplication: a * b mod p(x) in GF(2^8).
  # reduce is the low byte of the primitive polynomial.
  defp gf_mul(a, b, reduce) do
    gf_mul_loop(a, b, reduce, 0, 8)
  end

  defp gf_mul_loop(_aa, _bb, _reduce, result, 0), do: result

  defp gf_mul_loop(aa, bb, reduce, result, n) do
    result2 = if (bb &&& 1) != 0, do: bxor(result, aa), else: result
    hi = aa &&& 0x80
    aa2 = (aa <<< 1) &&& 0xFF
    aa3 = if hi != 0, do: bxor(aa2, reduce), else: aa2
    gf_mul_loop(aa3, bb >>> 1, reduce, result2, n - 1)
  end

  # Raise base to exp via repeated squaring.
  defp gf_pow(_base, _reduce, 0), do: 1
  defp gf_pow(0, _reduce, _exp), do: 0

  defp gf_pow(base, reduce, exp) do
    gf_pow_loop(base, reduce, exp, 1)
  end

  defp gf_pow_loop(_b, _reduce, 0, result), do: result

  defp gf_pow_loop(b, reduce, e, result) do
    result2 = if (e &&& 1) != 0, do: gf_mul(result, b, reduce), else: result
    b2 = gf_mul(b, b, reduce)
    gf_pow_loop(b2, reduce, e >>> 1, result2)
  end

  # Field-aware operation overloads — take a %GF256Field{} as first argument.

  @doc """
  Multiply two GF(256) elements using a parameterized field.

  When called as `multiply(field, a, b)` where `field` is a `%GF256Field{}`,
  uses Russian peasant multiplication with that field's polynomial.
  When called as `multiply(a, b)`, uses the module-level 0x11D tables.
  """
  def multiply(%CodingAdventures.GF256Field{polynomial: poly}, a, b) do
    gf_mul(a, b, poly &&& 0xFF)
  end

  @doc """
  Divide a by b using a parameterized field. Raises ArgumentError if b is 0.
  """
  def divide(%CodingAdventures.GF256Field{polynomial: poly}, a, b) do
    if b == 0, do: raise(ArgumentError, "GF256Field: division by zero")
    reduce = poly &&& 0xFF
    gf_mul(a, gf_pow(b, reduce, 254), reduce)
  end

  @doc """
  Raise base to exp using a parameterized field.
  """
  def power(%CodingAdventures.GF256Field{polynomial: poly}, base, exp) do
    gf_pow(base, poly &&& 0xFF, exp)
  end

  @doc """
  Compute the multiplicative inverse using a parameterized field.
  Raises ArgumentError if a is 0.
  inverse(a) = a^254 since a^255 = 1 in GF(2^8) (Fermat's little theorem).
  """
  def inverse(%CodingAdventures.GF256Field{polynomial: poly}, a) do
    if a == 0, do: raise(ArgumentError, "GF256Field: zero has no multiplicative inverse")
    gf_pow(a, poly &&& 0xFF, 254)
  end

  @doc """
  Add two elements in any GF(2^8) field (XOR is polynomial-independent).
  Provided for API symmetry with the field overloads.
  """
  def add(%CodingAdventures.GF256Field{}, a, b), do: bxor(a, b)

  @doc """
  Subtract two elements in any GF(2^8) field (same as add).
  """
  def subtract(%CodingAdventures.GF256Field{}, a, b), do: bxor(a, b)
end
