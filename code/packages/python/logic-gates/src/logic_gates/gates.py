"""Core logic gate implementations.

All gates operate on binary values (0 or 1). Invalid inputs raise ValueError.
"""

from functools import reduce


def _validate_bit(value: int, name: str = "input") -> None:
    """Validate that a value is a binary bit (0 or 1)."""
    if not isinstance(value, int) or isinstance(value, bool):
        msg = f"{name} must be an int, got {type(value).__name__}"
        raise TypeError(msg)
    if value not in (0, 1):
        msg = f"{name} must be 0 or 1, got {value}"
        raise ValueError(msg)


# === Fundamental gates ===


def NOT(a: int) -> int:
    """Invert a single bit. 0→1, 1→0."""
    _validate_bit(a, "a")
    return 1 if a == 0 else 0


def AND(a: int, b: int) -> int:
    """Output 1 only if both inputs are 1."""
    _validate_bit(a, "a")
    _validate_bit(b, "b")
    return 1 if a == 1 and b == 1 else 0


def OR(a: int, b: int) -> int:
    """Output 1 if either input is 1."""
    _validate_bit(a, "a")
    _validate_bit(b, "b")
    return 1 if a == 1 or b == 1 else 0


def XOR(a: int, b: int) -> int:
    """Output 1 if inputs are different."""
    _validate_bit(a, "a")
    _validate_bit(b, "b")
    return 1 if a != b else 0


def NAND(a: int, b: int) -> int:
    """NOT(AND) — output 0 only if both inputs are 1."""
    return NOT(AND(a, b))


def NOR(a: int, b: int) -> int:
    """NOT(OR) — output 1 only if both inputs are 0."""
    return NOT(OR(a, b))


def XNOR(a: int, b: int) -> int:
    """NOT(XOR) — output 1 if inputs are the same."""
    return NOT(XOR(a, b))


# === NAND-derived gates ===
# Every gate can be built from NAND alone (functional completeness).


def nand_not(a: int) -> int:
    """NOT built from NAND: NAND(a, a)."""
    return NAND(a, a)


def nand_and(a: int, b: int) -> int:
    """AND built from NAND: NOT(NAND(a, b))."""
    return nand_not(NAND(a, b))


def nand_or(a: int, b: int) -> int:
    """OR built from NAND: NAND(NOT(a), NOT(b))."""
    return NAND(nand_not(a), nand_not(b))


def nand_xor(a: int, b: int) -> int:
    """XOR built from NAND: NAND(NAND(a, NAND(a,b)), NAND(b, NAND(a,b)))."""
    nand_ab = NAND(a, b)
    return NAND(NAND(a, nand_ab), NAND(b, nand_ab))


# === Multi-input variants ===


def AND_N(*inputs: int) -> int:
    """AND with N inputs. Returns 1 only if ALL inputs are 1."""
    if len(inputs) < 2:
        msg = "AND_N requires at least 2 inputs"
        raise ValueError(msg)
    return reduce(AND, inputs)


def OR_N(*inputs: int) -> int:
    """OR with N inputs. Returns 1 if ANY input is 1."""
    if len(inputs) < 2:
        msg = "OR_N requires at least 2 inputs"
        raise ValueError(msg)
    return reduce(OR, inputs)
