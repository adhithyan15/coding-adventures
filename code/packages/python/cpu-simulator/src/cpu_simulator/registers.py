"""Register File — the CPU's fast, small storage.

=== What are registers? ===

Registers are the fastest storage in a computer. They sit inside the CPU
itself and can be read or written in a single clock cycle. A typical CPU
has between 8 and 32 registers, each holding one "word" of data (e.g., 32
bits on a 32-bit CPU).

Think of registers like the small whiteboard on your desk. You can glance
at it instantly (fast), but it only holds a few things. Memory (RAM) is
like a filing cabinet across the room — it holds much more, but you have
to walk over to get something (slow).

=== Why so few? ===

Registers are expensive to build because they need to be extremely fast.
Each register is made of flip-flops (built from logic gates), and the
wiring to connect them all to the ALU grows quadratically with the number
of registers. So CPUs use a small number of very fast registers combined
with large but slower memory.

=== Register conventions ===

Different architectures assign special meaning to certain registers:
  - RISC-V: x0 is hardwired to 0, x1 = return address, x2 = stack pointer
  - ARM: R13 = stack pointer, R14 = link register, R15 = program counter
  - Intel 4004: 16 4-bit registers + a 4-bit accumulator

Our RegisterFile is generic — the ISA simulator decides which registers
have special behavior (like x0 always being 0 in RISC-V).
"""

from dataclasses import dataclass, field


@dataclass
class RegisterFile:
    """A set of numbered registers, each holding an integer value.

    The register file is like a tiny array of named storage slots:

        ┌─────┬─────┬─────┬─────┬─────┬─────┐
        │ R0  │ R1  │ R2  │ R3  │ ... │ R15 │
        │  0  │  0  │  0  │  0  │     │  0  │
        └─────┴─────┴─────┴─────┴─────┴─────┘

    Read and write by register number:
        registers.read(1)       → value in R1
        registers.write(1, 42)  → R1 = 42

    Example:
        >>> regs = RegisterFile(num_registers=16, bit_width=32)
        >>> regs.write(1, 42)
        >>> regs.read(1)
        42
    """

    num_registers: int = 16
    bit_width: int = 32
    _values: list[int] = field(default_factory=list, repr=False)

    def __post_init__(self) -> None:
        """Initialize all registers to 0."""
        if not self._values:
            self._values = [0] * self.num_registers
        self._max_value = (1 << self.bit_width) - 1

    def read(self, index: int) -> int:
        """Read the value stored in register `index`.

        Example:
            >>> regs = RegisterFile(num_registers=4)
            >>> regs.write(2, 100)
            >>> regs.read(2)
            100
        """
        if not 0 <= index < self.num_registers:
            msg = f"Register index {index} out of range (0-{self.num_registers - 1})"
            raise IndexError(msg)
        return self._values[index]

    def write(self, index: int, value: int) -> None:
        """Write a value to register `index`.

        Values are masked to the register's bit width. For example, on a
        32-bit register file, writing 2^32 wraps to 0.

        Example:
            >>> regs = RegisterFile(num_registers=4, bit_width=8)
            >>> regs.write(0, 256)  # 256 doesn't fit in 8 bits
            >>> regs.read(0)
            0  # wrapped: 256 & 0xFF = 0
        """
        if not 0 <= index < self.num_registers:
            msg = f"Register index {index} out of range (0-{self.num_registers - 1})"
            raise IndexError(msg)
        self._values[index] = value & self._max_value

    def dump(self) -> dict[str, int]:
        """Return all register values as a dict for inspection.

        Example:
            >>> regs = RegisterFile(num_registers=4)
            >>> regs.write(1, 5)
            >>> regs.dump()
            {'R0': 0, 'R1': 5, 'R2': 0, 'R3': 0}
        """
        return {f"R{i}": v for i, v in enumerate(self._values)}
