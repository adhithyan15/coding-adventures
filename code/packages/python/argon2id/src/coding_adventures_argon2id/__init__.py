"""Argon2id — hybrid memory-hard password hashing (RFC 9106).

Argon2id is the recommended default Argon2 variant: it uses the
side-channel-resistant *data-independent* addressing (Argon2i) for
the first half of the first pass, and the GPU/ASIC-resistant
*data-dependent* addressing (Argon2d) for everything after.  The
algorithm is specified in RFC 9106 and in this repo's
``code/specs/KD03-argon2.md``.

This package depends on ``coding_adventures_blake2b`` for the outer
BLAKE2b calls (``H0`` and the variable-length ``H'``); the inner
compression round is an Argon2-specific modification of BLAKE2b's
round (an integer multiplication term replaces SIGMA), so it is
inlined here rather than imported from the BLAKE2b package.

The public surface is intentionally tiny: one function pair
(:func:`argon2id`, :func:`argon2id_hex`), both one-shot.  There is no
streaming hasher because Argon2's inputs are absorbed in a single
``H0`` call.
"""

from __future__ import annotations

import struct

from coding_adventures_blake2b import blake2b

__all__ = ["argon2id", "argon2id_hex"]

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------
#
# All arithmetic inside the compression function is 64-bit unsigned with
# wrap-on-overflow; Python's ``int`` is arbitrary-precision, so we mask
# explicitly at every step.  ``MASK32`` is used for the Argon2-specific
# ``trunc32`` in ``G_B`` and for extracting the low/high halves of block
# word zero in the Argon2d addressing path.

MASK64 = 0xFFFFFFFFFFFFFFFF
MASK32 = 0xFFFFFFFF

BLOCK_SIZE = 1024                 # bytes
BLOCK_WORDS = BLOCK_SIZE // 8     # = 128 × u64
SYNC_POINTS = 4                   # slices per pass
ADDRESSES_PER_BLOCK = BLOCK_WORDS  # = 128

VERSION = 0x13                    # Argon2 v1.3, the only supported version

TYPE_D = 0
TYPE_I = 1
TYPE_ID = 2

# ---------------------------------------------------------------------------
# BLAKE2b round (Argon2 flavour)
# ---------------------------------------------------------------------------
#
# Argon2's permutation ``P`` is one round of the BLAKE2b mixing schedule
# with two deviations from RFC 7693:
#
# 1. There is no SIGMA — each word is addressed directly by position,
#    as though we were consuming ``m[0..15]`` with the identity
#    permutation.  (Equivalently: the message schedule is absent.)
# 2. Every addition is augmented with ``2 * trunc32(a) * trunc32(b)``.
#    This is Argon2's only cryptographic addition on top of BLAKE2b,
#    and it is what makes ``G`` non-linear in a way that isn't
#    invertible by an attacker who knows only one operand.
#
# Rotation constants ``(32, 24, 16, 63)`` are identical to BLAKE2b's.


def _rotr64(x: int, n: int) -> int:
    """Rotate a 64-bit unsigned integer ``x`` right by ``n`` bits."""
    return ((x >> n) | (x << (64 - n))) & MASK64


def _GB(v: list[int], a: int, b: int, c: int, d: int) -> None:
    """Argon2's G_B — one quarter-round of the modified BLAKE2b mixer.

    Mutates ``v`` in place.  All arithmetic is 64-bit unsigned with
    wrap-on-overflow; the ``& MASK64`` mask simulates ``u64`` semantics
    on top of Python's arbitrary-precision ints.
    """
    va, vb, vc, vd = v[a], v[b], v[c], v[d]

    va = (va + vb + 2 * (va & MASK32) * (vb & MASK32)) & MASK64
    vd = _rotr64(vd ^ va, 32)
    vc = (vc + vd + 2 * (vc & MASK32) * (vd & MASK32)) & MASK64
    vb = _rotr64(vb ^ vc, 24)
    va = (va + vb + 2 * (va & MASK32) * (vb & MASK32)) & MASK64
    vd = _rotr64(vd ^ va, 16)
    vc = (vc + vd + 2 * (vc & MASK32) * (vd & MASK32)) & MASK64
    vb = _rotr64(vb ^ vc, 63)

    v[a], v[b], v[c], v[d] = va, vb, vc, vd


def _P(v: list[int]) -> None:
    """Permutation ``P`` — one BLAKE2b round applied to 16 × u64.

    The column step mixes each of the four columns of the 4×4 grid of
    words; the diagonal step mixes each of the four diagonals.  The
    same 8-call pattern BLAKE2b uses per round.
    """
    # Column step
    _GB(v, 0, 4, 8, 12)
    _GB(v, 1, 5, 9, 13)
    _GB(v, 2, 6, 10, 14)
    _GB(v, 3, 7, 11, 15)
    # Diagonal step
    _GB(v, 0, 5, 10, 15)
    _GB(v, 1, 6, 11, 12)
    _GB(v, 2, 7, 8, 13)
    _GB(v, 3, 4, 9, 14)


# ---------------------------------------------------------------------------
# Compression function G
# ---------------------------------------------------------------------------
#
# ``G(X, Y)`` takes two 1024-byte blocks (represented as lists of 128
# ``u64`` words) and returns a new 1024-byte block.  The block is
# logically an 8 × 8 matrix of 128-bit registers — equivalently, 8 rows
# × 8 double-column pairs of ``u64`` words.  ``P`` is applied to each of
# the 8 rows (a row is 16 contiguous words), then to each of the 8
# "columns" (gathered as word pairs at stride 16, starting at offset
# ``2*c``).  The feed-forward ``R XOR Q`` mirrors BLAKE2b's
# Davies-Meyer construction and is what keeps ``G`` non-invertible.


def _G(X: list[int], Y: list[int]) -> list[int]:
    R = [X[i] ^ Y[i] for i in range(BLOCK_WORDS)]
    Q = R.copy()

    # Row pass — apply P to each of the 8 rows (16 words each).
    for i in range(8):
        row = Q[i * 16 : (i + 1) * 16]
        _P(row)
        Q[i * 16 : (i + 1) * 16] = row

    # Column pass — gather pairs at stride 16 starting at 2c.
    for c in range(8):
        col = []
        for r in range(8):
            col.append(Q[r * 16 + 2 * c])
            col.append(Q[r * 16 + 2 * c + 1])
        _P(col)
        for r in range(8):
            Q[r * 16 + 2 * c] = col[2 * r]
            Q[r * 16 + 2 * c + 1] = col[2 * r + 1]

    return [R[i] ^ Q[i] for i in range(BLOCK_WORDS)]


# ---------------------------------------------------------------------------
# Block serialisation
# ---------------------------------------------------------------------------


def _block_to_bytes(block: list[int]) -> bytes:
    """Serialise a 128 × u64 block as 1024 little-endian bytes."""
    return struct.pack("<128Q", *block)


def _bytes_to_block(data: bytes) -> list[int]:
    """Parse 1024 bytes (little-endian) back into 128 × u64."""
    if len(data) != BLOCK_SIZE:
        raise ValueError(f"block must be {BLOCK_SIZE} bytes, got {len(data)}")
    return list(struct.unpack("<128Q", data))


# ---------------------------------------------------------------------------
# H' — variable-length BLAKE2b (RFC 9106 §3.3)
# ---------------------------------------------------------------------------
#
# For ``T <= 64`` bytes, ``H'`` is just ``BLAKE2b-T(LE32(T) || X)``.
#
# For ``T > 64``, ``H'`` emits ``T`` bytes by chaining 64-byte BLAKE2b
# outputs and keeping the first 32 bytes of each intermediate hash, then
# finishing with a final variable-size BLAKE2b that produces the tail
# ``(T - 32r)`` bytes, where ``r = ceil(T / 32) - 2``.
#
# The total length is exactly ``32 * r + (T - 32 * r) = T`` — no padding,
# no truncation.  The 32-byte overlap between consecutive outputs is
# Argon2's way of building a length-extensible hash from a fixed-size
# primitive.


def _blake2b_long(T: int, X: bytes) -> bytes:
    if T <= 0:
        raise ValueError(f"H' output length must be positive, got {T}")
    T_prefix = struct.pack("<I", T)

    if T <= 64:
        return blake2b(T_prefix + X, digest_size=T)

    r = (T + 31) // 32 - 2  # ceil(T/32) - 2
    V = blake2b(T_prefix + X, digest_size=64)  # V_1
    out = bytearray(V[:32])
    for _ in range(r - 1):
        V = blake2b(V, digest_size=64)  # V_2..V_r
        out += V[:32]
    final_size = T - 32 * r
    V = blake2b(V, digest_size=final_size)   # V_{r+1}
    out += V
    return bytes(out)


# ---------------------------------------------------------------------------
# index_alpha — map J1 to a reference column (RFC 9106 §3.4.1.1)
# ---------------------------------------------------------------------------


def _index_alpha(
    J1: int,
    r: int,
    sl: int,
    c: int,
    same_lane: bool,
    q: int,
    SL: int,
) -> int:
    """Map a 32-bit ``J1`` to a reference-block column within the lane
    selected by ``l'``.  ``c`` is the within-segment offset (0..SL-1),
    ``sl`` is the slice number, ``r`` is the pass number, and ``q`` is
    the lane length (``m' / p``).
    """
    # Size of the reference set (W) and start of the window.
    if r == 0:
        if sl == 0:
            # First slice of first pass — only same-lane prefix is valid.
            W = c - 1
            start = 0
        else:
            W = sl * SL + c - 1 if same_lane else sl * SL - (1 if c == 0 else 0)
            start = 0
    else:
        W = q - SL + c - 1 if same_lane else q - SL - (1 if c == 0 else 0)
        start = ((sl + 1) * SL) % q

    # Biased relative position (the J1^2 squashing).
    x = (J1 * J1) >> 32
    y = (W * x) >> 32
    rel = W - 1 - y

    return (start + rel) % q


# ---------------------------------------------------------------------------
# Argon2 core fill
# ---------------------------------------------------------------------------


def _fill_segment(
    memory: list[list[list[int]]],
    r: int,
    lane: int,
    sl: int,
    q: int,
    SL: int,
    p: int,
    m_prime: int,
    t: int,
    mem_type: int,
) -> None:
    """Fill one (pass, slice, lane) segment of the memory matrix.

    For Argon2id the addressing mode is chosen per-segment: the very
    first two slices of pass 0 use data-independent (Argon2i) addresses;
    everything afterwards uses data-dependent (Argon2d) addressing.
    """
    data_independent = (
        mem_type == TYPE_I
        or (mem_type == TYPE_ID and r == 0 and sl < 2)
    )

    # Argon2i state: an "input block" seeded with (pass, lane, slice,
    # total memory, total passes, type, counter, 0, 0, ...).  The
    # counter is bumped once per address-block generation within this
    # segment, giving up to 128 (J1, J2) pairs per bump.
    input_block: list[int] = [0] * BLOCK_WORDS
    address_block: list[int] = [0] * BLOCK_WORDS
    zero_block: list[int] = [0] * BLOCK_WORDS
    if data_independent:
        input_block[0] = r
        input_block[1] = lane
        input_block[2] = sl
        input_block[3] = m_prime
        input_block[4] = t
        input_block[5] = mem_type
        # input_block[6] (counter) stays 0 — bumped in _next_addresses.

    def _next_addresses() -> None:
        # counter++ then double-G with the zero block.
        input_block[6] = (input_block[6] + 1) & MASK64
        Z = _G(zero_block, input_block)
        addr = _G(zero_block, Z)
        # Copy into address_block (can't rebind the closure variable).
        for k in range(BLOCK_WORDS):
            address_block[k] = addr[k]

    # Skip the two pre-filled slots at the start of pass 0 slice 0.
    starting_c = 2 if (r == 0 and sl == 0) else 0
    # For pass 0 slice 0 the reference implementation calls
    # _next_addresses ONCE before the loop so the first address block is
    # available even though i=2 won't trigger the ``i % 128 == 0`` test.
    if data_independent and starting_c != 0:
        _next_addresses()

    for i in range(starting_c, SL):
        # Regenerate a fresh address block every 128 slots, skipping
        # ``i == 2`` in the pass-0-slice-0 case because we primed the
        # block before entering the loop (SL is typically << 128 so
        # this boundary is otherwise never hit).
        if (
            data_independent
            and i % ADDRESSES_PER_BLOCK == 0
            and not (r == 0 and sl == 0 and i == 2)
        ):
            _next_addresses()

        col = sl * SL + i          # absolute column in the lane
        prev_col = col - 1 if col > 0 else q - 1
        prev_block = memory[lane][prev_col]

        # Pull (J1, J2) — either from the address block (Argon2i path)
        # or from the first word of the previous block (Argon2d path).
        if data_independent:
            pseudo_rand = address_block[i % ADDRESSES_PER_BLOCK]
        else:
            pseudo_rand = prev_block[0]
        J1 = pseudo_rand & MASK32
        J2 = (pseudo_rand >> 32) & MASK32

        # Reference lane: forced same-lane for the very first slice of
        # pass 0, else chosen by J2 mod p.
        l_prime = lane if r == 0 and sl == 0 else J2 % p

        z_prime = _index_alpha(J1, r, sl, i, l_prime == lane, q, SL)
        ref_block = memory[l_prime][z_prime]

        new_block = _G(prev_block, ref_block)
        if r == 0:
            memory[lane][col] = new_block
        else:
            # Pass >= 1: XOR the new block with the existing block
            # (v1.3 semantics — overwrites the slot).
            existing = memory[lane][col]
            memory[lane][col] = [existing[k] ^ new_block[k] for k in range(BLOCK_WORDS)]


# ---------------------------------------------------------------------------
# Parameter validation
# ---------------------------------------------------------------------------


def _validate(
    password: bytes,
    salt: bytes,
    time_cost: int,
    memory_cost: int,
    parallelism: int,
    tag_length: int,
    key: bytes,
    associated_data: bytes,
    version: int,
) -> None:
    if not isinstance(password, (bytes, bytearray)):
        raise TypeError("password must be bytes")
    if not isinstance(salt, (bytes, bytearray)):
        raise TypeError("salt must be bytes")
    if not isinstance(key, (bytes, bytearray)):
        raise TypeError("key must be bytes")
    if not isinstance(associated_data, (bytes, bytearray)):
        raise TypeError("associated_data must be bytes")

    if len(salt) < 8:
        raise ValueError(f"salt must be at least 8 bytes, got {len(salt)}")
    if tag_length < 4:
        raise ValueError(f"tag_length must be >= 4, got {tag_length}")
    if tag_length > 0xFFFFFFFF:
        raise ValueError(f"tag_length must fit in 32 bits, got {tag_length}")
    if parallelism < 1 or parallelism > 0xFFFFFF:
        raise ValueError(f"parallelism must be in [1, 2^24-1], got {parallelism}")
    if memory_cost < 8 * parallelism:
        raise ValueError(
            f"memory_cost must be >= 8*parallelism ({8 * parallelism}),"
            f" got {memory_cost}"
        )
    if memory_cost > 0xFFFFFFFF:
        raise ValueError(
            f"memory_cost must fit in 32 bits, got {memory_cost}"
        )
    if time_cost < 1:
        raise ValueError(f"time_cost must be >= 1, got {time_cost}")
    if version != VERSION:
        raise ValueError(
            f"only Argon2 v1.3 (0x13) is supported; got 0x{version:02x}"
        )


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def argon2id(
    password: bytes,
    salt: bytes,
    time_cost: int,
    memory_cost: int,
    parallelism: int,
    tag_length: int,
    *,
    key: bytes = b"",
    associated_data: bytes = b"",
    version: int = VERSION,
) -> bytes:
    """Compute an Argon2id tag (RFC 9106 §3).

    Parameters
    ----------
    password
        The secret input.  Up to ``2**32 - 1`` bytes.
    salt
        Random salt.  At least 8 bytes; 16 is recommended.
    time_cost
        Number of passes over memory (``t`` in the RFC).  Must be >= 1.
    memory_cost
        Target memory in KiB (``m`` in the RFC).  Must be >= 8 * parallelism.
        Internally rounded *down* to a multiple of ``4 * parallelism``.
    parallelism
        Number of lanes (``p`` in the RFC).  1..2**24-1.
    tag_length
        Output length in bytes (``T`` in the RFC).  Must be >= 4.
    key
        Optional MAC key (``K``), default empty.
    associated_data
        Optional context data (``X``), default empty.
    version
        Protocol version.  Only ``0x13`` (v1.3) is supported.

    Returns
    -------
    bytes
        Exactly ``tag_length`` bytes.
    """
    _validate(
        password, salt, time_cost, memory_cost,
        parallelism, tag_length, key, associated_data, version,
    )

    # Round m down to the nearest multiple of 4*p.
    segment_length = memory_cost // (SYNC_POINTS * parallelism)
    m_prime = segment_length * SYNC_POINTS * parallelism
    lane_length = m_prime // parallelism
    q = lane_length
    SL = segment_length
    p = parallelism
    t = time_cost

    # Step 1 — H0 (64 bytes, RFC 9106 §3.2).
    h0 = blake2b(
        struct.pack("<I", p)
        + struct.pack("<I", tag_length)
        + struct.pack("<I", memory_cost)
        + struct.pack("<I", t)
        + struct.pack("<I", version)
        + struct.pack("<I", TYPE_ID)
        + struct.pack("<I", len(password)) + bytes(password)
        + struct.pack("<I", len(salt)) + bytes(salt)
        + struct.pack("<I", len(key)) + bytes(key)
        + struct.pack("<I", len(associated_data)) + bytes(associated_data),
        digest_size=64,
    )

    # Step 2 — allocate the memory matrix B[p][q], each slot a 128 × u64 block.
    memory: list[list[list[int]]] = [
        [[0] * BLOCK_WORDS for _ in range(q)] for _ in range(p)
    ]

    # Step 3 — first two blocks of every lane via H'.
    for i in range(p):
        b0 = _blake2b_long(
            BLOCK_SIZE, h0 + struct.pack("<I", 0) + struct.pack("<I", i),
        )
        b1 = _blake2b_long(
            BLOCK_SIZE, h0 + struct.pack("<I", 1) + struct.pack("<I", i),
        )
        memory[i][0] = _bytes_to_block(b0)
        memory[i][1] = _bytes_to_block(b1)

    # Step 4 — fill the rest of memory, pass by pass, slice by slice.
    # Lanes *within* a slice can in principle run in parallel; this
    # reference port is single-threaded.
    for r in range(t):
        for sl in range(SYNC_POINTS):
            for lane in range(p):
                _fill_segment(memory, r, lane, sl, q, SL, p, m_prime, t, TYPE_ID)

    # Step 5 — XOR the final column across lanes, then H' to T bytes.
    final_block = memory[0][q - 1].copy()
    for lane in range(1, p):
        for k in range(BLOCK_WORDS):
            final_block[k] ^= memory[lane][q - 1][k]

    return _blake2b_long(tag_length, _block_to_bytes(final_block))


def argon2id_hex(
    password: bytes,
    salt: bytes,
    time_cost: int,
    memory_cost: int,
    parallelism: int,
    tag_length: int,
    *,
    key: bytes = b"",
    associated_data: bytes = b"",
    version: int = VERSION,
) -> str:
    """Argon2id returning lowercase hex."""
    return argon2id(
        password, salt, time_cost, memory_cost,
        parallelism, tag_length,
        key=key, associated_data=associated_data, version=version,
    ).hex()
