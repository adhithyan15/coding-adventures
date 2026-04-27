# ============================================================================
# Ed25519: Digital Signatures on the Edwards Curve (RFC 8032)
# ============================================================================
#
# Ed25519 is an elliptic curve digital signature algorithm (EdDSA) designed by
# Daniel J. Bernstein et al. It uses the twisted Edwards curve:
#
#     -x^2 + y^2 = 1 + d*x^2*y^2    (mod p)
#
# where p = 2^255 - 19. Ed25519 provides:
#   - 32-byte public keys and 64-byte signatures
#   - 128-bit security level
#   - Deterministic signatures (no random nonce needed)
#   - Fast signing and verification
#
# ARCHITECTURE
# ============
# Elixir has native arbitrary-precision integers, so all arithmetic is
# straightforward. No limb-based representation is needed.
#
# EXTENDED COORDINATES
# ====================
# Points on the curve are represented as {x, y, z, t} where:
#   x = X/Z,  y = Y/Z,  T = X*Y/Z
#
# The identity point is {0, 1, 1, 0} -- affine (0, 1).
# The unified addition formula works for all point pairs including
# doubling, adding the identity, and adding inverses.
# ============================================================================

defmodule CodingAdventures.Ed25519 do
  @moduledoc """
  Pure Elixir implementation of Ed25519 digital signatures (RFC 8032).

  Ed25519 operates on the twisted Edwards curve -x^2 + y^2 = 1 + d*x^2*y^2
  over the prime field GF(2^255 - 19). It provides 128-bit security with
  32-byte public keys and 64-byte deterministic signatures.
  """

  import Bitwise

  # ==========================================================================
  # PART 1: CURVE CONSTANTS
  # ==========================================================================
  # All constants are Elixir arbitrary-precision integers.

  # The prime field: p = 2^255 - 19
  @field_prime (1 <<< 255) - 19

  # The curve parameter d = -121665/121666 mod p.
  # Pre-computed from: d = -121665 * modpow(121666, p-2, p) mod p
  @curve_d 37095705934669439343138083508754565189542113879843219016388785533085940283555

  # The group order: L = 2^252 + 27742317777372353535851937790883648493
  @group_order (1 <<< 252) + 27742317777372353535851937790883648493

  # sqrt(-1) mod p, used in the square root computation.
  # Pre-computed: modpow(2, (p-1)/4, p)
  @sqrt_minus_one 19681161376707505956807079304988542015446066515923890162744021073123829784752

  # Base point B coordinates (from RFC 8032 Section 5.1).
  # B_y = 4/5 mod p = 4 * modpow(5, p-2, p) mod p
  # B_x is the positive (even) square root recovered from the curve equation.
  @base_y 46316835694926478169428394003475163141307993866256225615783033603165251855960
  @base_x 15112221349535400772501151409588531511454012693041857206046113283949847762202

  # ==========================================================================
  # PART 2: FIELD ARITHMETIC (mod p = 2^255 - 19)
  # ==========================================================================
  # With Elixir's native big integers, field arithmetic is just modular ops.
  # We use the `rem` operator and ensure results are non-negative.

  # Modular addition: (a + b) mod p
  defp field_add(val_a, val_b), do: rem(val_a + val_b, @field_prime)

  # Modular subtraction: (a - b) mod p, ensuring non-negative result.
  # We add p before taking the remainder to avoid negative intermediates.
  defp field_sub(val_a, val_b), do: rem(val_a - val_b + @field_prime, @field_prime)

  # Modular multiplication: (a * b) mod p
  defp field_mul(val_a, val_b), do: rem(val_a * val_b, @field_prime)

  # Modular squaring: a^2 mod p (just multiplication, but named for clarity)
  defp field_sq(val_a), do: rem(val_a * val_a, @field_prime)

  # Modular negation: -a mod p = p - a (for non-zero a)
  defp field_neg(0), do: 0
  defp field_neg(val_a), do: @field_prime - rem(val_a, @field_prime)

  # --------------------------------------------------------------------------
  # Field Inversion: a^(p-2) mod p (Fermat's little theorem)
  # --------------------------------------------------------------------------
  # Since p is prime, a^(p-1) = 1 (mod p), so a^(p-2) is the inverse of a.
  # We use Elixir's built-in modular exponentiation which is efficient for
  # large exponents (it uses binary exponentiation internally).
  defp field_inv(val_a) do
    mod_pow(val_a, @field_prime - 2, @field_prime)
  end

  # --------------------------------------------------------------------------
  # Field Square Root for p = 5 (mod 8)
  # --------------------------------------------------------------------------
  # For primes p where p mod 8 = 5 (and 2^255 - 19 mod 8 = 5), the square
  # root of a is computed as:
  #
  #   candidate = a^((p+3)/8) mod p
  #
  # Then we check:
  #   - If candidate^2 == a: return candidate
  #   - If candidate^2 == -a: return candidate * sqrt(-1) mod p
  #   - Otherwise: a has no square root (not a quadratic residue)
  #
  # This works because (p+3)/8 = (p-1)/2 * 1/4 + 1/2, and by Euler's
  # criterion, a^((p-1)/2) = 1 if a is a QR.
  defp field_sqrt(val_a) do
    exponent = div(@field_prime + 3, 8)
    candidate = mod_pow(val_a, exponent, @field_prime)
    check = field_sq(candidate)

    cond do
      check == rem(val_a, @field_prime) ->
        {:ok, candidate}

      check == field_neg(val_a) ->
        {:ok, field_mul(candidate, @sqrt_minus_one)}

      true ->
        :error
    end
  end

  # --------------------------------------------------------------------------
  # Modular Exponentiation (binary method)
  # --------------------------------------------------------------------------
  # Computes base^exp mod modulus using repeated squaring.
  # This is O(log(exp)) multiplications.
  defp mod_pow(_base, 0, _modulus), do: 1
  defp mod_pow(base, exp, modulus) when exp > 0 do
    do_mod_pow(rem(base, modulus), exp, modulus, 1)
  end

  defp do_mod_pow(_base, 0, _modulus, acc), do: acc
  defp do_mod_pow(base, exp, modulus, acc) do
    acc = if (exp &&& 1) == 1, do: rem(acc * base, modulus), else: acc
    base = rem(base * base, modulus)
    do_mod_pow(base, exp >>> 1, modulus, acc)
  end

  # ==========================================================================
  # PART 3: POINT OPERATIONS (Extended Coordinates)
  # ==========================================================================
  # A point is a tuple {x, y, z, t} where the affine coordinates are
  # x_aff = x/z, y_aff = y/z, and t = x*y/z.

  # The identity point: affine (0, 1), extended {0, 1, 1, 0}.
  defp point_identity, do: {0, 1, 1, 0}

  # --------------------------------------------------------------------------
  # Point Addition (unified formula for twisted Edwards a=-1)
  # --------------------------------------------------------------------------
  # From the Hisil-Wong-Carter-Dawson paper, this formula works for ALL
  # point pairs (including P+P, P+O, P+(-P)) without branching.
  #
  # Input: P1 = {x1,y1,z1,t1}, P2 = {x2,y2,z2,t2}
  # Output: P3 = {x3,y3,z3,t3}
  #
  # Formulas (twisted Edwards with a = -1):
  #   val_a = x1*x2,  val_b = y1*y2
  #   val_c = t1*d*t2, val_d = z1*z2
  #   val_e = (x1+y1)*(x2+y2) - val_a - val_b
  #   val_f = val_d - val_c
  #   val_g = val_d + val_c
  #   val_h = val_b + val_a    (because a = -1, so -a*A = +A)
  #   x3 = val_e*val_f, y3 = val_g*val_h
  #   t3 = val_e*val_h, z3 = val_f*val_g
  defp point_add({x1, y1, z1, t1}, {x2, y2, z2, t2}) do
    val_a = field_mul(x1, x2)
    val_b = field_mul(y1, y2)
    val_c = field_mul(field_mul(t1, @curve_d), t2)
    val_d = field_mul(z1, z2)

    val_e = field_sub(
      field_mul(field_add(x1, y1), field_add(x2, y2)),
      field_add(val_a, val_b)
    )
    val_f = field_sub(val_d, val_c)
    val_g = field_add(val_d, val_c)
    val_h = field_add(val_b, val_a)

    {field_mul(val_e, val_f),
     field_mul(val_g, val_h),
     field_mul(val_f, val_g),
     field_mul(val_e, val_h)}
  end

  # --------------------------------------------------------------------------
  # Point Doubling
  # --------------------------------------------------------------------------
  # Doubling uses a cheaper formula than generic addition:
  #
  #   val_a = x1^2,  val_b = y1^2,  val_c = 2*z1^2
  #   val_d = -val_a   (because a = -1)
  #   val_e = (x1+y1)^2 - val_a - val_b
  #   val_g = val_d + val_b
  #   val_f = val_g - val_c
  #   val_h = val_d - val_b
  #   x3 = val_e*val_f, y3 = val_g*val_h
  #   t3 = val_e*val_h, z3 = val_f*val_g
  defp point_double({x1, y1, z1, _t1}) do
    val_a = field_sq(x1)
    val_b = field_sq(y1)
    val_c = field_mul(2, field_sq(z1))
    val_d = field_neg(val_a)

    val_e = field_sub(field_sq(field_add(x1, y1)), field_add(val_a, val_b))
    val_g = field_add(val_d, val_b)
    val_f = field_sub(val_g, val_c)
    val_h = field_sub(val_d, val_b)

    {field_mul(val_e, val_f),
     field_mul(val_g, val_h),
     field_mul(val_f, val_g),
     field_mul(val_e, val_h)}
  end

  # --------------------------------------------------------------------------
  # Scalar Multiplication: double-and-add, high-to-low bit scanning
  # --------------------------------------------------------------------------
  # For a scalar n and point P, compute n*P by scanning bits of n from
  # the most significant to the least. For each bit:
  #   - Double the accumulator
  #   - If the bit is 1, add P
  #
  # This is O(log n) point operations.
  defp scalar_mult(scalar, point) do
    bits = bit_length(scalar)
    do_scalar_mult(scalar, point, point_identity(), bits - 1)
  end

  defp do_scalar_mult(_scalar, _point, result, bit_pos) when bit_pos < 0, do: result
  defp do_scalar_mult(scalar, point, result, bit_pos) do
    result = point_double(result)
    result = if (scalar >>> bit_pos &&& 1) == 1 do
      point_add(result, point)
    else
      result
    end
    do_scalar_mult(scalar, point, result, bit_pos - 1)
  end

  # Number of significant bits in an integer.
  defp bit_length(0), do: 0
  defp bit_length(num) when num > 0 do
    do_bit_length(num, 0)
  end

  defp do_bit_length(0, count), do: count
  defp do_bit_length(num, count), do: do_bit_length(num >>> 1, count + 1)

  # ==========================================================================
  # PART 4: POINT ENCODING/DECODING (RFC 8032 Section 5.1.2)
  # ==========================================================================

  # Encode a point as 32 bytes: y in little-endian, with the sign of x
  # stored in the high bit of byte 31 (0-indexed).
  defp point_encode({pt_x, pt_y, pt_z, _pt_t}) do
    # Convert to affine: x = X/Z, y = Y/Z
    z_inv = field_inv(pt_z)
    x_aff = field_mul(pt_x, z_inv)
    y_aff = field_mul(pt_y, z_inv)

    # Encode y as 32 bytes little-endian
    y_bytes = int_to_le_bytes(y_aff, 32)

    # Set the high bit of byte 31 (0-indexed) to the low bit of x.
    # In a binary, byte 31 is the last byte.
    <<prefix::binary-size(31), last_byte>> = y_bytes
    last_byte = last_byte ||| ((x_aff &&& 1) <<< 7)
    <<prefix::binary, last_byte>>
  end

  # Decode a 32-byte encoded point.
  # Returns {:ok, point} or :error.
  defp point_decode(encoded) when byte_size(encoded) != 32, do: :error
  defp point_decode(encoded) do
    <<prefix::binary-size(31), last_byte>> = encoded

    # Extract the sign bit of x from the high bit of byte 31
    x_sign = (last_byte >>> 7) &&& 1

    # Clear the sign bit to get y
    cleared_last = last_byte &&& 0x7F
    y_bytes = <<prefix::binary, cleared_last>>
    pt_y = le_bytes_to_int(y_bytes)

    # Check y < p
    if pt_y >= @field_prime do
      :error
    else
      decode_x_from_y(pt_y, x_sign)
    end
  end

  # Given y and the sign bit of x, recover x from the curve equation:
  #   -x^2 + y^2 = 1 + d*x^2*y^2
  #   x^2 * (-1 - d*y^2) = 1 - y^2
  #   x^2 = (y^2 - 1) * inv(d*y^2 + 1)
  defp decode_x_from_y(pt_y, x_sign) do
    y2 = field_sq(pt_y)
    numerator = field_sub(y2, 1)
    denominator = field_add(field_mul(@curve_d, y2), 1)
    den_inv = field_inv(denominator)
    x2 = field_mul(numerator, den_inv)

    cond do
      # If x^2 = 0, x must be 0. Sign bit must be 0.
      x2 == 0 ->
        if x_sign == 1 do
          :error
        else
          {:ok, {0, pt_y, 1, 0}}
        end

      true ->
        case field_sqrt(x2) do
          {:ok, pt_x} ->
            # Ensure the sign (low bit) matches
            pt_x = if (pt_x &&& 1) != x_sign, do: field_neg(pt_x), else: pt_x
            {:ok, {pt_x, pt_y, 1, field_mul(pt_x, pt_y)}}

          :error ->
            :error
        end
    end
  end

  # ==========================================================================
  # PART 5: BYTE/INTEGER CONVERSION HELPERS
  # ==========================================================================

  # Convert a non-negative integer to a little-endian binary of given length.
  defp int_to_le_bytes(num, byte_count) do
    do_int_to_le_bytes(num, byte_count, <<>>)
  end

  defp do_int_to_le_bytes(_num, 0, acc), do: acc
  defp do_int_to_le_bytes(num, remaining, acc) do
    do_int_to_le_bytes(num >>> 8, remaining - 1, <<acc::binary, num &&& 0xFF>>)
  end

  # Convert a little-endian binary to a non-negative integer.
  defp le_bytes_to_int(bin) do
    bin
    |> :binary.bin_to_list()
    |> Enum.with_index()
    |> Enum.reduce(0, fn {byte_val, idx}, acc ->
      acc + (byte_val <<< (8 * idx))
    end)
  end

  # ==========================================================================
  # PART 6: SHA-512 INTEGRATION
  # ==========================================================================
  # Our SHA-512 module returns a binary. We need to convert to an integer
  # (interpreting as little-endian) for scalar arithmetic.

  defp sha512(data) do
    CodingAdventures.Sha512.sha512(data)
  end

  defp sha512_to_int(data) do
    hash_bin = sha512(data)
    le_bytes_to_int(hash_bin)
  end

  # ==========================================================================
  # PART 7: PUBLIC API
  # ==========================================================================

  # Build the base point B in extended coordinates.
  @base_point {@base_x, @base_y, 1, rem(@base_x * @base_y, (1 <<< 255) - 19)}

  # --------------------------------------------------------------------------
  # generate_keypair(seed) -> {public_key, secret_key}
  # --------------------------------------------------------------------------
  # Takes a 32-byte seed binary. Returns:
  #   public_key: 32-byte encoded point (the public key A)
  #   secret_key: 64-byte binary (seed <> public_key), for use in sign()
  #
  # The secret scalar is derived by hashing the seed with SHA-512 and
  # "clamping" the first 32 bytes:
  #   - Clear bits 0, 1, 2 (make divisible by cofactor 8)
  #   - Clear bit 255
  #   - Set bit 254

  @doc """
  Generate an Ed25519 keypair from a 32-byte seed.

  Returns `{public_key, secret_key}` where:
  - `public_key` is a 32-byte binary (the encoded public key point)
  - `secret_key` is a 64-byte binary (seed concatenated with public_key)
  """
  @spec generate_keypair(binary()) :: {binary(), binary()}
  def generate_keypair(seed) when byte_size(seed) == 32 do
    hash = sha512(seed)
    <<first_32::binary-size(32), _rest::binary-size(32)>> = hash

    # Clamp the first 32 bytes to get the secret scalar
    <<first_byte, mid::binary-size(30), last_byte>> = first_32
    clamped_first = first_byte &&& 248
    clamped_last = (last_byte &&& 127) ||| 64
    clamped = <<clamped_first, mid::binary, clamped_last>>

    scalar = le_bytes_to_int(clamped)

    # A = scalar * B (the public key point)
    pt_a = scalar_mult(scalar, @base_point)
    public_key = point_encode(pt_a)

    # Secret key = seed <> public_key
    secret_key = seed <> public_key

    {public_key, secret_key}
  end

  # --------------------------------------------------------------------------
  # sign(message, secret_key) -> signature
  # --------------------------------------------------------------------------
  # Creates a 64-byte deterministic signature.
  #
  # Steps:
  #   1. Hash the seed to get scalar a and prefix (last 32 bytes of hash).
  #   2. r = SHA-512(prefix || message) mod L  -- deterministic nonce
  #   3. R = r * B
  #   4. S = (r + SHA-512(R || A || message) * a) mod L
  #   5. Return encode(R) || encode(S)

  @doc """
  Sign a message with an Ed25519 secret key.

  Returns a 64-byte signature binary.
  """
  @spec sign(binary(), binary()) :: binary()
  def sign(message, secret_key) when byte_size(secret_key) == 64 do
    <<seed::binary-size(32), public_key::binary-size(32)>> = secret_key

    # Re-derive the scalar and prefix from the seed
    hash = sha512(seed)
    <<first_32::binary-size(32), prefix::binary-size(32)>> = hash

    <<first_byte, mid::binary-size(30), last_byte>> = first_32
    clamped_first = first_byte &&& 248
    clamped_last = (last_byte &&& 127) ||| 64
    clamped = <<clamped_first, mid::binary, clamped_last>>
    scalar_a = le_bytes_to_int(clamped)

    # r = SHA-512(prefix || message) mod L
    r_hash = sha512_to_int(prefix <> message)
    nonce_r = rem(r_hash, @group_order)

    # R = r * B
    r_point = scalar_mult(nonce_r, @base_point)
    r_encoded = point_encode(r_point)

    # k = SHA-512(R || A || message) mod L
    k_hash = sha512_to_int(r_encoded <> public_key <> message)
    challenge_k = rem(k_hash, @group_order)

    # S = (r + k * a) mod L
    scalar_s = rem(nonce_r + challenge_k * scalar_a, @group_order)

    # Encode S as 32 bytes LE
    s_encoded = int_to_le_bytes(scalar_s, 32)

    r_encoded <> s_encoded
  end

  # --------------------------------------------------------------------------
  # verify(message, signature, public_key) -> boolean
  # --------------------------------------------------------------------------
  # Verifies a 64-byte signature against a message and public key.
  #
  # Steps:
  #   1. Decode R (first 32 bytes) and A (the public key) as curve points.
  #   2. Decode S (last 32 bytes) as a scalar. Check S < L.
  #   3. k = SHA-512(R || A || message) mod L
  #   4. Check: S * B == R + k * A

  @doc """
  Verify an Ed25519 signature.

  Returns `true` if the signature is valid, `false` otherwise.
  """
  @spec verify(binary(), binary(), binary()) :: boolean()
  def verify(message, signature, public_key)
      when byte_size(signature) == 64 and byte_size(public_key) == 32 do
    <<r_encoded::binary-size(32), s_encoded::binary-size(32)>> = signature

    # Decode R
    with {:ok, r_point} <- point_decode(r_encoded),
         # Decode A (public key)
         {:ok, a_point} <- point_decode(public_key) do

      # Decode S as a scalar
      scalar_s = le_bytes_to_int(s_encoded)

      # Check S < L (malleability check)
      if scalar_s >= @group_order do
        false
      else
        # k = SHA-512(R || A || message) mod L
        k_hash = sha512_to_int(r_encoded <> public_key <> message)
        challenge_k = rem(k_hash, @group_order)

        # Verify: S * B == R + k * A
        lhs = scalar_mult(scalar_s, @base_point)
        rhs = point_add(r_point, scalar_mult(challenge_k, a_point))

        # Compare by encoding both to 32 bytes
        point_encode(lhs) == point_encode(rhs)
      end
    else
      :error -> false
    end
  end

  # Catch-all for invalid lengths
  def verify(_message, _signature, _public_key), do: false

  # --------------------------------------------------------------------------
  # Hex Utilities
  # --------------------------------------------------------------------------

  @doc "Decode a hex string to a binary."
  @spec from_hex(String.t()) :: binary()
  def from_hex(hex_str) do
    Base.decode16!(hex_str, case: :mixed)
  end

  @doc "Encode a binary as a lowercase hex string."
  @spec to_hex(binary()) :: String.t()
  def to_hex(bin) do
    Base.encode16(bin, case: :lower)
  end
end
