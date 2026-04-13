# frozen_string_literal: true

# ============================================================================
# x25519.rb — X25519 Elliptic Curve Diffie-Hellman (RFC 7748)
# ============================================================================
#
# X25519 is the Diffie-Hellman function on Curve25519, one of the most widely
# used key agreement protocols in modern cryptography. It is used in TLS 1.3,
# SSH, Signal, WireGuard, and many other protocols.
#
# The beauty of X25519 lies in its simplicity: the entire key exchange reduces
# to a single scalar multiplication on an elliptic curve, using only the
# x-coordinate (hence "X" 25519). This is the Montgomery ladder algorithm,
# which is naturally constant-time — a critical property for cryptographic
# implementations.
#
# ## The Math: Curve25519
#
# Curve25519 is a Montgomery curve defined by:
#
#   y^2 = x^3 + 486662x^2 + x   (mod p)
#
# where p = 2^255 - 19 (a prime). The constant 486662 is the curve parameter A.
# The constant a24 = (A - 2) / 4 = 121665 appears in the ladder formulas.
#
# ## Why Ruby?
#
# Ruby has native arbitrary-precision integers (Bignum), so field arithmetic
# over GF(2^255 - 19) is straightforward — no need for multi-limb
# representations or carry propagation. Ruby's `Integer#pow(exp, mod)` even
# gives us efficient modular exponentiation built-in.
#
# ============================================================================

module CodingAdventures
  module X25519
    # -----------------------------------------------------------------------
    # The prime field: p = 2^255 - 19
    # -----------------------------------------------------------------------
    # This prime was chosen by Daniel Bernstein because:
    # 1. It's very close to a power of 2, making reduction fast
    # 2. 19 is small, so the "correction" after 2^255 is cheap
    # 3. The resulting curve has excellent security properties

    P = (1 << 255) - 19

    # -----------------------------------------------------------------------
    # The curve constant a24 = 121665
    # -----------------------------------------------------------------------
    # The Montgomery curve y^2 = x^3 + Ax^2 + x has A = 486662.
    # The ladder formulas use a24 = (A - 2) / 4 = 121665.

    A24 = 121665

    # -----------------------------------------------------------------------
    # The base point: u = 9
    # -----------------------------------------------------------------------
    # The base point for Curve25519 has x-coordinate (u-coordinate) = 9.
    # Encoded as a 32-byte little-endian array.

    BASE_POINT = ([9] + [0] * 31).pack("C*").bytes.freeze

    # -----------------------------------------------------------------------
    # Field operations: the building blocks
    # -----------------------------------------------------------------------
    # All operations are modulo p. Ruby's native big integers handle
    # arbitrary precision automatically.

    class << self
      # Modular addition: (a + b) mod p
      def field_add(a, b)
        (a + b) % P
      end

      # Modular subtraction: (a - b) mod p
      # We add P to ensure the result is non-negative before taking mod.
      def field_sub(a, b)
        (a - b + P) % P
      end

      # Modular multiplication: (a * b) mod p
      def field_mul(a, b)
        (a * b) % P
      end

      # Modular squaring: a^2 mod p
      def field_square(a)
        (a * a) % P
      end

      # Modular inverse: a^(-1) mod p
      #
      # Uses Fermat's little theorem: for prime p and a != 0,
      #   a^(p-1) = 1 (mod p)
      #   therefore a^(p-2) = a^(-1) (mod p)
      #
      # Ruby's Integer#pow(exp, mod) uses efficient modular exponentiation
      # internally (square-and-multiply with Montgomery reduction).
      def field_invert(a)
        a.pow(P - 2, P)
      end

      # -------------------------------------------------------------------
      # Byte encoding/decoding: the wire format
      # -------------------------------------------------------------------
      # X25519 uses little-endian byte encoding for both scalars and field
      # elements. A 32-byte array represents a 256-bit number with the
      # least significant byte first.

      # Decode a 32-byte little-endian array into an Integer.
      def decode_little_endian(bytes)
        result = 0
        bytes.each_with_index do |byte, i|
          result |= byte << (8 * i)
        end
        result
      end

      # Encode an Integer as a 32-byte little-endian array.
      def encode_little_endian(n)
        result = Array.new(32, 0)
        value = n
        32.times do |i|
          result[i] = value & 0xff
          value >>= 8
        end
        result
      end

      # Decode a u-coordinate from 32 bytes.
      #
      # Per RFC 7748 Section 5, the high bit of the last byte is masked off.
      # This ensures the u-coordinate is in range [0, 2^255 - 1].
      def decode_u_coordinate(bytes)
        copy = bytes.dup
        copy[31] &= 0x7f
        decode_little_endian(copy)
      end

      # -------------------------------------------------------------------
      # Scalar clamping: preparing the private key
      # -------------------------------------------------------------------
      # Before using a 32-byte secret key as a scalar multiplier, we "clamp" it:
      #
      #   k[0]  &= 248   — Clear low 3 bits (multiple of 8, cofactor)
      #   k[31] &= 127   — Clear bit 255
      #   k[31] |= 64    — Set bit 254 (fixed length for timing safety)
      #
      # This ensures the scalar is in [2^254, 2^255 - 1] and a multiple of 8.

      def clamp_scalar(k_bytes)
        clamped = k_bytes.dup
        clamped[0] &= 248
        clamped[31] &= 127
        clamped[31] |= 64
        decode_little_endian(clamped)
      end

      # -------------------------------------------------------------------
      # Conditional swap (cswap): constant-time selection
      # -------------------------------------------------------------------
      # Swaps a and b if swap_bit is 1; leaves them unchanged if 0.
      # Uses XOR masking to avoid branches.

      def cswap(swap_bit, a, b)
        # mask is all 1s if swap_bit=1, all 0s if swap_bit=0
        # For Ruby big integers, -1 is an infinite string of 1 bits
        mask = -swap_bit
        dummy = mask & (a ^ b)
        [a ^ dummy, b ^ dummy]
      end

      # -------------------------------------------------------------------
      # The Montgomery Ladder: the heart of X25519
      # -------------------------------------------------------------------
      #
      # Computes [k]u on the Montgomery curve using only x-coordinates.
      #
      # The ladder maintains two points in projective coordinates:
      #   (x_2, z_2) and (x_3, z_3)
      # These always differ by the base point u.
      #
      # At each step, one point is doubled and the other undergoes
      # differential addition, based on the current bit of k.
      #
      # After processing all 255 bits, we convert from projective to
      # affine: result = x_2 * z_2^(-1) mod p.

      # Perform the X25519 function: scalar multiplication on Curve25519.
      #
      # @param scalar [Array<Integer>] 32-byte private scalar
      # @param u_bytes [Array<Integer>] 32-byte u-coordinate
      # @return [Array<Integer>] 32-byte result u-coordinate
      # @raise [ArgumentError] if inputs are wrong length
      # @raise [RuntimeError] if result is all zeros (low-order point)
      def x25519(scalar, u_bytes)
        raise ArgumentError, "Scalar must be exactly 32 bytes" unless scalar.length == 32
        raise ArgumentError, "U-coordinate must be exactly 32 bytes" unless u_bytes.length == 32

        # Step 1: Clamp the scalar and decode the u-coordinate
        k = clamp_scalar(scalar)
        u = decode_u_coordinate(u_bytes)

        # Step 2: Initialize the Montgomery ladder
        x_1 = u
        x_2 = 1
        z_2 = 0
        x_3 = u
        z_3 = 1
        swap = 0

        # Step 3: Process each bit of k from bit 254 down to bit 0
        254.downto(0) do |i|
          k_i = (k >> i) & 1
          swap ^= k_i
          x_2, x_3 = cswap(swap, x_2, x_3)
          z_2, z_3 = cswap(swap, z_2, z_3)
          swap = k_i

          # The Montgomery ladder step:
          #
          # A = x_2 + z_2; AA = A^2; B = x_2 - z_2; BB = B^2
          # E = AA - BB (= 4*x_2*z_2)
          #
          # Doubling:
          #   x_2 = AA * BB
          #   z_2 = E * (AA + a24 * E)
          #
          # Differential addition:
          #   C = x_3 + z_3; D = x_3 - z_3
          #   DA = D * A; CB = C * B
          #   x_3 = (DA + CB)^2
          #   z_3 = x_1 * (DA - CB)^2

          a = field_add(x_2, z_2)
          aa = field_square(a)
          b = field_sub(x_2, z_2)
          bb = field_square(b)
          e = field_sub(aa, bb)

          c = field_add(x_3, z_3)
          d = field_sub(x_3, z_3)
          da = field_mul(d, a)
          cb = field_mul(c, b)

          x_3 = field_square(field_add(da, cb))
          z_3 = field_mul(x_1, field_square(field_sub(da, cb)))
          x_2 = field_mul(aa, bb)
          z_2 = field_mul(e, field_add(aa, field_mul(A24, e)))
        end

        # Step 4: Final conditional swap
        x_2, x_3 = cswap(swap, x_2, x_3)
        z_2, z_3 = cswap(swap, z_2, z_3)

        # Step 5: Convert from projective to affine coordinates
        result = field_mul(x_2, field_invert(z_2))
        encoded = encode_little_endian(result)

        # Step 6: Check for all-zeros result
        if encoded.all?(&:zero?)
          raise "X25519 produced all-zero output - input is a low-order point"
        end

        encoded
      end

      # Multiply the scalar by the Curve25519 base point (u = 9).
      #
      # @param scalar [Array<Integer>] 32-byte private key
      # @return [Array<Integer>] 32-byte public key
      def x25519_base(scalar)
        x25519(scalar, BASE_POINT)
      end

      # Generate a Curve25519 public key from a private key.
      # Alias for x25519_base.
      #
      # @param private_key [Array<Integer>] 32-byte private key
      # @return [Array<Integer>] 32-byte public key
      def generate_keypair(private_key)
        x25519_base(private_key)
      end
    end
  end
end
