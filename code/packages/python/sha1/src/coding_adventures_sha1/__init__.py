"""sha1 — SHA-1 cryptographic hash function (FIPS 180-4) from scratch.

What Is SHA-1?
==============
SHA-1 (Secure Hash Algorithm 1) takes any sequence of bytes and produces a
fixed-size 20-byte (160-bit) "fingerprint" called a digest. The same input
always produces the same digest. Change even one bit of input and the digest
changes completely — the "avalanche effect".

We implement SHA-1 from scratch here (not using hashlib.sha1) so you can see
exactly how the algorithm works at the bit level. Reading this file should
leave you understanding why cryptographic hash functions are hard to reverse.

The Big Picture
===============
SHA-1 uses the Merkle-Damgård construction:

  Input message (any length)
       │
       ▼ pad to multiple of 512 bits
  ┌────────┬────────┬────────┐
  │ block₀ │ block₁ │  ...   │  (each 512 bits = 64 bytes)
  └────────┴────────┴────────┘
       │        │
       ▼        ▼
  [H₀..H₄]──►compress──►compress──►...──► 20-byte digest

The state is five 32-bit words (H₀..H₄). Each 64-byte block is "compressed"
into the state through 80 rounds of bit mixing. The final state is the digest.

Analogy: a blender. You start with a base liquid (the initial constants).
Add ingredients one chunk at a time (message blocks). Each blend mixes the
new ingredient with everything before it. You cannot un-blend to recover
the ingredients.

FIPS 180-4 reference test vectors:
  sha1(b"")    = da39a3ee5e6b4b0d3255bfef95601890afd80709
  sha1(b"abc") = a9993e364706816aba3e25717850c26c9cd0d89d
"""

from __future__ import annotations

import struct

__version__ = "0.1.0"

# ─── Initialization Constants ────────────────────────────────────────────────
#
# SHA-1 starts with these five 32-bit words as its initial state. They look
# like "nothing up my sleeve" numbers — chosen to have an obvious pattern
# (01234567, 89ABCDEF, ... in little-endian byte order) to prove no hidden
# mathematical backdoor is present.
#
#   H₀ = 0x67452301  →  bytes 67 45 23 01  →  reverse: 01 23 45 67
#   H₁ = 0xEFCDAB89  →  bytes EF CD AB 89  →  reverse: 89 AB CD EF
#   H₂ = 0x98BADCFE  →  bytes 98 BA DC FE  →  reverse: FE DC BA 98
#   H₃ = 0x10325476  →  bytes 10 32 54 76  →  reverse: 76 54 32 10

_INIT: tuple[int, int, int, int, int] = (
    0x67452301,
    0xEFCDAB89,
    0x98BADCFE,
    0x10325476,
    0xC3D2E1F0,
)

# Round constants — one per 20-round stage.
_K: tuple[int, int, int, int] = (
    0x5A827999,  # rounds 0–19   (floor(sqrt(2) * 2^30))
    0x6ED9EBA1,  # rounds 20–39  (floor(sqrt(3) * 2^30))
    0x8F1BBCDC,  # rounds 40–59  (floor(sqrt(5) * 2^30))
    0xCA62C1D6,  # rounds 60–79  (floor(sqrt(10) * 2^30))
)


def _rotl(n: int, x: int) -> int:
    """Circular left shift of x by n bits (32-bit words).

    Bits that "fall off" the left end reappear on the right. This differs
    from a regular left shift (<<) where overflow bits are discarded.

    Example with n=2, x=0b01101001 (8-bit for clarity):
      Regular:  01101001 << 2 = 10100100  (01 on the left is gone)
      Circular: 01101001 ROTL 2 = 10100110  (01 wraps to the right)

    We mask to 32 bits because Python integers have arbitrary precision.
    """
    return ((x << n) | (x >> (32 - n))) & 0xFFFFFFFF


def _pad(data: bytes) -> bytes:
    """Pad data to a multiple of 64 bytes per FIPS 180-4 §5.1.1.

    Rule:
      1. Append a single 0x80 byte (the '1' bit, followed by seven '0' bits).
      2. Append 0x00 bytes until length ≡ 56 (mod 64).
      3. Append the original bit length as a 64-bit big-endian integer.

    Example — padding "abc" (3 bytes = 24 bits):
      61 62 63 80 [52 zero bytes] 00 00 00 00 00 00 00 18
      ─────────── ──────────────  ─────────────────────────
      message+bit  padding zeros   length: 24 bits = 0x18
    """
    bit_len = len(data) * 8
    msg = bytearray(data) + bytearray([0x80])
    while len(msg) % 64 != 56:
        msg.append(0x00)
    msg += struct.pack(">Q", bit_len)
    return bytes(msg)


def _schedule(block: bytes) -> list[int]:
    """Expand a 64-byte block to an 80-word message schedule.

    The block is first parsed as 16 big-endian 32-bit words W[0..15].
    Words W[16..79] are derived using:

        W[i] = ROTL(1, W[i-3] XOR W[i-8] XOR W[i-14] XOR W[i-16])

    Why expand? More rounds = better avalanche. Each word depends on four
    earlier words, so a single bit change ripples through all 80 words,
    ensuring every output bit is influenced by every input bit.
    """
    W = list(struct.unpack(">16I", block))
    for i in range(16, 80):
        W.append(_rotl(1, W[i - 3] ^ W[i - 8] ^ W[i - 14] ^ W[i - 16]))
    return W


def _compress(
    state: tuple[int, int, int, int, int], block: bytes
) -> tuple[int, int, int, int, int]:
    """Mix one 64-byte block into the five-word state via 80 rounds.

    Four stages of 20 rounds each, with different auxiliary functions:

    Stage  Rounds   f(b,c,d)                          Purpose
    ─────  ──────   ──────────────────────────────    ────────────
      1    0–19     (b AND c) OR (NOT b AND d)        Selector/mux
      2   20–39     b XOR c XOR d                     Parity
      3   40–59     (b AND c)|(b AND d)|(c AND d)     Majority vote
      4   60–79     b XOR c XOR d                     Parity again

    Each round:
      temp = ROTL(5, a) + f(b,c,d) + e + K + W[t]  (mod 2³²)
      shift: e=d, d=c, c=ROTL(30,b), b=a, a=temp
    """
    h0, h1, h2, h3, h4 = state
    W = _schedule(block)
    a, b, c, d, e = h0, h1, h2, h3, h4

    for t in range(80):
        if t < 20:
            # Selector: if b=1 output c, if b=0 output d
            f = (b & c) | ((~b) & d)
            k = _K[0]
        elif t < 40:
            # Parity: 1 if an odd number of inputs are 1
            f = b ^ c ^ d
            k = _K[1]
        elif t < 60:
            # Majority: 1 if at least 2 of the 3 inputs are 1
            f = (b & c) | (b & d) | (c & d)
            k = _K[2]
        else:
            # Parity again (same formula, different constant)
            f = b ^ c ^ d
            k = _K[3]

        temp = (_rotl(5, a) + (f & 0xFFFFFFFF) + e + k + W[t]) & 0xFFFFFFFF
        e = d
        d = c
        c = _rotl(30, b)
        b = a
        a = temp

    # Davies-Meyer feed-forward: add the round result back to the input state.
    # This makes the compression hard to invert even if you ran all 80 rounds
    # backward — you still need to subtract the original state you don't have.
    return (
        (h0 + a) & 0xFFFFFFFF,
        (h1 + b) & 0xFFFFFFFF,
        (h2 + c) & 0xFFFFFFFF,
        (h3 + d) & 0xFFFFFFFF,
        (h4 + e) & 0xFFFFFFFF,
    )


# ─── Public API ───────────────────────────────────────────────────────────────


def sha1(data: bytes) -> bytes:
    """Compute the SHA-1 digest of data. Returns 20 bytes.

    This is the one-shot API: hash a complete message in a single call.

    Example::

        >>> sha1(b"abc").hex()
        'a9993e364706816aba3e25717850c26c9cd0d89d'
        >>> sha1(b"").hex()
        'da39a3ee5e6b4b0d3255bfef95601890afd80709'
    """
    padded = _pad(data)
    state = _INIT
    for offset in range(0, len(padded), 64):
        state = _compress(state, padded[offset : offset + 64])
    # Finalize: concatenate the five state words as big-endian 32-bit integers.
    # Big-endian = most significant byte first (natural human-readable order).
    return struct.pack(">5I", *state)


def sha1_hex(data: bytes) -> str:
    """Compute SHA-1 and return the 40-character lowercase hex string.

    Convenience wrapper around sha1(). Equivalent to sha1(data).hex().

    Example::

        >>> sha1_hex(b"abc")
        'a9993e364706816aba3e25717850c26c9cd0d89d'
    """
    return sha1(data).hex()


class SHA1:
    """Streaming SHA-1 hasher that accepts data in multiple chunks.

    Useful when the full message is not available at once — for example when
    reading a large file in chunks or hashing a network stream.

    The interface mirrors Python's hashlib API::

        h = SHA1()
        h.update(b"ab")
        h.update(b"c")
        h.hexdigest()   # → 'a9993e364706816aba3e25717850c26c9cd0d89d'

    Multiple update() calls are equivalent to a single sha1(all_data):
        SHA1().update(a).update(b).digest() == sha1(a + b)

    Implementation note:
        We accumulate data in a buffer. When the buffer has ≥64 bytes we
        compress the front 64 bytes into the state and discard them. On
        digest(), we pad whatever remains and compress the padding blocks.
    """

    def __init__(self) -> None:
        """Initialize with SHA-1's starting constants."""
        self._state: tuple[int, int, int, int, int] = _INIT
        self._buffer: bytearray = bytearray()
        self._byte_count: int = 0  # total bytes seen (used in padding length)

    def update(self, data: bytes) -> "SHA1":
        """Feed more bytes into the hash. Returns self for chaining."""
        self._buffer.extend(data)
        self._byte_count += len(data)
        # Compress any complete 64-byte blocks now to limit buffer size
        while len(self._buffer) >= 64:
            self._state = _compress(self._state, bytes(self._buffer[:64]))
            self._buffer = self._buffer[64:]
        return self

    def digest(self) -> bytes:
        """Return the 20-byte digest of all data fed so far.

        Non-destructive: calling digest() twice returns the same bytes, and
        you can continue calling update() after digest().
        """
        # Pad the remaining buffer using the TOTAL byte count (not buffer size)
        bit_len = self._byte_count * 8
        tail = bytearray(self._buffer) + bytearray([0x80])
        while len(tail) % 64 != 56:
            tail.append(0x00)
        tail += struct.pack(">Q", bit_len)
        # Compress the padding tail against a copy of the live state
        state = self._state
        for offset in range(0, len(tail), 64):
            state = _compress(state, bytes(tail[offset : offset + 64]))
        return struct.pack(">5I", *state)

    def hexdigest(self) -> str:
        """Return the 40-character hex string of the digest."""
        return self.digest().hex()

    def copy(self) -> "SHA1":
        """Return an independent copy of the current hasher state.

        Useful for computing multiple hashes that share a common prefix::

            h = SHA1()
            h.update(common_prefix)
            h1 = h.copy(); h1.update(b"suffix_a")
            h2 = h.copy(); h2.update(b"suffix_b")
        """
        other = SHA1()
        other._state = self._state
        other._buffer = bytearray(self._buffer)
        other._byte_count = self._byte_count
        return other
