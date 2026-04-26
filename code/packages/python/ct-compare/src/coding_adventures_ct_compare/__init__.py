"""Constant-time comparison helpers for byte strings and unsigned counters."""

from collections.abc import Sequence

ByteInput = bytes | bytearray | memoryview | Sequence[int]
_U64_MAX = (1 << 64) - 1


def _as_bytes(value: ByteInput) -> bytes:
    if isinstance(value, bytes):
        return value
    if isinstance(value, bytearray | memoryview):
        return bytes(value)
    return bytes(value)


def ct_eq(left: ByteInput, right: ByteInput) -> bool:
    """Return whether two byte sequences are equal without early exit."""

    left_bytes = _as_bytes(left)
    right_bytes = _as_bytes(right)
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

    left_bytes = _as_bytes(left)
    right_bytes = _as_bytes(right)
    if len(left_bytes) != len(right_bytes):
        raise ValueError("ct_select_bytes requires equal-length byte sequences")

    mask = 0xFF if choice else 0x00
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
    if not isinstance(value, int) or value < 0 or value > _U64_MAX:
        raise ValueError(f"{name} must be an unsigned 64-bit integer")


__all__ = ["ct_eq", "ct_eq_fixed", "ct_select_bytes", "ct_eq_u64"]
