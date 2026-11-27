"""The ``ir.fixed`` built-ins as arithmetic on Python integers (VER04r Appendix A.1).

An evaluator node of the ``ir.fixed`` level is a few lines of IR glue (typing,
cost, call counting: :mod:`.ir`) around one of the functions below.  Keeping the
*value* rules here, apart from the glue, means each rule can be read, tested and
broken (see :class:`~.semantics.Semantics`) on its own.

The one idea behind every function: a fixed-width value of ``w`` bits is a
**bit pattern** -- the residue ``value mod 2^w`` -- that a type *reads back*:
an unsigned type reads the pattern as it is, a signed type reads a pattern
with its top bit set as negative::

    pattern(-1i8) = 0b1111_1111 = 255        reread_i8(255) = -1
    pattern( 5u8) = 0b0000_0101 =   5        reread_u8(  5) =  5

Python integers are unbounded and ``&``, ``|``, ``^`` and ``>>`` already treat a
negative number as an infinite string of leading ones, so most operators below
are one line.  What takes care is everything *around* the width: a shift count
is an exact natural number (it may be ``2^64 - 1``) and is compared with ``w``
**before** any host shift happens, so a huge count never builds a huge number.

Options are the tuples the rest of the lane already uses: ``("some", v)`` and
``("none",)``.  An ``Option`` exists only as the *intermediate* value a producer
hands to ``unwrap_or`` (section 4.1, [D23]).
"""

from __future__ import annotations

from .arith import raw_binary, wrap
from .wtypes import Ty, fixed_range

SOME = "some"
NONE = ("none",)


def pattern(value: int, ty: Ty) -> int:
    """The ``w``-bit pattern of ``value``: ``value mod 2^w``, never negative."""
    return value & ((1 << ty.width) - 1)


# ---------------------------------------------------------------------------
# Shifts and rotations (the count is an exact Nat)
# ---------------------------------------------------------------------------
def shift(op: str, value: int, count: int, ty: Ty) -> int:
    """``shl``, ``shr`` and ``sar`` of ``value`` by ``count``, in type ``ty``.

    =====  =============================================  =====================
    op     rule                                           count >= w
    =====  =============================================  =====================
    shl    zero fill; the bits that leave are lost        0
    shr    **logical**: on the pattern, then re-read      0
    sar    arithmetic: the sign fills                     -1 if value < 0 else 0
    =====  =============================================  =====================

    ``shr(-1i8, 1)`` is ``127i8`` (the pattern ``ff`` shifts to ``7f``); it is not
    ``sar``.  The count is compared with ``w`` first: ``shl(1u8, 2^64-1)`` is
    ``0`` and costs one comparison, not a number with 2^64 bits.
    """
    bits = ty.width
    if op == "sar":
        return (-1 if value < 0 else 0) if count >= bits else value >> count
    if count >= bits:
        return 0
    if op == "shl":
        return wrap(value << count, ty)
    return wrap(pattern(value, ty) >> count, ty)  # shr: logical


def rotate(op: str, value: int, count: int, ty: Ty) -> int:
    """``rotl`` or ``rotr`` of the pattern by ``count mod w`` bits, re-read in ``ty``.

    The count is reduced first (``rotl(x, w) = x``); a rotation loses no bit::

        rotl(0b1000_0001u8, 1) = 0b0000_0011
        rotr(0b0000_0001u8, 1) = 0b1000_0000
    """
    bits = ty.width
    amount = count % bits
    if op == "rotr":
        amount = (
            bits - amount
        ) % bits  # a right rotation is a left one the other way round
    bits_pattern = pattern(value, ty)
    turned = ((bits_pattern << amount) | (bits_pattern >> (bits - amount))) & (
        (1 << bits) - 1
    )
    return wrap(turned, ty)


# ---------------------------------------------------------------------------
# Counting bits
# ---------------------------------------------------------------------------
def popcount(value: int, ty: Ty) -> int:
    """The number of one bits in the pattern."""
    return pattern(value, ty).bit_count()


def count_zeros(op: str, value: int, ty: Ty) -> int:
    """``clz`` (leading) or ``ctz`` (trailing) zero bits of the pattern.

    Both are ``w`` for the pattern ``0``: every bit is a zero and the count runs
    out of bits, not out of zeros.
    """
    bits_pattern = pattern(value, ty)
    if bits_pattern == 0:
        return ty.width
    if op == "clz":
        return ty.width - bits_pattern.bit_length()
    return (bits_pattern & -bits_pattern).bit_length() - 1


def bit(value: int, index: int, ty: Ty) -> bool:
    """Bit ``index`` of the pattern, least significant first; ``false`` past ``w``."""
    return index < ty.width and (pattern(value, ty) >> index) & 1 == 1


# ---------------------------------------------------------------------------
# Checked arithmetic and division: an Option, never a wrapped value
# ---------------------------------------------------------------------------
def checked(op: str, left: int, right: int, ty: Ty) -> tuple:
    """``checked_add``/``sub``/``mul``: ``None`` iff the exact result leaves ``ty``."""
    exact = raw_binary(op, left, right)
    low, high = fixed_range(ty)
    return (SOME, exact) if low <= exact <= high else NONE


def divide(op: str, left: int, right: int, ty: Ty | None) -> tuple:
    """``div``, ``rem``, ``div_trunc``, ``rem_trunc``: an Option, ``None`` on zero.

    ``div`` and ``rem`` are **floor** division on ``Int``, ``Nat`` and unsigned
    types (``rem`` has the divisor's sign, as Python's ``//`` and ``%`` do)::

        div(-7, 2) = -4        rem(-7, 2) = 1        div(7, -2) = -4

    ``div_trunc`` and ``rem_trunc`` are the signed ones, rounding **toward
    zero**, and are ``None`` as well for the one quotient that does not fit,
    ``MIN / -1``::

        div_trunc(-7i8, 2i8) = -3i8      rem_trunc(-7i8, 2i8) = -1i8

    ``ty`` is the signed type of a truncating operation and ``None`` otherwise.
    """
    if right == 0:
        return NONE
    if op == "div":
        return (SOME, left // right)
    if op == "rem":
        return (SOME, left % right)
    assert ty is not None
    if left == fixed_range(ty)[0] and right == -1:
        return NONE
    quotient = abs(left) // abs(right)
    if (left < 0) != (right < 0):
        quotient = -quotient
    return (SOME, quotient if op == "div_trunc" else left - right * quotient)


def to_bytes(value: int, ty: Ty, order: str) -> bytes:
    """``be_bytes`` / ``le_bytes``: the ``w/8`` bytes of the pattern."""
    return pattern(value, ty).to_bytes(ty.width // 8, order)  # type: ignore[arg-type]


def from_bytes(data: bytes, ty: Ty, order: str) -> int:
    """``from_be`` / ``from_le``: the pattern of ``data`` read back in ``ty``."""
    return wrap(int.from_bytes(data, order), ty)  # type: ignore[arg-type]
