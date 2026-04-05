"""sha512 -- SHA-512 cryptographic hash function (FIPS 180-4) from scratch.

What Is SHA-512?
================
SHA-512 is the 64-bit sibling of SHA-256 in the SHA-2 family.  It takes any
sequence of bytes and produces a fixed-size 64-byte (512-bit) digest.  The
same input always produces the same digest.  Change even one bit and the
output changes completely -- the "avalanche effect".

On 64-bit hardware SHA-512 is often *faster* than SHA-256 because it
processes data in 128-byte blocks using native 64-bit arithmetic, so each
block covers twice as much data.

How It Differs From SHA-256
===========================
SHA-512 shares the same Merkle-Damgard structure as SHA-256, but everything
is wider:

  Property          SHA-256      SHA-512
  ───────────────   ────────     ────────
  Word size         32-bit       64-bit
  State words       8 x 32-bit   8 x 64-bit
  Block size        64 bytes     128 bytes
  Rounds            64           80
  Round constants   64 (32-bit)  80 (64-bit)
  Length field       64-bit       128-bit
  Digest size       32 bytes     64 bytes

The rotation amounts also differ -- they are tuned for 64-bit words:

  SHA-256                              SHA-512
  Sigma0 = ROTR(2)^ROTR(13)^ROTR(22)   Sigma0 = ROTR(28)^ROTR(34)^ROTR(39)
  Sigma1 = ROTR(6)^ROTR(11)^ROTR(25)   Sigma1 = ROTR(14)^ROTR(18)^ROTR(41)
  sigma0 = ROTR(7)^ROTR(18)^SHR(3)     sigma0 = ROTR(1)^ROTR(8)^SHR(7)
  sigma1 = ROTR(17)^ROTR(19)^SHR(10)   sigma1 = ROTR(19)^ROTR(61)^SHR(6)

The Big Picture -- Merkle-Damgard Construction
==============================================
  Input message (any length)
       |
       v  pad to a multiple of 1024 bits (128 bytes)
  +----------+----------+----------+
  |  block_0 |  block_1 |   ...    |   (each 1024 bits = 128 bytes)
  +----------+----------+----------+
       |           |
       v           v
  [H0..H7] --> compress --> compress --> ... --> 64-byte digest

The state is eight 64-bit words (H0..H7).  Each 128-byte block is
"compressed" into the state through 80 rounds of bit mixing.  The final
state, serialized as big-endian bytes, is the digest.

FIPS 180-4 reference test vectors:
  sha512(b"")    = cf83e1357eefb8bd...f927da3e  (128 hex chars)
  sha512(b"abc") = ddaf35a193617aba...a54ca49f  (128 hex chars)
"""

from __future__ import annotations

import struct

__version__ = "0.1.0"

# ---- Mask for 64-bit arithmetic ----
#
# Python integers have arbitrary precision, so after every addition we mask
# to 64 bits.  This constant makes the code self-documenting.
_MASK64 = 0xFFFFFFFFFFFFFFFF

# ---- Initial Hash Values (FIPS 180-4 section 5.3.5) ----
#
# These are the fractional parts of the square roots of the first eight
# primes (2, 3, 5, 7, 11, 13, 17, 19), truncated to 64 bits.
#
# They serve as "nothing up my sleeve" numbers -- the derivation from well-
# known mathematical constants proves there is no hidden backdoor.
_INIT: tuple[int, ...] = (
    0x6A09E667F3BCC908,  # frac(sqrt(2))
    0xBB67AE8584CAA73B,  # frac(sqrt(3))
    0x3C6EF372FE94F82B,  # frac(sqrt(5))
    0xA54FF53A5F1D36F1,  # frac(sqrt(7))
    0x510E527FADE682D1,  # frac(sqrt(11))
    0x9B05688C2B3E6C1F,  # frac(sqrt(13))
    0x1F83D9ABFB41BD6B,  # frac(sqrt(17))
    0x5BE0CD19137E2179,  # frac(sqrt(19))
)

# ---- Round Constants (FIPS 180-4 section 4.2.3) ----
#
# 80 constants, one per round, derived from the fractional parts of the
# cube roots of the first 80 primes (2..409).  Truncated to 64 bits.
#
# Like the initial hash values these are "nothing up my sleeve" numbers.
_K: tuple[int, ...] = (
    0x428A2F98D728AE22, 0x7137449123EF65CD, 0xB5C0FBCFEC4D3B2F, 0xE9B5DBA58189DBBC,
    0x3956C25BF348B538, 0x59F111F1B605D019, 0x923F82A4AF194F9B, 0xAB1C5ED5DA6D8118,
    0xD807AA98A3030242, 0x12835B0145706FBE, 0x243185BE4EE4B28C, 0x550C7DC3D5FFB4E2,
    0x72BE5D74F27B896F, 0x80DEB1FE3B1696B1, 0x9BDC06A725C71235, 0xC19BF174CF692694,
    0xE49B69C19EF14AD2, 0xEFBE4786384F25E3, 0x0FC19DC68B8CD5B5, 0x240CA1CC77AC9C65,
    0x2DE92C6F592B0275, 0x4A7484AA6EA6E483, 0x5CB0A9DCBD41FBD4, 0x76F988DA831153B5,
    0x983E5152EE66DFAB, 0xA831C66D2DB43210, 0xB00327C898FB213F, 0xBF597FC7BEEF0EE4,
    0xC6E00BF33DA88FC2, 0xD5A79147930AA725, 0x06CA6351E003826F, 0x142929670A0E6E70,
    0x27B70A8546D22FFC, 0x2E1B21385C26C926, 0x4D2C6DFC5AC42AED, 0x53380D139D95B3DF,
    0x650A73548BAF63DE, 0x766A0ABB3C77B2A8, 0x81C2C92E47EDAEE6, 0x92722C851482353B,
    0xA2BFE8A14CF10364, 0xA81A664BBC423001, 0xC24B8B70D0F89791, 0xC76C51A30654BE30,
    0xD192E819D6EF5218, 0xD69906245565A910, 0xF40E35855771202A, 0x106AA07032BBD1B8,
    0x19A4C116B8D2D0C8, 0x1E376C085141AB53, 0x2748774CDF8EEB99, 0x34B0BCB5E19B48A8,
    0x391C0CB3C5C95A63, 0x4ED8AA4AE3418ACB, 0x5B9CCA4F7763E373, 0x682E6FF3D6B2B8A3,
    0x748F82EE5DEFB2FC, 0x78A5636F43172F60, 0x84C87814A1F0AB72, 0x8CC702081A6439EC,
    0x90BEFFFA23631E28, 0xA4506CEBDE82BDE9, 0xBEF9A3F7B2C67915, 0xC67178F2E372532B,
    0xCA273ECEEA26619C, 0xD186B8C721C0C207, 0xEADA7DD6CDE0EB1E, 0xF57D4F7FEE6ED178,
    0x06F067AA72176FBA, 0x0A637DC5A2C898A6, 0x113F9804BEF90DAE, 0x1B710B35131C471B,
    0x28DB77F523047D84, 0x32CAAB7B40C72493, 0x3C9EBE0A15C9BEBC, 0x431D67C49C100D4C,
    0x4CC5D4BECB3E42B6, 0x597F299CFC657E2A, 0x5FCB6FAB3AD6FAEC, 0x6C44198C4A475817,
)


# ---- Bitwise Helpers ----
#
# SHA-512 uses right-rotations (ROTR) and right-shifts (SHR) on 64-bit words.
# Unlike SHA-1 which uses left-rotations, SHA-2 family (both 256 and 512) use
# right-rotations.

def _rotr(n: int, x: int) -> int:
    """Circular right shift of x by n bits within a 64-bit word.

    Bits that "fall off" the right end wrap around to the left side.  This is
    the inverse of a left-rotation.

    Example with n=2, x=0b01101001 (8-bit for clarity):
      Regular shift right:  01101001 >> 2 = 00011010  (01 on right is lost)
      Circular right:       01101001 ROTR 2 = 01011010  (01 wraps to top)

    We mask to 64 bits because Python integers have arbitrary precision.
    """
    return ((x >> n) | (x << (64 - n))) & _MASK64


def _shr(n: int, x: int) -> int:
    """Logical right shift of x by n bits (zero-fill from the left).

    Unlike ROTR, bits that fall off the right are simply discarded.
    """
    return (x >> n) & _MASK64


# ---- Sigma Functions ----
#
# SHA-512 defines four bitwise mixing functions.  The "big" Sigma functions
# (upper-case) operate on the working variables during compression.  The
# "small" sigma functions (lower-case) operate during message schedule
# expansion.
#
# Each function combines three rotated/shifted copies of the same word using
# XOR.  This creates a complex, non-linear bit pattern that is crucial for
# the avalanche effect -- flipping one input bit flips roughly half the
# output bits.

def _big_sigma0(x: int) -> int:
    """Upper-case Sigma_0(x) = ROTR(28,x) XOR ROTR(34,x) XOR ROTR(39,x).

    Used in the compression loop to mix the 'a' working variable.
    """
    return _rotr(28, x) ^ _rotr(34, x) ^ _rotr(39, x)


def _big_sigma1(x: int) -> int:
    """Upper-case Sigma_1(x) = ROTR(14,x) XOR ROTR(18,x) XOR ROTR(41,x).

    Used in the compression loop to mix the 'e' working variable.
    """
    return _rotr(14, x) ^ _rotr(18, x) ^ _rotr(41, x)


def _small_sigma0(x: int) -> int:
    """Lower-case sigma_0(x) = ROTR(1,x) XOR ROTR(8,x) XOR SHR(7,x).

    Used in the message schedule to expand the 16 input words to 80 words.
    """
    return _rotr(1, x) ^ _rotr(8, x) ^ _shr(7, x)


def _small_sigma1(x: int) -> int:
    """Lower-case sigma_1(x) = ROTR(19,x) XOR ROTR(61,x) XOR SHR(6,x).

    Used in the message schedule to expand the 16 input words to 80 words.
    """
    return _rotr(19, x) ^ _rotr(61, x) ^ _shr(6, x)


# ---- Logical Functions ----
#
# SHA-2 uses two standard Boolean functions during compression:
#
#   Ch(x,y,z)  = "Choice" -- for each bit position, x chooses between y and z.
#                If x_i=1 pick y_i, if x_i=0 pick z_i.
#
#   Maj(x,y,z) = "Majority" -- for each bit position, the output is 1 if
#                at least 2 of the 3 inputs are 1.
#
# Truth tables:
#   x y z | Ch(x,y,z)    x y z | Maj(x,y,z)
#   0 0 0 |     0        0 0 0 |     0
#   0 0 1 |     1        0 0 1 |     0
#   0 1 0 |     0        0 1 0 |     0
#   0 1 1 |     1        0 1 1 |     1
#   1 0 0 |     0        1 0 0 |     0
#   1 0 1 |     0        1 0 1 |     1
#   1 1 0 |     1        1 1 0 |     1
#   1 1 1 |     1        1 1 1 |     1

def _ch(x: int, y: int, z: int) -> int:
    """Choice: Ch(x,y,z) = (x AND y) XOR (NOT x AND z)."""
    return (x & y) ^ ((~x) & z) & _MASK64


def _maj(x: int, y: int, z: int) -> int:
    """Majority: Maj(x,y,z) = (x AND y) XOR (x AND z) XOR (y AND z)."""
    return (x & y) ^ (x & z) ^ (y & z)


# ---- Padding ----
#
# The compression function needs exactly 128-byte (1024-bit) blocks.  Padding
# extends the message per FIPS 180-4 section 5.1.2:
#
#   1. Append a single 0x80 byte (the '1' bit followed by seven '0' bits).
#   2. Append 0x00 bytes until length == 112 (mod 128).
#   3. Append the original bit length as a 128-bit big-endian integer.
#
# Why 112 mod 128?  We need 16 bytes for the length field, and 112 + 16 = 128.
#
# Example -- "abc" (3 bytes = 24 bits):
#   61 62 63 80 [108 zero bytes] 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 18
#   ─────────── ────────────────  ─────────────────────────────────────────────────
#   message+bit  padding zeros     128-bit length: 24 bits = 0x18

def _pad(data: bytes) -> bytes:
    """Pad data to a multiple of 128 bytes per FIPS 180-4 section 5.1.2."""
    bit_len = len(data) * 8
    msg = bytearray(data) + bytearray([0x80])
    # Pad to 112 mod 128 (leaving 16 bytes for the length)
    while len(msg) % 128 != 112:
        msg.append(0x00)
    # Append 128-bit big-endian length.  For practical purposes the message
    # length fits in 64 bits, so the upper 64 bits are zero.
    msg += struct.pack(">Q", 0)           # upper 64 bits of bit length
    msg += struct.pack(">Q", bit_len)     # lower 64 bits of bit length
    return bytes(msg)


# ---- Message Schedule ----
#
# Each 128-byte block is parsed as 16 big-endian 64-bit words W[0..15].
# These are expanded to 80 words:
#
#   W[t] = sigma1(W[t-2]) + W[t-7] + sigma0(W[t-15]) + W[t-16]   for t >= 16
#
# The sigma functions mix bits from earlier words, ensuring that a single bit
# change in the input block propagates through all 80 schedule words.

def _schedule(block: bytes) -> list[int]:
    """Expand a 128-byte block to an 80-word message schedule."""
    # Parse 16 big-endian 64-bit words.  ">16Q" means 16 unsigned 64-bit
    # integers in big-endian byte order.
    W: list[int] = list(struct.unpack(">16Q", block))
    for t in range(16, 80):
        w = (
            _small_sigma1(W[t - 2])
            + W[t - 7]
            + _small_sigma0(W[t - 15])
            + W[t - 16]
        ) & _MASK64
        W.append(w)
    return W


# ---- Compression Function ----
#
# 80 rounds of mixing fold one 128-byte block into the eight-word state.
#
# Each round:
#   T1 = h + Sigma1(e) + Ch(e,f,g) + K[t] + W[t]
#   T2 = Sigma0(a) + Maj(a,b,c)
#   shift: h=g, g=f, f=e, e=d+T1, d=c, c=b, b=a, a=T1+T2
#
# After 80 rounds, the working variables are added back to the input state
# (Davies-Meyer feed-forward), which makes the function non-invertible.

def _compress(
    state: tuple[int, ...], block: bytes
) -> tuple[int, ...]:
    """Mix one 128-byte block into the eight-word state via 80 rounds."""
    W = _schedule(block)

    a, b, c, d, e, f, g, h = state

    for t in range(80):
        # T1 combines the "e-side" mixing with the round constant and schedule word
        t1 = (h + _big_sigma1(e) + _ch(e, f, g) + _K[t] + W[t]) & _MASK64
        # T2 combines the "a-side" mixing
        t2 = (_big_sigma0(a) + _maj(a, b, c)) & _MASK64
        # Shift the working variables
        h = g
        g = f
        f = e
        e = (d + t1) & _MASK64
        d = c
        c = b
        b = a
        a = (t1 + t2) & _MASK64

    # Davies-Meyer feed-forward: add compressed output back to input state.
    # This prevents inversion even if an attacker can run all 80 rounds backward.
    return (
        (state[0] + a) & _MASK64,
        (state[1] + b) & _MASK64,
        (state[2] + c) & _MASK64,
        (state[3] + d) & _MASK64,
        (state[4] + e) & _MASK64,
        (state[5] + f) & _MASK64,
        (state[6] + g) & _MASK64,
        (state[7] + h) & _MASK64,
    )


# ---- Public API ----


def sha512(data: bytes) -> bytes:
    """Compute the SHA-512 digest of data.  Returns 64 bytes.

    This is the one-shot API: hash a complete message in a single call.

    Example::

        >>> sha512(b"abc").hex()
        'ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a\
2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f'
        >>> len(sha512(b""))
        64
    """
    padded = _pad(data)
    state = _INIT
    for offset in range(0, len(padded), 128):
        state = _compress(state, padded[offset : offset + 128])
    # Finalize: concatenate the eight state words as big-endian 64-bit integers.
    return struct.pack(">8Q", *state)


def sha512_hex(data: bytes) -> str:
    """Compute SHA-512 and return the 128-character lowercase hex string.

    Convenience wrapper around sha512().  Equivalent to sha512(data).hex().

    Example::

        >>> sha512_hex(b"abc")
        'ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a\
2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f'
    """
    return sha512(data).hex()


class SHA512Hasher:
    """Streaming SHA-512 hasher that accepts data in multiple chunks.

    Useful when the full message is not available at once -- for example when
    reading a large file in chunks or hashing a network stream.

    The interface mirrors Python's hashlib API::

        h = SHA512Hasher()
        h.update(b"ab")
        h.update(b"c")
        h.hex_digest()   # -> 'ddaf35a1...'

    Multiple update() calls are equivalent to a single sha512(all_data):
        SHA512Hasher().update(a).update(b).digest() == sha512(a + b)

    Implementation note:
        We accumulate data in a buffer.  When the buffer has >=128 bytes we
        compress the front 128 bytes into the state and discard them.  On
        digest(), we pad whatever remains and compress the padding blocks.
    """

    def __init__(self) -> None:
        """Initialize with SHA-512's starting constants."""
        self._state: tuple[int, ...] = _INIT
        self._buffer: bytearray = bytearray()
        self._byte_count: int = 0  # total bytes seen (used in padding)

    def update(self, data: bytes) -> "SHA512Hasher":
        """Feed more bytes into the hash.  Returns self for chaining."""
        self._buffer.extend(data)
        self._byte_count += len(data)
        # Compress any complete 128-byte blocks now to limit buffer size
        while len(self._buffer) >= 128:
            self._state = _compress(self._state, bytes(self._buffer[:128]))
            self._buffer = self._buffer[128:]
        return self

    def digest(self) -> bytes:
        """Return the 64-byte digest of all data fed so far.

        Non-destructive: calling digest() twice returns the same bytes, and
        you can continue calling update() after digest().
        """
        # Pad the remaining buffer using the TOTAL byte count (not buffer size)
        bit_len = self._byte_count * 8
        tail = bytearray(self._buffer) + bytearray([0x80])
        while len(tail) % 128 != 112:
            tail.append(0x00)
        tail += struct.pack(">Q", 0)           # upper 64 bits
        tail += struct.pack(">Q", bit_len)     # lower 64 bits
        # Compress the padding tail against a copy of the live state
        state = self._state
        for offset in range(0, len(tail), 128):
            state = _compress(state, bytes(tail[offset : offset + 128]))
        return struct.pack(">8Q", *state)

    def hex_digest(self) -> str:
        """Return the 128-character hex string of the digest."""
        return self.digest().hex()

    def copy(self) -> "SHA512Hasher":
        """Return an independent copy of the current hasher state.

        Useful for computing multiple hashes that share a common prefix::

            h = SHA512Hasher()
            h.update(common_prefix)
            h1 = h.copy(); h1.update(b"suffix_a")
            h2 = h.copy(); h2.update(b"suffix_b")
        """
        other = SHA512Hasher()
        other._state = self._state
        other._buffer = bytearray(self._buffer)
        other._byte_count = self._byte_count
        return other
