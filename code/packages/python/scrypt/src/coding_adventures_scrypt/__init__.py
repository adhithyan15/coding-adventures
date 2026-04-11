"""scrypt -- Memory-Hard Password-Based Key Derivation Function — RFC 7914.

What Is scrypt?
===============
scrypt is a password-based key derivation function designed by Colin Percival
in 2009. Like PBKDF2, it stretches a password into a cryptographic key using
many iterations. Unlike PBKDF2, scrypt is *memory-hard*: an attacker cannot
trade memory for time. Even with custom ASICs or FPGAs, the attacker must
allocate large amounts of RAM proportional to the N (work factor) parameter.

scrypt is used in:
- Litecoin cryptocurrency (the mining algorithm)
- Password managers (1Password, many others)
- Key derivation in various disk encryption tools
- Signal Protocol (as an additional layer)

Why Memory-Hardness Matters
============================
With PBKDF2 and bcrypt, an attacker building custom silicon can run many
iterations in parallel with tiny per-core memory. A GPU with 10,000 cores can
test 10,000 passwords simultaneously with only modest memory.

scrypt's ROMix function requires each block of computation to read from a
large, pseudorandomly-addressed table (the "V" array) that is N × 128r bytes.
With N=2^20 and r=8, that is 1 GiB per password guess. Even with custom chips,
you cannot fit many parallel instances in affordable hardware.

The scrypt Construction (RFC 7914)
====================================

  DK = scrypt(Password, Salt, N, r, p, dkLen)

  Step 1: B = PBKDF2-SHA256(Password, Salt, 1, p × 128r)
           Expands the password+salt into p independent 128r-byte blocks.

  Step 2: For each block B[i]:
             B[i] = ROMix(B[i], N, r)
           Applies the memory-hard mixing function independently to each block.

  Step 3: DK = PBKDF2-SHA256(Password, B, 1, dkLen)
           Collapses all the mixed blocks back into the desired key length.

Layers of the Algorithm
========================

  scrypt                   ← top-level API
    └── PBKDF2-SHA256      ← key stretching / expansion
          └── HMAC-SHA256  ← pseudorandom function
                └── SHA-256
    └── ROMix              ← memory-hard function (RFC 7914 § 5)
          └── BlockMix     ← block-level mixing (RFC 7914 § 4)
                └── Salsa20/8 core  ← 64-byte pseudorandom permutation (RFC 7914 § 3)

Parameters and Their Effect
============================
  N   (CPU/memory cost) — must be a power of 2, >= 2
      Determines the size of the V table (N × 128r bytes) and the number of
      iterations in ROMix. Doubling N doubles both time and memory.

  r   (block size factor) — must be >= 1
      The Salsa20/8 operates on 64-byte blocks; BlockMix operates on 2r of
      them (128r bytes total). Larger r = wider blocks = more memory bandwidth.

  p   (parallelism) — must be >= 1
      The number of independent ROMix operations. Each can run on a separate
      CPU core. Increasing p without changing N still increases time but not
      peak memory (each core only holds one V table at a time).

RFC 7914 Test Vectors
======================
  Vector 1 (trivial):
    scrypt(b"", b"", N=16, r=1, p=1, dkLen=64)
    → 77d6576238657b203b19ca42c18a0497f16b4844e307...
    (verified against Python hashlib.scrypt / OpenSSL)

  Vector 2 (real-world-ish):
    scrypt(b"password", b"NaCl", N=1024, r=8, p=16, dkLen=64)
    → fdbabe1c9d3472007856e7190d01e9fe7c6ad7cbc823...
    (verified against Python hashlib.scrypt / OpenSSL)

PBKDF2 Dependency
==================
This module delegates to coding_adventures_pbkdf2.pbkdf2_hmac_sha256 for its
two PBKDF2-SHA256 calls (Steps 1 and 3 of the scrypt construction). RFC 7914
test vector 1 uses an empty password, so both calls pass allow_empty_password=True.
The scrypt public API itself does not enforce non-empty passwords, remaining
lenient for RFC compatibility.
"""

from __future__ import annotations

import struct

from coding_adventures_pbkdf2 import pbkdf2_hmac_sha256 as _pbkdf2_hmac_sha256

__version__ = "0.1.0"

# =============================================================================
# Constants
# =============================================================================

# 2^32 mask for modular arithmetic (Salsa20/8 words are 32-bit)
_M32: int = 0xFFFFFFFF


# =============================================================================
# Salsa20/8 Core
# =============================================================================
#
# Salsa20/8 is a 64-byte → 64-byte pseudorandom permutation. It is the heart
# of BlockMix, and therefore of scrypt itself. "8" refers to 8 rounds (4 double-
# rounds, each consisting of a column round and a row round).
#
# The internal state is a 4×4 matrix of 32-bit words. We index it linearly
# as x[0..15]:
#
#   x[0]  x[1]  x[2]  x[3]
#   x[4]  x[5]  x[6]  x[7]
#   x[8]  x[9]  x[10] x[11]
#   x[12] x[13] x[14] x[15]
#
# Quarter-round (QR):
#   Given four indices a, b, c, d into x:
#
#     x[b] ^= rotl32(x[a] + x[d], 7)
#     x[c] ^= rotl32(x[b] + x[a], 9)
#     x[d] ^= rotl32(x[c] + x[b], 13)
#     x[a] ^= rotl32(x[d] + x[c], 18)
#
# Double-round = column round + row round:
#   Column round applies QR to the four columns (top-to-bottom):
#     QR(0, 4, 8, 12)   QR(5, 9, 13, 1)   QR(10, 14, 2, 6)   QR(15, 3, 7, 11)
#   Row round applies QR to the four rows (left-to-right):
#     QR(0, 1, 2, 3)    QR(5, 6, 7, 4)    QR(10, 11, 8, 9)   QR(15, 12, 13, 14)
#
# After 8 rounds (4 double-rounds), add the working state back to the original
# state word-by-word (mod 2^32), then serialize as 16 little-endian uint32s.


def _rotl32(x: int, n: int) -> int:
    """Rotate a 32-bit integer x left by n bits.

    This is the core operation in Salsa20's quarter-round. We keep the result
    in 32 bits by masking with 0xFFFFFFFF.

    Example: rotl32(0x00000001, 1) == 0x00000002
             rotl32(0x80000000, 1) == 0x00000001  (top bit wraps around)
    """
    return ((x << n) | (x >> (32 - n))) & _M32


def _salsa20_8(data: bytes) -> bytes:
    """Salsa20/8 core permutation: 64 bytes → 64 bytes.

    Implements RFC 7914 § 3. The input is parsed as sixteen 32-bit little-
    endian words, scrambled with 8 rounds of quarter-round operations, and the
    final state is added to the initial state and re-serialized.

    Parameters
    ----------
    data:
        Exactly 64 bytes of input.

    Returns
    -------
    bytes
        Exactly 64 bytes of pseudorandom output.
    """
    # Parse 64 bytes → 16 little-endian uint32 words.
    z = list(struct.unpack_from("<16I", data))  # original state (saved for final XOR)
    x = list(z)                                 # working state

    # 4 double-rounds = 8 total rounds
    for _ in range(4):
        # ── Column round ──────────────────────────────────────────────────────
        # Apply QR(a,b,c,d) to each of the four columns of the 4×4 matrix.
        #
        # Column 0: indices 0, 4, 8, 12
        x[4]  ^= _rotl32((x[0]  + x[12]) & _M32, 7)
        x[8]  ^= _rotl32((x[4]  + x[0])  & _M32, 9)
        x[12] ^= _rotl32((x[8]  + x[4])  & _M32, 13)
        x[0]  ^= _rotl32((x[12] + x[8])  & _M32, 18)
        # Column 1: indices 5, 9, 13, 1
        x[9]  ^= _rotl32((x[5]  + x[1])  & _M32, 7)
        x[13] ^= _rotl32((x[9]  + x[5])  & _M32, 9)
        x[1]  ^= _rotl32((x[13] + x[9])  & _M32, 13)
        x[5]  ^= _rotl32((x[1]  + x[13]) & _M32, 18)
        # Column 2: indices 10, 14, 2, 6
        x[14] ^= _rotl32((x[10] + x[6])  & _M32, 7)
        x[2]  ^= _rotl32((x[14] + x[10]) & _M32, 9)
        x[6]  ^= _rotl32((x[2]  + x[14]) & _M32, 13)
        x[10] ^= _rotl32((x[6]  + x[2])  & _M32, 18)
        # Column 3: indices 15, 3, 7, 11
        x[3]  ^= _rotl32((x[15] + x[11]) & _M32, 7)
        x[7]  ^= _rotl32((x[3]  + x[15]) & _M32, 9)
        x[11] ^= _rotl32((x[7]  + x[3])  & _M32, 13)
        x[15] ^= _rotl32((x[11] + x[7])  & _M32, 18)

        # ── Row round ─────────────────────────────────────────────────────────
        # Apply QR(a,b,c,d) to each of the four rows of the 4×4 matrix.
        #
        # Row 0: indices 0, 1, 2, 3
        x[1]  ^= _rotl32((x[0]  + x[3])  & _M32, 7)
        x[2]  ^= _rotl32((x[1]  + x[0])  & _M32, 9)
        x[3]  ^= _rotl32((x[2]  + x[1])  & _M32, 13)
        x[0]  ^= _rotl32((x[3]  + x[2])  & _M32, 18)
        # Row 1: indices 5, 6, 7, 4
        x[6]  ^= _rotl32((x[5]  + x[4])  & _M32, 7)
        x[7]  ^= _rotl32((x[6]  + x[5])  & _M32, 9)
        x[4]  ^= _rotl32((x[7]  + x[6])  & _M32, 13)
        x[5]  ^= _rotl32((x[4]  + x[7])  & _M32, 18)
        # Row 2: QR(10, 11, 8, 9) — a=10, b=11, c=8, d=9
        x[11] ^= _rotl32((x[10] + x[9])  & _M32, 7)
        x[8]  ^= _rotl32((x[11] + x[10]) & _M32, 9)
        x[9]  ^= _rotl32((x[8]  + x[11]) & _M32, 13)
        x[10] ^= _rotl32((x[9]  + x[8])  & _M32, 18)
        # Row 3: QR(15, 12, 13, 14) — a=15, b=12, c=13, d=14
        x[12] ^= _rotl32((x[15] + x[14]) & _M32, 7)
        x[13] ^= _rotl32((x[12] + x[15]) & _M32, 9)
        x[14] ^= _rotl32((x[13] + x[12]) & _M32, 13)
        x[15] ^= _rotl32((x[14] + x[13]) & _M32, 18)

    # Final addition: result[i] = (working[i] + original[i]) mod 2^32
    result = [(x[i] + z[i]) & _M32 for i in range(16)]

    # Serialize back to 64 bytes (little-endian uint32 words)
    return struct.pack("<16I", *result)


# =============================================================================
# XOR Helpers
# =============================================================================


def _xor64(a: bytes, b: bytes) -> bytearray:
    """XOR two 64-byte sequences together, returning a bytearray.

    Used in BlockMix to XOR the running block x with each input block.
    """
    return bytearray(x ^ y for x, y in zip(a, b))


def _xor_blocks(
    x_blocks: list[bytearray],
    v_blocks: list[bytes],
) -> list[bytearray]:
    """XOR two lists of 64-byte blocks element-wise.

    Used in ROMix's lookup phase:
      x = BlockMix(x XOR V[j])
    where XOR applies block-by-block.
    """
    return [bytearray(a ^ b for a, b in zip(xb, vb)) for xb, vb in zip(x_blocks, v_blocks)]


# =============================================================================
# BlockMix
# =============================================================================
#
# BlockMix (RFC 7914 § 4) operates on a sequence of 2r blocks of 64 bytes each
# (total: 128r bytes). It applies Salsa20/8 to each block, XOR-mixing each
# block with the result of the previous Salsa20/8 call, then outputs the blocks
# in an interleaved order:
#
#   Output: [y[0], y[2], y[4], ..., y[2r-2], y[1], y[3], ..., y[2r-1]]
#
# That is, all even-indexed results first, then all odd-indexed results. This
# interleaving is what makes the V-table lookups in ROMix pseudorandom —
# consecutive indices in the V table correspond to non-consecutive blocks.
#
# Algorithm (RFC 7914 § 4):
#   X = B[2r−1]            (start from the last block)
#   for i = 0 to 2r−1:
#     T = X XOR B[i]
#     X = Salsa20/8(T)
#     Y[i] = X
#   B' = [Y[0], Y[2], ..., Y[2r-2]] || [Y[1], Y[3], ..., Y[2r-1]]


def _block_mix(blocks: list[bytearray], r: int) -> list[bytearray]:
    """BlockMix: mix 2r blocks of 64 bytes and return them reordered.

    Parameters
    ----------
    blocks:
        A list of 2r bytearrays, each exactly 64 bytes.
    r:
        The block factor (1 block = 64 bytes, total = 128r bytes).

    Returns
    -------
    list[bytearray]
        A new list of 2r bytearrays in the interleaved output order.
    """
    x = bytes(blocks[-1])    # X = last block (index 2r−1)
    y: list[bytes] = []

    for i in range(2 * r):
        # T = X XOR B[i]; then X = Salsa20/8(T)
        x = _salsa20_8(_xor64(x, blocks[i]))
        y.append(x)

    # Interleave: even-indexed outputs first, then odd-indexed outputs
    return (
        [bytearray(y[i]) for i in range(0, 2 * r, 2)]
        + [bytearray(y[i]) for i in range(1, 2 * r, 2)]
    )


# =============================================================================
# Integerify
# =============================================================================
#
# Integerify reads the last block of the current state (after BlockMix) and
# interprets its first 8 bytes as a little-endian 64-bit integer. This gives
# a pseudorandom index into the V table.
#
# After BlockMix with r=1 (2 blocks total), the output ordering is [y[0], y[1]].
# x[-1] is y[1], which was the last Salsa20/8 output in the odd group.


def _integerify(x: list[bytearray]) -> int:
    """Extract a 64-bit integer from the last block of the state.

    Takes the first 8 bytes of the last block as a little-endian uint64.
    This pseudorandom index is used to pick a row from the V table in ROMix.
    """
    last_block = x[-1]
    return struct.unpack_from("<Q", last_block, 0)[0]


# =============================================================================
# ROMix
# =============================================================================
#
# ROMix (RFC 7914 § 5) is the memory-hard function. It:
#   1. Fills a table V of N copies of the block (snapshot after each BlockMix)
#   2. Performs N pseudorandom lookups into V, XOR-ing and re-mixing each time
#
# The memory requirement is N × 128r bytes. For N=2^20, r=8 that is 1 GiB.
#
# Algorithm (RFC 7914 § 5):
#   X = B
#   for i = 0 to N−1:
#     V[i] = X
#     X = BlockMix(X)
#   for i = 0 to N−1:
#     j = Integerify(X) mod N
#     X = BlockMix(X XOR V[j])
#   B' = X


def _ro_mix(b_bytes: bytes, n: int, r: int) -> bytes:
    """ROMix: the memory-hard function at the heart of scrypt.

    Parameters
    ----------
    b_bytes:
        Input block, exactly 128r bytes.
    n:
        Number of entries in the V table (must be a power of 2).
    r:
        Block factor.

    Returns
    -------
    bytes
        Mixed output, exactly 128r bytes.
    """
    # Parse b_bytes into 2r blocks of 64 bytes each
    x: list[bytearray] = [
        bytearray(b_bytes[i * 64 : (i + 1) * 64]) for i in range(2 * r)
    ]

    # Phase 1: Fill the V table
    # v[i] is a snapshot of x after i BlockMix operations.
    # Storing as tuples of bytes (immutable) to avoid accidental mutation.
    v: list[list[bytes]] = []
    for _ in range(n):
        v.append([bytes(b) for b in x])   # snapshot current x
        x = _block_mix(x, r)              # advance x = BlockMix(x)

    # After the loop: x = BlockMix^N(B), which equals BlockMix(V[N−1]).

    # Phase 2: N pseudorandom lookups
    for _ in range(n):
        j = _integerify(x) % n                 # pick a random row index
        vj = v[j]                              # fetch V[j]
        x = _block_mix(_xor_blocks(x, vj), r) # X = BlockMix(X XOR V[j])

    # Serialize back to bytes
    return b"".join(bytes(b) for b in x)


# =============================================================================
# Public API
# =============================================================================


def scrypt(
    password: bytes,
    salt: bytes,
    n: int,
    r: int,
    p: int,
    dk_len: int,
) -> bytes:
    """Derive a key using scrypt (RFC 7914).

    scrypt is a memory-hard password-based key derivation function. It is more
    resistant to brute-force attacks than PBKDF2 or bcrypt because any attempt
    to crack the password requires large amounts of RAM.

    Parameters
    ----------
    password:
        The secret password. May be empty (RFC 7914 vector 1 uses an empty password).
    salt:
        A random salt. Should be at least 16 bytes; can be any length.
    n:
        CPU/memory cost factor. Must be a power of 2 and >= 2.
        Typical values: 1024 (low), 16384 (interactive), 1048576 (offline).
        The V table in ROMix uses N × 128r bytes of memory.
    r:
        Block size factor. Must be >= 1.
        Typical value: 8 (gives 1 KiB blocks).
    p:
        Parallelism factor. Must be >= 1.
        Typical values: 1 (low), 16 (interactive).
    dk_len:
        Desired key length in bytes. Must be between 1 and 2^20.

    Returns
    -------
    bytes
        Derived key of exactly *dk_len* bytes.

    Raises
    ------
    ValueError
        If any parameter is invalid (see validation section).
    TypeError
        If password or salt is not bytes or bytearray.

    Examples
    --------
    >>> # RFC 7914 Test Vector 1 (verified against Python hashlib.scrypt)
    >>> scrypt(b"", b"", 16, 1, 1, 64).hex()[:16]
    '77d6576238657b20'

    >>> # RFC 7914 Test Vector 2
    >>> # (slow — N=1024, p=16)
    >>> # scrypt(b"password", b"NaCl", 1024, 8, 16, 64).hex()[:16]
    >>> # 'fdbabe1c9d347200'
    """
    # ── Input validation ──────────────────────────────────────────────────────
    if not isinstance(password, (bytes, bytearray)):
        raise TypeError("scrypt password must be bytes or bytearray")
    if not isinstance(salt, (bytes, bytearray)):
        raise TypeError("scrypt salt must be bytes or bytearray")

    # Note: we allow empty password here because RFC 7914 vector 1 uses "".
    # The caller documentation says "non-empty" but the implementation is lenient
    # for RFC compatibility. If you want strict enforcement, uncomment:
    # if len(password) == 0:
    #     raise ValueError("scrypt password must not be empty")

    if not isinstance(n, int) or n < 2 or (n & (n - 1)) != 0:
        raise ValueError("scrypt N must be a power of 2 and >= 2")
    if not isinstance(r, int) or r < 1:
        raise ValueError("scrypt r must be a positive integer")
    if not isinstance(p, int) or p < 1:
        raise ValueError("scrypt p must be a positive integer")
    if not isinstance(dk_len, int) or dk_len < 1 or dk_len > 2**20:
        raise ValueError("scrypt dk_len must be between 1 and 2^20")
    if p * r > 2**30:
        raise ValueError("scrypt p * r exceeds limit (2^30)")
    if n > 2**20:
        raise ValueError("scrypt N must not exceed 2^20")

    # ── Step 1: Expand password+salt into p independent 128r-byte blocks ─────
    #
    # B = PBKDF2-SHA256(password, salt, 1, p × 128 × r)
    #
    # With iterations=1, this is just one HMAC call per output block. The
    # purpose is to stretch the password+salt into exactly the right number of
    # bytes to seed p parallel ROMix operations.
    block_len = 128 * r
    b_total = _pbkdf2_hmac_sha256(password, salt, 1, p * block_len, allow_empty_password=True)

    # Split B into p independent blocks of 128r bytes each
    blocks = [b_total[i * block_len : (i + 1) * block_len] for i in range(p)]

    # ── Step 2: Apply ROMix to each block independently ───────────────────────
    #
    # Each block goes through the full N-entry V table allocation and lookup.
    # This is the memory-hard phase — each ROMix call needs N × 128r bytes.
    mixed = b"".join(_ro_mix(block, n, r) for block in blocks)

    # ── Step 3: Collapse all mixed blocks back to the desired key length ───────
    #
    # DK = PBKDF2-SHA256(password, B', 1, dkLen)
    #
    # The final PBKDF2 call uses the mixed B' as the salt. This collapses the
    # p × 128r bytes of mixed output into exactly dk_len bytes of derived key.
    return _pbkdf2_hmac_sha256(password, mixed, 1, dk_len, allow_empty_password=True)


def scrypt_hex(
    password: bytes,
    salt: bytes,
    n: int,
    r: int,
    p: int,
    dk_len: int,
) -> str:
    """Derive a key using scrypt and return it as a lowercase hex string.

    Convenience wrapper around :func:`scrypt` that hex-encodes the output.

    Parameters
    ----------
    password, salt, n, r, p, dk_len:
        Same as :func:`scrypt`.

    Returns
    -------
    str
        Lowercase hex string of length 2 × dk_len.

    Examples
    --------
    >>> len(scrypt_hex(b"key", b"salt", 16, 1, 1, 32))
    64
    """
    return scrypt(password, salt, n, r, p, dk_len).hex()
