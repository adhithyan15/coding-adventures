"""The single arithmetic rule of VER04r section 3 [D4].

    Every ``+``, ``-``, ``*`` and unary ``-`` in the model IR on a fixed-width
    type wraps modulo ``2^w``, in two's complement for signed types.

That is the whole rule for ``U8..U64`` and ``I8..I64``.  A table makes it
concrete (``w`` is the bit width):

====================  ===========================================  ==========
operand type          result of ``+ - *`` and unary ``-``           example
====================  ===========================================  ==========
``U8 .. U64``         ``value mod 2^w``                      ``0u8 - 1u8 = 255u8``
``I8 .. I64``         wrapped into ``-2^(w-1) .. 2^(w-1)-1``  ``127i8 + 1i8 = -128i8``
``Nat``               exact, and the result type is ``Int``         ``3 - 5 = -2``
``Int``               exact                                         (model-internal)
====================  ===========================================  ==========

Two's complement in one line: a signed ``w``-bit value is the unsigned residue
``value mod 2^w``, read back as negative when its top bit is set.  So wrapping
is *one* modulo followed by *one* conditional subtraction, for every width, and
there is no separate "overflow" case to get wrong.

Exact integers are not unbounded
--------------------------------
``Nat`` and ``Int`` are exact, but an evaluator that let them grow without limit
could be made to build a number with billions of digits from a corpus of a few
hundred nodes.  So every exact value must stay inside the *window*
``-(2^127 - 1) .. 2^127 - 1`` (section 3.2), so that it fits a signed 128-bit
integer in every lane, and a node that leaves it is the kit's ``ir-overflow``,
**never** a wrapped or saturated value.  The product of two in-range values can
need 254 bits before the check, which is why the check is applied to the exact
result and not to the operands.
"""

from __future__ import annotations

from .wtypes import Ty

WINDOW = (1 << 127) - 1  # the exact window is -WINDOW .. WINDOW


class WindowExceeded(Exception):
    """An exact value left the window; the IR node turns this into ``ir-overflow``."""


def wrap(value: int, ty: Ty) -> int:
    """Reduce ``value`` into the range of fixed-width ``ty`` (two's complement)."""
    bits, signed = ty.width, ty.signed
    residue = value % (1 << bits)  # Python's % is never negative
    if signed and residue >= 1 << (bits - 1):
        residue -= 1 << bits
    return residue


def raw_binary(op: str, left: int, right: int) -> int:
    """The *exact* result of ``add``, ``sub`` or ``mul`` (before any wrapping)."""
    if op == "add":
        return left + right
    if op == "sub":
        return left - right
    return left * right


def check_window(value: int) -> int:
    """``value`` if inside the exact window, else raise :class:`WindowExceeded`."""
    if not -WINDOW <= value <= WINDOW:
        raise WindowExceeded
    return value
