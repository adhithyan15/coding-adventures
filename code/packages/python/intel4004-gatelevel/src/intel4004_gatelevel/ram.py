"""RAM — 4 banks × 4 registers × 20 nibbles, built from flip-flops.

=== The 4004's RAM architecture ===

The Intel 4004 used separate RAM chips (Intel 4002), each containing:
    - 4 registers
    - Each register has 16 main characters + 4 status characters
    - Each character is a 4-bit nibble
    - Total per chip: 4 × 20 × 4 = 320 bits

The full system supports up to 4 RAM banks (4 chips), selected by the
DCL instruction. Within a bank, the SRC instruction sets which register
and character to access.

In real hardware, each nibble is stored in 4 D flip-flops. The full
RAM system uses 4 × 4 × 20 × 4 = 1,280 flip-flops. We simulate this
using the register() function from the logic_gates package.

=== Addressing ===

RAM is addressed in two steps:
    1. DCL sets the bank (0–3, from accumulator bits 0–2)
    2. SRC sends an 8-bit address from a register pair:
       - High nibble → register index (0–3)
       - Low nibble → character index (0–15)
"""

from __future__ import annotations

from logic_gates import register

from intel4004_gatelevel.bits import bits_to_int, int_to_bits


class RAM:
    """4004 RAM: 4 banks × 4 registers × (16 main + 4 status) nibbles.

    Every nibble is stored in 4 D flip-flops from the sequential logic
    package. Reading and writing physically route through flip-flop
    state transitions.
    """

    def __init__(self) -> None:
        """Initialize all RAM to 0."""
        # main[bank][reg][char] = flip-flop state for one nibble
        self._main: list[list[list[list[dict[str, int]]]]] = []
        self._status: list[list[list[list[dict[str, int]]]]] = []

        for _bank in range(4):
            bank_main = []
            bank_status = []
            for _reg in range(4):
                reg_main = []
                for _char in range(16):
                    _, state = register([0, 0, 0, 0], clock=0, width=4)
                    _, state = register(
                        [0, 0, 0, 0], clock=1, state=state, width=4
                    )
                    reg_main.append(state)
                bank_main.append(reg_main)

                reg_status = []
                for _stat in range(4):
                    _, state = register([0, 0, 0, 0], clock=0, width=4)
                    _, state = register(
                        [0, 0, 0, 0], clock=1, state=state, width=4
                    )
                    reg_status.append(state)
                bank_status.append(reg_status)

            self._main.append(bank_main)
            self._status.append(bank_status)

        # Output ports (one per bank, written by WMP)
        self._output: list[int] = [0, 0, 0, 0]

    def read_main(
        self, bank: int, reg: int, char: int
    ) -> int:
        """Read a main character (4-bit nibble) from RAM."""
        state = self._main[bank & 3][reg & 3][char & 0xF]
        output, _ = register([0, 0, 0, 0], clock=0, state=state, width=4)
        return bits_to_int(output)

    def write_main(
        self, bank: int, reg: int, char: int, value: int
    ) -> None:
        """Write a 4-bit value to a main character."""
        bits = int_to_bits(value & 0xF, 4)
        state = self._main[bank & 3][reg & 3][char & 0xF]
        _, state = register(bits, clock=0, state=state, width=4)
        _, state = register(bits, clock=1, state=state, width=4)
        self._main[bank & 3][reg & 3][char & 0xF] = state

    def read_status(
        self, bank: int, reg: int, index: int
    ) -> int:
        """Read a status character (0–3) from RAM."""
        state = self._status[bank & 3][reg & 3][index & 3]
        output, _ = register([0, 0, 0, 0], clock=0, state=state, width=4)
        return bits_to_int(output)

    def write_status(
        self, bank: int, reg: int, index: int, value: int
    ) -> None:
        """Write a 4-bit value to a status character."""
        bits = int_to_bits(value & 0xF, 4)
        state = self._status[bank & 3][reg & 3][index & 3]
        _, state = register(bits, clock=0, state=state, width=4)
        _, state = register(bits, clock=1, state=state, width=4)
        self._status[bank & 3][reg & 3][index & 3] = state

    def read_output(self, bank: int) -> int:
        """Read a RAM output port value."""
        return self._output[bank & 3]

    def write_output(self, bank: int, value: int) -> None:
        """Write to a RAM output port (WMP instruction)."""
        self._output[bank & 3] = value & 0xF

    def reset(self) -> None:
        """Reset all RAM to 0."""
        for bank in range(4):
            for reg in range(4):
                for char in range(16):
                    self.write_main(bank, reg, char, 0)
                for stat in range(4):
                    self.write_status(bank, reg, stat, 0)
            self._output[bank] = 0

    @property
    def gate_count(self) -> int:
        """4 banks × 4 regs × 20 nibbles × 4 bits × 6 gates/ff ≈ 7680.

        Plus addressing/decoding: ~200 gates.
        """
        return 7880
