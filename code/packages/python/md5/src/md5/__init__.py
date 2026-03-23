"""md5 — MD5 message digest algorithm (RFC 1321) from scratch.

What Is MD5?
============
MD5 (Message Digest 5) takes any sequence of bytes and produces a fixed-size
16-byte (128-bit) "fingerprint" called a digest. The same input always produces
the same digest. Change even one bit of input and the digest changes completely.

Created by Ron Rivest in 1991 as an improvement over MD4. Standardized in
RFC 1321. MD5 is cryptographically broken (collision attacks since 2004) and
should NOT be used for security purposes (digital signatures, password hashing,
TLS certificates). It remains valid for: non-security checksums, UUID v3, and
legacy systems that already use it.

How MD5 Differs From SHA-1
============================
The most important difference is byte order:

  Property     SHA-1       MD5
  ──────────   ─────────   ─────────────
  Output size  20 bytes    16 bytes
  State words  5 (H₀..H₄)  4 (A,B,C,D)
  Rounds       80          64
  Block size   512 bits    512 bits
  Word order   Big-endian  LITTLE-ENDIAN ← key difference!

Big-endian (SHA-1): most significant byte first.  0x0A0B0C0D → 0A 0B 0C 0D
Little-endian (MD5): LEAST significant byte first. 0x0A0B0C0D → 0D 0C 0B 0A

This is the #1 source of MD5 implementation bugs. Concretely:
- SHA-1 reads block words with struct.unpack(">16I", block)   (big-endian)
- MD5 reads block words with struct.unpack("<16I", block)     (LITTLE-endian)
- SHA-1 writes the final hash with struct.pack(">5I", ...)    (big-endian)
- MD5 writes the final hash with struct.pack("<4I", ...)      (LITTLE-endian)

The T-Table (64 Precomputed Constants)
=======================================
MD5 uses 64 constants T[1..64], one per round. Each is derived from the sine
function — a transcendental number with unpredictable bit patterns, ensuring no
hidden mathematical backdoor:

  T[i] = floor(abs(sin(i)) × 2^32)   for i = 1..64

Why sine? Because sin(i) for integer i produces pseudo-random values between
-1 and 1. Scaling by 2^32 and flooring gives a 32-bit integer. The pattern is
"obviously" derived from a standard mathematical function, which proves to
anyone that the constants were not chosen to weaken the algorithm.

Example:
  sin(1) ≈ 0.8414709848...
  abs(sin(1)) × 2^32 = 0.8414709848 × 4294967296 = 3614090360.02...
  floor(3614090360.02) = 3614090360 = 0xD76AA478 = T[1]

RFC 1321 Test Vectors
======================
  md5(b"")         → "d41d8cd98f00b204e9800998ecf8427e"
  md5(b"a")        → "0cc175b9c0f1b6a831c399e269772661"
  md5(b"abc")      → "900150983cd24fb0d6963f7d28e17f72"
  md5(b"message digest") → "f96b697d7cb7938d525a2f31aaf161d0"
"""

from __future__ import annotations

import math
import struct

__version__ = "0.1.0"

# ─── T-Table: 64 Constants Derived From Sine ─────────────────────────────────
#
# T[i] = floor(abs(sin(i)) × 2^32)  for i in 1..64
#
# These are computed once at module load time. The derivation from sin() is what
# makes them trustworthy "nothing up my sleeve" numbers.
#
# We index from 0 internally (T[0] = T[1] in the RFC), so T[i] in the code
# corresponds to T[i+1] in RFC 1321.

_T: list[int] = [int(abs(math.sin(i + 1)) * (2**32)) & 0xFFFFFFFF for i in range(64)]

# ─── Round Shift Amounts ──────────────────────────────────────────────────────
#
# Each of the 64 rounds has a specific left-rotation amount. These are arranged
# in 4 groups of 16. The pattern repeats: [7,12,17,22], [5,9,14,20], etc.
# The RFC provides these as fixed values — they were chosen empirically for
# good diffusion.

_S: list[int] = [
    7, 12, 17, 22,  7, 12, 17, 22,  7, 12, 17, 22,  7, 12, 17, 22,  # rounds 0–15
    5,  9, 14, 20,  5,  9, 14, 20,  5,  9, 14, 20,  5,  9, 14, 20,  # rounds 16–31
    4, 11, 16, 23,  4, 11, 16, 23,  4, 11, 16, 23,  4, 11, 16, 23,  # rounds 32–47
    6, 10, 15, 21,  6, 10, 15, 21,  6, 10, 15, 21,  6, 10, 15, 21,  # rounds 48–63
]

# ─── Initialization Constants ─────────────────────────────────────────────────
#
# Same "nothing up my sleeve" values as SHA-1's H₀..H₃ — both came from NIST
# around the same time. The pattern is 01234567, 89ABCDEF, ... reversed in
# bytes.
#
#   A = 0x67452301 → bytes 67 45 23 01 → reverse: 01 23 45 67
#   B = 0xEFCDAB89 → bytes EF CD AB 89 → reverse: 89 AB CD EF

_INIT: tuple[int, int, int, int] = (0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476)


# ─── Helper: Circular Left Shift ─────────────────────────────────────────────
#
# Same concept as SHA-1's ROTL — bits that fall off the left reappear on the
# right. SHA-1 rotates left; MD5 also rotates left.

def _rotl(n: int, x: int) -> int:
    """Circular left shift of x by n bits (32-bit words)."""
    return ((x << n) | (x >> (32 - n))) & 0xFFFFFFFF


# ─── Padding ──────────────────────────────────────────────────────────────────
#
# Almost identical to SHA-1's padding, except the length is appended as a
# 64-bit LITTLE-endian integer (the "Q<" format in struct), not big-endian.
#
# Rule:
#   1. Append 0x80 byte.
#   2. Append zeros until length ≡ 56 (mod 64).
#   3. Append original bit length as a 64-bit LITTLE-endian integer.
#
# Example — "abc" (3 bytes = 24 bits):
#   61 62 63 80 [52 zero bytes] 18 00 00 00 00 00 00 00
#                               ^^
#   24 = 0x18, stored LITTLE-endian: 18 00 00 00 00 00 00 00

def _pad(data: bytes) -> bytes:
    """Pad data to a multiple of 64 bytes per RFC 1321 §3.1."""
    bit_len = len(data) * 8
    msg = bytearray(data) + bytearray([0x80])
    while len(msg) % 64 != 56:
        msg.append(0x00)
    msg += struct.pack("<Q", bit_len)  # ← LITTLE-ENDIAN (differs from SHA-1)
    return bytes(msg)


# ─── Compression Function ─────────────────────────────────────────────────────
#
# 64 rounds of mixing fold one 64-byte block into the four-word state.
#
# The block is parsed as 16 LITTLE-ENDIAN 32-bit words M[0..15]:
#   struct.unpack("<16I", block)
#
# Four stages of 16 rounds each, each with a different auxiliary function:
#
#   Stage  Rounds  f(B,C,D)                    Purpose
#   ─────  ──────  ──────────────────────────  ─────────────────
#     1    0–15    (B & C) | (~B & D)          Selector (same as SHA-1 rounds 0–19)
#     2    16–31   (D & B) | (~D & C)          Selector (B and D roles swapped)
#     3    32–47   B ^ C ^ D                   Parity
#     4    48–63   C ^ (B | ~D)                "I" function (unusual!)
#
# The "I" function in stage 4 is the most unusual. Let's see its truth table:
#
#   B  C  D  | ~D | B | ~D | C ^ (B | ~D)
#   ─────────────────────────────────────────
#   0  0  0  |  1 |  1 |  1   (1^0=1)   → 1
#   0  0  1  |  0 |  0 |  0   (0^0=0)   → 0
#   0  1  0  |  1 |  1 |  1   (1^1=0)   → 0  wait... 0^(0|1) = 0^1 = 1
#
# Actually the I function is C XOR (B OR NOT D):
#   When D=0 → ~D=1 → B|~D=1 → result = C^1 (flips C)
#   When D=1 → ~D=0 → B|~D=B → result = C^B (parity of B and C)
# It mixes differently from the F/G/H functions, increasing diffusion.
#
# Each round:
#   F = auxiliary(B,C,D) according to stage
#   A = B + ROTL(S[i], A + F + M[g] + T[i])   (mod 2^32)
#   shift: D=C, C=B, B=new_A, A=old_D
#
# The message word index g depends on the stage:
#   Stage 1: g = i          (sequential)
#   Stage 2: g = (5*i+1)%16 (skip by 5)
#   Stage 3: g = (3*i+5)%16 (skip by 3)
#   Stage 4: g = (7*i)%16   (skip by 7)
#
# Davies-Meyer feed-forward: after all 64 rounds, add the compressed output
# back to the input state to prevent invertibility.

def _compress(
    state: tuple[int, int, int, int], block: bytes
) -> tuple[int, int, int, int]:
    """Mix one 64-byte block into the four-word state via 64 rounds."""
    M = struct.unpack("<16I", block)  # LITTLE-endian word parsing
    a0, b0, c0, d0 = state
    a, b, c, d = a0, b0, c0, d0

    for i in range(64):
        if i < 16:
            # Selector: if b=1 → c, if b=0 → d
            f = (b & c) | ((~b) & d)
            g = i
        elif i < 32:
            # Selector with b/d roles swapped
            f = (d & b) | ((~d) & c)
            g = (5 * i + 1) % 16
        elif i < 48:
            # Parity: 1 if an odd number of inputs are 1
            f = b ^ c ^ d
            g = (3 * i + 5) % 16
        else:
            # I function: C XOR (B OR NOT D)
            f = c ^ (b | (~d))
            g = (7 * i) % 16

        f = f & 0xFFFFFFFF
        new_a = (b + _rotl(_S[i], (a + f + M[g] + _T[i]) & 0xFFFFFFFF)) & 0xFFFFFFFF
        a, b, c, d = d, new_a, b, c

    return (
        (a0 + a) & 0xFFFFFFFF,
        (b0 + b) & 0xFFFFFFFF,
        (c0 + c) & 0xFFFFFFFF,
        (d0 + d) & 0xFFFFFFFF,
    )


# ─── Public API ───────────────────────────────────────────────────────────────


def md5(data: bytes) -> bytes:
    """Compute the MD5 digest of data. Returns 16 bytes.

    This is the one-shot API: hash a complete message in a single call.

    NOTE: MD5 is cryptographically broken. Do NOT use for passwords, digital
    signatures, or security-sensitive checksums. Use for UUID v3 or legacy
    compatibility only.

    Example::

        >>> md5(b"abc").hex()
        '900150983cd24fb0d6963f7d28e17f72'
        >>> md5(b"").hex()
        'd41d8cd98f00b204e9800998ecf8427e'
    """
    padded = _pad(data)
    state = _INIT
    for offset in range(0, len(padded), 64):
        state = _compress(state, padded[offset : offset + 64])
    # Finalize: concatenate the four state words as LITTLE-endian 32-bit integers.
    # "<4I" = four little-endian unsigned 32-bit integers = 16 bytes total.
    return struct.pack("<4I", *state)


def md5_hex(data: bytes) -> str:
    """Compute MD5 and return the 32-character lowercase hex string.

    Example::

        >>> md5_hex(b"abc")
        '900150983cd24fb0d6963f7d28e17f72'
    """
    return md5(data).hex()


class MD5:
    """Streaming MD5 hasher that accepts data in multiple chunks.

    Useful when the full message is not available at once.

    The interface mirrors Python's hashlib API::

        h = MD5()
        h.update(b"ab")
        h.update(b"c")
        h.hexdigest()   # → '900150983cd24fb0d6963f7d28e17f72'

    Multiple update() calls are equivalent to a single md5(all_data).
    """

    def __init__(self) -> None:
        self._state: tuple[int, int, int, int] = _INIT
        self._buffer: bytearray = bytearray()
        self._byte_count: int = 0

    def update(self, data: bytes) -> "MD5":
        """Feed more bytes into the hash. Returns self for chaining."""
        self._buffer.extend(data)
        self._byte_count += len(data)
        while len(self._buffer) >= 64:
            self._state = _compress(self._state, bytes(self._buffer[:64]))
            self._buffer = self._buffer[64:]
        return self

    def digest(self) -> bytes:
        """Return the 16-byte digest of all data fed so far.

        Non-destructive: calling digest() twice returns the same bytes.
        """
        bit_len = self._byte_count * 8
        tail = bytearray(self._buffer) + bytearray([0x80])
        while len(tail) % 64 != 56:
            tail.append(0x00)
        tail += struct.pack("<Q", bit_len)  # ← LITTLE-ENDIAN length
        state = self._state
        for offset in range(0, len(tail), 64):
            state = _compress(state, bytes(tail[offset : offset + 64]))
        return struct.pack("<4I", *state)

    def hexdigest(self) -> str:
        """Return the 32-character hex string of the digest."""
        return self.digest().hex()

    def copy(self) -> "MD5":
        """Return an independent copy of the current hasher state."""
        other = MD5()
        other._state = self._state
        other._buffer = bytearray(self._buffer)
        other._byte_count = self._byte_count
        return other
