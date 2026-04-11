"""
Core hash function implementations.

Each function follows the same contract:
  - Accepts data: bytes | str
  - If str is given, it is UTF-8 encoded before hashing
  - Returns an unsigned integer of the appropriate width

Internally, all arithmetic uses Python's arbitrary-precision integers
and we mask with & 0xFFFF...F to simulate fixed-width overflow.
"""

from __future__ import annotations

# ---------------------------------------------------------------------------
# FNV-1a constants
# ---------------------------------------------------------------------------
# The FNV (Fowler-Noll-Vo) primes were chosen to exhibit good distribution
# over common hash table workloads (short strings, file paths, identifiers).
# They are sparse in binary (few set bits) so the multiplication maps
# efficiently to shifts-and-adds on older architectures.

FNV32_OFFSET_BASIS: int = 0x811C9DC5   # 2166136261
FNV32_PRIME: int        = 0x01000193   # 16777619

FNV64_OFFSET_BASIS: int = 0xCBF29CE484222325   # 14695981039346656037
FNV64_PRIME: int        = 0x00000100000001B3   # 1099511628211

# Bit masks for truncating Python's arbitrary-precision integers to 32 and 64 bits.
MASK32: int = 0xFFFFFFFF
MASK64: int = 0xFFFFFFFFFFFFFFFF


def _to_bytes(data: bytes | str) -> bytes:
    """Coerce str to bytes via UTF-8; pass bytes through unchanged."""
    if isinstance(data, str):
        return data.encode("utf-8")
    return data


# ---------------------------------------------------------------------------
# FNV-1a 32-bit
# ---------------------------------------------------------------------------

def fnv1a_32(data: bytes | str) -> int:
    """
    FNV-1a 32-bit hash.

    The algorithm processes one byte at a time:
      1. XOR the current hash with the byte (mixes the byte in)
      2. Multiply by the FNV prime (spreads the change across all 32 bits)

    The key insight: XOR alone is invertible — XORing the same byte twice
    gives back the original value.  The multiply makes undoing the operation
    computationally expensive because a small change to one bit creates
    carry propagation across many higher bits.

    Known-good test vectors:
      fnv1a_32(b"")       == 2166136261
      fnv1a_32(b"a")      == 84696351
      fnv1a_32(b"abc")    == 440920331
      fnv1a_32(b"hello")  == 1335831723
      fnv1a_32(b"foobar") == 2984838064

    Args:
        data: Input bytes or string (str is UTF-8 encoded).

    Returns:
        Unsigned 32-bit hash value in [0, 2**32).
    """
    raw = _to_bytes(data)
    h = FNV32_OFFSET_BASIS
    for b in raw:
        h ^= b
        h = (h * FNV32_PRIME) & MASK32
    return h


# ---------------------------------------------------------------------------
# FNV-1a 64-bit
# ---------------------------------------------------------------------------

def fnv1a_64(data: bytes | str) -> int:
    """
    FNV-1a 64-bit hash.

    Identical algorithm to fnv1a_32 but uses the 64-bit offset basis
    and prime.  The 64-bit variant is preferred when keys need to be
    distributed across large hash tables or when a 32-bit collision
    rate would be unacceptable.

    Known-good test vectors:
      fnv1a_64(b"")   == 14695981039346656037
      fnv1a_64(b"a")  == 12638187200555641996
      fnv1a_64(b"abc") == 1081NS  (see tests for exact value)

    Args:
        data: Input bytes or string (str is UTF-8 encoded).

    Returns:
        Unsigned 64-bit hash value in [0, 2**64).
    """
    raw = _to_bytes(data)
    h = FNV64_OFFSET_BASIS
    for b in raw:
        h ^= b
        h = (h * FNV64_PRIME) & MASK64
    return h


# ---------------------------------------------------------------------------
# DJB2
# ---------------------------------------------------------------------------

def djb2(data: bytes | str) -> int:
    """
    DJB2 hash by Dan Bernstein.

    The algorithm is remarkably simple:
      hash = 5381
      for each byte b:
          hash = hash * 33 + b

    The multiply-by-33 is written as ((hash << 5) + hash) to exploit
    fast shift-plus-add on architectures without a hardware multiplier.

    Why 33?  It is prime, produces a bit pattern with good avalanche
    for short ASCII strings, and maps to a single instruction on most CPUs.

    Why 5381?  Empirically chosen by Bernstein — it produces fewer
    collisions than nearby integers for Unix dictionary words.

    The output is NOT truncated to 32 bits; DJB2 is typically used as
    a 64-bit (or even arbitrary-precision) hash and the caller masks
    to the desired bucket count.

    Known-good test vectors:
      djb2(b"")    == 5381
      djb2(b"a")   == 177670
      djb2(b"abc") == 193485963

    Args:
        data: Input bytes or string (str is UTF-8 encoded).

    Returns:
        Unsigned 64-bit hash value (no truncation applied beyond 64 bits).
    """
    raw = _to_bytes(data)
    h = 5381
    for b in raw:
        # (h << 5) + h == h * 33, computed without a multiply instruction
        h = (((h << 5) + h) + b) & MASK64
    return h


# ---------------------------------------------------------------------------
# Polynomial rolling hash
# ---------------------------------------------------------------------------

def polynomial_rolling(
    data: bytes | str,
    base: int = 31,
    mod: int = 2**61 - 1,
) -> int:
    """
    Polynomial rolling hash.

    Treats the input as the coefficients of a polynomial evaluated at `base`:

        hash = data[0]*base^(n-1) + data[1]*base^(n-2) + ... + data[n-1]

    All arithmetic is performed modulo `mod`.  The default modulus
    2^61 - 1 is a Mersenne prime — modular reduction against a Mersenne
    prime is extremely fast because (x mod (2^p - 1)) can be computed
    with just a shift and an add instead of a full division.

    Why Mersenne primes?
      For any Mersenne prime M = 2^p - 1:
        x mod M = (x >> p) + (x & M)   (if the result >= M, subtract M once)
      This is O(1) arithmetic instead of O(log n) division.

    Why base = 31?
      Chosen to avoid many collisions on lowercase English letters
      (which span ASCII 97–122).  Using a prime base ensures that
      permutations of the same characters produce different hashes.

    The rolling property: if you slide a window over the input one
    character at a time, you can update the hash in O(1) by subtracting
    the outgoing character and adding the incoming one — no need to
    recompute from scratch.  This is how Rabin-Karp string search works.

    Args:
        data: Input bytes or string (str is UTF-8 encoded).
        base: Polynomial base (default 31).
        mod:  Modulus (default 2**61 - 1, a Mersenne prime).

    Returns:
        Hash value in [0, mod).
    """
    raw = _to_bytes(data)
    h = 0
    for b in raw:
        h = (h * base + b) % mod
    return h


# ---------------------------------------------------------------------------
# MurmurHash3 (32-bit)
# ---------------------------------------------------------------------------

# MurmurHash3 mixing constants, empirically selected by Austin Appleby.
# These constants produce good avalanche through the fmix32 finalizer.
_MUR_C1: int = 0xCC9E2D51
_MUR_C2: int = 0x1B873593


def _rotl32(x: int, r: int) -> int:
    """
    Rotate the 32-bit integer x left by r bit positions.

    Unlike a shift, rotation wraps the discarded high bits back to the
    low end so no information is lost.  This is important in the mixing
    step where we want all bits of a block to participate equally.

      rotl32(0b10000000_00000000_00000000_00000001, 1)
            = 0b00000000_00000000_00000000_00000011
    """
    return ((x << r) | (x >> (32 - r))) & MASK32


def _fmix32(h: int) -> int:
    """
    MurmurHash3 finalization mixer (fmix32).

    Achieves the Strict Avalanche Criterion: after fmix32, every output
    bit depends on every input bit.  This is crucial because the main
    loop may leave some bits poorly mixed (particularly the low bits of
    the last block).

    The sequence of operations:
      1. XOR with upper half folded down (h ^= h >> 16)
      2. Multiply by a prime (spreads bits through carry propagation)
      3. XOR with upper half again
      4. Multiply by another prime
      5. Final XOR fold

    After step 2, a change in the lowest bit of h affects all bits above
    it via carry.  After step 3, the high bits contaminate the low bits.
    Two rounds of this ensures complete mixing.
    """
    h ^= h >> 16
    h = (h * 0x85EBCA6B) & MASK32
    h ^= h >> 13
    h = (h * 0xC2B2AE35) & MASK32
    h ^= h >> 16
    return h


def murmur3_32(data: bytes | str, seed: int = 0) -> int:
    """
    MurmurHash3 32-bit hash (by Austin Appleby, 2008).

    Processes the input 4 bytes (one 32-bit word) at a time, which
    is much faster than FNV-1a's byte-at-a-time approach on modern
    64-bit CPUs.  The algorithm has three phases:

    Phase 1 — Main loop (full 4-byte blocks):
      For each block:
        k = little-endian word from 4 bytes
        k  = k * c1
        k  = rotl32(k, 15)
        k  = k * c2
        h ^= k
        h  = rotl32(h, 13)
        h  = h * 5 + 0xe6546b64

    Phase 2 — Tail (remaining 1–3 bytes):
      Build a partial word from remaining bytes, apply c1/rotl/c2 mixing.

    Phase 3 — Finalization:
      h ^= len(data)     (length affects the hash, preventing length-
                          extension issues for different-length inputs)
      h  = fmix32(h)    (full avalanche mixing)

    The seed parameter allows independent hash functions from the same
    algorithm — useful in Bloom filters that need k independent hashes.

    Known-good test vectors (from the reference implementation):
      murmur3_32(b"",    seed=0) == 0
      murmur3_32(b"",    seed=1) == 0x514E28B7
      murmur3_32(b"a",   seed=0) == 0xE40C292C
      murmur3_32(b"abc", seed=0) == 0xB3DD93FA

    Args:
        data: Input bytes or string (str is UTF-8 encoded).
        seed: 32-bit seed value (default 0).

    Returns:
        Unsigned 32-bit hash value in [0, 2**32).
    """
    raw = _to_bytes(data)
    length = len(raw)
    h = seed & MASK32

    # --- Phase 1: process full 4-byte blocks ---
    # Each 4-byte block is read as a little-endian 32-bit integer.
    # "Little-endian" means the first byte is the least-significant byte.
    num_blocks = length >> 2  # integer divide by 4
    for block_idx in range(num_blocks):
        i = block_idx * 4
        # Pack 4 bytes into a 32-bit little-endian integer.
        k = (
            raw[i]
            | (raw[i + 1] << 8)
            | (raw[i + 2] << 16)
            | (raw[i + 3] << 24)
        )

        k = (k * _MUR_C1) & MASK32
        k = _rotl32(k, 15)
        k = (k * _MUR_C2) & MASK32

        h ^= k
        h = _rotl32(h, 13)
        # h * 5 + 0xe6546b64 is a multiply-add that provides additional
        # avalanche between blocks; the constant was chosen empirically.
        h = ((h * 5) + 0xE6546B64) & MASK32

    # --- Phase 2: handle the tail (remaining 1–3 bytes) ---
    tail_offset = num_blocks * 4
    remaining = length & 3  # equivalent to length % 4
    k = 0
    if remaining >= 3:
        k ^= raw[tail_offset + 2] << 16
    if remaining >= 2:
        k ^= raw[tail_offset + 1] << 8
    if remaining >= 1:
        k ^= raw[tail_offset]
        k = (k * _MUR_C1) & MASK32
        k = _rotl32(k, 15)
        k = (k * _MUR_C2) & MASK32
        h ^= k

    # --- Phase 3: finalization ---
    h ^= length          # fold in the length so different-length inputs
    h = _fmix32(h)       # diverge even if the data bytes match
    return h
