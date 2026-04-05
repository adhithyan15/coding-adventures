"""sha256 -- SHA-256 cryptographic hash function (FIPS 180-4) from scratch.

What Is SHA-256?
================
SHA-256 (Secure Hash Algorithm 256) belongs to the SHA-2 family, designed by
the NSA and published by NIST in 2001 (FIPS 180-2, updated in FIPS 180-4).
It produces a 256-bit (32-byte) digest and is the backbone of modern
cryptography -- TLS certificates, Bitcoin proof-of-work, git commit IDs,
code signing, and password hashing all rely on SHA-256.

Unlike MD5 (broken 2004) and SHA-1 (broken 2017), SHA-256 remains secure
with no known practical collision or preimage attacks. Its birthday bound is
2^128, making brute-force collision search computationally infeasible.

How It Differs from SHA-1
=========================
SHA-256 shares the same Merkle-Damgard construction as SHA-1 but with:
  - 8 state words (not 5) -- each 32 bits wide
  - 64 rounds (not 80) per block
  - 64 round constants K[0..63] from cube roots of first 64 primes
  - A more complex message schedule with two "small sigma" functions
  - Two "big Sigma" functions replacing SHA-1's simple ROTL(5,a)/ROTL(30,b)
  - Ch and Maj auxiliary functions (SHA-1's function selection by round stage
    is replaced by a uniform round structure using both Ch and Maj every round)

The Big Picture: Merkle-Damgard Construction
============================================

  Input message (any length)
       |
       v  pad to multiple of 512 bits
  +--------+--------+--------+
  | block0 | block1 |  ...   |  (each 512 bits = 64 bytes)
  +--------+--------+--------+
       |        |
       v        v
  [H0..H7]-->compress-->compress-->...-->  32-byte digest

The state is eight 32-bit words (H0..H7). Each 64-byte block is compressed
into the state through 64 rounds of bit mixing. The final state IS the digest.

Analogy: a blender. Start with a base liquid (eight initial constants derived
from the square roots of the first 8 primes). Add ingredients one chunk at a
time (message blocks). Each blend mixes the new ingredient with everything
before it. You cannot un-blend to recover the original ingredients.

FIPS 180-4 Reference Test Vectors
==================================
  sha256(b"")    = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
  sha256(b"abc") = ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
"""

from __future__ import annotations

import struct

__version__ = "0.1.0"


# === Initialization Constants ================================================
#
# SHA-256 starts with eight 32-bit words derived from the FRACTIONAL parts of
# the square roots of the first 8 prime numbers (2, 3, 5, 7, 11, 13, 17, 19).
#
# For prime p, take sqrt(p), keep only the fractional part, multiply by 2^32,
# and take the floor. For example:
#
#   sqrt(2) = 1.4142135623...
#   fractional part = 0.4142135623...
#   0.4142135623... * 2^32 = 1779033703.9520... -> floor = 0x6A09E667
#
# These are "nothing up my sleeve" numbers -- their derivation is transparent,
# proving no hidden mathematical backdoor exists.

_INIT: tuple[int, int, int, int, int, int, int, int] = (
    0x6A09E667,  # sqrt(2)
    0xBB67AE85,  # sqrt(3)
    0x3C6EF372,  # sqrt(5)
    0xA54FF53A,  # sqrt(7)
    0x510E527F,  # sqrt(11)
    0x9B05688C,  # sqrt(13)
    0x1F83D9AB,  # sqrt(17)
    0x5BE0CD19,  # sqrt(19)
)


# === Round Constants ==========================================================
#
# 64 round constants, one per round, derived from the FRACTIONAL parts of the
# cube roots of the first 64 prime numbers (2, 3, 5, ..., 311).
#
# Same derivation as the init constants but using cube roots instead of square
# roots. For prime p:
#
#   K[i] = floor(frac(cbrt(p_i)) * 2^32)
#
# Having unique constants for each round prevents round symmetry attacks --
# if every round used the same constant, the attacker could exploit the
# structural repetition.

_K: tuple[int, ...] = (
    0x428A2F98, 0x71374491, 0xB5C0FBCF, 0xE9B5DBA5,
    0x3956C25B, 0x59F111F1, 0x923F82A4, 0xAB1C5ED5,
    0xD807AA98, 0x12835B01, 0x243185BE, 0x550C7DC3,
    0x72BE5D74, 0x80DEB1FE, 0x9BDC06A7, 0xC19BF174,
    0xE49B69C1, 0xEFBE4786, 0x0FC19DC6, 0x240CA1CC,
    0x2DE92C6F, 0x4A7484AA, 0x5CB0A9DC, 0x76F988DA,
    0x983E5152, 0xA831C66D, 0xB00327C8, 0xBF597FC7,
    0xC6E00BF3, 0xD5A79147, 0x06CA6351, 0x14292967,
    0x27B70A85, 0x2E1B2138, 0x4D2C6DFC, 0x53380D13,
    0x650A7354, 0x766A0ABB, 0x81C2C92E, 0x92722C85,
    0xA2BFE8A1, 0xA81A664B, 0xC24B8B70, 0xC76C51A3,
    0xD192E819, 0xD6990624, 0xF40E3585, 0x106AA070,
    0x19A4C116, 0x1E376C08, 0x2748774C, 0x34B0BCB5,
    0x391C0CB3, 0x4ED8AA4A, 0x5B9CCA4F, 0x682E6FF3,
    0x748F82EE, 0x78A5636F, 0x84C87814, 0x8CC70208,
    0x90BEFFFA, 0xA4506CEB, 0xBEF9A3F7, 0xC67178F2,
)

# Mask for keeping values within 32 bits. Python integers have arbitrary
# precision, so we must mask after every addition or bitwise NOT to simulate
# 32-bit unsigned arithmetic.
_MASK = 0xFFFFFFFF


# === Bit Manipulation Helpers =================================================
#
# SHA-256 uses six auxiliary functions. They operate on 32-bit words and
# combine rotations, shifts, and boolean operations to create non-linear
# mixing that resists cryptanalysis.
#
# Notation:
#   ROTR(n, x) = circular right rotation of x by n bits
#   SHR(n, x)  = logical right shift of x by n bits
#   ^          = XOR
#   &          = AND
#   ~          = bitwise NOT


def _rotr(n: int, x: int) -> int:
    """Circular right rotation of a 32-bit word by n positions.

    Bits that "fall off" the right end reappear on the left. This differs
    from a right shift (>>) where the bits are simply discarded.

    Example with n=3, x=0b11010010 (8-bit for clarity):
      Shift:    11010010 >> 3  = 00011010  (110 is lost)
      Rotate:   11010010 ROTR3 = 01011010  (010 wraps to the left)

    Implementation: the right half (x >> n) provides the non-wrapped bits,
    and the left half (x << (32-n)) provides the wrapped-around bits.
    """
    return ((x >> n) | (x << (32 - n))) & _MASK


def _shr(n: int, x: int) -> int:
    """Logical right shift of a 32-bit word. Unlike rotation, shifted-out
    bits are discarded (replaced with zeros on the left).
    """
    return x >> n


def _ch(x: int, y: int, z: int) -> int:
    """Choice function: for each bit position, if x=1 choose y, else choose z.

    Truth table:
      x | y | z | Ch
      --+---+---+----
      0 | 0 | 0 |  0   (x=0, pick z=0)
      0 | 0 | 1 |  1   (x=0, pick z=1)
      0 | 1 | 0 |  0   (x=0, pick z=0)
      0 | 1 | 1 |  1   (x=0, pick z=1)
      1 | 0 | 0 |  0   (x=1, pick y=0)
      1 | 0 | 1 |  0   (x=1, pick y=0)
      1 | 1 | 0 |  1   (x=1, pick y=1)
      1 | 1 | 1 |  1   (x=1, pick y=1)

    Used in SHA-256's round function to mix three working variables (e, f, g).
    Think of it as a 1-bit multiplexer: x is the selector, y and z are inputs.
    """
    return ((x & y) ^ (~x & z)) & _MASK


def _maj(x: int, y: int, z: int) -> int:
    """Majority function: output is 1 if at least 2 of the 3 inputs are 1.

    Truth table:
      x | y | z | Maj
      --+---+---+-----
      0 | 0 | 0 |  0   (0 ones: minority)
      0 | 0 | 1 |  0   (1 one:  minority)
      0 | 1 | 0 |  0   (1 one:  minority)
      0 | 1 | 1 |  1   (2 ones: majority)
      1 | 0 | 0 |  0   (1 one:  minority)
      1 | 0 | 1 |  1   (2 ones: majority)
      1 | 1 | 0 |  1   (2 ones: majority)
      1 | 1 | 1 |  1   (3 ones: majority)

    Used to mix working variables (a, b, c). Ensures that even if one variable
    is "stuck", the other two still influence the output.
    """
    return ((x & y) ^ (x & z) ^ (y & z)) & _MASK


def _big_sigma0(x: int) -> int:
    r"""Big Sigma 0 (\u03a30): used on working variable 'a' in the round function.

    \u03a30(x) = ROTR(2, x) XOR ROTR(13, x) XOR ROTR(22, x)

    The three different rotation amounts (2, 13, 22) ensure that each bit of
    the input influences many different bit positions in the output. This
    creates diffusion -- small input changes cause large output changes.

    Why these specific rotation amounts? They were chosen by the NSA to
    maximize the "branch number" (a measure of diffusion strength) of the
    compression function. Each pair of rotations shares no common factor
    with 32, ensuring all bit positions are covered.
    """
    return _rotr(2, x) ^ _rotr(13, x) ^ _rotr(22, x)


def _big_sigma1(x: int) -> int:
    r"""Big Sigma 1 (\u03a31): used on working variable 'e' in the round function.

    \u03a31(x) = ROTR(6, x) XOR ROTR(11, x) XOR ROTR(25, x)

    Same idea as \u03a30 but with different rotation amounts. Having different
    constants for the 'a' path and 'e' path breaks structural symmetry --
    the two halves of the round function behave differently, making
    cryptanalysis harder.
    """
    return _rotr(6, x) ^ _rotr(11, x) ^ _rotr(25, x)


def _small_sigma0(x: int) -> int:
    r"""Small sigma 0 (\u03c30): used in the message schedule expansion.

    \u03c30(x) = ROTR(7, x) XOR ROTR(18, x) XOR SHR(3, x)

    Note the SHR (shift, not rotate) in the third term. This destroys
    information -- bits shifted out are gone. This is intentional: it makes
    the message schedule a one-way function, preventing an attacker from
    working backward from later schedule words to earlier ones.
    """
    return _rotr(7, x) ^ _rotr(18, x) ^ _shr(3, x)


def _small_sigma1(x: int) -> int:
    r"""Small sigma 1 (\u03c31): used in the message schedule expansion.

    \u03c31(x) = ROTR(17, x) XOR ROTR(19, x) XOR SHR(10, x)

    Partner to \u03c30 with different rotation/shift amounts. Together they ensure
    each message schedule word W[t] for t >= 16 depends on four earlier words
    through two different non-linear mixing paths.
    """
    return _rotr(17, x) ^ _rotr(19, x) ^ _shr(10, x)


# === Padding ==================================================================
#
# The compression function needs exactly 64-byte (512-bit) blocks. Padding
# extends the message per FIPS 180-4 section 5.1.1:
#
#   1. Append a single 0x80 byte (the '1' bit followed by seven '0' bits).
#   2. Append 0x00 bytes until length == 56 (mod 64).
#   3. Append the original bit length as a 64-bit big-endian integer.
#
# Why 56 mod 64? Because 56 + 8 = 64 -- we need room for the 8-byte length
# field to complete the final block.
#
# Example -- padding "abc" (3 bytes = 24 bits):
#   61 62 63 80 [52 zero bytes] 00 00 00 00 00 00 00 18
#   ~~~~~~~~    ~~~~~~~~~~~~~~  ~~~~~~~~~~~~~~~~~~~~~~~~
#   message+1bit  zero padding  length: 24 bits = 0x18
#
# If the message is >= 56 bytes mod 64, the padding overflows into an extra
# block. For instance, a 56-byte message needs: 56 + 1 = 57 bytes, which
# exceeds 56, so padding extends to 120 bytes (two full blocks).


def _pad(data: bytes) -> bytes:
    """Pad data to a multiple of 64 bytes per Merkle-Damgard rules."""
    bit_len = len(data) * 8
    msg = bytearray(data) + bytearray([0x80])
    while len(msg) % 64 != 56:
        msg.append(0x00)
    # Append 64-bit big-endian length. ">Q" = big-endian unsigned 64-bit.
    msg += struct.pack(">Q", bit_len)
    return bytes(msg)


# === Message Schedule =========================================================
#
# Each 64-byte block is expanded into a 64-word "message schedule" W[0..63].
#
# The first 16 words come directly from the block (parsed as big-endian uint32).
# Words 16..63 are derived from earlier words:
#
#   W[t] = sigma1(W[t-2]) + W[t-7] + sigma0(W[t-15]) + W[t-16]
#
# This is more complex than SHA-1's schedule (which used a simple XOR + rotate).
# The two sigma functions create stronger diffusion -- each new word depends
# on four earlier words through non-linear operations, making it much harder
# for an attacker to control the schedule.
#
# Visual dependency graph for W[16]:
#
#   W[0] ---sigma0---> +
#   W[1] -------------> +
#   W[9] -------------> +
#   W[14] --sigma1---> +  ---> W[16]


def _schedule(block: bytes) -> list[int]:
    """Expand a 64-byte block into a 64-word message schedule."""
    # Parse 16 big-endian 32-bit words from the block
    W = list(struct.unpack(">16I", block))
    for t in range(16, 64):
        W.append(
            (_small_sigma1(W[t - 2]) + W[t - 7]
             + _small_sigma0(W[t - 15]) + W[t - 16]) & _MASK
        )
    return W


# === Compression Function =====================================================
#
# The heart of SHA-256. Each 64-byte block is "compressed" into the 8-word
# state through 64 rounds.
#
# Working variables a, b, c, d, e, f, g, h are initialized from the current
# state. Each round computes:
#
#   T1 = h + Sigma1(e) + Ch(e, f, g) + K[t] + W[t]
#   T2 = Sigma0(a) + Maj(a, b, c)
#
# Then the variables are shifted:
#   h = g
#   g = f
#   f = e
#   e = d + T1
#   d = c
#   c = b
#   b = a
#   a = T1 + T2
#
# Round structure diagram (one round):
#
#   a    b    c    d    e    f    g    h
#   |    |    |    |    |    |    |    |
#   +--Maj--+    |    +--Ch---+    |
#   |  |         |    |  |         |
#  S0  |         |   S1  |         |
#   |  |         |    |  |         |
#   +--+    +----+    +--+  K[t]+W[t]
#   |       |         |       |
#   T2      |         +---T1--+
#   |       |              |
#   +---T1--+---T2    d+T1
#       |                  |
#       a'   b'   c'  d'   e'   f'   g'   h'
#
# After all 64 rounds, the working variables are added back to the input
# state (Davies-Meyer feed-forward). This addition makes the compression
# non-invertible -- even if you could reverse all 64 rounds, you'd need to
# subtract the original state that you don't know.

_State = tuple[int, int, int, int, int, int, int, int]


def _compress(state: _State, block: bytes) -> _State:
    """Compress one 64-byte block into the eight-word state."""
    h0, h1, h2, h3, h4, h5, h6, h7 = state
    W = _schedule(block)
    a, b, c, d, e, f, g, h = h0, h1, h2, h3, h4, h5, h6, h7

    for t in range(64):
        t1 = (h + _big_sigma1(e) + _ch(e, f, g) + _K[t] + W[t]) & _MASK
        t2 = (_big_sigma0(a) + _maj(a, b, c)) & _MASK
        h = g
        g = f
        f = e
        e = (d + t1) & _MASK
        d = c
        c = b
        b = a
        a = (t1 + t2) & _MASK

    # Davies-Meyer feed-forward
    return (
        (h0 + a) & _MASK,
        (h1 + b) & _MASK,
        (h2 + c) & _MASK,
        (h3 + d) & _MASK,
        (h4 + e) & _MASK,
        (h5 + f) & _MASK,
        (h6 + g) & _MASK,
        (h7 + h) & _MASK,
    )


# === Public API ===============================================================


def sha256(data: bytes) -> bytes:
    """Compute the SHA-256 digest of data. Returns 32 bytes.

    This is the one-shot API: hash a complete message in a single call.

    Example::

        >>> sha256(b"abc").hex()
        'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad'
        >>> sha256(b"").hex()
        'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855'
    """
    padded = _pad(data)
    state = _INIT
    for offset in range(0, len(padded), 64):
        state = _compress(state, padded[offset : offset + 64])
    # Finalize: concatenate the eight state words as big-endian 32-bit integers.
    # ">8I" = eight unsigned 32-bit integers, big-endian byte order.
    return struct.pack(">8I", *state)


def sha256_hex(data: bytes) -> str:
    """Compute SHA-256 and return the 64-character lowercase hex string.

    Convenience wrapper around sha256(). Equivalent to sha256(data).hex().

    Example::

        >>> sha256_hex(b"abc")
        'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad'
    """
    return sha256(data).hex()


class SHA256Hasher:
    """Streaming SHA-256 hasher that accepts data in multiple chunks.

    Useful when the full message is not available at once -- for example when
    reading a large file in chunks or hashing a network stream.

    The interface is designed for chaining::

        h = SHA256Hasher()
        h.update(b"ab")
        h.update(b"c")
        h.hex_digest()   # -> 'ba7816bf...'

    Multiple update() calls are equivalent to a single sha256(all_data)::

        SHA256Hasher().update(a).update(b).digest() == sha256(a + b)

    Implementation note:
        We accumulate data in a buffer. When the buffer has >= 64 bytes we
        compress the front 64 bytes into the state and discard them. On
        digest(), we pad whatever remains and compress the padding blocks.
        The padding uses the TOTAL byte count (not the buffer size) to
        correctly encode the original message length.
    """

    def __init__(self) -> None:
        """Initialize with SHA-256's starting constants."""
        self._state: _State = _INIT
        self._buffer: bytearray = bytearray()
        self._byte_count: int = 0  # total bytes seen across all update() calls

    def update(self, data: bytes) -> "SHA256Hasher":
        """Feed more bytes into the hash. Returns self for chaining.

        You can call update() any number of times with any chunk sizes.
        The hash result depends only on the concatenation of all chunks,
        not on how they were split::

            h.update(b"abc")          # same as
            h.update(b"a").update(b"bc")  # same as
            h.update(b"a").update(b"b").update(b"c")
        """
        self._buffer.extend(data)
        self._byte_count += len(data)
        # Compress any complete 64-byte blocks now to limit memory usage
        while len(self._buffer) >= 64:
            self._state = _compress(self._state, bytes(self._buffer[:64]))
            self._buffer = self._buffer[64:]
        return self

    def digest(self) -> bytes:
        """Return the 32-byte digest of all data fed so far.

        Non-destructive: calling digest() twice returns the same bytes, and
        you can continue calling update() after digest(). The internal state
        is not modified -- we work on copies of the buffer and state.
        """
        # Pad the remaining buffer using the TOTAL byte count (not buffer length)
        bit_len = self._byte_count * 8
        tail = bytearray(self._buffer) + bytearray([0x80])
        while len(tail) % 64 != 56:
            tail.append(0x00)
        tail += struct.pack(">Q", bit_len)
        # Compress the padding tail against a COPY of the live state
        state = self._state
        for offset in range(0, len(tail), 64):
            state = _compress(state, bytes(tail[offset : offset + 64]))
        return struct.pack(">8I", *state)

    def hex_digest(self) -> str:
        """Return the 64-character lowercase hex string of the digest."""
        return self.digest().hex()

    def copy(self) -> "SHA256Hasher":
        """Return an independent deep copy of the current hasher state.

        Useful for computing multiple hashes that share a common prefix::

            h = SHA256Hasher()
            h.update(common_prefix)
            h1 = h.copy(); h1.update(b"suffix_a")
            h2 = h.copy(); h2.update(b"suffix_b")
            # h1 and h2 have different digests; h is unchanged.

        The copy shares no mutable state with the original -- modifying one
        has no effect on the other.
        """
        other = SHA256Hasher()
        other._state = self._state  # tuple is immutable, no copy needed
        other._buffer = bytearray(self._buffer)  # bytearray IS mutable: copy
        other._byte_count = self._byte_count
        return other
