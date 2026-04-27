"""Argon2d — data-dependent memory-hard password hashing (RFC 9106).

Argon2d uses *data-dependent* addressing throughout every segment: the
reference block for each new block is chosen from the first 64 bits of
the previously computed block.  This maximises GPU/ASIC resistance at
the cost of leaking a noisy channel through memory-access timing, so
Argon2d is appropriate in contexts where side-channel attacks are not
in the threat model (e.g. proof-of-work).  For password hashing prefer
Argon2id.

The algorithm is specified in RFC 9106 and in this repo's
``code/specs/KD03-argon2.md``.  This package depends on
``coding_adventures_blake2b`` for the outer BLAKE2b calls (``H0`` and
the variable-length ``H'``); the inner compression round is an
Argon2-specific modification of BLAKE2b's round (an integer
multiplication term replaces SIGMA), so it is inlined here rather than
imported from the BLAKE2b package.

The public surface is intentionally tiny: one function pair
(:func:`argon2d`, :func:`argon2d_hex`), both one-shot.  There is no
streaming hasher because Argon2's inputs are absorbed in a single
``H0`` call.
"""

from __future__ import annotations

import struct

from coding_adventures_blake2b import blake2b

__all__ = ["argon2d", "argon2d_hex"]

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

MASK64 = 0xFFFFFFFFFFFFFFFF
MASK32 = 0xFFFFFFFF

BLOCK_SIZE = 1024
BLOCK_WORDS = BLOCK_SIZE // 8
SYNC_POINTS = 4

VERSION = 0x13
TYPE_D = 0

# ---------------------------------------------------------------------------
# BLAKE2b round (Argon2 flavour): no SIGMA, with 2*trunc32(a)*trunc32(b).
# ---------------------------------------------------------------------------


def _rotr64(x: int, n: int) -> int:
    return ((x >> n) | (x << (64 - n))) & MASK64


def _GB(v: list[int], a: int, b: int, c: int, d: int) -> None:
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
    _GB(v, 0, 4, 8, 12)
    _GB(v, 1, 5, 9, 13)
    _GB(v, 2, 6, 10, 14)
    _GB(v, 3, 7, 11, 15)
    _GB(v, 0, 5, 10, 15)
    _GB(v, 1, 6, 11, 12)
    _GB(v, 2, 7, 8, 13)
    _GB(v, 3, 4, 9, 14)


# ---------------------------------------------------------------------------
# Compression function G — block as 8×8 of 128-bit registers.
# ---------------------------------------------------------------------------


def _G(X: list[int], Y: list[int]) -> list[int]:
    R = [X[i] ^ Y[i] for i in range(BLOCK_WORDS)]
    Q = R.copy()

    # Row pass — P on each of 8 rows of 16 words.
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
    return struct.pack("<128Q", *block)


def _bytes_to_block(data: bytes) -> list[int]:
    if len(data) != BLOCK_SIZE:
        raise ValueError(f"block must be {BLOCK_SIZE} bytes, got {len(data)}")
    return list(struct.unpack("<128Q", data))


# ---------------------------------------------------------------------------
# H' — variable-length BLAKE2b (RFC 9106 §3.3)
# ---------------------------------------------------------------------------


def _blake2b_long(T: int, X: bytes) -> bytes:
    if T <= 0:
        raise ValueError(f"H' output length must be positive, got {T}")
    T_prefix = struct.pack("<I", T)

    if T <= 64:
        return blake2b(T_prefix + X, digest_size=T)

    r = (T + 31) // 32 - 2
    V = blake2b(T_prefix + X, digest_size=64)
    out = bytearray(V[:32])
    for _ in range(r - 1):
        V = blake2b(V, digest_size=64)
        out += V[:32]
    final_size = T - 32 * r
    V = blake2b(V, digest_size=final_size)
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
    if r == 0:
        if sl == 0:
            W = c - 1
            start = 0
        else:
            W = sl * SL + c - 1 if same_lane else sl * SL - (1 if c == 0 else 0)
            start = 0
    else:
        W = q - SL + c - 1 if same_lane else q - SL - (1 if c == 0 else 0)
        start = ((sl + 1) * SL) % q

    x = (J1 * J1) >> 32
    y = (W * x) >> 32
    rel = W - 1 - y

    return (start + rel) % q


# ---------------------------------------------------------------------------
# Argon2d fill — data-dependent addressing in every segment.
# ---------------------------------------------------------------------------


def _fill_segment(
    memory: list[list[list[int]]],
    r: int,
    lane: int,
    sl: int,
    q: int,
    SL: int,
    p: int,
) -> None:
    starting_c = 2 if (r == 0 and sl == 0) else 0

    for i in range(starting_c, SL):
        col = sl * SL + i
        prev_col = col - 1 if col > 0 else q - 1
        prev_block = memory[lane][prev_col]

        pseudo_rand = prev_block[0]
        J1 = pseudo_rand & MASK32
        J2 = (pseudo_rand >> 32) & MASK32

        l_prime = lane if (r == 0 and sl == 0) else (J2 % p)

        z_prime = _index_alpha(J1, r, sl, i, l_prime == lane, q, SL)
        ref_block = memory[l_prime][z_prime]

        new_block = _G(prev_block, ref_block)
        if r == 0:
            memory[lane][col] = new_block
        else:
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


def argon2d(
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
    """Compute an Argon2d tag (RFC 9106 §3)."""
    _validate(
        password, salt, time_cost, memory_cost,
        parallelism, tag_length, key, associated_data, version,
    )

    segment_length = memory_cost // (SYNC_POINTS * parallelism)
    m_prime = segment_length * SYNC_POINTS * parallelism
    lane_length = m_prime // parallelism
    q = lane_length
    SL = segment_length
    p = parallelism
    t = time_cost

    h0 = blake2b(
        struct.pack("<I", p)
        + struct.pack("<I", tag_length)
        + struct.pack("<I", memory_cost)
        + struct.pack("<I", t)
        + struct.pack("<I", version)
        + struct.pack("<I", TYPE_D)
        + struct.pack("<I", len(password)) + bytes(password)
        + struct.pack("<I", len(salt)) + bytes(salt)
        + struct.pack("<I", len(key)) + bytes(key)
        + struct.pack("<I", len(associated_data)) + bytes(associated_data),
        digest_size=64,
    )

    memory: list[list[list[int]]] = [
        [[0] * BLOCK_WORDS for _ in range(q)] for _ in range(p)
    ]

    for i in range(p):
        b0 = _blake2b_long(
            BLOCK_SIZE, h0 + struct.pack("<I", 0) + struct.pack("<I", i),
        )
        b1 = _blake2b_long(
            BLOCK_SIZE, h0 + struct.pack("<I", 1) + struct.pack("<I", i),
        )
        memory[i][0] = _bytes_to_block(b0)
        memory[i][1] = _bytes_to_block(b1)

    for r in range(t):
        for sl in range(SYNC_POINTS):
            for lane in range(p):
                _fill_segment(memory, r, lane, sl, q, SL, p)

    final_block = memory[0][q - 1].copy()
    for lane in range(1, p):
        for k in range(BLOCK_WORDS):
            final_block[k] ^= memory[lane][q - 1][k]

    return _blake2b_long(tag_length, _block_to_bytes(final_block))


def argon2d_hex(
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
    """Argon2d returning lowercase hex."""
    return argon2d(
        password, salt, time_cost, memory_cost,
        parallelism, tag_length,
        key=key, associated_data=associated_data, version=version,
    ).hex()
