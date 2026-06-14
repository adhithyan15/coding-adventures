"""register_file.py — Gate-level 64-bit register file for the Alpha AXP 21064.

The register file stores 32 general-purpose registers (r0–r31), each 64 bits
wide, as lists of 64 bits.  The program counter (PC) is stored separately as
a 64-bit bit list.

Register 31 (r31)
──────────────────
The Alpha AXP hardwires register 31 to 0.  Reads always return 0; writes are
silently discarded.  This is the same design as MIPS $zero and simplifies the
ISA by eliminating a dedicated NOP instruction:

  BIS r31, r31, r31   →  NOP (OR 0 with 0, write to r31 = discard)
  BIS r31, lit8, Rd   →  MOVEI Rd, lit8  (load small immediate)

Gate-level storage
──────────────────
Each register is stored as a list[int] of 64 bits (a "register flip-flop bank").
On a real chip, each bit would be stored in a D flip-flop; reads are combinational
(the flip-flop output drives the bus) and writes are clocked (the D input is loaded
on the rising edge).

Here we model this as list[int] containing 0 or 1 values.

PC increment
────────────
The PC is incremented by 4 on each instruction fetch.  We use add_64bit (which
routes through ripple_carry_adder) for this increment.

Why 4?  Alpha instructions are 32-bit (4 bytes) fixed-width, stored little-endian.
Incrementing the 64-bit PC by 4 moves to the next instruction.
"""

from __future__ import annotations

from .bits import add_64bit, bits_to_int, int_to_bits

# Register count and the hardwired-zero register number
_NUM_REGS: int = 32
_REG_ZERO: int = 31


class RegisterFile64:
    """64-bit gate-level register file for the DEC Alpha AXP 21064.

    Stores 32 GPRs and the PC as lists of 64 bits (LSB-first).  All
    read/write operations go through the bit-list interface.

    Register 31 (r31) is hardwired to zero:
      - Reads always return 0 (the bit list is all zeros).
      - Writes are silently discarded.

    Example
    ───────
    >>> rf = RegisterFile64()
    >>> rf.write_reg(0, 42)
    >>> rf.read_reg(0)
    42
    >>> rf.write_reg(31, 99)  # discard
    >>> rf.read_reg(31)
    0
    """

    def __init__(self) -> None:
        # 32 registers × 64 bits each, all initialized to 0
        self._regs: list[list[int]] = [
            [0] * 64 for _ in range(_NUM_REGS)
        ]
        # PC stored as 64 bits
        self._pc: list[int] = [0] * 64

    # ── Register access ────────────────────────────────────────────────────────

    def read_reg(self, n: int) -> int:
        """Read GPR n as a 64-bit unsigned integer.

        r31 always returns 0 regardless of what is stored there.

        Parameters
        ──────────
        n : register number 0–31
        """
        if n == _REG_ZERO:
            return 0
        return bits_to_int(self._regs[n])

    def write_reg(self, n: int, value: int) -> None:
        """Write a 64-bit value to GPR n.

        Writes to r31 are silently discarded.

        Parameters
        ──────────
        n     : register number 0–31
        value : 64-bit unsigned integer to store
        """
        if n == _REG_ZERO:
            return  # discard
        self._regs[n] = int_to_bits(value & 0xFFFF_FFFF_FFFF_FFFF, 64)

    # ── PC access ──────────────────────────────────────────────────────────────

    def read_pc(self) -> int:
        """Read the program counter as a 64-bit unsigned integer."""
        return bits_to_int(self._pc)

    def write_pc(self, value: int) -> None:
        """Write the program counter.  Only the low 16 bits matter for 64KB memory."""
        self._pc = int_to_bits(value & 0xFFFF_FFFF_FFFF_FFFF, 64)

    def increment_pc(self, by: int = 4) -> None:
        """Increment the PC by `by` bytes using gate-level add_64bit.

        Standard Alpha instruction advance: by=4 (32-bit fixed-width instructions).

        The add routes through ripple_carry_adder (64 full adders), so this
        is a genuine gate-level operation.
        """
        old_pc = bits_to_int(self._pc)
        new_pc, _carry, _ov = add_64bit(old_pc, by, 0)
        self._pc = int_to_bits(new_pc & 0xFFFF_FFFF_FFFF_FFFF, 64)

    # ── Snapshot interface ─────────────────────────────────────────────────────

    def get_regs_tuple(self) -> tuple[int, ...]:
        """Return all 32 register values as a tuple (for AlphaState snapshot)."""
        return tuple(bits_to_int(self._regs[i]) for i in range(_NUM_REGS))

    def reset(self) -> None:
        """Reset all registers and PC to zero."""
        self._regs = [[0] * 64 for _ in range(_NUM_REGS)]
        self._pc = [0] * 64
