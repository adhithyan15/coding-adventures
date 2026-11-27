"""Helpers shared by the case tables of the v2 conformance kit (slice S5a).

The tables state each case's *inputs* and its *intent*; the oracle (``conformance.py``) derives the
outcome, and the build fails, naming the case id, if the two disagree.  The helpers here only build
inputs and write intents the author can compute by hand: ranges, wrapped sums and little-endian words.
They deliberately use plain Python integer arithmetic and ``struct``-free byte formatting, a different
route from the oracle's evaluator, so that a table is a second opinion and not a copy.
"""

from __future__ import annotations

from coding_adventures_warrant_oracle.conformance import Row

#: width, signedness of the eight fixed-width types (section 2.1).
FIXED = {
    "U8": (8, False),
    "U16": (16, False),
    "U32": (32, False),
    "U64": (64, False),
    "I8": (8, True),
    "I16": (16, True),
    "I32": (32, True),
    "I64": (64, True),
}


def lo(t: str) -> int:
    bits, signed = FIXED[t]
    return -(1 << (bits - 1)) if signed else 0


def hi(t: str) -> int:
    bits, signed = FIXED[t]
    return (1 << (bits - 1)) - 1 if signed else (1 << bits) - 1


def wrap(value: int, t: str) -> int:
    """``value`` read back in ``t``: modulo ``2**w``, two's complement for a signed type (section 3.2)."""
    bits, signed = FIXED[t]
    r = value % (1 << bits)
    return r - (1 << bits) if signed and r >= 1 << (bits - 1) else r


def word(value: int) -> str:
    """Eight bytes, little-endian, two's complement: the payload of a ``10`` or ``11`` stream value."""
    return (value % (1 << 64)).to_bytes(8, "little").hex()


def u32le(n: int) -> str:
    return n.to_bytes(4, "little").hex()


def jhex(text: str) -> str:
    """The UTF-8 bytes of a JSON text, as the lowercase hex of ``input_hex``."""
    return text.encode("utf-8").hex()


def row(slug: str, summary: str, fields: dict, intent: dict, requires: list | None = None) -> Row:
    """A table row.  ``requires`` is ``None`` for an S5 case (it must compute ``["ir.core"]``, R24) and the expected
    ``requires`` of a case of a later slice (S10d); the build checks it against :func:`requires_of`'s."""
    return Row(slug, summary, fields, intent, requires)
