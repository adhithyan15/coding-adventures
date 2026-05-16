"""register_file.py — Gate-level register file for the MIPS R2000 simulator.

The MIPS R2000 has 32 general-purpose registers (GPRs), plus HI, LO, and PC.
In a real chip, each register is implemented as 32 D flip-flops — one per bit.
We model this by storing registers as list[list[int]]: a 2D array where
``_gprs[n]`` is a 32-element list of bits (LSB-first) for register n.

Register file structure
───────────────────────
  _gprs: list[list[int]]   — 32 × 32 bits, one row per GPR
  _hi:   list[int]         — 32-bit HI register (flip-flop array)
  _lo:   list[int]         — 32-bit LO register
  _pc:   list[int]         — 32-bit Program Counter

R0 ($zero) is special: reads always return 0, writes are silently discarded.
This matches the hardware: R0 is tied to ground (constant 0) on the real chip.

PC increment
────────────
``increment_pc`` uses ``add_32bit`` (gate-level ripple-carry adder) to add 4.
On the real processor, the adder that calculates PC+4 is a fast carry-select
adder; our ripple-carry version is behaviorally equivalent.
"""

from __future__ import annotations

from .bits import add_32bit, bits_to_int, int_to_bits


class RegisterFile32:
    """MIPS R2000 register file stored as flip-flop bit arrays.

    Each of the 32 GPRs is stored as a list[int] of 32 bits (LSB-first).
    HI, LO, and PC are stored similarly.  All public methods convert to/from
    Python ints at the boundary, operating on bit lists internally.

    R0 guard
    ────────
    All writes to register 0 are silently discarded.  Reads from register 0
    always return 0, regardless of what is stored internally.  This matches
    the MIPS R2000 hardware, where R0 is a constant-zero source.

    Invariants
    ──────────
    - Every bit list has exactly 32 elements, each 0 or 1.
    - ``_gprs[0]`` may contain any values internally but is never read.
    """

    def __init__(self) -> None:
        # 32 registers × 32 bits each — the "flip-flop" storage
        self._gprs: list[list[int]] = [[0] * 32 for _ in range(32)]
        # Special registers
        self._hi: list[int] = [0] * 32
        self._lo: list[int] = [0] * 32
        self._pc: list[int] = [0] * 32

    # ── General-purpose registers ──────────────────────────────────────────────

    def read_reg(self, n: int) -> int:
        """Read GPR n as an unsigned 32-bit integer.

        R0 ($zero) always returns 0 regardless of stored value.

        Args:
            n: Register number (0–31).

        Returns:
            Unsigned 32-bit integer value.
        """
        if n == 0:
            return 0
        return bits_to_int(self._gprs[n])

    def write_reg(self, n: int, value: int) -> None:
        """Write an unsigned 32-bit integer to GPR n.

        Writes to R0 are silently discarded (R0 is hardwired to 0).

        Args:
            n:     Register number (0–31).
            value: 32-bit unsigned value to write.
        """
        if n == 0:
            return  # R0 is hardwired zero — write is a no-op
        self._gprs[n] = int_to_bits(value & 0xFFFF_FFFF, 32)

    # ── HI register ───────────────────────────────────────────────────────────

    def read_hi(self) -> int:
        """Read the HI register as an unsigned 32-bit integer."""
        return bits_to_int(self._hi)

    def write_hi(self, value: int) -> None:
        """Write an unsigned 32-bit integer to the HI register."""
        self._hi = int_to_bits(value & 0xFFFF_FFFF, 32)

    # ── LO register ───────────────────────────────────────────────────────────

    def read_lo(self) -> int:
        """Read the LO register as an unsigned 32-bit integer."""
        return bits_to_int(self._lo)

    def write_lo(self, value: int) -> None:
        """Write an unsigned 32-bit integer to the LO register."""
        self._lo = int_to_bits(value & 0xFFFF_FFFF, 32)

    # ── Program Counter ───────────────────────────────────────────────────────

    def read_pc(self) -> int:
        """Read the Program Counter as an unsigned 32-bit integer."""
        return bits_to_int(self._pc)

    def write_pc(self, value: int) -> None:
        """Write an unsigned 32-bit integer to the Program Counter."""
        self._pc = int_to_bits(value & 0xFFFF_FFFF, 32)

    def increment_pc(self, by: int = 4) -> None:
        """Increment the PC by ``by`` bytes using a gate-level ripple-carry adder.

        On the real MIPS R2000, a dedicated incrementer (an adder hardwired
        to add 4) advances the PC after each instruction fetch.  We model this
        with ``add_32bit`` which routes through the arithmetic package.

        Args:
            by: Number of bytes to add (default 4 = one 32-bit instruction).
        """
        current = bits_to_int(self._pc)
        new_pc, _, _ = add_32bit(current, by & 0xFFFF_FFFF)
        self._pc = int_to_bits(new_pc & 0xFFFF_FFFF, 32)
