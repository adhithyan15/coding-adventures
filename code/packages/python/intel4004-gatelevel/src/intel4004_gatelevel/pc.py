"""Program counter — 12-bit register with increment and load.

=== The 4004's program counter ===

The program counter (PC) holds the address of the next instruction to
fetch from ROM. It's 12 bits wide, addressing 4096 bytes of ROM.

In real hardware, the PC is:
- A 12-bit register (12 D flip-flops)
- An incrementer (chain of half-adders for PC+1 or PC+2)
- A load input (for jump instructions)

The incrementer uses half-adders chained together. To add 1:
    bit0 → half_adder(bit0, 1) → sum0, carry
    bit1 → half_adder(bit1, carry) → sum1, carry
    ...and so on for all 12 bits.

This is simpler than a full adder chain because we're always adding
a constant (1 or 2), so one input is fixed.
"""

from __future__ import annotations

from arithmetic import half_adder
from logic_gates import register

from intel4004_gatelevel.bits import bits_to_int, int_to_bits


class ProgramCounter:
    """12-bit program counter built from flip-flops and half-adders.

    Supports:
        - increment(): PC += 1 (for 1-byte instructions)
        - increment2(): PC += 2 (for 2-byte instructions)
        - load(addr): PC = addr (for jumps)
        - read(): current PC value
    """

    def __init__(self) -> None:
        """Initialize PC to 0."""
        _, state = register(
            [0] * 12, clock=0, state=None, width=12
        )
        _, self._state = register(
            [0] * 12, clock=1, state=state, width=12
        )

    def read(self) -> int:
        """Read current PC value (0–4095)."""
        output, _ = register(
            [0] * 12, clock=0, state=self._state, width=12
        )
        return bits_to_int(output)

    def load(self, address: int) -> None:
        """Load a new address into the PC (for jumps)."""
        bits = int_to_bits(address & 0xFFF, 12)
        _, state = register(
            bits, clock=0, state=self._state, width=12
        )
        _, self._state = register(bits, clock=1, state=state, width=12)

    def increment(self) -> None:
        """Increment PC by 1 using a chain of half-adders.

        This is how a real incrementer works:
            carry_in = 1 (we're adding 1)
            For each bit position:
                (new_bit, carry) = half_adder(old_bit, carry)
        """
        current_bits = int_to_bits(self.read(), 12)
        carry = 1  # Adding 1
        new_bits = []
        for bit in current_bits:
            sum_bit, carry = half_adder(bit, carry)
            new_bits.append(sum_bit)
        self.load(bits_to_int(new_bits))

    def increment2(self) -> None:
        """Increment PC by 2 (for 2-byte instructions).

        Two cascaded increments through the half-adder chain.
        """
        self.increment()
        self.increment()

    def reset(self) -> None:
        """Reset PC to 0."""
        self.load(0)

    @property
    def gate_count(self) -> int:
        """12-bit register (72 gates) + 12 half-adders (24 gates) = 96."""
        return 96
