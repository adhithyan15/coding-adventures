"""Hardware call stack — 3 levels of 12-bit return addresses.

=== The 4004's stack ===

The Intel 4004 has a 3-level hardware call stack. This is NOT a
software stack in RAM — it's three physical 12-bit registers plus
a 2-bit circular pointer, all built from D flip-flops.

Why only 3 levels? The 4004 was designed for calculators, which had
simple call structures. Three levels of subroutine nesting was enough
for the Busicom 141-PF calculator's firmware.

=== Silent overflow ===

When you push a 4th address, the stack wraps silently — the oldest
return address is overwritten. There is no stack overflow exception.
This matches the real hardware behavior. The 4004's designers saved
transistors by not including overflow detection.
"""

from __future__ import annotations

from logic_gates import register

from intel4004_gatelevel.bits import bits_to_int, int_to_bits


class HardwareStack:
    """3-level × 12-bit hardware call stack.

    Built from 3 × 12 = 36 D flip-flops for storage, plus a 2-bit
    pointer that wraps modulo 3.
    """

    def __init__(self) -> None:
        """Initialize stack with 3 empty slots and pointer at 0."""
        self._levels: list[list[dict[str, int]]] = []
        for _ in range(3):
            _, state = register([0] * 12, clock=0, width=12)
            _, state = register([0] * 12, clock=1, state=state, width=12)
            self._levels.append(state)
        self._pointer = 0  # 0, 1, or 2

    def push(self, address: int) -> None:
        """Push a return address. Wraps silently on overflow.

        In real hardware: the pointer selects which of the 3 registers
        to write, then the pointer increments mod 3.
        """
        bits = int_to_bits(address & 0xFFF, 12)
        _, state = register(
            bits, clock=0, state=self._levels[self._pointer], width=12
        )
        _, self._levels[self._pointer] = register(
            bits, clock=1, state=state, width=12
        )
        self._pointer = (self._pointer + 1) % 3

    def pop(self) -> int:
        """Pop and return the top address.

        Decrements pointer mod 3, then reads that register.
        """
        self._pointer = (self._pointer - 1) % 3
        output, _ = register(
            [0] * 12, clock=0, state=self._levels[self._pointer], width=12
        )
        return bits_to_int(output)

    def reset(self) -> None:
        """Reset all stack levels to 0 and pointer to 0."""
        for i in range(3):
            bits = [0] * 12
            _, state = register(bits, clock=0, width=12)
            _, self._levels[i] = register(
                bits, clock=1, state=state, width=12
            )
        self._pointer = 0

    @property
    def depth(self) -> int:
        """Current pointer position (not true depth, since we wrap)."""
        return self._pointer

    @property
    def gate_count(self) -> int:
        """3 × 12-bit registers (216 gates) + pointer logic (~10 gates)."""
        return 226
