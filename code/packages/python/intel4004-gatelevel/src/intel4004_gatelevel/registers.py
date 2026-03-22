"""Register file — 16 × 4-bit registers built from D flip-flops.

=== How registers work in hardware ===

A register is a group of D flip-flops that share a clock signal. Each
flip-flop stores one bit. A 4-bit register has 4 flip-flops. The Intel
4004 has 16 such registers (R0–R15), for a total of 64 flip-flops just
for the register file.

In this simulation, each register call goes through:
    data bits → D flip-flop × 4 → output bits

The flip-flops are edge-triggered: they capture new data on the rising
edge of the clock. Between edges, the stored value is stable.

=== Register pairs ===

The 4004 organizes its 16 registers into 8 pairs:
    P0 = R0:R1, P1 = R2:R3, ..., P7 = R14:R15

A register pair holds an 8-bit value (high nibble in even register,
low nibble in odd register). Pairs are used for:
    - FIM: load 8-bit immediate
    - SRC: set RAM address
    - FIN: indirect ROM read
    - JIN: indirect jump

=== Accumulator ===

The accumulator is a separate 4-bit register, not part of the R0–R15
file. It has its own dedicated flip-flops and is connected directly to
the ALU's output bus.
"""

from __future__ import annotations

from logic_gates import register

from intel4004_gatelevel.bits import bits_to_int, int_to_bits


class RegisterFile:
    """16 × 4-bit register file built from D flip-flops.

    Each of the 16 registers is a group of 4 D flip-flops from the
    logic_gates sequential module. Reading and writing go through
    actual flip-flop state transitions.
    """

    def __init__(self) -> None:
        """Initialize 16 registers, each with 4-bit flip-flop state."""
        # Each register's state is a list of dicts (one per flip-flop)
        # Initialize all to 0 by clocking in zeros
        self._states: list[list[dict[str, int]]] = []
        for _ in range(16):
            # Initialize state by clocking zeros through
            _, state = register(
                [0, 0, 0, 0], clock=0, state=None, width=4
            )
            _, state = register(
                [0, 0, 0, 0], clock=1, state=state, width=4
            )
            self._states.append(state)

    def read(self, index: int) -> int:
        """Read a register value. Returns 4-bit integer (0–15).

        In real hardware, this would route through a 16-to-1 multiplexer
        built from gates. We simulate the flip-flop read directly.
        """
        # Read current output from flip-flops (clock=0, no write)
        output, _ = register(
            [0, 0, 0, 0], clock=0, state=self._states[index], width=4
        )
        return bits_to_int(output)

    def write(self, index: int, value: int) -> None:
        """Write a 4-bit value to a register.

        In real hardware: decoder selects the register, data bus presents
        the value, clock edge latches it into the flip-flops.
        """
        bits = int_to_bits(value & 0xF, 4)
        # Clock low (setup)
        _, state = register(
            bits, clock=0, state=self._states[index], width=4
        )
        # Clock high (capture on rising edge)
        _, state = register(bits, clock=1, state=state, width=4)
        self._states[index] = state

    def read_pair(self, pair_index: int) -> int:
        """Read an 8-bit value from a register pair.

        Pair 0 = R0:R1 (R0=high nibble, R1=low nibble).
        """
        high = self.read(pair_index * 2)
        low = self.read(pair_index * 2 + 1)
        return (high << 4) | low

    def write_pair(self, pair_index: int, value: int) -> None:
        """Write an 8-bit value to a register pair."""
        self.write(pair_index * 2, (value >> 4) & 0xF)
        self.write(pair_index * 2 + 1, value & 0xF)

    def reset(self) -> None:
        """Reset all registers to 0 by clocking in zeros."""
        for i in range(16):
            self.write(i, 0)

    @property
    def gate_count(self) -> int:
        """Gate count for the register file.

        16 registers × 4 bits × ~6 gates per D flip-flop = 384 gates.
        Plus 4-to-16 decoder for write select: ~32 gates.
        Plus 16-to-1 mux for read select: ~64 gates.
        Total: ~480 gates.
        """
        return 480


class Accumulator:
    """4-bit accumulator register built from D flip-flops.

    The accumulator is the 4004's main working register. Almost every
    arithmetic and logic operation reads from or writes to it.
    """

    def __init__(self) -> None:
        """Initialize accumulator to 0."""
        _, state = register([0, 0, 0, 0], clock=0, width=4)
        _, self._state = register(
            [0, 0, 0, 0], clock=1, state=state, width=4
        )

    def read(self) -> int:
        """Read the accumulator value (0–15)."""
        output, _ = register(
            [0, 0, 0, 0], clock=0, state=self._state, width=4
        )
        return bits_to_int(output)

    def write(self, value: int) -> None:
        """Write a 4-bit value to the accumulator."""
        bits = int_to_bits(value & 0xF, 4)
        _, state = register(bits, clock=0, state=self._state, width=4)
        _, self._state = register(bits, clock=1, state=state, width=4)

    def reset(self) -> None:
        """Reset to 0."""
        self.write(0)

    @property
    def gate_count(self) -> int:
        """4 D flip-flops × ~6 gates = 24 gates."""
        return 24


class CarryFlag:
    """1-bit carry/borrow flag built from a D flip-flop.

    The carry flag is set by arithmetic operations and read by
    conditional jumps and multi-digit BCD arithmetic.
    """

    def __init__(self) -> None:
        """Initialize carry to 0 (False)."""
        _, state = register([0], clock=0, width=1)
        _, self._state = register([0], clock=1, state=state, width=1)

    def read(self) -> bool:
        """Read carry flag as a boolean."""
        output, _ = register(
            [0], clock=0, state=self._state, width=1
        )
        return output[0] == 1

    def write(self, value: bool) -> None:
        """Write carry flag."""
        bit = [1 if value else 0]
        _, state = register(bit, clock=0, state=self._state, width=1)
        _, self._state = register(bit, clock=1, state=state, width=1)

    def reset(self) -> None:
        """Reset to 0."""
        self.write(False)

    @property
    def gate_count(self) -> int:
        """1 D flip-flop × ~6 gates = 6 gates."""
        return 6
