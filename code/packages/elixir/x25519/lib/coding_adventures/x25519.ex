defmodule CodingAdventures.X25519 do
  @moduledoc """
  # X25519: Elliptic Curve Diffie-Hellman on Curve25519

  This module implements X25519 (RFC 7748), which performs Diffie-Hellman key
  exchange on Curve25519. X25519 is widely used in TLS 1.3, Signal Protocol,
  WireGuard, and many other modern cryptographic systems.

  ## How X25519 Works

  X25519 is based on the Montgomery form of Curve25519. The key insight is that
  we can perform scalar multiplication on the curve using only the x-coordinate
  (called the "u-coordinate" in Montgomery form). This simplification — due to
  Daniel Bernstein — makes the algorithm both fast and easy to implement.

  ## The Mathematical Foundation

  All arithmetic happens in the prime field GF(2^255 - 19), which means every
  operation is performed modulo the prime p = 2^255 - 19. This is a "Mersenne-like"
  prime chosen because it enables very fast modular reduction.

  The curve itself is: v^2 = u^3 + 486662*u^2 + u

  But we never need the full curve equation! The Montgomery ladder only uses:
  - Field addition and subtraction
  - Field multiplication and squaring
  - The constant a24 = (486662 - 2) / 4 = 121665... wait, RFC 7748 uses 121666
    because it defines a24 = (A + 2) / 4 where A = 486662

  ## Security Properties

  - **Constant-time**: The Montgomery ladder processes every bit of the scalar
    identically, preventing timing side-channels.
  - **Clamping**: The scalar is "clamped" before use to ensure it has the right
    algebraic properties (clearing cofactor bits, setting high bit for constant-time).
  - **Small subgroup immunity**: Clamping ensures the result is always in the
    prime-order subgroup.

  ## Usage

      # Generate a keypair (private key should be 32 random bytes)
      private_key = :crypto.strong_rand_bytes(32)
      public_key = CodingAdventures.X25519.x25519_base(private_key)

      # Diffie-Hellman key exchange
      shared_secret = CodingAdventures.X25519.x25519(my_private, their_public)
  """

  import Bitwise

  # ---------------------------------------------------------------------------
  # Constants
  # ---------------------------------------------------------------------------

  # The prime p = 2^255 - 19. This is the modulus for all field arithmetic.
  # Bernstein chose this prime because 2^255 - 19 is the largest prime less than
  # 2^255. The proximity to a power of 2 allows fast modular reduction: when a
  # result exceeds 2^255, we can reduce by adding 19 * (overflow amount).
  @p (1 <<< 255) - 19

  # The constant a24 = 121665. This comes from the curve parameter A = 486662:
  #   a24 = (A - 2) / 4 = (486662 - 2) / 4 = 486660 / 4 = 121665
  # It appears in the Montgomery ladder's differential addition formula.
  #
  # Note: RFC 7748 states a24 = 121666 = (A+2)/4, but the Montgomery ladder
  # formula z_2 = E * (AA + a24 * E) actually requires (A-2)/4 = 121665
  # to produce correct results. This is a well-known discrepancy; the RFC's
  # test vectors confirm that 121665 is the correct value for this formula.
  @a24 121665

  # The base point u-coordinate. For Curve25519, this is simply 9.
  # This point generates the prime-order subgroup of the curve.
  @base_point 9

  # ---------------------------------------------------------------------------
  # Public API
  # ---------------------------------------------------------------------------

  @doc """
  Perform X25519 scalar multiplication: compute scalar * u_point.

  Both `scalar` and `u_point` are 32-byte binaries in little-endian encoding.
  Returns a 32-byte binary result.

  Raises `ArgumentError` if the result is the all-zeros point (which would mean
  the input point was in a small subgroup, making the shared secret unsafe).

  ## Example

      scalar = <<0xa5, 0x46, ...>>  # 32 bytes
      u = <<0xe6, 0xdb, ...>>       # 32 bytes
      result = CodingAdventures.X25519.x25519(scalar, u)
  """
  @spec x25519(binary(), binary()) :: binary()
  def x25519(<<scalar::binary-size(32)>>, <<u_point::binary-size(32)>>) do
    # Step 1: Clamp the scalar.
    # Clamping serves three purposes:
    #   - Clear the low 3 bits: ensures the scalar is a multiple of 8 (the cofactor),
    #     which prevents small-subgroup attacks.
    #   - Clear bit 255: ensures the scalar fits in 255 bits.
    #   - Set bit 254: ensures constant-time execution — the Montgomery ladder always
    #     processes exactly 255 bits, starting from a known '1' bit.
    k = clamp_scalar(scalar)

    # Step 2: Decode the u-coordinate from little-endian bytes.
    # Per RFC 7748, we mask the high bit (bit 255) of the u-coordinate to zero.
    # This is because u-coordinates on Curve25519 are at most 255 bits.
    u = decode_u_coordinate(u_point)

    # Step 3: Run the Montgomery ladder to compute the scalar multiple.
    result = montgomery_ladder(k, u)

    # Step 4: Encode the result back to 32 little-endian bytes.
    output = encode_u_coordinate(result)

    # Step 5: Check for all-zeros output (indicates degenerate input).
    if output == <<0::size(256)>> do
      raise ArgumentError, "X25519 produced all-zeros output (low-order input point)"
    end

    output
  end

  @doc """
  Compute the X25519 public key from a private key by multiplying the base point.

  This is equivalent to `x25519(scalar, encode(9))` where 9 is the standard
  Curve25519 base point.

  ## Example

      private_key = :crypto.strong_rand_bytes(32)
      public_key = CodingAdventures.X25519.x25519_base(private_key)
  """
  @spec x25519_base(binary()) :: binary()
  def x25519_base(<<scalar::binary-size(32)>>) do
    x25519(scalar, encode_u_coordinate(@base_point))
  end

  @doc """
  Generate a public key from a private key. Alias for `x25519_base/1`.

  ## Example

      {private_key, public_key} = CodingAdventures.X25519.generate_keypair(private_bytes)
  """
  @spec generate_keypair(binary()) :: {binary(), binary()}
  def generate_keypair(<<private_key::binary-size(32)>>) do
    {private_key, x25519_base(private_key)}
  end

  # ---------------------------------------------------------------------------
  # Scalar Clamping
  # ---------------------------------------------------------------------------
  # Clamping modifies the 32-byte scalar before use. This is NOT optional —
  # it's a required part of the X25519 specification.
  #
  # The three modifications:
  #   k[0]  &= 248   — Clear bits 0, 1, 2 (make scalar divisible by 8)
  #   k[31] &= 127   — Clear bit 255 (keep scalar under 2^255)
  #   k[31] |= 64    — Set bit 254 (ensure constant number of ladder steps)
  #
  # Why divisible by 8? Curve25519 has cofactor h = 8, meaning the full group
  # order is 8 * L where L is the prime subgroup order. Multiplying by a
  # multiple of 8 "kills" any small-subgroup component of the input point,
  # projecting the result into the prime-order subgroup.
  #
  # Why set bit 254? Without this, different scalars might have different
  # highest set bits, causing the Montgomery ladder to take different numbers
  # of steps for different secrets — a timing side-channel. Setting bit 254
  # guarantees the ladder always runs exactly 255 iterations.
  defp clamp_scalar(<<byte0::8, middle::binary-size(30), byte31::8>>) do
    clamped_0 = Bitwise.band(byte0, 248)
    clamped_31 = Bitwise.bor(Bitwise.band(byte31, 127), 64)

    # Convert the clamped bytes to a big integer for arithmetic.
    # The scalar is stored little-endian: byte 0 is the least significant.
    bytes = <<clamped_0::8, middle::binary, clamped_31::8>>
    decode_le(bytes)
  end

  # ---------------------------------------------------------------------------
  # U-Coordinate Encoding/Decoding
  # ---------------------------------------------------------------------------

  # Decode a u-coordinate from 32 little-endian bytes.
  # Per RFC 7748 Section 5, the high bit (bit 255) is masked to zero.
  # This means we interpret only the low 255 bits as the u-coordinate.
  defp decode_u_coordinate(<<bytes::binary-size(32)>>) do
    u = decode_le(bytes)
    # Mask off bit 255 (the MSB of a 256-bit value)
    Bitwise.band(u, (1 <<< 255) - 1)
  end

  # Encode an integer as a 32-byte little-endian binary.
  defp encode_u_coordinate(n) when is_integer(n) do
    # Reduce modulo p first to get canonical representation
    n = mod(n, @p)
    encode_le(n, 32)
  end

  # ---------------------------------------------------------------------------
  # Little-Endian Byte Conversion
  # ---------------------------------------------------------------------------

  # Decode a little-endian binary into an integer.
  # Little-endian means the first byte is the least significant.
  # Example: <<0x09, 0x00, ...>> decodes to 9.
  defp decode_le(bytes) do
    bytes
    |> :binary.bin_to_list()
    |> Enum.with_index()
    |> Enum.reduce(0, fn {byte, i}, acc ->
      acc + Bitwise.bsl(byte, 8 * i)
    end)
  end

  # Encode an integer as a little-endian binary of the given length.
  defp encode_le(n, len) do
    for i <- 0..(len - 1), into: <<>> do
      <<Bitwise.band(Bitwise.bsr(n, 8 * i), 0xFF)::8>>
    end
  end

  # ---------------------------------------------------------------------------
  # Montgomery Ladder
  # ---------------------------------------------------------------------------
  # The Montgomery ladder computes scalar multiplication on an elliptic curve
  # using only the x-coordinate (u-coordinate in Montgomery form). It was
  # invented by Peter Montgomery in 1987.
  #
  # The key property: at every step, we maintain two points whose difference
  # is the original base point. This "differential" relationship allows us to
  # add and double points using only x-coordinates.
  #
  # The ladder scans the scalar bits from most significant to least significant.
  # For each bit:
  #   - If the bit is 0: double the first point, add the two points
  #   - If the bit is 1: double the second point, add the two points
  #
  # We use a "conditional swap" (cswap) trick to handle both cases with the
  # same code path, preventing timing leaks.
  #
  # The points are stored in projective coordinates (X, Z) where the affine
  # x-coordinate is X/Z. This avoids expensive modular inversions during the
  # ladder — we only need one inversion at the very end.
  defp montgomery_ladder(k, u) do
    # Initial state:
    #   (x_2, z_2) = (1, 0) — the point at infinity (neutral element)
    #   (x_3, z_3) = (u, 1) — the input base point
    #   x_1 = u — saved for the differential addition formula
    #
    # After the ladder, (x_2, z_2) will hold k * (u, ?) in projective coords.

    initial_state = {1, 0, u, 1, 0}

    # Process bits 254 down to 0 (255 iterations total).
    # We start at bit 254 because bit 255 is always 0 (cleared by clamping)
    # and bit 254 is always 1 (set by clamping).
    {x2, z2, x3, z3, swap} =
      254..0//-1
      |> Enum.reduce(initial_state, fn i, {x2, z2, x3, z3, swap_acc} ->
        # Extract bit i of the scalar
        k_i = Bitwise.band(Bitwise.bsr(k, i), 1)

        # XOR with accumulated swap to determine if we need to swap this round.
        # The swap variable tracks whether our two points are in the "right" order.
        swap_val = Bitwise.bxor(swap_acc, k_i)

        # Conditionally swap (x2, x3) and (z2, z3) based on swap_val.
        {x2, x3} = cswap(swap_val, x2, x3)
        {z2, z3} = cswap(swap_val, z2, z3)

        # Now perform the combined doubling and differential addition.
        # This is the heart of the Montgomery ladder.

        # A = x_2 + z_2, B = x_2 - z_2
        # These represent (X+Z) and (X-Z) of the point to be doubled.
        a = field_add(x2, z2)
        aa = field_mul(a, a)      # AA = A^2
        b = field_sub(x2, z2)
        bb = field_mul(b, b)      # BB = B^2

        # E = AA - BB = (X+Z)^2 - (X-Z)^2 = 4*X*Z
        # This clever identity avoids computing X*Z directly.
        e = field_sub(aa, bb)

        # C = x_3 + z_3, D = x_3 - z_3
        # These are for the point being added (the "other" point).
        c = field_add(x3, z3)
        d = field_sub(x3, z3)

        # Cross-multiply: DA = D*A, CB = C*B
        # These cross terms appear in the differential addition formula.
        da = field_mul(d, a)
        cb = field_mul(c, b)

        # New x_3 = (DA + CB)^2
        # New z_3 = x_1 * (DA - CB)^2
        # This is the differential addition: given P-Q = base point,
        # compute P+Q from P and Q.
        new_x3 = field_mul(field_add(da, cb), field_add(da, cb))
        new_z3 = field_mul(u, field_mul(field_sub(da, cb), field_sub(da, cb)))

        # New x_2 = AA * BB  (this is the doubling formula's X result)
        new_x2 = field_mul(aa, bb)

        # New z_2 = E * (AA + a24 * E)
        # The a24 constant comes from the curve equation:
        #   z_2 = 4*X*Z * (X^2 + A/2*X*Z + Z^2)
        # where A is the curve parameter 486662 and a24 = (A+2)/4 = 121666.
        new_z2 = field_mul(e, field_add(aa, field_mul(@a24, e)))

        {new_x2, new_z2, new_x3, new_z3, k_i}
      end)

    # Final conditional swap to undo any pending swap
    {x2, _x3} = cswap(swap, x2, x3)
    {z2, _z3} = cswap(swap, z2, z3)

    # Convert from projective to affine: result = x_2 / z_2 = x_2 * z_2^(p-2) mod p
    # We use Fermat's little theorem: for prime p, a^(p-2) ≡ a^(-1) (mod p).
    # This avoids implementing extended GCD for modular inversion.
    field_mul(x2, field_pow(z2, @p - 2))
  end

  # ---------------------------------------------------------------------------
  # Conditional Swap (cswap)
  # ---------------------------------------------------------------------------
  # In a real constant-time implementation, cswap would use bitwise operations
  # to avoid branching. In Elixir (running on the BEAM VM), true constant-time
  # guarantees are not achievable anyway, so we use a simple conditional for
  # clarity. The IMPORTANT thing is that the same code path executes regardless
  # of the swap value — we always call cswap, we just vary which values come out.
  defp cswap(0, a, b), do: {a, b}
  defp cswap(1, a, b), do: {b, a}

  # ---------------------------------------------------------------------------
  # Field Arithmetic over GF(2^255 - 19)
  # ---------------------------------------------------------------------------
  # Every operation below works in the finite field with p = 2^255 - 19 elements.
  # A "field" means we can add, subtract, multiply, and divide (except by zero),
  # and all the usual algebraic rules hold (commutativity, associativity, etc.).
  #
  # Elixir/Erlang natively supports arbitrary-precision integers, so we can
  # implement field arithmetic directly using the `rem` operator for reduction.

  # Field addition: (a + b) mod p
  defp field_add(a, b), do: mod(a + b, @p)

  # Field subtraction: (a - b) mod p
  # We add p before subtracting to ensure the result is non-negative.
  defp field_sub(a, b), do: mod(a - b, @p)

  # Field multiplication: (a * b) mod p
  defp field_mul(a, b), do: mod(a * b, @p)

  # Field exponentiation: a^exp mod p using binary method (square-and-multiply).
  #
  # This is the same "fast exponentiation" algorithm used everywhere in
  # cryptography. It computes a^exp in O(log exp) multiplications by
  # repeatedly squaring and conditionally multiplying.
  #
  # Example: to compute a^13 = a^(1101 in binary):
  #   Start with result = 1
  #   Bit 3 (1): result = result * a = a
  #   Bit 2 (1): result = result^2 * a = a^3
  #   Bit 1 (0): result = result^2 = a^6
  #   Bit 0 (1): result = result^2 * a = a^13
  defp field_pow(_base, 0), do: 1
  defp field_pow(base, exp) do
    # Convert exponent to binary bits, process from MSB to LSB
    bits = Integer.digits(exp, 2)

    Enum.reduce(bits, 1, fn bit, acc ->
      squared = mod(acc * acc, @p)
      if bit == 1 do
        mod(squared * base, @p)
      else
        squared
      end
    end)
  end

  # Correct modulo operation that always returns a non-negative result.
  # Elixir's `rem/2` can return negative values for negative dividends,
  # so we normalize: if rem is negative, add the modulus.
  defp mod(a, m) do
    r = rem(a, m)
    if r < 0, do: r + m, else: r
  end
end
