"""Canonical value streams and SHA-256 framing (VER04r sections 2.4, 5.4, 5.6).

A corpus does not ship its inputs.  It ships *digests* of two byte streams per
suite (and, in v2, per chunk): the stream of every admitted case's input values
and the stream of every case's expected value.  A lane that skips, reorders or
mis-evaluates a single case produces a different digest.  This module is the
oracle's own writer of those streams.

Value encodings (``warrant-stream-v2``; every ``v1`` row keeps its bytes)
-------------------------------------------------------------------------
=====================  ====================================================
``Bool``               ``00`` or ``01``
``U8 U16 U32 U64 Nat`` ``10`` then 8 bytes little-endian, unsigned
``I8 .. I64``          ``11`` then 8 bytes two's complement, sign-extended
``Bytes``, ``Bytes<N>`` ``20`` then u32-LE length, then the bytes
``List<T>``            ``30`` then u32-LE count, then the elements
``Option<T>``          ``40`` (none) / ``41`` plus the inner value
``Ok(v)`` / ``Err(E.C)`` ``50`` plus inner / ``51``, u32-LE length, UTF-8 case
tuple, record          ``60``, u32-LE component count, components
variant                ``62``, u32 length + UTF-8 constructor, u32 field count, fields
=====================  ====================================================

A top-level binder tuple is a plain *concatenation* of its binders' values; a
tuple *value* carries the ``60`` tag, because nested composites must be
self-delimiting or two different values could serialise to the same bytes.

Framing
-------
Each stream starts with a context prefix::

    header  suite-id  u32(#binders)  type-of-each-binder  result-type

where each name is ``u32-LE length`` + UTF-8, and the header is
``warrant-stream-v1\\0`` if every type in the suite is in the v1 set, else
``warrant-stream-v2\\0`` (section 1.2).  A *chunk* stream appends ``u32-LE chunk
index`` to the prefix; chunk digests are independent and combine by *listing*
(``warrant-chunks-v1``), never by hashing a combined stream.
"""

from __future__ import annotations

import hashlib

from . import ast as A
from .errors import OracleError
from .lower import FIXED, Spec

HEADER_V1 = b"warrant-stream-v1\0"
HEADER_V2 = b"warrant-stream-v2\0"
CHUNKS_FRAME = b"warrant-chunks-v1\0"
FNV_OFFSET = 0xCBF29CE484222325
FNV_PRIME = 0x100000001B3
_M64 = (1 << 64) - 1


def u32(n: int) -> bytes:
    if not 0 <= n < 1 << 32:
        raise OracleError("limit", "a length does not fit u32")
    return n.to_bytes(4, "little")


def field(text: str) -> bytes:
    raw = text.encode("utf-8")
    return u32(len(raw)) + raw


def in_v1_set(ty: A.Ty) -> bool:
    """Is ``ty`` in the v1 type set ``{Bool U8 U64 Nat Bytes Bytes<N> List/Option/Result of v1}``?"""
    head = ty.head
    if head in ("Bool", "U8", "U64", "Nat", "Bytes"):
        return True
    if head in ("List", "Option"):
        return in_v1_set(ty.args[0])
    if head == "Result":
        return in_v1_set(ty.args[0])
    return False


def make_encoder(spec: Spec, ty: A.Ty):
    """Return ``encode(value) -> bytes`` for values of type ``ty``."""
    head = ty.head
    if head == "Bool":
        return lambda v: b"\x01" if v else b"\x00"
    if head in ("U8", "U16", "U32", "U64", "Nat"):
        return lambda v: b"\x10" + v.to_bytes(8, "little")
    if head in FIXED:  # signed
        return lambda v: b"\x11" + (v & _M64).to_bytes(8, "little")
    if head == "Bytes":
        return lambda v: b"\x20" + u32(len(v)) + v
    if head == "Option":
        inner = make_encoder(spec, ty.args[0])
        return lambda v: b"\x40" if v[0] == "none" else b"\x41" + inner(v[1])
    if head == "List":
        inner = make_encoder(spec, ty.args[0])
        return lambda v: b"\x30" + u32(len(v[1])) + b"".join(inner(x) for x in v[1])
    if head == "Result":
        inner = make_encoder(spec, ty.args[0])
        return lambda v: b"\x50" + inner(v[1]) if v[0] == "ok" else b"\x51" + field(v[1])
    if head == "()":
        parts = [make_encoder(spec, t) for t in ty.args]
        return lambda v: b"\x60" + u32(len(parts)) + b"".join(e(x) for e, x in zip(parts, v))
    if head in spec.records:
        parts = [make_encoder(spec, t) for _, t in spec.records[head]]
        return lambda v: b"\x60" + u32(len(parts)) + b"".join(e(x) for e, x in zip(parts, v))
    if head in spec.variants:
        table = {
            name: [make_encoder(spec, t) for _, t in fields] for name, fields in spec.variants[head]
        }

        def encode_variant(v: tuple) -> bytes:
            name, fields = v
            encs = table[name]
            return (
                b"\x62"
                + field(name)
                + u32(len(encs))
                + b"".join(e(x) for e, x in zip(encs, fields))
            )

        return encode_variant
    raise OracleError("generation", "a value of this type has no stream encoding")


def context_prefix(suite_id: str, binder_types: list[A.Ty], result: A.Ty, header: bytes) -> bytes:
    """The shared stream prefix: header, suite id, binder types, result type."""
    return (
        header
        + field(suite_id)
        + u32(len(binder_types))
        + b"".join(field(str(t)) for t in binder_types)
        + field(str(result))
    )


def suite_header(binder_types: list[A.Ty], result: A.Ty) -> bytes:
    """``v1`` if every type of the suite is in the v1 set, else ``v2`` (section 1.2)."""
    ok = all(in_v1_set(t) for t in binder_types) and in_v1_set(result)
    return HEADER_V1 if ok else HEADER_V2


def chunk_digest_framing(digests: list[bytes]) -> str:
    """``SHA-256("warrant-chunks-v1\\0" || u32 m || the m raw chunk digests)``."""
    h = hashlib.sha256(CHUNKS_FRAME)
    h.update(u32(len(digests)))
    for d in digests:
        h.update(d)
    return h.hexdigest()


def fnv1a64(text: str) -> int:
    """FNV-1a, 64 bit, over the UTF-8 bytes (section 5.6)."""
    h = FNV_OFFSET
    for byte in text.encode("utf-8"):
        h = ((h ^ byte) * FNV_PRIME) & _M64
    return h


def slim_base_set(suite_id: str, m: int) -> list[int]:
    """The corpus-chosen chunk subset of a ``slim`` run, before witness chunks.

    ``m <= 16``: every chunk.  Otherwise ``k = ceil(m / 14)``,
    ``o = FNV-1a-64(suite id) mod k`` and the set is the first and last chunk plus
    every ``k``-th chunk starting at ``o`` (a 2^24-case suite has ``m = 256``,
    ``k = 19``).
    """
    if m <= 16:
        return list(range(m))
    k = -(-m // 14)
    o = fnv1a64(suite_id) % k
    chosen = {0, m - 1}
    chosen.update(range(o, m, k))
    return sorted(chosen)
