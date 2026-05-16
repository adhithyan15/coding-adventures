"""bits.py — 32-bit bit-list conversion helpers for the PowerPC 601 gate-level simulator.

This module is the bridge between the "integer world" (the Python API, test
programs, memory addresses) and the "gate world" (lists of 0/1 values flowing
through AND, OR, XOR, NOT primitives).

All actual arithmetic in this module uses Python integer operations because we
are doing bookkeeping (packing/unpacking bits), NOT simulating data-path
operations.  Data-path operations — ADD, SUB, AND, OR, XOR, NOT — live in
alu.py and must route through gate primitives.

LSB-first ordering
──────────────────
We use LSB-first bit lists throughout.  This matches the convention used by
the arithmetic package's ripple_carry_adder:

    int_to_bits(5, 8) → [1, 0, 1, 0, 0, 0, 0, 0]
                         ^bit0 (2^0=1, set)
                                ^bit2 (2^2=4, set)

This is the natural representation for a ripple-carry adder: bit[0] feeds
the first full adder (carry in = 0), bit[1] feeds the second, and so on.

Overflow detection
──────────────────
For a two's-complement addition of N-bit values:
  overflow = XOR(carry_into_bit_(N-1), carry_out_of_bit_(N-1))

For a 32-bit add:
  - carry_into_bit_31 = carry propagated from the ripple chain up to bit 30
  - carry_out         = carry out of bit 31 (returned by ripple_carry_adder)
  - overflow          = XOR(carry_into_31, carry_out)

We obtain carry_into_31 by running a 31-bit adder on bits[0:31], then using
that carry_out as carry_into_31 for the final (bit-31) full adder.

Shift and rotate
──────────────────
Shifts and rotates are implemented via bit-list manipulation.  A left shift
by k positions moves bit[i] to bit[i+k], filling the low k positions with 0.
A right shift moves bit[i+k] to bit[i], filling the high k positions with
either 0 (logical) or the sign bit (arithmetic).

Rotation wraps: bit[i] moves to bit[(i+k) % 32], so no bits are lost.
"""

from __future__ import annotations

from arithmetic import ripple_carry_adder
from logic_gates import NOT, OR, XOR

# ── Integer ↔ bit-list bridge ──────────────────────────────────────────────────


def int_to_bits(value: int, width: int) -> list[int]:
    """Convert an integer to a LSB-first bit list of the given width.

    The value is first masked to `width` bits so that negative Python ints
    and values wider than `width` are handled correctly.

    Examples
    ────────
    >>> int_to_bits(5, 8)
    [1, 0, 1, 0, 0, 0, 0, 0]
    >>> int_to_bits(0, 4)
    [0, 0, 0, 0]
    >>> int_to_bits(255, 8)
    [1, 1, 1, 1, 1, 1, 1, 1]
    """
    mask = (1 << width) - 1
    v = value & mask
    return [(v >> i) & 1 for i in range(width)]


def bits_to_int(bits: list[int]) -> int:
    """Convert a LSB-first bit list to a non-negative integer.

    Examples
    ────────
    >>> bits_to_int([1, 0, 1, 0])
    5
    >>> bits_to_int([0, 0, 0, 0])
    0
    """
    result = 0
    for i, b in enumerate(bits):
        result |= b << i
    return result


# ── 32-bit gate-level arithmetic helpers ──────────────────────────────────────


def add_32bit(a: int, b: int, carry_in: int = 0) -> tuple[int, int, int]:
    """Add two unsigned 32-bit values via ripple_carry_adder (32 full adders).

    Returns (result, carry_out, overflow).

    Overflow detection for signed two's-complement arithmetic:
      overflow = XOR(carry_into_bit_31, carry_out_of_bit_31)

    We split the add into a 31-bit ripple (bits 0–30) to obtain the carry
    into bit 31, then add the single bit 31 separately.

    Parameters
    ──────────
    a, b      : unsigned 32-bit integers
    carry_in  : initial carry (0 or 1)

    Returns
    ───────
    result    : unsigned 32-bit integer (bits 0–31 of a + b + carry_in)
    carry_out : carry out of bit 31
    overflow  : 1 if signed overflow occurred, 0 otherwise

    Example
    ───────
    >>> add_32bit(1, 1)
    (2, 0, 0)
    >>> add_32bit(0xFFFFFFFF, 1)
    (0, 1, 0)
    >>> add_32bit(0x7FFFFFFF, 1)  # max positive + 1 → overflow
    (2147483648, 0, 1)
    """
    a_bits = int_to_bits(a & 0xFFFF_FFFF, 32)
    b_bits = int_to_bits(b & 0xFFFF_FFFF, 32)

    # Full 32-bit ripple add
    sum_bits, carry_out = ripple_carry_adder(a_bits, b_bits, carry_in)

    # Carry into bit 31: run 31-bit adder on bits[0:31]
    _low_sum, carry_into_31 = ripple_carry_adder(
        a_bits[:31], b_bits[:31], carry_in
    )
    overflow = XOR(carry_into_31, carry_out)

    return bits_to_int(sum_bits), carry_out, overflow


def add_64bit(a: int, b: int, carry_in: int = 0) -> tuple[int, int]:
    """Add two 64-bit values via two ripple_carry_adder calls (64 full adders).

    Returns (result, carry_out).  Used for 64-bit product accumulation in
    multiply (MULLW/MULHW).

    Parameters
    ──────────
    a, b      : unsigned 64-bit integers (Python ints, may be any size)
    carry_in  : initial carry (0 or 1)

    Example
    ───────
    >>> add_64bit(1, 1)
    (2, 0)
    >>> add_64bit(0xFFFFFFFFFFFFFFFF, 1)
    (0, 1)
    """
    mask64 = (1 << 64) - 1
    a_bits = int_to_bits(a & mask64, 64)
    b_bits = int_to_bits(b & mask64, 64)
    sum_bits, carry_out = ripple_carry_adder(a_bits, b_bits, carry_in)
    return bits_to_int(sum_bits), carry_out


# ── Bitwise inversion via NOT gates ───────────────────────────────────────────


def invert_32bit(value: int) -> int:
    """Bitwise NOT of a 32-bit value: apply NOT to each of the 32 bits.

    Routes through 32 NOT gate calls, one per bit.  This is how the physical
    ALU's complement unit works: an inverter on each bit line.

    Example
    ───────
    >>> hex(invert_32bit(0))
    '0xffffffff'
    >>> invert_32bit(0xFFFF_FFFF)
    0
    >>> hex(invert_32bit(0xAAAAAAAA))
    '0x55555555'
    """
    bits = int_to_bits(value & 0xFFFF_FFFF, 32)
    inverted = [NOT(b) for b in bits]
    return bits_to_int(inverted)


# ── Zero and parity detection ──────────────────────────────────────────────────


def compute_zero(bits: list[int]) -> int:
    """Return 1 if ALL bits in the list are 0, otherwise return 0.

    Gate-level implementation: OR all bits together, then NOT.
    This mirrors the hardware NOR-tree that feeds the zero flag.

    A single OR-reduction tree:
      combined = bits[0] | bits[1] | bits[2] | ...
      result   = NOT(combined)

    Example
    ───────
    >>> compute_zero([0, 0, 0, 0])
    1
    >>> compute_zero([0, 1, 0, 0])
    0
    """
    combined = bits[0]
    for b in bits[1:]:
        combined = OR(combined, b)
    return NOT(combined)


def compute_parity(bits: list[int]) -> int:
    """XOR-tree parity: 1 if an odd number of bits are set, 0 if even.

    Gate-level implementation: XOR all bits together in a reduction tree.
    This is equivalent to the parity bit in error detection hardware.

    Example
    ───────
    >>> compute_parity([1, 0, 1, 0])  # two 1s → even → parity = 0
    0
    >>> compute_parity([1, 1, 1, 0])  # three 1s → odd → parity = 1
    1
    """
    combined = bits[0]
    for b in bits[1:]:
        combined = XOR(combined, b)
    return combined


# ── Shift operations via bit-list manipulation ─────────────────────────────────


def shl_32(value: int, shamt: int) -> int:
    """Shift left logical 32-bit by shamt positions (zero fill at LSB).

    Implemented via bit-list manipulation: the bit that was at position i
    moves to position i+shamt; positions 0..shamt-1 are filled with 0.

    Clamped: shamt outside [0, 31] returns 0 (nothing survives the shift).

    Example
    ───────
    >>> shl_32(1, 4)
    16
    >>> shl_32(0xFFFFFFFF, 1)
    4294967294
    >>> shl_32(1, 32)
    0
    """
    shamt = shamt & 0x3F  # keep 6-bit value; >=32 produces 0
    if shamt >= 32:
        return 0
    bits = int_to_bits(value & 0xFFFF_FFFF, 32)
    # Bits at positions shamt..31 come from source positions 0..31-shamt
    shifted = [0] * shamt + bits[: 32 - shamt]
    return bits_to_int(shifted)


def shr_32_logical(value: int, shamt: int) -> int:
    """Shift right logical 32-bit: shift right by shamt, filling zeros at MSB.

    Example
    ───────
    >>> shr_32_logical(16, 4)
    1
    >>> shr_32_logical(0xFFFFFFFF, 1)
    2147483647
    >>> shr_32_logical(1, 32)
    0
    """
    shamt = shamt & 0x3F
    if shamt >= 32:
        return 0
    bits = int_to_bits(value & 0xFFFF_FFFF, 32)
    # Bit at position i+shamt moves to position i; top shamt positions become 0
    shifted = bits[shamt:] + [0] * shamt
    return bits_to_int(shifted)


def shr_32_arith(value: int, shamt: int) -> int:
    """Shift right arithmetic 32-bit: shift right, filling with sign bit.

    The sign bit (bit 31) is replicated into the vacated high positions.
    This preserves the sign of two's-complement negative numbers, making
    arithmetic right shift equivalent to floor-division by 2^shamt.

    Example
    ───────
    >>> shr_32_arith(8, 3)
    1
    >>> hex(shr_32_arith(0xFFFFFFFF, 1))  # -1 >> 1 = -1 (still all 1s)
    '0xffffffff'
    >>> shr_32_arith(0x80000000, 1)  # min-int >> 1 = 0xC0000000
    3221225472
    """
    shamt = shamt & 0x3F
    if shamt >= 32:
        shamt = 31  # saturate: arithmetic shift ≥32 replicates sign
    bits = int_to_bits(value & 0xFFFF_FFFF, 32)
    sign_bit = bits[31]  # MSB = sign bit
    # Fill with sign bit at the top
    shifted = bits[shamt:] + [sign_bit] * shamt
    return bits_to_int(shifted)


def rotl_32(value: int, shamt: int) -> int:
    """Rotate left 32-bit by shamt positions.

    Rotation wraps bits around: bit at position i moves to position (i+shamt)%32.
    No bits are lost — the bits shifted out of the top reappear at the bottom.

    Used by RLWINM, RLWIMI, RLWNM instructions.

    Example
    ───────
    >>> rotl_32(1, 1)   # bit 0 → bit 1
    2
    >>> rotl_32(0x80000000, 1)  # MSB wraps to LSB
    1
    >>> rotl_32(0xDEADBEEF, 8)
    3203391471
    """
    shamt = shamt & 31  # modulo 32
    if shamt == 0:
        return value & 0xFFFF_FFFF
    bits = int_to_bits(value & 0xFFFF_FFFF, 32)
    # Rotate: bit at position i goes to (i + shamt) % 32
    # In LSB-first list: shifted = bits[32-shamt:] + bits[:32-shamt]
    # But that rotates RIGHT. For rotate LEFT:
    #   After rotation, position j holds what was at position (j - shamt) % 32
    #   = bits[(j - shamt) % 32]
    # Equivalently: take bits from index (32-shamt) onward, then wrap
    rotated = bits[32 - shamt :] + bits[: 32 - shamt]
    return bits_to_int(rotated)
