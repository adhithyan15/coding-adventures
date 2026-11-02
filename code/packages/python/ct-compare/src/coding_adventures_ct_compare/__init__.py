"""Constant-time comparison helpers for byte strings and unsigned counters.

This is the Python lane of CT01 (``code/specs/CT01-constant-time-compare.md``).
The helpers compare and select *secret* bytes without an early exit, so the
work they do depends only on public lengths, not on the secret contents.

Why validation comes first
--------------------------
A constant-time comparison is only as strong as the bytes it compares. The
first version of this lane converted every input with ``bytes(value)``, and
``bytes`` has a trap: given an integer ``n`` it does not make the one-byte
value ``n``, it makes ``n`` *zero bytes*::

    bytes(3) == b"\\x00\\x00\\x00"

So ``ct_eq(3, b"\\0\\0\\0")`` returned ``True``: two unrelated inputs compared
equal. Every input is now checked before use:

=====================  ==========================================
Input                  Result
=====================  ==========================================
``bytes``              used as is
``bytearray``          copied to ``bytes``
``memoryview``         copied to ``bytes`` (byte-format views only)
list/tuple of ints     each must be an ``int`` (not ``bool``) 0-255
a *subclass* of any    ``TypeError`` (it could lie about its bytes)
anything else          ``TypeError``
=====================  ==========================================

The checks look at the *type* and *range* of each element, never at whether
two inputs match, so they reveal nothing about secret contents for valid
inputs.

Why the selection mask is computed arithmetically
-------------------------------------------------
CT01 forbids a branch on the secret ``choice`` bit, so the mask that picks
``left`` or ``right`` is ``-int(choice) & 0xFF`` (``0x00`` for ``False``,
``0xFF`` for ``True``) rather than ``0xFF if choice else 0x00``. CPython may
still execute this however it likes; CT01 does not promise that an
interpreter preserves the source's shape, and neither does this lane.
"""

from collections.abc import Sequence

ByteInput = bytes | bytearray | memoryview | Sequence[int]
_U64_MAX = (1 << 64) - 1


def _as_bytes(value: ByteInput, name: str) -> bytes:
    # Only the exact built-in types are accepted. A *subclass* of list, bytes
    # or bytearray can override __bytes__, __len__, __getitem__ or __iter__
    # and show validation one set of bytes and the comparison another, so it
    # is rejected rather than trusted. Each accepted input is read exactly
    # once, into a fresh copy, so nothing can change between validation and
    # use.
    kind = type(value)
    if kind is bytes:
        return value
    if kind is bytearray or kind is memoryview:
        view = memoryview(value)
        # A view of 32-bit integers, say, is not a byte string; refuse it
        # rather than reinterpret its memory.
        if view.format not in ("B", "b", "c"):
            raise TypeError(f"{name} must be a byte-format memoryview")
        return view.tobytes()
    if kind is list or kind is tuple:
        # tuple() of an exact list or tuple is a single C-level snapshot.
        items = tuple(value)
        out = bytearray(len(items))
        for index, element in enumerate(items):
            # bool is a subclass of int, so True would otherwise pass as 1.
            if type(element) is not int or not 0 <= element <= 255:
                raise ValueError(f"{name}[{index}] must be an integer from 0 to 255")
            out[index] = element
        return bytes(out)
    raise TypeError(
        f"{name} must be bytes, bytearray, memoryview, or a list or tuple of "
        "byte values (subclasses are not accepted)"
    )


def ct_eq(left: ByteInput, right: ByteInput) -> bool:
    """Return whether two byte sequences are equal without early exit."""

    left_bytes = _as_bytes(left, "left")
    right_bytes = _as_bytes(right, "right")
    if len(left_bytes) != len(right_bytes):
        return False

    accumulator = 0
    for index in range(len(left_bytes)):
        accumulator |= left_bytes[index] ^ right_bytes[index]
    return accumulator == 0


def ct_eq_fixed(left: ByteInput, right: ByteInput) -> bool:
    """Fixed-size companion to :func:`ct_eq` for dynamic Python callers."""

    return ct_eq(left, right)


def ct_select_bytes(left: ByteInput, right: ByteInput, choice: bool) -> bytes:
    """Select ``left`` when ``choice`` is true, otherwise ``right``."""

    left_bytes = _as_bytes(left, "left")
    right_bytes = _as_bytes(right, "right")
    if len(left_bytes) != len(right_bytes):
        raise ValueError("ct_select_bytes requires equal-length byte sequences")
    # Truthiness would make 0, "" and [] select `right` and "0" select `left`.
    # Only a real bool is a valid choice; checking its type reveals nothing.
    if type(choice) is not bool:
        raise TypeError("ct_select_bytes requires a bool choice")

    mask = -int(choice) & 0xFF
    return bytes(
        right_bytes[index] ^ ((left_bytes[index] ^ right_bytes[index]) & mask)
        for index in range(len(left_bytes))
    )


def ct_eq_u64(left: int, right: int) -> bool:
    """Return whether two unsigned 64-bit integers are equal."""

    _validate_u64(left, "left")
    _validate_u64(right, "right")
    diff = (left ^ right) & _U64_MAX
    folded = (diff | ((-diff) & _U64_MAX)) >> 63
    return folded == 0


def _validate_u64(value: int, name: str) -> None:
    # bool is a subclass of int; True is not an unsigned 64-bit counter.
    if type(value) is not int or value < 0 or value > _U64_MAX:
        raise ValueError(f"{name} must be an unsigned 64-bit integer")


__all__ = ["ct_eq", "ct_eq_fixed", "ct_select_bytes", "ct_eq_u64"]
