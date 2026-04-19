"""blake2b -- BLAKE2b cryptographic hash function (RFC 7693) from scratch.

What Is BLAKE2b?
================
BLAKE2b is a modern cryptographic hash function that is both *faster* than
MD5 on 64-bit hardware and *as secure* as SHA-3 against known attacks.  It
was designed in 2012 to replace SHA-2 in performance-sensitive contexts
without sacrificing any security margin.

Four features set BLAKE2b apart from the SHA family this package ships
alongside:

  1. Variable output length.  Ask for any number of bytes in [1, 64] and
     BLAKE2b produces a digest of that exact length.  There is no
     separate "BLAKE2b-256" construction -- it is just BLAKE2b with the
     ``digest_size`` parameter set to 32.

  2. Built-in keyed mode.  Supply a key and BLAKE2b becomes a MAC in a
     single pass -- faster than HMAC-SHA-512 and provably as secure.

  3. Salt and personalization folded into the initial state, so you can
     domain-separate two applications that share the same key.

  4. ARX-only (Add, Rotate, XOR) core.  No table lookups, no S-boxes --
     the primitive is identical in spirit to ChaCha20's quarter-round.

How Does It Fit In This Repo?
=============================
BLAKE2b is a prerequisite for Argon2 (the memory-hard password hashing
function).  Argon2 builds on BLAKE2b both for its initial hash step
(H0) and, when the requested output exceeds 64 bytes, for the
"BLAKE2b-long" expansion that this package intentionally does not
include.  BLAKE2b is also a building block for libsodium, WireGuard,
Noise Protocol, and IPFS content addressing, so having a clean
from-scratch implementation gives us a reference for reading those
ecosystems.

The Algorithm In One Diagram
============================
  Input bytes (any length)          Key (optional, 0..64 bytes)
         |                                 |
         |   (if keyed, prepend key block) |
         +-----------------<---------------+
         v
  +----------+----------+----------+
  |  block_0 |  block_1 |   ...    |  (each 128 bytes)
  +----------+----------+----------+
         |
         v
     [h[0..7]] -> F -> F -> ... -> F(final=true) -> digest[:nn]

The state ``h`` is eight 64-bit words, initialized from SHA-512's IVs
XOR-ed with a parameter block that encodes the output length, key
length, salt, and personalization.  The compression function ``F``
mixes one 128-byte block into the state across 12 rounds.  Each round
applies the ``G`` quarter-round to all four columns, then to all four
diagonals -- the same column-then-diagonal pattern ChaCha20 uses.

Key Invariant (and common bug)
==============================
The *last* real block must be compressed with the final flag set.  For
message lengths that are an exact multiple of 128 bytes, do NOT add an
empty padding block -- just flag the last real block.  This differs
from Merkle-Damgard hashes (SHA-2, MD5) that always add a length-
encoding block.  Getting this wrong is the classic BLAKE2 off-by-one.

Reference: RFC 7693 (https://datatracker.ietf.org/doc/html/rfc7693).
Test vectors: RFC 7693 Appendix A and the official BLAKE2 test suite.
"""

from __future__ import annotations

import struct

__version__ = "0.1.0"

# ---- 64-bit mask ----
#
# Python integers are arbitrary precision, so every addition needs to be
# wrapped to 64 bits after the fact.  This constant makes the masking
# self-documenting.
_MASK64 = 0xFFFFFFFFFFFFFFFF

# ---- Initial Hash Values (IVs) ----
#
# Identical to SHA-512's IVs: the fractional parts of the square roots of
# the first eight primes, truncated to 64 bits.  BLAKE2b deliberately
# reuses these "nothing up my sleeve" constants so reviewers can verify
# there is no hidden backdoor simply by checking against SHA-512.
_IV: tuple[int, ...] = (
    0x6A09E667F3BCC908,  # frac(sqrt(2))
    0xBB67AE8584CAA73B,  # frac(sqrt(3))
    0x3C6EF372FE94F82B,  # frac(sqrt(5))
    0xA54FF53A5F1D36F1,  # frac(sqrt(7))
    0x510E527FADE682D1,  # frac(sqrt(11))
    0x9B05688C2B3E6C1F,  # frac(sqrt(13))
    0x1F83D9ABFB41BD6B,  # frac(sqrt(17))
    0x5BE0CD19137E2179,  # frac(sqrt(19))
)

# ---- Message Schedule (SIGMA) ----
#
# Ten permutations of the integers 0..15.  Round ``i`` of the compression
# function uses ``SIGMA[i % 10]`` to pick which message words to mix in
# which order.  Rounds 10 and 11 reuse SIGMA[0] and SIGMA[1] -- there are
# only 10 distinct rows, but the function runs for 12 rounds.
#
# The permutations were chosen by the BLAKE2 designers to spread each
# message word across diverse positions in the working vector within a
# small number of rounds, maximizing diffusion.
_SIGMA: tuple[tuple[int, ...], ...] = (
    ( 0,  1,  2,  3,  4,  5,  6,  7,  8,  9, 10, 11, 12, 13, 14, 15),
    (14, 10,  4,  8,  9, 15, 13,  6,  1, 12,  0,  2, 11,  7,  5,  3),
    (11,  8, 12,  0,  5,  2, 15, 13, 10, 14,  3,  6,  7,  1,  9,  4),
    ( 7,  9,  3,  1, 13, 12, 11, 14,  2,  6,  5, 10,  4,  0, 15,  8),
    ( 9,  0,  5,  7,  2,  4, 10, 15, 14,  1, 11, 12,  6,  8,  3, 13),
    ( 2, 12,  6, 10,  0, 11,  8,  3,  4, 13,  7,  5, 15, 14,  1,  9),
    (12,  5,  1, 15, 14, 13,  4, 10,  0,  7,  6,  3,  9,  2,  8, 11),
    (13, 11,  7, 14, 12,  1,  3,  9,  5,  0, 15,  4,  8,  6,  2, 10),
    ( 6, 15, 14,  9, 11,  3,  0,  8, 12,  2, 13,  7,  1,  4, 10,  5),
    (10,  2,  8,  4,  7,  6,  1,  5, 15, 11,  9, 14,  3, 12, 13,  0),
)

# Block size in bytes.  Every compression call consumes exactly 128 bytes.
_BLOCK_SIZE = 128


def _rotr64(x: int, n: int) -> int:
    """Circular right shift of ``x`` by ``n`` bits within a 64-bit word.

    Bits that "fall off" the right wrap back to the top.  The mask to
    ``_MASK64`` is required because Python's integers are unbounded -- a
    shift left on a 64-bit value would otherwise produce a 128-bit-wide
    result.
    """
    return ((x >> n) | (x << (64 - n))) & _MASK64


def _G(
    v: list[int],
    a: int,
    b: int,
    c: int,
    d: int,
    x: int,
    y: int,
) -> None:
    """The BLAKE2b quarter-round.

    Mutates four words ``v[a], v[b], v[c], v[d]`` of the working vector
    by mixing them with two message words ``x`` and ``y``.  Uses only
    additions, XORs, and rotations -- the "ARX" primitive family.

    Rotation constants (R1, R2, R3, R4) = (32, 24, 16, 63).  These are
    RFC 7693 Appendix D values; changing any one breaks compatibility.

    Why this works cryptographically::

        +  (add)      non-linear in GF(2) because carries propagate
        X  (xor)      linear in GF(2) -- cheap diffusion
        R  (rotate)   moves bits across word boundaries so additions
                      and XORs affect each other on later rounds

    Composing ARX steps defeats linear and differential cryptanalysis
    without any S-box or table lookup.
    """
    v[a] = (v[a] + v[b] + x) & _MASK64
    v[d] = _rotr64(v[d] ^ v[a], 32)
    v[c] = (v[c] + v[d]) & _MASK64
    v[b] = _rotr64(v[b] ^ v[c], 24)
    v[a] = (v[a] + v[b] + y) & _MASK64
    v[d] = _rotr64(v[d] ^ v[a], 16)
    v[c] = (v[c] + v[d]) & _MASK64
    v[b] = _rotr64(v[b] ^ v[c], 63)


def _F(
    h: list[int],
    block: bytes,
    t: int,
    final: bool,
) -> None:
    """Compression function: mix one 128-byte block into the state.

    Parameters
    ----------
    h     : eight-word state, mutated in place.
    block : 128 bytes (the current message block, zero-padded if last).
    t     : total byte count fed into the hash so far, *including* the
            bytes in this block (even if the block itself was zero-
            padded).  Stored as 128 bits, split across ``v[12]`` and
            ``v[13]``.
    final : True iff this is the last compression call for this hash.
            Triggers the ``v[14]`` inversion that differentiates the
            final block from any intermediate block -- preventing
            length-extension attacks at the construction level.
    """
    # Parse the block as sixteen little-endian 64-bit words.
    m = struct.unpack("<16Q", block)

    # Working vector: 8 state words followed by 8 IV words.
    v = list(h) + list(_IV)

    # Fold the byte counter into v[12..13].  The counter is 128-bit, so we
    # split it into a low and high 64-bit half.
    v[12] ^= t & _MASK64
    v[13] ^= (t >> 64) & _MASK64

    # On the last block, invert v[14] to make the final compression
    # distinguishable from any intermediate one.  v[15] is reserved for
    # tree hashing, which this spec does not implement.
    if final:
        v[14] ^= _MASK64

    # Twelve rounds.  Each round applies G to four columns, then to
    # four diagonals.  The column-then-diagonal pattern is identical to
    # ChaCha20's "double round".
    for i in range(12):
        s = _SIGMA[i % 10]
        # Columns
        _G(v, 0, 4,  8, 12, m[s[ 0]], m[s[ 1]])
        _G(v, 1, 5,  9, 13, m[s[ 2]], m[s[ 3]])
        _G(v, 2, 6, 10, 14, m[s[ 4]], m[s[ 5]])
        _G(v, 3, 7, 11, 15, m[s[ 6]], m[s[ 7]])
        # Diagonals
        _G(v, 0, 5, 10, 15, m[s[ 8]], m[s[ 9]])
        _G(v, 1, 6, 11, 12, m[s[10]], m[s[11]])
        _G(v, 2, 7,  8, 13, m[s[12]], m[s[13]])
        _G(v, 3, 4,  9, 14, m[s[14]], m[s[15]])

    # Feed-forward: XOR both halves of the working vector back into the
    # state.  This Davies-Meyer-style step makes the compression
    # function one-way -- even an attacker who can run all 12 rounds
    # backward cannot invert the final XOR because they do not know the
    # pre-image of the mix.
    for i in range(8):
        h[i] ^= v[i] ^ v[i + 8]


def _validate(
    digest_size: int,
    key: bytes,
    salt: bytes,
    personal: bytes,
) -> None:
    """Argument validation shared by the one-shot API and the hasher."""
    if not 1 <= digest_size <= 64:
        raise ValueError(
            f"digest_size must be in [1, 64], got {digest_size}"
        )
    if len(key) > 64:
        raise ValueError(f"key length must be in [0, 64], got {len(key)}")
    if salt and len(salt) != 16:
        raise ValueError(
            f"salt must be exactly 16 bytes (or empty), got {len(salt)}"
        )
    if personal and len(personal) != 16:
        raise ValueError(
            f"personal must be exactly 16 bytes (or empty), got "
            f"{len(personal)}"
        )


def _initial_state(
    digest_size: int,
    key_len: int,
    salt: bytes,
    personal: bytes,
) -> list[int]:
    """Build the parameter-block-XOR-ed initial state.

    The parameter block is 64 bytes, laid out as eight 64-bit little-
    endian words (RFC 7693 section 2.5)::

        byte offset   field             size (bytes)
        0             digest_length     1
        1             key_length        1
        2             fanout            1   (sequential: 1)
        3             depth             1   (sequential: 1)
        4-7           leaf_length       4   (sequential: 0)
        8-15          node_offset       8   (sequential: 0)
        16            node_depth        1   (sequential: 0)
        17            inner_length      1   (sequential: 0)
        18-31         reserved          14
        32-47         salt              16
        48-63         personal          16

    We XOR each 64-bit parameter word into the matching IV word to get
    the initial state ``h``.
    """
    p = bytearray(64)
    p[0] = digest_size
    p[1] = key_len
    p[2] = 1  # fanout = 1 (sequential)
    p[3] = 1  # depth  = 1 (sequential)
    # bytes 4..31 stay zero for sequential mode
    if salt:
        p[32:48] = salt
    if personal:
        p[48:64] = personal

    param_words = struct.unpack("<8Q", bytes(p))
    return [iv ^ pw for iv, pw in zip(_IV, param_words, strict=True)]


class Blake2bHasher:
    """Streaming BLAKE2b hasher.

    Usage::

        h = Blake2bHasher(digest_size=32, key=b"my_key")
        h.update(b"hello")
        h.update(b" world")
        h.hex_digest()   # -> 64-char hex string

    Multiple calls to ``digest()`` or ``hex_digest()`` return the same
    value.  ``update()`` remains callable after ``digest()``.
    """

    def __init__(
        self,
        digest_size: int = 64,
        key: bytes = b"",
        salt: bytes = b"",
        personal: bytes = b"",
    ) -> None:
        _validate(digest_size, key, salt, personal)
        self._digest_size = digest_size
        self._state: list[int] = _initial_state(
            digest_size, len(key), salt, personal
        )
        # Buffer always holds strictly less than one full block after
        # ``update()`` returns (unless empty at construction).  We must
        # *not* compress a full block immediately on fill, because we
        # do not yet know whether it is the last block -- and the last
        # block is the one that must be flagged final.
        self._buffer = bytearray()
        self._byte_count = 0  # total bytes fed through F (excludes unflushed buffer)

        if key:
            # Keyed mode: the key, zero-padded to a full block, is the
            # first compression input.  Subsequent updates treat the
            # padded key as if it had been the opening of the message.
            key_block = bytearray(_BLOCK_SIZE)
            key_block[: len(key)] = key
            self._buffer.extend(key_block)

    def update(self, data: bytes) -> Blake2bHasher:
        """Feed more bytes into the hash.  Returns self for chaining.

        We compress any *full* blocks that are provably not the last
        one: whenever the buffer has more than one block's worth of
        data, the leading block cannot be the final block and can be
        flushed safely.
        """
        self._buffer.extend(data)
        # Keep at least one byte in the buffer so ``digest()`` always
        # has a final block to flag final.  Equivalently: flush only
        # when the buffer strictly exceeds one block.
        while len(self._buffer) > _BLOCK_SIZE:
            self._byte_count += _BLOCK_SIZE
            _F(
                self._state,
                bytes(self._buffer[:_BLOCK_SIZE]),
                self._byte_count,
                final=False,
            )
            del self._buffer[:_BLOCK_SIZE]
        return self

    def digest(self) -> bytes:
        """Finalize (non-destructively) and return the digest bytes.

        We copy the state so repeated calls to ``digest()`` all return
        the same value and additional ``update()`` calls can still be
        made afterward.  The buffer and byte count are likewise left
        untouched -- only temporary variables are finalized.
        """
        # Copy state so repeated digest() calls produce identical output.
        state = list(self._state)
        # The remaining buffer is the final block.  Zero-pad to 128 bytes.
        final_block = bytearray(self._buffer)
        byte_count = self._byte_count + len(final_block)
        final_block.extend(b"\x00" * (_BLOCK_SIZE - len(final_block)))
        _F(state, bytes(final_block), byte_count, final=True)
        # Serialize state as little-endian 64-bit words and truncate.
        full = struct.pack("<8Q", *state)
        return full[: self._digest_size]

    def hex_digest(self) -> str:
        """Return the digest as a lowercase hex string."""
        return self.digest().hex()

    def copy(self) -> Blake2bHasher:
        """Return an independent deep copy of this hasher.

        Useful for hashing multiple messages that share a common prefix::

            h = Blake2bHasher()
            h.update(common_prefix)
            h_a = h.copy(); h_a.update(b"suffix_a")
            h_b = h.copy(); h_b.update(b"suffix_b")
        """
        other = Blake2bHasher.__new__(Blake2bHasher)
        other._digest_size = self._digest_size
        other._state = list(self._state)
        other._buffer = bytearray(self._buffer)
        other._byte_count = self._byte_count
        return other


def blake2b(
    data: bytes,
    digest_size: int = 64,
    key: bytes = b"",
    salt: bytes = b"",
    personal: bytes = b"",
) -> bytes:
    """One-shot BLAKE2b.  Returns ``digest_size`` bytes.

    Example::

        >>> blake2b(b"abc").hex()[:16]
        'ba80a53f981c4d0d'
        >>> len(blake2b(b"", digest_size=32))
        32
    """
    h = Blake2bHasher(
        digest_size=digest_size, key=key, salt=salt, personal=personal
    )
    h.update(data)
    return h.digest()


def blake2b_hex(
    data: bytes,
    digest_size: int = 64,
    key: bytes = b"",
    salt: bytes = b"",
    personal: bytes = b"",
) -> str:
    """One-shot BLAKE2b returning lowercase hex."""
    return blake2b(data, digest_size, key, salt, personal).hex()
