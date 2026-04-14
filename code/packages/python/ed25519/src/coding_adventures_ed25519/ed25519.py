"""Ed25519 digital signatures (RFC 8032) -- core implementation.

This module implements every layer of Ed25519 from the ground up:

  1. Field arithmetic in GF(2^255 - 19)
  2. Extended twisted Edwards curve point operations
  3. Point encoding/decoding (32-byte compressed form)
  4. Key generation, signing, and verification

Each section includes detailed mathematical explanations so that someone
learning elliptic curve cryptography can follow along.

Dependency: SHA-512
===================
Ed25519 uses SHA-512 internally for hashing seeds, generating deterministic
nonces, and computing challenge scalars. We import it from our own from-scratch
SHA-512 implementation.
"""

from __future__ import annotations

from coding_adventures_sha512 import sha512

# ═══════════════════════════════════════════════════════════════════════════════
# SECTION 1: CONSTANTS
# ═══════════════════════════════════════════════════════════════════════════════
#
# The curve Ed25519 is defined over the prime field GF(p) where p = 2^255 - 19.
# All arithmetic on coordinates is done modulo this prime.
#
# The "d" constant defines the specific curve shape. It comes from the curve
# equation: -x^2 + y^2 = 1 + d*x^2*y^2.
#
# L is the order of the base point -- the number of times you can add the base
# point to itself before getting back to the identity. It is a large prime,
# which ensures the discrete logarithm problem (finding the scalar from a point)
# is hard.

p = 2**255 - 19
"""The prime modulus for field arithmetic: 2^255 - 19.

This prime was chosen because:
  - It is close to a power of 2, making reduction efficient
  - The -19 offset is small, simplifying modular reduction
  - It provides ~128 bits of security (254-bit prime)
"""

d = 37095705934669439343138083508754565189542113879843219016388785533085940283555
"""The curve parameter d in -x^2 + y^2 = 1 + d*x^2*y^2.

This is -121665/121666 mod p. The specific value was chosen to give the curve
desirable security properties and efficient arithmetic.
"""

L = 7237005577332262213973186563042994240857116359379907606001950938285454250989
"""The order of the base point (the subgroup order).

This is a 253-bit prime. Every valid Ed25519 scalar is reduced modulo L.
The value is: 2^252 + 27742317777372353535851937790883648493.
"""

# ── Base Point ──
#
# The base point B = (B_x, B_y) is a specific point on the curve that generates
# the prime-order subgroup used for all Ed25519 operations. Its coordinates are
# defined in RFC 8032 Section 5.1.
#
# B_y = 4/5 mod p (yes, the fraction 4/5 taken modulo the prime p).
# B_x is the positive square root satisfying the curve equation.

B_x = 15112221349535400772501151409588531511454012693041857206046113283949847762202
B_y = 46316835694926478169428394003475163141307993866256225615783033603165251855960

# ── Square Root of -1 ──
#
# In GF(p), -1 has a square root because p ≡ 5 (mod 8). This constant is used
# in the point decompression algorithm when computing square roots.
#
# SQRT_M1 = 2^((p-1)/4) mod p, which satisfies SQRT_M1^2 ≡ -1 (mod p).

SQRT_M1 = 19681161376707505956807079304988542015446066515923890162744021073123829784752


# ═══════════════════════════════════════════════════════════════════════════════
# SECTION 2: FIELD ARITHMETIC IN GF(2^255 - 19)
# ═══════════════════════════════════════════════════════════════════════════════
#
# All curve operations ultimately reduce to arithmetic in this field. Python's
# native big integers make this straightforward -- we just need modular
# arithmetic using the % operator and pow() with three arguments for modular
# exponentiation.
#
# The key operations:
#   - Addition, subtraction, multiplication: standard operators with % p
#   - Inversion: a^(-1) = a^(p-2) mod p (by Fermat's little theorem)
#   - Square root: needed for point decompression


def field_inv(a: int) -> int:
    """Compute the modular inverse a^(-1) mod p using Fermat's little theorem.

    For any nonzero element a in GF(p), we have a^(p-1) ≡ 1 (mod p).
    Therefore a^(p-2) * a ≡ 1 (mod p), so a^(p-2) is the inverse.

    Python's three-argument pow(base, exp, mod) uses fast modular
    exponentiation (square-and-multiply), so this is efficient even
    for 255-bit exponents.

    Example:
        >>> field_inv(3) * 3 % p == 1
        True
    """
    return pow(a, p - 2, p)


def field_sqrt(a: int) -> int:
    """Compute the square root of a in GF(p), or raise if none exists.

    Since p ≡ 5 (mod 8), we can use the formula:

        candidate = a^((p+3)/8) mod p

    Then we check:
      - If candidate^2 ≡ a (mod p): return candidate
      - If candidate^2 ≡ -a (mod p): return candidate * SQRT_M1 mod p
      - Otherwise: a is not a quadratic residue (no square root exists)

    Why does this work? For p ≡ 5 (mod 8):
      - (p+3)/8 is an integer
      - a^((p+3)/8) squared gives a^((p+3)/4)
      - By Euler's criterion, a^((p-1)/2) ≡ ±1 (mod p) for QR/non-QR
      - The algebra works out to give us ±sqrt(a) or ±sqrt(-a)

    Raises:
        ValueError: if a is not a quadratic residue mod p
    """
    # Candidate square root via Atkin's algorithm for p ≡ 5 (mod 8)
    candidate = pow(a, (p + 3) // 8, p)

    # Check: does candidate^2 equal a?
    if (candidate * candidate) % p == a % p:
        return candidate

    # If candidate^2 ≡ -a, multiply by sqrt(-1) to fix the sign
    if (candidate * candidate) % p == (-a) % p:
        return (candidate * SQRT_M1) % p

    # No square root exists -- a is a quadratic non-residue
    msg = "not a quadratic residue"
    raise ValueError(msg)


# ═══════════════════════════════════════════════════════════════════════════════
# SECTION 3: EXTENDED TWISTED EDWARDS POINT OPERATIONS
# ═══════════════════════════════════════════════════════════════════════════════
#
# Points on the curve are represented in "extended twisted Edwards coordinates"
# as 4-tuples (X, Y, Z, T) where:
#
#   x = X/Z    (affine x-coordinate)
#   y = Y/Z    (affine y-coordinate)
#   T = X*Y/Z  (auxiliary coordinate for faster addition)
#
# This projective representation avoids divisions during point addition.
# We only need one division at the very end when encoding a point.
#
# The formulas below come from the EFD (Explicit-Formulas Database):
#   https://hyperelliptic.org/EFD/g1p/auto-twisted-extended.html

# Type alias for clarity
Point = tuple[int, int, int, int]

# The identity element: the point (0, 1) in affine coordinates, which is
# (0, 1, 1, 0) in extended coordinates.
IDENTITY: Point = (0, 1, 1, 0)

# The base point in extended coordinates
B: Point = (B_x, B_y, 1, (B_x * B_y) % p)


def point_add(p1: Point, p2: Point) -> Point:
    """Add two points on the twisted Edwards curve.

    Uses the "unified" addition formula for twisted Edwards curves with a = -1:

        A = X1*X2       B = Y1*Y2       C = T1*d*T2     D = Z1*Z2
        E = (X1+Y1)*(X2+Y2) - A - B    F = D - C       G = D + C
        H = B + A       (note: +A because a = -1, so -a*A = +A)
        X3 = E*F        Y3 = G*H        T3 = E*H        Z3 = F*G

    This formula is "complete" -- it works for all input pairs including:
      - Adding a point to itself (doubling)
      - Adding the identity
      - Adding inverses (result is identity)

    No conditional branches means no timing side channels.

    Cost: 9 multiplications in GF(p)
    """
    x1, y1, z1, t1 = p1
    x2, y2, z2, t2 = p2

    a_val = (x1 * x2) % p
    b_val = (y1 * y2) % p
    c_val = (t1 * d * t2) % p
    d_val = (z1 * z2) % p

    e_val = ((x1 + y1) * (x2 + y2) - a_val - b_val) % p
    f_val = (d_val - c_val) % p
    g_val = (d_val + c_val) % p
    h_val = (b_val + a_val) % p  # +a_val because curve parameter a = -1

    x3 = (e_val * f_val) % p
    y3 = (g_val * h_val) % p
    t3 = (e_val * h_val) % p
    z3 = (f_val * g_val) % p

    return (x3, y3, z3, t3)


def point_double(pt: Point) -> Point:
    """Double a point on the twisted Edwards curve.

    Dedicated doubling is slightly faster than using the general addition
    formula because we can exploit the fact that both inputs are the same point.

    For a = -1 twisted Edwards curves:

        A = X1^2         B = Y1^2        C = 2*Z1^2
        D = -A           (since a = -1)
        E = (X1+Y1)^2 - A - B           G = D + B
        F = G - C        H = D - B
        X3 = E*F         Y3 = G*H        T3 = E*H        Z3 = F*G

    Cost: 4 squarings + 4 multiplications in GF(p)
    """
    x1, y1, z1, _t1 = pt

    a_val = (x1 * x1) % p
    b_val = (y1 * y1) % p
    c_val = (2 * z1 * z1) % p

    d_val = (-a_val) % p  # a = -1 in this curve, so D = -A
    e_val = ((x1 + y1) * (x1 + y1) - a_val - b_val) % p
    g_val = (d_val + b_val) % p
    f_val = (g_val - c_val) % p
    h_val = (d_val - b_val) % p

    x3 = (e_val * f_val) % p
    y3 = (g_val * h_val) % p
    t3 = (e_val * h_val) % p
    z3 = (f_val * g_val) % p

    return (x3, y3, z3, t3)


def scalar_mult(s: int, pt: Point) -> Point:
    """Multiply a point by a scalar using double-and-add.

    This is the elliptic curve equivalent of exponentiation. We scan the bits
    of the scalar from high to low:

        result = identity
        for each bit of s (from MSB to LSB):
            result = double(result)
            if bit == 1:
                result = add(result, pt)

    This is O(log s) doublings and at most O(log s) additions.

    For a 253-bit scalar (Ed25519's subgroup order), this takes about 253
    doublings and ~126 additions on average.

    WARNING: This implementation is NOT constant-time. A production
    implementation would use a fixed-window or Montgomery ladder to prevent
    timing attacks. For educational purposes, the simple double-and-add is
    clearest.
    """
    if s == 0:
        return IDENTITY
    if s == 1:
        return pt

    # Find the highest set bit
    result = IDENTITY
    for i in range(s.bit_length() - 1, -1, -1):
        result = point_double(result)
        if (s >> i) & 1:
            result = point_add(result, pt)

    return result


# ═══════════════════════════════════════════════════════════════════════════════
# SECTION 4: POINT ENCODING AND DECODING
# ═══════════════════════════════════════════════════════════════════════════════
#
# Ed25519 uses a compact 32-byte encoding for points. Only the y-coordinate
# is stored (since x can be recovered from y using the curve equation), plus
# one bit indicating the "sign" (parity) of x.
#
# This is analogous to point compression in other EC systems, but Ed25519
# uses it as the only encoding format.
#
# Encoding:
#   1. Convert from projective to affine: x = X*inv(Z), y = Y*inv(Z)
#   2. Encode y as 32 bytes, little-endian
#   3. Set the high bit of the last byte to the low bit of x (the "sign")
#
# Decoding:
#   1. Read the sign bit from the high bit of byte[31], then clear it
#   2. Decode y from the remaining 32 bytes (little-endian)
#   3. Compute x^2 from the curve equation: x^2 = (y^2-1) / (d*y^2+1)
#   4. Compute x = sqrt(x^2)
#   5. If x's parity doesn't match the sign bit, negate x


def point_encode(pt: Point) -> bytes:
    """Encode a curve point as 32 bytes per RFC 8032.

    The encoding stores the y-coordinate in little-endian format with the
    sign (low bit) of x packed into the high bit of the last byte. This
    gives us a compact 32-byte representation of a 64-byte (x, y) point.

    Returns:
        32 bytes representing the compressed point
    """
    x1, y1, z1, _t1 = pt

    # Convert from projective (X, Y, Z, T) to affine (x, y)
    z_inv = field_inv(z1)
    x_aff = (x1 * z_inv) % p
    y_aff = (y1 * z_inv) % p

    # Encode y as 32 bytes, little-endian
    encoded = bytearray(y_aff.to_bytes(32, "little"))

    # Pack the sign of x into the high bit of the last byte
    # The "sign" is the low bit (parity) of x
    encoded[31] |= (x_aff & 1) << 7

    return bytes(encoded)


def point_decode(data: bytes) -> Point:
    """Decode a 32-byte compressed point per RFC 8032.

    This reverses the encoding process:
      1. Extract the sign bit (high bit of last byte)
      2. Decode y (clearing the sign bit first)
      3. Recover x from the curve equation
      4. Adjust x's sign if needed

    Raises:
        ValueError: if the encoding is invalid (y >= p, no square root, etc.)
    """
    if len(data) != 32:
        msg = f"point encoding must be 32 bytes, got {len(data)}"
        raise ValueError(msg)

    # Step 1: Extract the sign bit from the high bit of byte[31]
    sign = (data[31] >> 7) & 1

    # Step 2: Clear the sign bit and decode y as little-endian integer
    y_bytes = bytearray(data)
    y_bytes[31] &= 0x7F  # clear high bit
    y_val = int.from_bytes(y_bytes, "little")

    # Reject if y >= p (non-canonical encoding)
    if y_val >= p:
        msg = "y coordinate out of range"
        raise ValueError(msg)

    # Step 3: Recover x^2 from the curve equation
    #
    # The curve is: -x^2 + y^2 = 1 + d*x^2*y^2
    # Rearranging:  x^2*(-1 - d*y^2) = 1 - y^2
    #               x^2 = (y^2 - 1) / (d*y^2 + 1)   (negating both sides)
    #
    # Note: d*y^2 + 1 is never zero mod p because -1/d is not a quadratic
    # residue.
    y_sq = (y_val * y_val) % p
    x_sq = ((y_sq - 1) * field_inv((d * y_sq + 1) % p)) % p

    # Step 4: Compute x = sqrt(x^2)
    if x_sq == 0:
        if sign == 1:
            msg = "invalid point: x=0 but sign bit is set"
            raise ValueError(msg)
        return (0, y_val, 1, 0)

    x_val = field_sqrt(x_sq)

    # Step 5: Ensure x has the correct sign (parity)
    if (x_val & 1) != sign:
        x_val = p - x_val

    # Return the point in extended coordinates
    return (x_val, y_val, 1, (x_val * y_val) % p)


# ═══════════════════════════════════════════════════════════════════════════════
# SECTION 5: CLAMPING
# ═══════════════════════════════════════════════════════════════════════════════
#
# "Clamping" modifies the raw hash bytes to produce a safe scalar for use
# as a secret key. Three bits are forced:
#
#   - Clear the 3 lowest bits of byte[0]: forces the scalar to be a multiple
#     of 8, which ensures the public key is in the prime-order subgroup (not
#     a small-order point). This prevents small-subgroup attacks.
#
#   - Clear bit 255 (high bit of byte[31]): ensures the scalar is < 2^255,
#     which avoids reduction artifacts.
#
#   - Set bit 254 (second-highest bit of byte[31]): ensures the scalar is
#     >= 2^254, giving a constant number of bits. This prevents timing attacks
#     based on scalar length.


def _clamp(h: bytes) -> int:
    """Clamp the first 32 bytes of a SHA-512 hash to produce a valid scalar.

    Returns the clamped value as an integer.
    """
    raw = bytearray(h[:32])
    raw[0] &= 248    # clear 3 low bits: scalar is multiple of 8
    raw[31] &= 127   # clear high bit: scalar < 2^255
    raw[31] |= 64    # set bit 254: scalar >= 2^254
    return int.from_bytes(raw, "little")


# ═══════════════════════════════════════════════════════════════════════════════
# SECTION 6: PUBLIC API -- KEY GENERATION, SIGNING, VERIFICATION
# ═══════════════════════════════════════════════════════════════════════════════


def generate_keypair(seed: bytes) -> tuple[bytes, bytes]:
    """Generate an Ed25519 key pair from a 32-byte seed.

    The seed is the "master secret" -- it should be generated from a
    cryptographically secure random source. The seed is hashed with SHA-512
    to produce both the secret scalar and a prefix used for deterministic
    nonce generation.

    The secret key returned is 64 bytes: seed || public_key. This format
    follows RFC 8032 and allows both signing (which needs the seed) and
    extracting the public key (bytes 32..64) from a single value.

    Args:
        seed: 32 bytes of cryptographic randomness

    Returns:
        (public_key, secret_key) where:
          - public_key is 32 bytes (the encoded curve point A = a*B)
          - secret_key is 64 bytes (seed || public_key)

    Raises:
        ValueError: if seed is not exactly 32 bytes
    """
    if len(seed) != 32:
        msg = f"seed must be 32 bytes, got {len(seed)}"
        raise ValueError(msg)

    # Step 1: Hash the seed with SHA-512 to get 64 bytes
    h = sha512(seed)

    # Step 2: Clamp the first 32 bytes to get the secret scalar
    a = _clamp(h)

    # Step 3: Compute the public key as A = a * B
    big_a = scalar_mult(a, B)
    public_key = point_encode(big_a)

    # Step 4: Assemble the secret key as seed || public_key
    secret_key = seed + public_key

    return (public_key, secret_key)


def sign(message: bytes, secret_key: bytes) -> bytes:
    """Sign a message with an Ed25519 secret key.

    Ed25519 signing is deterministic -- the same message and key always produce
    the same signature. This is achieved by deriving the nonce from a hash of
    the secret key prefix and message, rather than from a random source.

    Deterministic signing prevents catastrophic nonce reuse. In ECDSA, reusing
    a random nonce with two different messages leaks the secret key (this is
    how Sony's PS3 signing key was extracted). Ed25519 is immune to this.

    The signature is 64 bytes: R (32 bytes, an encoded curve point) followed
    by S (32 bytes, a scalar mod L).

    Args:
        message: the message to sign (any length)
        secret_key: 64 bytes (seed || public_key), as returned by generate_keypair

    Returns:
        64 bytes: the Ed25519 signature (R || S)

    Raises:
        ValueError: if secret_key is not exactly 64 bytes
    """
    if len(secret_key) != 64:
        msg = f"secret key must be 64 bytes, got {len(secret_key)}"
        raise ValueError(msg)

    # Extract seed and public key from the secret key
    seed = secret_key[:32]
    public_key = secret_key[32:64]

    # Hash the seed to recover the clamped scalar and the prefix
    h = sha512(seed)
    a = _clamp(h)
    prefix = h[32:64]  # the upper 32 bytes, used for nonce generation

    # Step 1: Deterministic nonce
    #
    # r = SHA-512(prefix || message) mod L
    #
    # The prefix is unique to this key, and combined with the message gives
    # a per-message nonce that is deterministic but unpredictable to an
    # attacker without the secret key.
    r_hash = sha512(prefix + message)
    r = int.from_bytes(r_hash, "little") % L

    # Step 2: Compute R = r * B and encode it
    big_r = scalar_mult(r, B)
    r_bytes = point_encode(big_r)

    # Step 3: Compute challenge k = SHA-512(R || public_key || message) mod L
    #
    # This binds the signature to both the message and the public key,
    # preventing cross-key signature forgery.
    k_hash = sha512(r_bytes + public_key + message)
    k = int.from_bytes(k_hash, "little") % L

    # Step 4: Compute S = (r + k * a) mod L
    #
    # This is the core of the Schnorr-style proof: the signer demonstrates
    # knowledge of the secret scalar a without revealing it.
    s_val = (r + k * a) % L
    s_bytes = s_val.to_bytes(32, "little")

    # The signature is R || S (64 bytes total)
    return r_bytes + s_bytes


def verify(message: bytes, signature: bytes, public_key: bytes) -> bool:
    """Verify an Ed25519 signature.

    The verification equation is:

        S * B  ==  R + k * A

    where:
      - S and R are decoded from the signature
      - A is decoded from the public key
      - k = SHA-512(R || public_key || message) mod L

    This works because the signer computed S = r + k*a (mod L), so:
      S*B = (r + k*a)*B = r*B + k*a*B = R + k*A

    If any component was tampered with, the equation will not hold.

    Args:
        message: the message that was signed
        signature: 64 bytes (R || S)
        public_key: 32 bytes (the signer's public key)

    Returns:
        True if the signature is valid, False otherwise.
        Never raises exceptions for invalid signatures -- returns False instead.
    """
    # Validate input lengths
    if len(signature) != 64:
        return False
    if len(public_key) != 32:
        return False

    # Step 1: Split the signature into R (point) and S (scalar)
    r_bytes = signature[:32]
    s_bytes = signature[32:64]

    # Step 2: Decode S as a little-endian integer
    s_val = int.from_bytes(s_bytes, "little")

    # Reject if S >= L (non-canonical scalar)
    if s_val >= L:
        return False

    # Step 3: Decode R and A as curve points
    try:
        big_r = point_decode(r_bytes)
    except ValueError:
        return False

    try:
        big_a = point_decode(public_key)
    except ValueError:
        return False

    # Step 4: Recompute k = SHA-512(R || public_key || message) mod L
    k_hash = sha512(r_bytes + public_key + message)
    k = int.from_bytes(k_hash, "little") % L

    # Step 5: Check the verification equation: S*B == R + k*A
    #
    # Left side: S * B (scalar multiplication of the base point)
    lhs = scalar_mult(s_val, B)

    # Right side: R + k*A (scalar mult of public key, then add R)
    rhs = point_add(big_r, scalar_mult(k, big_a))

    # Compare in affine coordinates (convert from projective)
    # Two projective points (X1,Y1,Z1,T1) and (X2,Y2,Z2,T2) are equal iff:
    #   X1*Z2 == X2*Z1 and Y1*Z2 == Y2*Z1
    lx, ly, lz, _lt = lhs
    rx, ry, rz, _rt = rhs

    return (lx * rz - rx * lz) % p == 0 and (ly * rz - ry * lz) % p == 0
