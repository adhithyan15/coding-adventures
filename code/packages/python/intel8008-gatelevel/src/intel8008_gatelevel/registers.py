"""Register file for the Intel 8008 gate-level simulator.

=== The 8008 Register Architecture ===

The Intel 8008 has 7 working registers (A, B, C, D, E, H, L) plus the
M pseudo-register (indirect memory access via H:L pair). This is a massive
improvement over the 4004's accumulator-only architecture.

In hardware, each register is a bank of 8 D flip-flops (one per bit). The
D flip-flop is the basic memory element: it captures the value at its
D input on the rising edge of the clock and holds it at Q until the next
clock edge.

=== Gate cost per register ===

Each 8-bit register contains:
    8 × D_flip_flop(D, clock_edge)
    = 8 × SR_latch(D, NOT(D))  [standard D flip-flop construction]
    = 8 × 2 NOR gates = 16 NOR gates per register
    (Each NOR gate = 4 transistors in CMOS)

7 registers × 16 NOR gates = 112 NOR gates = 448 transistors.

Plus the flag register (4 bits) = 4 × 2 = 8 NOR gates = 32 transistors.

=== Register encoding ===

The 8008 uses 3-bit fields to encode register operands:
    000 = B    001 = C    010 = D    011 = E
    100 = H    101 = L    110 = M    111 = A

Index 6 (M) is NOT a physical register — it's a pseudo-register that
directs memory access to the address in [H:L]. The register file raises
ValueError if you try to read or write index 6.

=== Simplification note ===

The `register()` function from `logic_gates.sequential` models the
combinational update logic of a D flip-flop. For simplicity, this
implementation stores register state as integer values and applies the
register() model on each access. The clock is implicit (every step() call
advances the clock by one cycle). This matches how the 4004 gate-level
simulator handles registers.
"""

from __future__ import annotations

from intel8008_gatelevel.bits import bits_to_int, int_to_bits

# Register index constants (match 3-bit hardware encoding)
REG_B = 0
REG_C = 1
REG_D = 2
REG_E = 3
REG_H = 4
REG_L = 5
REG_M = 6   # pseudo-register — raises ValueError if accessed directly
REG_A = 7


class RegisterFile:
    """7 × 8-bit registers for the Intel 8008 gate-level simulator.

    Stores register state in bit lists (LSB-first), as the real hardware
    stores in D flip-flops. Reading a register returns the bits held in
    its flip-flops. Writing clocks new bits into the flip-flops.

    Each register is modeled as a list of 8 bits (0 or 1), matching the
    physical representation in the hardware.

    Usage:
        >>> rf = RegisterFile()
        >>> rf.write(REG_A, 42)
        >>> rf.read(REG_A)
        42
        >>> rf.read_bits(REG_B)  # raw bit list
        [0, 0, 0, 0, 0, 0, 0, 0]
    """

    def __init__(self) -> None:
        """Initialize all registers to zero (power-on state)."""
        # 8 registers; index 6 (M) is never used but we allocate for indexing
        # Each register: 8 bits, LSB-first
        self._state: list[list[int]] = [[0] * 8 for _ in range(8)]

    def read(self, reg: int) -> int:
        """Read an 8-bit integer value from a register.

        Args:
            reg: Register index 0–7 (not 6 — M is a pseudo-register).

        Returns:
            8-bit integer (0–255) from the register's flip-flops.

        Raises:
            ValueError: If reg == 6 (M pseudo-register).
        """
        if reg == REG_M:
            msg = "Register M (index 6) is a pseudo-register — resolve to memory address first"
            raise ValueError(msg)
        return bits_to_int(self._state[reg])

    def read_bits(self, reg: int) -> list[int]:
        """Read the raw bit list from a register (8 bits, LSB first).

        This returns the gate-level representation directly, without
        converting to an integer. Useful when feeding bits to the ALU.

        Args:
            reg: Register index 0–7 (not 6 — M is a pseudo-register).

        Returns:
            List of 8 bits, LSB at index 0.
        """
        if reg == REG_M:
            msg = "Register M (index 6) is a pseudo-register"
            raise ValueError(msg)
        return self._state[reg][:]

    def write(self, reg: int, value: int) -> None:
        """Write an 8-bit integer value into a register.

        Simulates clocking a new value into the 8 D flip-flops that
        make up the register. The value is decomposed into bits first.

        Args:
            reg:   Register index 0–7 (not 6 — M raises ValueError).
            value: 8-bit value to store (0–255). Higher bits are masked.
        """
        if reg == REG_M:
            msg = "Register M (index 6) is a pseudo-register — write to memory instead"
            raise ValueError(msg)
        self._state[reg] = int_to_bits(value & 0xFF, 8)

    def write_bits(self, reg: int, bits: list[int]) -> None:
        """Write a bit list into a register.

        Args:
            reg:  Register index 0–7 (not 6).
            bits: List of 8 bits (LSB first).
        """
        if reg == REG_M:
            msg = "Register M (index 6) is a pseudo-register"
            raise ValueError(msg)
        self._state[reg] = bits[:8]

    @property
    def a(self) -> int:
        """Read the accumulator (register A = index 7)."""
        return self.read(REG_A)

    @a.setter
    def a(self, value: int) -> None:
        """Write the accumulator."""
        self.write(REG_A, value)

    @property
    def h(self) -> int:
        """Read register H (high byte of address pair)."""
        return self.read(REG_H)

    @property
    def l(self) -> int:
        """Read register L (low byte of address pair)."""
        return self.read(REG_L)

    @property
    def hl_address(self) -> int:
        """14-bit memory address formed from H and L.

        Only the low 6 bits of H contribute to the address (bits 13–8).
        L contributes bits 7–0.

        In hardware, this is computed via AND/OR gates:
            address = (H[5] << 13) | (H[4] << 12) | ... | (H[0] << 8) | L
        which simplifies to: (H & 0x3F) << 8 | L
        """
        h_bits = self._state[REG_H]
        l_bits = self._state[REG_L]
        # Take only the low 6 bits of H for addressing
        addr_bits = l_bits + h_bits[:6]  # 14 bits: L[7:0] + H[5:0]
        return bits_to_int(addr_bits)

    def reset(self) -> None:
        """Reset all registers to zero."""
        self._state = [[0] * 8 for _ in range(8)]
