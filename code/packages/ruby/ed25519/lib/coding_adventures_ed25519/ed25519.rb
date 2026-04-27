# frozen_string_literal: true

# Ed25519 Digital Signatures (RFC 8032)
#
# Ed25519 is a high-speed, high-security digital signature scheme built on
# the twisted Edwards curve:
#
#   -x^2 + y^2 = 1 + d*x^2*y^2     over GF(2^255 - 19)
#
# Why Twisted Edwards Curves?
# ===========================
# Edwards curves have a remarkable property: their addition formula is
# "complete" -- it works for ALL pairs of points, including doubling and
# the identity. No special cases needed! This eliminates an entire class
# of timing side-channel attacks.
#
# The "twisted" variant (coefficient a = -1) enables faster arithmetic
# while preserving completeness.
#
# Key Properties
# ==============
# - 128-bit security level (equivalent to ~3072-bit RSA)
# - Deterministic signatures (no random nonce needed)
# - 32-byte public keys, 64-byte signatures
# - Resistant to timing attacks (complete addition formula)
#
# Ruby Integer Note
# =================
# Ruby has arbitrary-precision integers, so we never overflow. All modular
# arithmetic is done with Ruby's built-in `%` operator. Unlike some
# languages, Ruby's `%` always returns a non-negative result when the
# divisor is positive: (-3) % 5 == 2. This is exactly what we want for
# modular arithmetic!
#
# Dependencies
# ============
# SHA-512 from coding_adventures_sha512 -- used for key derivation,
# deterministic nonce generation, and challenge hash computation.

require "coding_adventures_sha512"

module CodingAdventures
  module Ed25519
    # ── Field Constants ──────────────────────────────────────────────────
    #
    # p = 2^255 - 19: the prime modulus defining the field GF(p).
    # This Mersenne-like prime was chosen for fast modular reduction.

    P = (1 << 255) - 19

    # d is the curve parameter in -x^2 + y^2 = 1 + d*x^2*y^2.
    # It equals -121665/121666 mod p.

    D = 37095705934669439343138083508754565189542113879843219016388785533085940283555

    # L is the order of the base point B -- the number of points in the
    # prime-order subgroup. Every scalar is reduced modulo L.

    L = 7237005577332262213973186563042994240857116359379907606001950938285454250989

    # ── Square Root of -1 ────────────────────────────────────────────────
    #
    # In GF(p), -1 has a square root because p ≡ 5 (mod 8).
    # SQRT_M1 = 2^((p-1)/4) mod p, satisfying SQRT_M1^2 ≡ -1 (mod p).

    SQRT_M1 = 19681161376707505956807079304988542015446066515923890162744021073123829784752

    # ── Base Point B ─────────────────────────────────────────────────────
    #
    # The generator of the prime-order subgroup. y = 4/5 mod p,
    # x is the positive (even) square root.

    B_Y = 46316835694926478169428394003475163141307993866256225615783033603165251855960
    B_X = 15112221349535400772501151409588531511454012693041857206046113283949847762202

    # ── Modular Arithmetic ───────────────────────────────────────────────
    #
    # Ruby's % always returns a non-negative result when the divisor is
    # positive, so we don't need the ((a % m) + m) % m trick that
    # JavaScript requires. This simplifies the code considerably.

    # Modular exponentiation: base^exp mod m
    #
    # Uses the square-and-multiply algorithm. Ruby has Integer#pow(exp, mod)
    # built in, which is optimized and constant-time for large integers.
    def self.mod_pow(base, exp, m)
      base.pow(exp, m)
    end

    # Modular inverse via Fermat's little theorem: a^(-1) = a^(p-2) mod p
    def self.mod_inv(a, m)
      mod_pow(a, m - 2, m)
    end

    # ── Field Square Root ────────────────────────────────────────────────
    #
    # Since p ≡ 5 (mod 8), we use the Atkin algorithm:
    #   1. candidate = a^((p+3)/8) mod p
    #   2. If candidate^2 ≡ a (mod p), return candidate
    #   3. If candidate^2 ≡ -a (mod p), return candidate * SQRT_M1 mod p
    #   4. Otherwise, no square root exists
    def self.field_sqrt(a)
      exp = (P + 3) >> 3
      candidate = mod_pow(a, exp, P)
      check = (candidate * candidate) % P

      return candidate if check == a % P
      return (candidate * SQRT_M1) % P if check == (-a) % P

      nil
    end

    # ── Extended Point Representation ────────────────────────────────────
    #
    # Points are represented as [X, Y, Z, T] where:
    #   x = X/Z, y = Y/Z, T = X*Y/Z
    #
    # This avoids expensive modular inversions during point operations.

    IDENTITY = [0, 1, 1, 0].freeze
    BASE_POINT = [B_X, B_Y, 1, (B_X * B_Y) % P].freeze

    # ── Point Addition ───────────────────────────────────────────────────
    #
    # Unified addition on twisted Edwards curve -x^2 + y^2 = 1 + d*x^2*y^2.
    #
    # This "complete" formula works for ALL input pairs -- no special cases
    # for doubling, identity, or inverse points.
    #
    # Note: H = B + A (not B - A) because a = -1 in the twist.
    def self.point_add(p1, p2)
      x1, y1, z1, t1 = p1
      x2, y2, z2, t2 = p2

      a = (x1 * x2) % P
      b = (y1 * y2) % P
      c = (t1 * D % P * t2) % P
      dd = (z1 * z2) % P
      e = ((x1 + y1) * (x2 + y2) - a - b) % P
      f = (dd - c) % P
      g = (dd + c) % P
      h = (b + a) % P # a = -1, so H = B - (-1)*A = B + A

      [(e * f) % P, (g * h) % P, (f * g) % P, (e * h) % P]
    end

    # ── Point Doubling ───────────────────────────────────────────────────
    #
    # Dedicated doubling formula with fewer multiplications than addition.
    # D_val = -A because a = -1.
    def self.point_double(pt)
      x1, y1, z1, _t1 = pt

      a = (x1 * x1) % P
      b = (y1 * y1) % P
      c = (2 * ((z1 * z1) % P)) % P
      dd = (-a) % P # a = -1
      e = (((x1 + y1) * (x1 + y1)) % P - a - b) % P
      g = (dd + b) % P
      f = (g - c) % P
      h = (dd - b) % P

      [(e * f) % P, (g * h) % P, (f * g) % P, (e * h) % P]
    end

    # ── Scalar Multiplication ────────────────────────────────────────────
    #
    # Compute n * point using double-and-add (low bit to high bit).
    # This is the elliptic curve equivalent of modular exponentiation.
    def self.scalar_mul(n, point)
      n = n % L
      return IDENTITY if n == 0

      result = IDENTITY
      temp = point

      while n > 0
        result = point_add(result, temp) if n.odd?
        temp = point_double(temp)
        n >>= 1
      end
      result
    end

    # ── Point Encoding ───────────────────────────────────────────────────
    #
    # Encode a point as 32 bytes:
    #   1. Normalize to affine: x = X/Z, y = Y/Z
    #   2. Encode y as 32 bytes little-endian
    #   3. Set high bit of byte[31] to low bit of x (sign bit)
    def self.encode_point(point)
      x_coord, y_coord, z_coord, _t = point
      z_inv = mod_inv(z_coord, P)
      x_val = (x_coord * z_inv) % P
      y_val = (y_coord * z_inv) % P

      bytes = Array.new(32, 0)
      yy = y_val
      32.times do |i|
        bytes[i] = yy & 0xFF
        yy >>= 8
      end
      bytes[31] |= (x_val & 1) << 7
      bytes.pack("C*")
    end

    # ── Point Decoding ───────────────────────────────────────────────────
    #
    # Decode a 32-byte compressed point.
    # Steps:
    #   1. Extract sign bit from high bit of byte 31
    #   2. Decode y from 255 bits (little-endian)
    #   3. Compute x^2 = (y^2 - 1) / (1 + d*y^2) mod p
    #   4. Square root to get x
    #   5. Correct sign if needed
    def self.decode_point(bytes_str)
      raw = bytes_str.bytes
      return nil unless raw.length == 32

      sign = (raw[31] >> 7) & 1

      y_val = 0
      31.downto(0) do |i|
        y_val = (y_val << 8) | raw[i]
      end
      y_val &= (1 << 255) - 1

      return nil if y_val >= P

      y2 = (y_val * y_val) % P
      numerator = (y2 - 1) % P
      denominator = (D * y2 + 1) % P
      x2 = (numerator * mod_inv(denominator, P)) % P

      if x2 == 0
        return nil if sign != 0

        return [0, y_val, 1, 0]
      end

      x_val = field_sqrt(x2)
      return nil if x_val.nil?

      x_val = (-x_val) % P if (x_val & 1) != sign

      [x_val, y_val, 1, (x_val * y_val) % P]
    end

    # ── Key Clamping ─────────────────────────────────────────────────────
    #
    # Ed25519 clamps the private scalar from SHA-512(seed):
    #   1. Clear lowest 3 bits (multiple of 8 = cofactor)
    #   2. Clear bit 255
    #   3. Set bit 254 (fixed bit length)
    def self.clamp_scalar(hash_bytes)
      clamped = hash_bytes[0, 32].bytes.to_a
      clamped[0] &= 248
      clamped[31] &= 127
      clamped[31] |= 64
      clamped.pack("C*")
    end

    # ── Byte Helpers ─────────────────────────────────────────────────────

    # Convert a binary string to a little-endian integer
    def self.bytes_to_scalar(bytes_str)
      raw = bytes_str.bytes
      n = 0
      (raw.length - 1).downto(0) do |i|
        n = (n << 8) | raw[i]
      end
      n
    end

    # Convert a scalar to 32-byte little-endian binary string
    def self.scalar_to_bytes(n)
      val = n % L
      bytes = Array.new(32, 0)
      32.times do |i|
        bytes[i] = val & 0xFF
        val >>= 8
      end
      bytes.pack("C*")
    end

    # ── Public API ───────────────────────────────────────────────────────

    # Generate an Ed25519 keypair from a 32-byte seed.
    #
    # The seed is expanded via SHA-512:
    #   - First 32 bytes -> clamped scalar (private)
    #   - Last 32 bytes -> nonce prefix (for signing)
    #
    # Returns [public_key, secret_key] where:
    #   - public_key: 32-byte binary string (compressed point)
    #   - secret_key: 64-byte binary string (seed || public_key)
    #
    # Example:
    #   pub, sec = CodingAdventures::Ed25519.generate_keypair(seed)
    def self.generate_keypair(seed)
      hash = CodingAdventures::Sha512.sha512(seed)
      clamped = clamp_scalar(hash)
      a = bytes_to_scalar(clamped)

      point_a = scalar_mul(a, BASE_POINT)
      public_key = encode_point(point_a)

      secret_key = seed.b + public_key
      [public_key, secret_key]
    end

    # Sign a message with an Ed25519 secret key.
    #
    # Deterministic: same message + key always produces the same signature.
    #
    # Algorithm:
    #   1. r = SHA-512(prefix || message) mod L (deterministic nonce)
    #   2. R = r * B (commitment)
    #   3. k = SHA-512(R || A || message) mod L (challenge)
    #   4. S = (r + k * a) mod L (response)
    #
    # Returns a 64-byte binary string (R || S).
    def self.sign(message, secret_key)
      seed = secret_key[0, 32]
      public_key = secret_key[32, 32]

      hash = CodingAdventures::Sha512.sha512(seed)
      clamped = clamp_scalar(hash)
      a = bytes_to_scalar(clamped)
      prefix = hash[32, 32]

      # Deterministic nonce
      r_hash = CodingAdventures::Sha512.sha512(prefix + message.b)
      r = bytes_to_scalar(r_hash) % L

      # Commitment
      r_point = encode_point(scalar_mul(r, BASE_POINT))

      # Challenge
      k_hash = CodingAdventures::Sha512.sha512(r_point + public_key + message.b)
      k = bytes_to_scalar(k_hash) % L

      # Response
      s = (r + k * a) % L

      r_point + scalar_to_bytes(s)
    end

    # Verify an Ed25519 signature.
    #
    # Checks: S * B == R + SHA-512(R || A || message) * A
    #
    # Returns true if the signature is valid, false otherwise.
    def self.verify(message, signature, public_key)
      return false unless signature.bytesize == 64
      return false unless public_key.bytesize == 32

      r_bytes = signature[0, 32]
      r_point = decode_point(r_bytes)
      return false if r_point.nil?

      s = bytes_to_scalar(signature[32, 32])
      return false if s >= L

      a_point = decode_point(public_key)
      return false if a_point.nil?

      k_hash = CodingAdventures::Sha512.sha512(r_bytes + public_key + message.b)
      k = bytes_to_scalar(k_hash) % L

      lhs = scalar_mul(s, BASE_POINT)
      rhs = point_add(r_point, scalar_mul(k, a_point))

      encode_point(lhs) == encode_point(rhs)
    end
  end
end
