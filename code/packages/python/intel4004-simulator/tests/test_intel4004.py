"""Tests for the Intel 4004 simulator.

These tests verify each instruction independently, then test them together
in the x = 1 + 2 end-to-end program. The key constraint tested throughout:
all values are 4 bits (0-15), enforced by masking with & 0xF.
"""

from intel4004_simulator.simulator import Intel4004Simulator


# ---------------------------------------------------------------------------
# LDM — Load immediate into accumulator
# ---------------------------------------------------------------------------


class TestLDM:
    """LDM N (0xDN): Load a 4-bit immediate value into the accumulator."""

    def test_ldm_sets_accumulator(self) -> None:
        """LDM 5 should set A = 5."""
        sim = Intel4004Simulator()
        # LDM 5 = 0xD5, HLT = 0x01
        traces = sim.run(bytes([0xD5, 0x01]))

        assert sim.accumulator == 5
        assert traces[0].mnemonic == "LDM 5"
        assert traces[0].accumulator_before == 0
        assert traces[0].accumulator_after == 5

    def test_ldm_zero(self) -> None:
        """LDM 0 should set A = 0."""
        sim = Intel4004Simulator()
        traces = sim.run(bytes([0xD0, 0x01]))

        assert sim.accumulator == 0
        assert traces[0].mnemonic == "LDM 0"

    def test_ldm_max_value(self) -> None:
        """LDM 15 should set A = 15 (the maximum 4-bit value)."""
        sim = Intel4004Simulator()
        traces = sim.run(bytes([0xDF, 0x01]))

        assert sim.accumulator == 15
        assert traces[0].mnemonic == "LDM 15"


# ---------------------------------------------------------------------------
# XCH — Exchange accumulator with register
# ---------------------------------------------------------------------------


class TestXCH:
    """XCH RN (0xBN): Swap the accumulator and register N."""

    def test_xch_swaps_values(self) -> None:
        """XCH R0 should swap A and R0.

        Start: A=7, R0=0. After XCH: A=0, R0=7.
        """
        sim = Intel4004Simulator()
        # LDM 7 (A=7), XCH R0 (swap A and R0), HLT
        traces = sim.run(bytes([0xD7, 0xB0, 0x01]))

        assert sim.accumulator == 0  # A got R0's old value (0)
        assert sim.registers[0] == 7  # R0 got A's old value (7)

    def test_xch_is_symmetric(self) -> None:
        """Two XCH operations on the same register restore original state."""
        sim = Intel4004Simulator()
        # LDM 3 (A=3), XCH R5 (R5=3, A=0), XCH R5 (A=3, R5=0), HLT
        sim.run(bytes([0xD3, 0xB5, 0xB5, 0x01]))

        assert sim.accumulator == 3
        assert sim.registers[5] == 0

    def test_xch_high_register(self) -> None:
        """XCH R15 should work with the highest register number."""
        sim = Intel4004Simulator()
        # LDM 9, XCH R15, HLT
        sim.run(bytes([0xD9, 0xBF, 0x01]))

        assert sim.registers[15] == 9
        assert sim.accumulator == 0


# ---------------------------------------------------------------------------
# ADD — Add register to accumulator
# ---------------------------------------------------------------------------


class TestADD:
    """ADD RN (0x8N): A = A + RN, set carry on overflow."""

    def test_add_basic(self) -> None:
        """2 + 3 = 5, no carry."""
        sim = Intel4004Simulator()
        # LDM 3, XCH R0 (R0=3), LDM 2 (A=2), ADD R0 (A=5), HLT
        sim.run(bytes([0xD3, 0xB0, 0xD2, 0x80, 0x01]))

        assert sim.accumulator == 5
        assert sim.carry is False

    def test_add_carry_on_overflow(self) -> None:
        """15 + 1 = 0 with carry. This is the 4-bit overflow behavior.

        In 4 bits: 1111 + 0001 = 10000, which truncates to 0000 with carry.
        """
        sim = Intel4004Simulator()
        # LDM 1, XCH R0 (R0=1), LDM 15 (A=15), ADD R0 (A=0, carry=1), HLT
        sim.run(bytes([0xD1, 0xB0, 0xDF, 0x80, 0x01]))

        assert sim.accumulator == 0
        assert sim.carry is True

    def test_add_no_carry_at_boundary(self) -> None:
        """8 + 7 = 15, no carry (exactly at the maximum)."""
        sim = Intel4004Simulator()
        # LDM 7, XCH R0 (R0=7), LDM 8 (A=8), ADD R0 (A=15), HLT
        sim.run(bytes([0xD7, 0xB0, 0xD8, 0x80, 0x01]))

        assert sim.accumulator == 15
        assert sim.carry is False

    def test_add_both_max(self) -> None:
        """15 + 15 = 14 with carry (30 in decimal, 0x1E masked to 0xE)."""
        sim = Intel4004Simulator()
        # LDM 15, XCH R0, LDM 15, ADD R0, HLT
        sim.run(bytes([0xDF, 0xB0, 0xDF, 0x80, 0x01]))

        assert sim.accumulator == 14  # 30 & 0xF = 14
        assert sim.carry is True


# ---------------------------------------------------------------------------
# SUB — Subtract register from accumulator
# ---------------------------------------------------------------------------


class TestSUB:
    """SUB RN (0x9N): A = A - RN, set carry (borrow) on underflow."""

    def test_sub_basic(self) -> None:
        """5 - 3 = 2, no borrow."""
        sim = Intel4004Simulator()
        # LDM 3, XCH R0 (R0=3), LDM 5 (A=5), SUB R0 (A=2), HLT
        sim.run(bytes([0xD3, 0xB0, 0xD5, 0x90, 0x01]))

        assert sim.accumulator == 2
        assert sim.carry is False

    def test_sub_borrow_on_underflow(self) -> None:
        """0 - 1 = 15 with borrow. The 4-bit wraparound.

        In 4 bits: 0000 - 0001 = 1111 (15) with borrow.
        """
        sim = Intel4004Simulator()
        # LDM 1, XCH R0 (R0=1), LDM 0 (A=0), SUB R0 (A=15, carry=1), HLT
        sim.run(bytes([0xD1, 0xB0, 0xD0, 0x90, 0x01]))

        assert sim.accumulator == 15
        assert sim.carry is True

    def test_sub_equal_values(self) -> None:
        """7 - 7 = 0, no borrow."""
        sim = Intel4004Simulator()
        # LDM 7, XCH R0, LDM 7, SUB R0, HLT
        sim.run(bytes([0xD7, 0xB0, 0xD7, 0x90, 0x01]))

        assert sim.accumulator == 0
        assert sim.carry is False


# ---------------------------------------------------------------------------
# 4-bit masking — the fundamental constraint
# ---------------------------------------------------------------------------


class TestFourBitMasking:
    """All values must be masked to 4 bits (0-15). This is the defining
    characteristic of the 4004 — it's a 4-bit machine."""

    def test_accumulator_never_exceeds_15(self) -> None:
        """After any operation, the accumulator must be in range [0, 15]."""
        sim = Intel4004Simulator()
        # LDM 15, XCH R0, LDM 15, ADD R0 -> would be 30, must be 14
        sim.run(bytes([0xDF, 0xB0, 0xDF, 0x80, 0x01]))

        assert 0 <= sim.accumulator <= 15

    def test_registers_never_exceed_15(self) -> None:
        """After any operation, registers must be in range [0, 15]."""
        sim = Intel4004Simulator()
        # Store values in several registers
        sim.run(bytes([
            0xDF, 0xB0,  # LDM 15, XCH R0
            0xDA, 0xB1,  # LDM 10, XCH R1
            0xD0, 0xB2,  # LDM 0, XCH R2
            0x01,        # HLT
        ]))

        for i, val in enumerate(sim.registers):
            assert 0 <= val <= 15, f"R{i} = {val} is out of 4-bit range"

    def test_sub_wraps_to_4_bits(self) -> None:
        """Subtraction wraps around in 4 bits: 3 - 5 = 14."""
        sim = Intel4004Simulator()
        # LDM 5, XCH R0 (R0=5), LDM 3 (A=3), SUB R0 (A=3-5=-2 -> 14), HLT
        sim.run(bytes([0xD5, 0xB0, 0xD3, 0x90, 0x01]))

        assert sim.accumulator == 14  # -2 & 0xF = 14
        assert sim.carry is True  # borrow occurred


# ---------------------------------------------------------------------------
# HLT — Halt execution
# ---------------------------------------------------------------------------


class TestHLT:
    """HLT (0x01): Stop the CPU."""

    def test_hlt_stops_execution(self) -> None:
        """HLT should set the halted flag and stop run()."""
        sim = Intel4004Simulator()
        traces = sim.run(bytes([0x01]))

        assert sim.halted is True
        assert len(traces) == 1
        assert traces[0].mnemonic == "HLT"

    def test_hlt_mid_program(self) -> None:
        """Instructions after HLT should not execute."""
        sim = Intel4004Simulator()
        # HLT, LDM 5 (should never run)
        traces = sim.run(bytes([0x01, 0xD5]))

        assert sim.halted is True
        assert sim.accumulator == 0  # LDM 5 never executed
        assert len(traces) == 1

    def test_step_after_halt_raises(self) -> None:
        """Stepping after HLT should raise an error."""
        sim = Intel4004Simulator()
        sim.run(bytes([0x01]))

        try:
            sim.step()
            assert False, "Should have raised RuntimeError"
        except RuntimeError:
            pass  # Expected


# ---------------------------------------------------------------------------
# End-to-end: x = 1 + 2
# ---------------------------------------------------------------------------


class TestEndToEnd:
    """The canonical x = 1 + 2 program, testing the full instruction flow."""

    def test_x_equals_1_plus_2(self) -> None:
        """Compute 1 + 2 = 3, store in R1.

        Program:
            LDM 1      A = 1               0xD1
            XCH R0     R0 = 1, A = 0       0xB0
            LDM 2      A = 2               0xD2
            ADD R0     A = 2 + 1 = 3       0x80
            XCH R1     R1 = 3, A = 0       0xB1
            HLT        stop                0x01
        """
        sim = Intel4004Simulator()
        program = bytes([0xD1, 0xB0, 0xD2, 0x80, 0xB1, 0x01])
        traces = sim.run(program)

        # Final state
        assert sim.registers[1] == 3  # R1 = 1 + 2 = 3
        assert sim.registers[0] == 1  # R0 still holds the 1
        assert sim.accumulator == 0  # Last XCH cleared A
        assert sim.carry is False  # No overflow occurred
        assert sim.halted is True

        # Verify trace
        assert len(traces) == 6
        assert traces[0].mnemonic == "LDM 1"
        assert traces[1].mnemonic == "XCH R0"
        assert traces[2].mnemonic == "LDM 2"
        assert traces[3].mnemonic == "ADD R0"
        assert traces[4].mnemonic == "XCH R1"
        assert traces[5].mnemonic == "HLT"

    def test_trace_accumulator_flow(self) -> None:
        """Verify the accumulator values through each step of x = 1 + 2.

        This traces the data flow through the accumulator bottleneck:
            LDM 1:   A: 0 -> 1
            XCH R0:  A: 1 -> 0  (swapped with R0)
            LDM 2:   A: 0 -> 2
            ADD R0:  A: 2 -> 3  (added R0=1)
            XCH R1:  A: 3 -> 0  (swapped with R1)
            HLT:     A: 0 -> 0
        """
        sim = Intel4004Simulator()
        program = bytes([0xD1, 0xB0, 0xD2, 0x80, 0xB1, 0x01])
        traces = sim.run(program)

        expected_acc = [
            (0, 1),  # LDM 1
            (1, 0),  # XCH R0
            (0, 2),  # LDM 2
            (2, 3),  # ADD R0
            (3, 0),  # XCH R1
            (0, 0),  # HLT
        ]

        for trace, (before, after) in zip(traces, expected_acc):
            assert trace.accumulator_before == before, (
                f"{trace.mnemonic}: expected A_before={before}, got {trace.accumulator_before}"
            )
            assert trace.accumulator_after == after, (
                f"{trace.mnemonic}: expected A_after={after}, got {trace.accumulator_after}"
            )
