"""Configurable Logic Block (CLB) — the core compute tile of an FPGA.

=== What is a CLB? ===

A CLB is the primary logic resource in an FPGA. It's a tile on the FPGA
grid that contains multiple slices, each with LUTs, flip-flops, and carry
chains. CLBs are connected to each other through the routing fabric.

=== CLB Architecture ===

Our CLB follows the Xilinx-style architecture with 2 slices:

    ┌──────────────────────────────────────────────┐
    │                     CLB                       │
    │                                               │
    │  ┌─────────────────────┐                      │
    │  │       Slice 0       │                      │
    │  │  [LUT A] [LUT B]   │                      │
    │  │  [FF A]  [FF B]    │                      │
    │  │  [carry chain]      │                      │
    │  └─────────┬───────────┘                      │
    │            │ carry                             │
    │  ┌─────────▼───────────┐                      │
    │  │       Slice 1       │                      │
    │  │  [LUT A] [LUT B]   │                      │
    │  │  [FF A]  [FF B]    │                      │
    │  │  [carry chain]      │                      │
    │  └─────────────────────┘                      │
    │                                               │
    └──────────────────────────────────────────────┘

The carry chain flows from slice 0 → slice 1, enabling fast multi-bit
arithmetic within a single CLB. For wider operations, the carry chain
continues between adjacent CLBs in the same column.

=== CLB Capacity ===

One CLB with 2 slices × 2 LUTs per slice = 4 LUTs total.

A 4-input LUT can implement any boolean function of 4 variables, so
one CLB provides 4 independent boolean functions (or fewer wider
functions when LUTs are combined via carry chains).
"""

from __future__ import annotations

from dataclasses import dataclass

from fpga.slice import Slice, SliceOutput


@dataclass(frozen=True)
class CLBOutput:
    """Output from a CLB evaluation.

    Attributes:
        slice0: Output from slice 0
        slice1: Output from slice 1
    """

    slice0: SliceOutput
    slice1: SliceOutput


class CLB:
    """Configurable Logic Block — contains 2 slices.

    The carry chain connects slice 0's carry_out to slice 1's carry_in,
    enabling fast multi-bit arithmetic.

    Parameters:
        lut_inputs: Number of inputs per LUT (2 to 6, default 4)

    Example — 2-bit adder using carry chain:
        >>> clb = CLB(lut_inputs=4)
        >>> # Each slice computes one bit of the addition
        >>> # LUT A = XOR (sum bit), LUT B = AND (generate carry)
        >>> xor_tt = [0]*16; xor_tt[1] = 1; xor_tt[2] = 1
        >>> and_tt = [0]*16; and_tt[3] = 1
        >>> clb.slice0.configure(xor_tt, and_tt, carry_enabled=True)
        >>> clb.slice1.configure(xor_tt, and_tt, carry_enabled=True)
    """

    def __init__(self, lut_inputs: int = 4) -> None:
        self._slice0 = Slice(lut_inputs=lut_inputs)
        self._slice1 = Slice(lut_inputs=lut_inputs)
        self._k = lut_inputs

    @property
    def slice0(self) -> Slice:
        """First slice."""
        return self._slice0

    @property
    def slice1(self) -> Slice:
        """Second slice."""
        return self._slice1

    @property
    def k(self) -> int:
        """Number of LUT inputs per slice."""
        return self._k

    def evaluate(
        self,
        slice0_inputs_a: list[int],
        slice0_inputs_b: list[int],
        slice1_inputs_a: list[int],
        slice1_inputs_b: list[int],
        clock: int,
        carry_in: int = 0,
    ) -> CLBOutput:
        """Evaluate both slices in the CLB.

        The carry chain flows: carry_in → slice0 → slice1.

        Parameters:
            slice0_inputs_a: Inputs to slice 0's LUT A
            slice0_inputs_b: Inputs to slice 0's LUT B
            slice1_inputs_a: Inputs to slice 1's LUT A
            slice1_inputs_b: Inputs to slice 1's LUT B
            clock:           Clock signal (0 or 1)
            carry_in:        External carry input (default 0)

        Returns:
            CLBOutput containing both slices' outputs.
        """
        # Evaluate slice 0 first (carry chain starts here)
        out0 = self._slice0.evaluate(
            slice0_inputs_a,
            slice0_inputs_b,
            clock,
            carry_in=carry_in,
        )

        # Slice 1 receives carry from slice 0
        out1 = self._slice1.evaluate(
            slice1_inputs_a,
            slice1_inputs_b,
            clock,
            carry_in=out0.carry_out,
        )

        return CLBOutput(slice0=out0, slice1=out1)
