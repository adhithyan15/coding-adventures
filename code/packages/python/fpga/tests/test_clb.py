"""Tests for CLB (Configurable Logic Block).

Coverage targets:
- Basic evaluation with both slices
- Carry chain propagation between slices
- Properties and access to sub-slices
"""

from __future__ import annotations

from fpga.clb import CLB, CLBOutput

# ─── Helpers ──────────────────────────────────────────────────────────

def and_tt() -> list[int]:
    tt = [0] * 16
    tt[3] = 1
    return tt


def xor_tt() -> list[int]:
    tt = [0] * 16
    tt[1] = 1
    tt[2] = 1
    return tt


def const_one() -> list[int]:
    return [1] * 16


def const_zero() -> list[int]:
    return [0] * 16


ZEROS = [0, 0, 0, 0]


# ─── CLBOutput ────────────────────────────────────────────────────────

class TestCLBOutput:
    def test_fields(self) -> None:
        from fpga.slice import SliceOutput

        s0 = SliceOutput(output_a=1, output_b=0, carry_out=0)
        s1 = SliceOutput(output_a=0, output_b=1, carry_out=0)
        out = CLBOutput(slice0=s0, slice1=s1)
        assert out.slice0.output_a == 1
        assert out.slice1.output_b == 1


# ─── Basic Evaluation ────────────────────────────────────────────────

class TestCLBEvaluation:
    def test_both_slices_evaluate(self) -> None:
        clb = CLB(lut_inputs=4)
        clb.slice0.configure(lut_a_table=and_tt(), lut_b_table=xor_tt())
        clb.slice1.configure(lut_a_table=xor_tt(), lut_b_table=and_tt())

        out = clb.evaluate(
            slice0_inputs_a=[1, 1, 0, 0],
            slice0_inputs_b=[1, 0, 0, 0],
            slice1_inputs_a=[0, 1, 0, 0],
            slice1_inputs_b=[1, 1, 0, 0],
            clock=0,
        )
        assert out.slice0.output_a == 1  # AND(1,1)
        assert out.slice0.output_b == 1  # XOR(1,0)
        assert out.slice1.output_a == 1  # XOR(0,1)
        assert out.slice1.output_b == 1  # AND(1,1)

    def test_all_zeros(self) -> None:
        clb = CLB(lut_inputs=4)
        clb.slice0.configure(lut_a_table=and_tt(), lut_b_table=and_tt())
        clb.slice1.configure(lut_a_table=and_tt(), lut_b_table=and_tt())

        out = clb.evaluate(ZEROS, ZEROS, ZEROS, ZEROS, clock=0)
        assert out.slice0.output_a == 0
        assert out.slice0.output_b == 0
        assert out.slice1.output_a == 0
        assert out.slice1.output_b == 0


# ─── Carry Chain ──────────────────────────────────────────────────────

class TestCLBCarryChain:
    def test_carry_propagates_slice0_to_slice1(self) -> None:
        """Carry out of slice 0 becomes carry in of slice 1."""
        clb = CLB(lut_inputs=4)
        # Slice 0: generate carry (both LUTs output 1)
        clb.slice0.configure(
            lut_a_table=const_one(),
            lut_b_table=const_one(),
            carry_enabled=True,
        )
        # Slice 1: propagate carry (A=1, B=0 → XOR=1)
        clb.slice1.configure(
            lut_a_table=const_one(),
            lut_b_table=const_zero(),
            carry_enabled=True,
        )

        out = clb.evaluate(ZEROS, ZEROS, ZEROS, ZEROS, clock=0, carry_in=0)
        assert out.slice0.carry_out == 1
        # Slice 1: A=1, B=0, carry_in=1 (from slice 0)
        # carry_out = (1 AND 0) OR (1 AND (1 XOR 0)) = 0 OR 1 = 1
        assert out.slice1.carry_out == 1

    def test_external_carry_in(self) -> None:
        """External carry_in feeds into slice 0."""
        clb = CLB(lut_inputs=4)
        # Both LUTs output 0: carry_out = (0 AND 0) OR (cin AND 0) = 0
        clb.slice0.configure(
            lut_a_table=const_zero(),
            lut_b_table=const_zero(),
            carry_enabled=True,
        )
        clb.slice1.configure(
            lut_a_table=const_zero(),
            lut_b_table=const_zero(),
            carry_enabled=True,
        )

        out = clb.evaluate(ZEROS, ZEROS, ZEROS, ZEROS, clock=0, carry_in=1)
        assert out.slice0.carry_out == 0  # Kill
        assert out.slice1.carry_out == 0  # Kill


# ─── Properties ───────────────────────────────────────────────────────

class TestCLBProperties:
    def test_k(self) -> None:
        clb = CLB(lut_inputs=3)
        assert clb.k == 3

    def test_slice_access(self) -> None:
        clb = CLB(lut_inputs=4)
        assert clb.slice0 is not clb.slice1
        assert clb.slice0.k == 4
        assert clb.slice1.k == 4
