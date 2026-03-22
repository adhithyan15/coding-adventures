"""Slice — the building block of a Configurable Logic Block (CLB).

=== What is a Slice? ===

A slice is one "lane" inside a CLB. It combines:
- 2 LUTs (A and B) for combinational logic
- 2 D flip-flops for registered (sequential) outputs
- 2 output MUXes that choose between combinational or registered output
- Carry chain logic for fast arithmetic

The output MUX is critical: it lets the same slice be used for both
combinational circuits (bypass the flip-flop) and sequential circuits
(register the LUT output on the clock edge).

=== Slice Architecture ===

    inputs_a ──→ [LUT A] ──→ ┌─────────┐
                              │ MUX_A   │──→ output_a
                   ┌─→ [FF A]─→│(sel=ff_a)│
                   │          └─────────┘
                   │
    inputs_b ──→ [LUT B] ──→ ┌─────────┐
                              │ MUX_B   │──→ output_b
                   ┌─→ [FF B]─→│(sel=ff_b)│
                   │          └─────────┘
                   │
    carry_in ──→ [CARRY] ────────────────→ carry_out

    clock ──────→ [FF A] [FF B]

=== Carry Chain ===

For arithmetic operations, the carry chain connects adjacent slices
to propagate carry bits without going through the general routing
fabric. This is what makes FPGA arithmetic fast — dedicated carry
logic is hardwired between slices.

Our carry chain computes:
    carry_out = (LUT_A_out AND LUT_B_out) OR (carry_in AND (LUT_A_out XOR LUT_B_out))

This is the standard full-adder carry equation where LUT_A computes
the generate signal and LUT_B computes the propagate signal. When
carry is disabled, carry_out = 0.
"""

from __future__ import annotations

from dataclasses import dataclass

from logic_gates.combinational import mux2
from logic_gates.gates import AND, OR, XOR

from fpga.lut import LUT


@dataclass(frozen=True)
class SliceOutput:
    """Output from a single slice evaluation.

    Attributes:
        output_a:  LUT A result (combinational or registered)
        output_b:  LUT B result (combinational or registered)
        carry_out: Carry chain output (0 if carry disabled)
    """

    output_a: int
    output_b: int
    carry_out: int


class Slice:
    """One slice of a CLB: 2 LUTs + 2 flip-flops + output MUXes + carry chain.

    Parameters:
        lut_inputs: Number of inputs per LUT (2 to 6, default 4)

    Example — combinational AND + XOR:
        >>> s = Slice(lut_inputs=4)
        >>> and_tt = [0]*16; and_tt[3] = 1
        >>> xor_tt = [0]*16; xor_tt[1] = 1; xor_tt[2] = 1
        >>> s.configure(lut_a_table=and_tt, lut_b_table=xor_tt)
        >>> out = s.evaluate([1, 1, 0, 0], [1, 0, 0, 0], clock=0)
        >>> out.output_a  # AND(1,1) = 1
        1
        >>> out.output_b  # XOR(1,0) = 1
        1
    """

    def __init__(self, lut_inputs: int = 4) -> None:
        self._lut_a = LUT(k=lut_inputs)
        self._lut_b = LUT(k=lut_inputs)
        self._k = lut_inputs

        # Flip-flop state (using dict to match d_flip_flop API)
        self._ff_a_state: dict[str, int] = {
            "master_q": 0,
            "master_q_bar": 1,
            "slave_q": 0,
            "slave_q_bar": 1,
        }
        self._ff_b_state: dict[str, int] = {
            "master_q": 0,
            "master_q_bar": 1,
            "slave_q": 0,
            "slave_q_bar": 1,
        }

        # Configuration
        self._ff_a_enabled = False
        self._ff_b_enabled = False
        self._carry_enabled = False

    def configure(
        self,
        lut_a_table: list[int],
        lut_b_table: list[int],
        ff_a_enabled: bool = False,
        ff_b_enabled: bool = False,
        carry_enabled: bool = False,
    ) -> None:
        """Configure the slice's LUTs, flip-flops, and carry chain.

        Parameters:
            lut_a_table:   Truth table for LUT A (2^k entries)
            lut_b_table:   Truth table for LUT B (2^k entries)
            ff_a_enabled:  Route LUT A output through flip-flop A
            ff_b_enabled:  Route LUT B output through flip-flop B
            carry_enabled: Enable carry chain computation
        """
        self._lut_a.configure(lut_a_table)
        self._lut_b.configure(lut_b_table)
        self._ff_a_enabled = ff_a_enabled
        self._ff_b_enabled = ff_b_enabled
        self._carry_enabled = carry_enabled

        # Reset flip-flop state on reconfiguration
        self._ff_a_state = {
            "master_q": 0, "master_q_bar": 1,
            "slave_q": 0, "slave_q_bar": 1,
        }
        self._ff_b_state = {
            "master_q": 0, "master_q_bar": 1,
            "slave_q": 0, "slave_q_bar": 1,
        }

    def evaluate(
        self,
        inputs_a: list[int],
        inputs_b: list[int],
        clock: int,
        carry_in: int = 0,
    ) -> SliceOutput:
        """Evaluate the slice for one half-cycle.

        Parameters:
            inputs_a: Input bits for LUT A (length k)
            inputs_b: Input bits for LUT B (length k)
            clock:    Clock signal (0 or 1)
            carry_in: Carry input from previous slice (default 0)

        Returns:
            SliceOutput with output_a, output_b, and carry_out.
        """
        # Import here to avoid circular dependency at module level
        from logic_gates.sequential import d_flip_flop

        # Evaluate LUTs (combinational — always computed)
        lut_a_out = self._lut_a.evaluate(inputs_a)
        lut_b_out = self._lut_b.evaluate(inputs_b)

        # Flip-flop A: route through if enabled
        if self._ff_a_enabled:
            q_a, _, self._ff_a_state = d_flip_flop(
                lut_a_out, clock, **self._ff_a_state
            )
            # MUX: select registered (1) or combinational (0)
            output_a = mux2(lut_a_out, q_a, sel=1)
        else:
            output_a = lut_a_out

        # Flip-flop B: route through if enabled
        if self._ff_b_enabled:
            q_b, _, self._ff_b_state = d_flip_flop(
                lut_b_out, clock, **self._ff_b_state
            )
            output_b = mux2(lut_b_out, q_b, sel=1)
        else:
            output_b = lut_b_out

        # Carry chain: standard full-adder carry equation
        #   carry_out = (A AND B) OR (carry_in AND (A XOR B))
        if self._carry_enabled:
            carry_out = OR(
                AND(lut_a_out, lut_b_out),
                AND(carry_in, XOR(lut_a_out, lut_b_out)),
            )
        else:
            carry_out = 0

        return SliceOutput(
            output_a=output_a,
            output_b=output_b,
            carry_out=carry_out,
        )

    @property
    def lut_a(self) -> LUT:
        """LUT A (for inspection)."""
        return self._lut_a

    @property
    def lut_b(self) -> LUT:
        """LUT B (for inspection)."""
        return self._lut_b

    @property
    def k(self) -> int:
        """Number of LUT inputs."""
        return self._k
