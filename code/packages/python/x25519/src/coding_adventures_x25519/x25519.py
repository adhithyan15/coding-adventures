"""
X25519 — Elliptic Curve Diffie-Hellman on Curve25519 (RFC 7748)
================================================================

X25519 is one of the most widely deployed key-agreement protocols on the
internet.  Every TLS 1.3 handshake you make (HTTPS, SSH, Signal, WireGuard)
almost certainly uses it.

The beauty of X25519 lies in its simplicity:

    shared_secret = x25519(my_private_key, your_public_key)

Both parties compute the same 32-byte shared secret, yet an eavesdropper
who sees both public keys cannot derive it.  This is the Diffie-Hellman
problem on an elliptic curve.

How it works — a bird's-eye view
---------------------------------

1. **The prime field GF(p)** — All arithmetic is modulo p = 2^255 - 19.
   This is a Mersenne-like prime chosen by Dan Bernstein because it makes
   reduction extremely fast (subtracting 19 instead of a huge modulus).

2. **Curve25519** — The elliptic curve y^2 = x^3 + 486662·x^2 + x over
   GF(p).  This is a *Montgomery curve*, which means we can do scalar
   multiplication using only the *x-coordinate* — we never need y at all.

3. **The Montgomery ladder** — A constant-time algorithm that computes
   k·P (scalar multiplication) by walking the bits of k from high to low,
   maintaining two running points and conditionally swapping them.

4. **Scalar clamping** — The private key is "clamped" to ensure it has
   specific bit patterns that guarantee security (cofactor clearing) and
   consistent key length.

Why constant-time matters
--------------------------

Every branch, every early-return, every data-dependent array index can
leak information through timing.  An attacker measuring how long your
function takes can reconstruct your private key bit by bit.  The
Montgomery ladder with conditional swap (cswap) ensures every execution
follows the exact same code path regardless of the key bits.

In this educational implementation, Python's arbitrary-precision integers
are NOT truly constant-time at the hardware level, but the *algorithm*
is structured correctly.  A production implementation would use fixed-width
limbs and assembly.
"""

from __future__ import annotations


# ============================================================================
# The Prime Field GF(2^255 - 19)
# ============================================================================
#
# All arithmetic in X25519 happens modulo this prime.  Why 2^255 - 19?
#
# - 2^255 - 19 is the largest prime less than 2^255.
# - Being close to a power of 2 makes modular reduction fast: instead of
#   a generic Barrett or Montgomery reduction, we can exploit the fact
#   that 2^255 ≡ 19 (mod p).
# - 255 bits fits neatly in 32 bytes with one bit to spare (the high bit
#   is always masked off).

P: int = (1 << 255) - 19
"""The prime modulus for Curve25519's field: 2^255 - 19."""

A24: int = 121666
"""
The constant (A + 2) / 4 where A = 486662 is the Montgomery curve parameter.

Curve25519: y^2 = x^3 + 486662·x^2 + x

In the Montgomery ladder, we need (A + 2) / 4 = (486662 + 2) / 4 = 121666.
This constant appears in the step where we compute:

    z_2 = E * (AA + a24 * E)

where E = AA - BB captures the "difference" information needed to stay on
the curve.
"""


# ============================================================================
# Field Arithmetic
# ============================================================================
#
# These are the building blocks.  Every higher-level operation reduces to
# sequences of these five operations.  In Python we can use native big
# integers and the % operator; in C or Rust you'd implement these with
# fixed-width limb arrays.


def field_add(a: int, b: int) -> int:
    """Add two field elements: (a + b) mod p.

    Addition in a prime field is just regular addition followed by reduction.
    If the sum exceeds p, we subtract p to bring it back into range [0, p).
    """
    return (a + b) % P


def field_sub(a: int, b: int) -> int:
    """Subtract two field elements: (a - b) mod p.

    If a < b, the result would be negative, but Python's % always returns
    a non-negative result for positive modulus, so (a - b) % p correctly
    wraps around.
    """
    return (a - b) % P


def field_mul(a: int, b: int) -> int:
    """Multiply two field elements: (a * b) mod p.

    The intermediate product can be up to (p-1)^2 ≈ 2^510, which is about
    510 bits.  Python handles this natively with arbitrary-precision ints.
    """
    return (a * b) % P


def field_square(a: int) -> int:
    """Square a field element: a^2 mod p.

    Squaring is separated from general multiplication because it can be
    optimized: in limbed arithmetic, many cross-terms are doubled rather
    than computed separately.  In Python, it's the same as field_mul(a, a).
    """
    return (a * a) % P


def field_invert(a: int) -> int:
    """Compute the multiplicative inverse: a^(-1) mod p.

    Uses Fermat's little theorem: for prime p, a^(p-1) ≡ 1 (mod p),
    therefore a^(p-2) ≡ a^(-1) (mod p).

    This is elegant but not the fastest approach.  A production implementation
    might use the extended Euclidean algorithm or a specialized addition chain.
    Python's built-in pow(a, p-2, p) uses efficient binary exponentiation
    (square-and-multiply), making this O(log p) ≈ 255 squarings and ~128
    multiplications.
    """
    return pow(a, P - 2, P)


# ============================================================================
# Constant-Time Conditional Swap
# ============================================================================


def cswap(swap: int, x_2: int, x_3: int) -> tuple[int, int]:
    """Conditionally swap two values in constant time.

    If swap == 1, return (x_3, x_2).
    If swap == 0, return (x_2, x_3).

    In a production implementation, this would use bitwise masking:

        mask = -swap  (all 1s if swap=1, all 0s if swap=0)
        dummy = mask & (x_2 ^ x_3)
        x_2 ^= dummy
        x_3 ^= dummy

    This avoids any branch, making the swap invisible to timing analysis.
    Our Python implementation uses the same technique, even though Python's
    big integers aren't truly constant-time at the hardware level.

    Parameters
    ----------
    swap : int
        0 or 1 — whether to swap.
    x_2, x_3 : int
        The two field elements to conditionally swap.

    Returns
    -------
    tuple[int, int]
        (x_2, x_3) if swap == 0, or (x_3, x_2) if swap == 1.
    """
    # Create a mask: if swap is 1, mask is all-ones (in 2's complement).
    # For arbitrary-precision Python ints, we just use the XOR trick:
    # dummy captures the difference between x_2 and x_3 only when swap == 1.
    dummy = swap * (x_2 ^ x_3)
    x_2 ^= dummy
    x_3 ^= dummy
    return x_2, x_3


# ============================================================================
# Encoding and Decoding
# ============================================================================
#
# X25519 uses little-endian byte encoding for both scalars and u-coordinates.
# A 32-byte array represents a 256-bit integer with the least significant
# byte first.


def decode_u_coordinate(u_bytes: bytes) -> int:
    """Decode a u-coordinate from 32 bytes (little-endian).

    Per RFC 7748 Section 5, the high bit of the last byte is masked off.
    This ensures the decoded value is at most 2^255 - 1, which is within
    the range needed for the field.

    Why mask the high bit?
    ----------------------
    The field prime p = 2^255 - 19 requires only 255 bits.  Byte 31 (the
    most significant byte) contributes bits 248-255.  Bit 255 is unused
    for valid field elements, so we clear it to canonicalize the input.
    """
    if len(u_bytes) != 32:
        msg = f"u-coordinate must be exactly 32 bytes, got {len(u_bytes)}"
        raise ValueError(msg)

    # Mask off the high bit of byte 31
    u_list = bytearray(u_bytes)
    u_list[31] &= 0x7F  # 0x7F = 0b01111111 — clears bit 7 of byte 31

    return int.from_bytes(u_list, byteorder="little")


def decode_scalar(k_bytes: bytes) -> int:
    """Decode and clamp a scalar (private key) from 32 bytes.

    Clamping performs three bit manipulations:

    1. k[0] &= 248 (0xF8)  — Clear the three lowest bits.
       This ensures the scalar is a multiple of 8, which "clears the
       cofactor."  Curve25519 has cofactor h = 8, meaning there are 8
       points of small order.  By making k divisible by 8, we guarantee
       that k·P lands in the prime-order subgroup, preventing small
       subgroup attacks.

    2. k[31] &= 127 (0x7F) — Clear the highest bit (bit 255).
       This ensures the scalar fits in 255 bits, matching the field size.

    3. k[31] |= 64 (0x40)  — Set bit 254.
       This ensures all scalars have the same bit length, which means the
       Montgomery ladder always performs exactly 255 iterations.  Without
       this, short scalars would finish early, leaking information about
       the key length through timing.
    """
    if len(k_bytes) != 32:
        msg = f"Scalar must be exactly 32 bytes, got {len(k_bytes)}"
        raise ValueError(msg)

    k_list = bytearray(k_bytes)

    # Clear the three lowest bits — make k a multiple of 8 (cofactor clearing)
    k_list[0] &= 248

    # Clear bit 255 — keep k in [0, 2^255)
    k_list[31] &= 127

    # Set bit 254 — ensure constant-time execution (fixed bit length)
    k_list[31] |= 64

    return int.from_bytes(k_list, byteorder="little")


def encode_u_coordinate(u: int) -> bytes:
    """Encode a field element as 32 bytes (little-endian).

    The value is first reduced mod p to ensure canonical encoding — there's
    exactly one byte representation for each field element.
    """
    u = u % P
    return u.to_bytes(32, byteorder="little")


# ============================================================================
# The Montgomery Ladder — The Heart of X25519
# ============================================================================
#
# The Montgomery ladder computes scalar multiplication k·u on Curve25519
# using only the x-coordinate (called "u" in Montgomery form).
#
# It maintains two points throughout:
#   - (x_2, z_2) — the "main" accumulator
#   - (x_3, z_3) — always one step ahead
#
# These are in *projective coordinates*, meaning the actual x-coordinate
# is x/z rather than x alone.  This avoids expensive field inversions
# during the loop (we only invert once at the very end).
#
# On each iteration:
#   1. Look at the current bit of k
#   2. Conditionally swap the two points (based on XOR of current and
#      previous bits)
#   3. Perform a "differential addition" and "doubling" step
#   4. The invariant is maintained: (x_3, z_3) = (x_2, z_2) + (x_1, z_1)
#
# After all 255 bits, convert from projective to affine: result = x_2 / z_2.
#
# Diagram of one ladder step:
#
#     Before:  P2 = (x_2 : z_2),  P3 = (x_3 : z_3),  P1 = base point
#
#     A = x_2 + z_2          C = x_3 + z_3
#     B = x_2 - z_2          D = x_3 - z_3
#     AA = A^2                DA = D * A
#     BB = B^2                CB = C * B
#     E = AA - BB
#
#     New x_2 = AA * BB           New x_3 = (DA + CB)^2
#     New z_2 = E * (BB + a24*E)  New z_3 = x_1 * (DA - CB)^2
#
#     After:  P2' = 2·P2,  P3' = P2 + P3  (differential addition)


def x25519(k_bytes: bytes, u_bytes: bytes) -> bytes:
    """Compute the X25519 function: scalar multiplication on Curve25519.

    This is the core primitive.  Given a 32-byte scalar (private key) and
    a 32-byte u-coordinate (public key or base point), it returns the
    32-byte u-coordinate of the resulting point.

    Parameters
    ----------
    k_bytes : bytes
        The 32-byte scalar (will be clamped).
    u_bytes : bytes
        The 32-byte u-coordinate of the input point.

    Returns
    -------
    bytes
        The 32-byte u-coordinate of k·u.

    Raises
    ------
    ValueError
        If the result is the all-zeros point (point at infinity), which
        indicates a malicious or degenerate input.
    """
    k = decode_scalar(k_bytes)
    u = decode_u_coordinate(u_bytes)

    # ---- Initialize the Montgomery ladder ----
    #
    # We start with:
    #   P2 = (1 : 0) — the point at infinity (identity element)
    #   P3 = (u : 1) — the input point
    #
    # After the ladder, P2 will hold k·(u), and P3 will hold (k+1)·(u).
    x_1 = u
    x_2 = 1
    z_2 = 0
    x_3 = u
    z_3 = 1

    swap = 0

    # ---- Main loop: iterate from bit 254 down to bit 0 ----
    #
    # Why start at bit 254 and not 255?  Because clamping set bit 254
    # (the highest meaningful bit) and cleared bit 255.  So the effective
    # scalar always has exactly 255 bits with bit 254 = 1.
    for t in range(254, -1, -1):
        # Extract bit t of the scalar k
        k_t = (k >> t) & 1

        # XOR with previous swap value.  This is the "differential" part:
        # we only swap when the current bit differs from the previous one.
        swap ^= k_t

        # Conditionally swap P2 and P3
        x_2, x_3 = cswap(swap, x_2, x_3)
        z_2, z_3 = cswap(swap, z_2, z_3)

        # Remember the current bit for next iteration
        swap = k_t

        # ---- Montgomery ladder step ----
        #
        # This simultaneously computes:
        #   P2 ← 2·P2          (point doubling)
        #   P3 ← P2 + P3       (differential addition using P1 = base)

        # --- Doubling side (P2) ---
        a = field_add(x_2, z_2)      # A = x_2 + z_2
        aa = field_square(a)          # AA = A^2
        b = field_sub(x_2, z_2)      # B = x_2 - z_2
        bb = field_square(b)          # BB = B^2
        e = field_sub(aa, bb)         # E = AA - BB = (x+z)^2 - (x-z)^2 = 4xz

        # --- Addition side (P3) ---
        c = field_add(x_3, z_3)      # C = x_3 + z_3
        d = field_sub(x_3, z_3)      # D = x_3 - z_3
        da = field_mul(d, a)          # DA = D * A
        cb = field_mul(c, b)          # CB = C * B

        # New P3 (addition result)
        x_3 = field_square(field_add(da, cb))           # x_3 = (DA + CB)^2
        z_3 = field_mul(x_1, field_square(field_sub(da, cb)))  # z_3 = x_1 * (DA - CB)^2

        # New P2 (doubling result)
        #
        # The doubling formula for Z comes from expanding:
        #   Z_{2n} = 4·x·z · (x^2 + A·x·z + z^2)
        #
        # Rewriting x^2 + A·x·z + z^2:
        #   = (x - z)^2 + (A + 2)·x·z
        #   = BB + ((A + 2)/4) · E       where E = 4·x·z
        #
        # So: z_2 = E · (BB + a24 · E)  with a24 = (A + 2)/4 = 121666
        x_2 = field_mul(aa, bb)                         # x_2 = AA * BB
        z_2 = field_mul(e, field_add(bb, field_mul(A24, e)))  # z_2 = E * (BB + a24 * E)

    # ---- Final conditional swap ----
    # Undo the last swap to get the correct result in P2
    x_2, x_3 = cswap(swap, x_2, x_3)
    z_2, z_3 = cswap(swap, z_2, z_3)

    # ---- Convert from projective to affine coordinates ----
    #
    # The actual u-coordinate is x_2 / z_2 in the field.
    # Division in GF(p) is multiplication by the inverse.
    result = field_mul(x_2, field_invert(z_2))
    result_bytes = encode_u_coordinate(result)

    # Check for the all-zeros result (point at infinity)
    if result_bytes == b"\x00" * 32:
        msg = "X25519 produced the all-zeros output (low-order point)"
        raise ValueError(msg)

    return result_bytes


# ============================================================================
# High-Level API
# ============================================================================


# The base point for Curve25519 is u = 9, encoded as 32 bytes (little-endian).
# This is a generator of the prime-order subgroup.
#
# Why 9?  Bernstein chose this as the smallest valid u-coordinate that
# generates the full prime-order subgroup.  It's a matter of convention —
# any generator would work, but 9 is simple and memorable.
BASE_POINT: bytes = (9).to_bytes(32, byteorder="little")


def x25519_base(scalar_bytes: bytes) -> bytes:
    """Compute scalar multiplication with the standard base point (u = 9).

    This is equivalent to x25519(scalar, base_point) and is the standard
    way to derive a public key from a private key.

    Parameters
    ----------
    scalar_bytes : bytes
        The 32-byte private key.

    Returns
    -------
    bytes
        The 32-byte public key (u-coordinate).
    """
    return x25519(scalar_bytes, BASE_POINT)


def generate_keypair(private_key: bytes) -> bytes:
    """Generate a public key from a private key.

    This is simply x25519_base — included for API clarity.
    In practice, the private key should be 32 bytes of cryptographically
    secure random data (e.g., from os.urandom(32)).

    Parameters
    ----------
    private_key : bytes
        32 bytes of random data.

    Returns
    -------
    bytes
        The 32-byte public key.

    Example
    -------
    >>> import os
    >>> private = os.urandom(32)
    >>> public = generate_keypair(private)
    >>> len(public)
    32
    """
    return x25519_base(private_key)
