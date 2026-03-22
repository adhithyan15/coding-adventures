"""Tests for Slice — LUTs + flip-flops + carry chain.

Coverage targets:
- Combinational-only evaluation (no flip-flops)
- Registered output (flip-flops enabled)
- Carry chain computation
- Configuration and reconfiguration
"""

from __future__ import annotations

from fpga.slice import Slice, SliceOutput

# ─── Helper truth tables ─────────────────────────────────────────────

def and_tt() -> list[int]:
    """2-input AND using I0, I1 in k=4."""
    tt = [0] * 16
    tt[3] = 1
    return tt


def xor_tt() -> list[int]:
    """2-input XOR using I0, I1 in k=4."""
    tt = [0] * 16
    tt[1] = 1
    tt[2] = 1
    return tt


def const_one_tt() -> list[int]:
    """Always outputs 1."""
    return [1] * 16


def const_zero_tt() -> list[int]:
    """Always outputs 0."""
    return [0] * 16


# ─── SliceOutput ──────────────────────────────────────────────────────

class TestSliceOutput:
    def test_fields(self) -> None:
        out = SliceOutput(output_a=1, output_b=0, carry_out=1)
        assert out.output_a == 1
        assert out.output_b == 0
        assert out.carry_out == 1

    def test_frozen(self) -> None:
        out = SliceOutput(output_a=0, output_b=0, carry_out=0)
        try:
            out.output_a = 1  # type: ignore[misc]
            raised = False
        except AttributeError:
            raised = True
        assert raised


# ─── Combinational Mode ──────────────────────────────────────────────

class TestSliceCombinational:
    def test_and_xor(self) -> None:
        s = Slice(lut_inputs=4)
        s.configure(lut_a_table=and_tt(), lut_b_table=xor_tt())
        out = s.evaluate([1, 1, 0, 0], [1, 0, 0, 0], clock=0)
        assert out.output_a == 1  # AND(1,1) = 1
        assert out.output_b == 1  # XOR(1,0) = 1
        assert out.carry_out == 0  # carry disabled

    def test_both_zero(self) -> None:
        s = Slice(lut_inputs=4)
        s.configure(lut_a_table=and_tt(), lut_b_table=and_tt())
        out = s.evaluate([0, 0, 0, 0], [0, 0, 0, 0], clock=0)
        assert out.output_a == 0
        assert out.output_b == 0

    def test_k_property(self) -> None:
        s = Slice(lut_inputs=3)
        assert s.k == 3

    def test_lut_properties(self) -> None:
        s = Slice(lut_inputs=4)
        s.configure(lut_a_table=and_tt(), lut_b_table=xor_tt())
        assert s.lut_a.truth_table == and_tt()
        assert s.lut_b.truth_table == xor_tt()


# ─── Registered Mode ─────────────────────────────────────────────────

class TestSliceRegistered:
    def test_ff_a_captures_on_clock_edge(self) -> None:
        """Flip-flop captures LUT output on clock 0→1 transition."""
        s = Slice(lut_inputs=4)
        s.configure(
            lut_a_table=const_one_tt(),
            lut_b_table=const_zero_tt(),
            ff_a_enabled=True,
        )
        # Clock low: master absorbs data
        s.evaluate([0, 0, 0, 0], [0, 0, 0, 0], clock=0)
        # Clock high: slave outputs
        out1 = s.evaluate([0, 0, 0, 0], [0, 0, 0, 0], clock=1)
        assert out1.output_a == 1  # FF captured the 1 from LUT A
        assert out1.output_b == 0  # No FF on B

    def test_ff_b_captures(self) -> None:
        s = Slice(lut_inputs=4)
        s.configure(
            lut_a_table=const_zero_tt(),
            lut_b_table=const_one_tt(),
            ff_b_enabled=True,
        )
        s.evaluate([0, 0, 0, 0], [0, 0, 0, 0], clock=0)
        out = s.evaluate([0, 0, 0, 0], [0, 0, 0, 0], clock=1)
        assert out.output_a == 0
        assert out.output_b == 1  # FF captured the 1

    def test_reconfigure_resets_ff_state(self) -> None:
        """Reconfiguring should reset flip-flop state."""
        s = Slice(lut_inputs=4)
        s.configure(
            lut_a_table=const_one_tt(),
            lut_b_table=const_zero_tt(),
            ff_a_enabled=True,
        )
        # Capture a 1
        s.evaluate([0, 0, 0, 0], [0, 0, 0, 0], clock=0)
        s.evaluate([0, 0, 0, 0], [0, 0, 0, 0], clock=1)
        # Reconfigure with all zeros
        s.configure(
            lut_a_table=const_zero_tt(),
            lut_b_table=const_zero_tt(),
            ff_a_enabled=True,
        )
        # FF state should be reset
        out = s.evaluate([0, 0, 0, 0], [0, 0, 0, 0], clock=0)
        # At clock=0, master absorbs data (0), slave retains reset value (0)
        assert out.output_a == 0


# ─── Carry Chain ──────────────────────────────────────────────────────

class TestSliceCarryChain:
    def test_carry_generate(self) -> None:
        """When both LUTs output 1: carry_out = 1 regardless of carry_in."""
        s = Slice(lut_inputs=4)
        s.configure(
            lut_a_table=const_one_tt(),
            lut_b_table=const_one_tt(),
            carry_enabled=True,
        )
        out = s.evaluate([0, 0, 0, 0], [0, 0, 0, 0], clock=0, carry_in=0)
        assert out.carry_out == 1

    def test_carry_propagate(self) -> None:
        """When LUT_A XOR LUT_B = 1, carry_in propagates to carry_out."""
        s = Slice(lut_inputs=4)
        s.configure(
            lut_a_table=const_one_tt(),
            lut_b_table=const_zero_tt(),
            carry_enabled=True,
        )
        # A=1, B=0: A XOR B = 1, A AND B = 0
        # carry_out = 0 OR (carry_in AND 1) = carry_in
        out0 = s.evaluate([0, 0, 0, 0], [0, 0, 0, 0], clock=0, carry_in=0)
        assert out0.carry_out == 0
        out1 = s.evaluate([0, 0, 0, 0], [0, 0, 0, 0], clock=0, carry_in=1)
        assert out1.carry_out == 1

    def test_carry_kill(self) -> None:
        """When both LUTs output 0: carry_out = 0 regardless of carry_in."""
        s = Slice(lut_inputs=4)
        s.configure(
            lut_a_table=const_zero_tt(),
            lut_b_table=const_zero_tt(),
            carry_enabled=True,
        )
        out = s.evaluate([0, 0, 0, 0], [0, 0, 0, 0], clock=0, carry_in=1)
        assert out.carry_out == 0

    def test_carry_disabled_is_zero(self) -> None:
        """When carry_enabled=False, carry_out is always 0."""
        s = Slice(lut_inputs=4)
        s.configure(
            lut_a_table=const_one_tt(),
            lut_b_table=const_one_tt(),
            carry_enabled=False,
        )
        out = s.evaluate([0, 0, 0, 0], [0, 0, 0, 0], clock=0, carry_in=1)
        assert out.carry_out == 0
