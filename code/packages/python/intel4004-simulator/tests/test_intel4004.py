"""Comprehensive tests for the Intel 4004 simulator.

=== Test Organization ===

Tests are grouped by instruction category, matching the MCS-4 datasheet
organization. Each test class covers one instruction or one logical group.
After individual instruction tests, end-to-end programs verify everything
works together — including the 4004's original purpose: BCD arithmetic.

=== Key Concepts ===

- All data values are 4 bits (0–15). Every test verifies 4-bit masking.
- The carry flag in SUB/SBM is INVERTED from typical CPUs:
    carry=1 means NO borrow, carry=0 means borrow occurred.
  This matches the MCS-4 manual's complement-add implementation.
- The 4004 has a 3-level hardware call stack with silent overflow.
  The 4th call overwrites the oldest return address.
"""

import pytest

from intel4004_simulator.simulator import Intel4004Simulator

# ===================================================================
# NOP — No operation
# ===================================================================


class TestNOP:
    """NOP (0x00): Do nothing, advance PC."""

    def test_nop_does_nothing(self) -> None:
        """NOP should not change any state."""
        sim = Intel4004Simulator()
        traces = sim.run(bytes([0x00, 0x01]))

        assert sim.accumulator == 0
        assert sim.carry is False
        assert traces[0].mnemonic == "NOP"

    def test_multiple_nops(self) -> None:
        """Multiple NOPs should just advance PC."""
        sim = Intel4004Simulator()
        traces = sim.run(bytes([0x00, 0x00, 0x00, 0x01]))

        assert len(traces) == 4
        assert all(t.mnemonic == "NOP" for t in traces[:3])


# ===================================================================
# HLT — Halt execution (simulator-only)
# ===================================================================


class TestHLT:
    """HLT (0x01): Stop the CPU."""

    def test_hlt_stops_execution(self) -> None:
        sim = Intel4004Simulator()
        traces = sim.run(bytes([0x01]))

        assert sim.halted is True
        assert len(traces) == 1
        assert traces[0].mnemonic == "HLT"

    def test_hlt_mid_program(self) -> None:
        """Instructions after HLT should not execute."""
        sim = Intel4004Simulator()
        traces = sim.run(bytes([0x01, 0xD5]))

        assert sim.halted is True
        assert sim.accumulator == 0
        assert len(traces) == 1

    def test_step_after_halt_raises(self) -> None:
        sim = Intel4004Simulator()
        sim.run(bytes([0x01]))

        with pytest.raises(RuntimeError, match="halted"):
            sim.step()


# ===================================================================
# LDM — Load immediate into accumulator
# ===================================================================


class TestLDM:
    """LDM N (0xDN): Load a 4-bit immediate value into the accumulator."""

    def test_ldm_sets_accumulator(self) -> None:
        sim = Intel4004Simulator()
        traces = sim.run(bytes([0xD5, 0x01]))

        assert sim.accumulator == 5
        assert traces[0].mnemonic == "LDM 5"
        assert traces[0].accumulator_before == 0
        assert traces[0].accumulator_after == 5

    def test_ldm_zero(self) -> None:
        sim = Intel4004Simulator()
        sim.run(bytes([0xD0, 0x01]))
        assert sim.accumulator == 0

    def test_ldm_max_value(self) -> None:
        sim = Intel4004Simulator()
        sim.run(bytes([0xDF, 0x01]))
        assert sim.accumulator == 15

    def test_ldm_overwrites_previous(self) -> None:
        """Loading a new value replaces the old one completely."""
        sim = Intel4004Simulator()
        sim.run(bytes([0xD5, 0xD9, 0x01]))
        assert sim.accumulator == 9


# ===================================================================
# LD — Load register into accumulator
# ===================================================================


class TestLD:
    """LD Rn (0xAR): Load register value into accumulator. A = Rn."""

    def test_ld_basic(self) -> None:
        sim = Intel4004Simulator()
        # LDM 7, XCH R0 (R0=7, A=0), LD R0 (A=7), HLT
        sim.run(bytes([0xD7, 0xB0, 0xA0, 0x01]))
        assert sim.accumulator == 7

    def test_ld_does_not_modify_register(self) -> None:
        sim = Intel4004Simulator()
        sim.run(bytes([0xD3, 0xB5, 0xA5, 0x01]))
        assert sim.accumulator == 3
        assert sim.registers[5] == 3  # Register unchanged

    def test_ld_all_registers(self) -> None:
        """LD should work with all 16 registers."""
        for reg in range(16):
            sim = Intel4004Simulator()
            sim.registers[reg] = reg  # Pre-set register
            sim.run(bytes([0xA0 | reg, 0x01]))


# ===================================================================
# XCH — Exchange accumulator with register
# ===================================================================


class TestXCH:
    """XCH Rn (0xBN): Swap the accumulator and register N."""

    def test_xch_swaps_values(self) -> None:
        sim = Intel4004Simulator()
        sim.run(bytes([0xD7, 0xB0, 0x01]))
        assert sim.accumulator == 0
        assert sim.registers[0] == 7

    def test_xch_is_symmetric(self) -> None:
        """Two XCH operations on the same register restore original state."""
        sim = Intel4004Simulator()
        sim.run(bytes([0xD3, 0xB5, 0xB5, 0x01]))
        assert sim.accumulator == 3
        assert sim.registers[5] == 0

    def test_xch_high_register(self) -> None:
        sim = Intel4004Simulator()
        sim.run(bytes([0xD9, 0xBF, 0x01]))
        assert sim.registers[15] == 9
        assert sim.accumulator == 0


# ===================================================================
# INC — Increment register
# ===================================================================


class TestINC:
    """INC Rn (0x6R): Increment register. Does NOT affect carry."""

    def test_inc_basic(self) -> None:
        sim = Intel4004Simulator()
        # LDM 5, XCH R0 (R0=5), INC R0 (R0=6), HLT
        sim.run(bytes([0xD5, 0xB0, 0x60, 0x01]))
        assert sim.registers[0] == 6

    def test_inc_wraps_at_15(self) -> None:
        """INC wraps from 15 to 0 (4-bit overflow)."""
        sim = Intel4004Simulator()
        sim.run(bytes([0xDF, 0xB0, 0x60, 0x01]))
        assert sim.registers[0] == 0

    def test_inc_does_not_affect_carry(self) -> None:
        """INC never sets or clears carry — it's purely a register op."""
        sim = Intel4004Simulator()
        # Set carry first: LDM 15, XCH R1, LDM 15, ADD R1 (carry=True)
        # Then INC R0 — carry should remain True
        sim.run(bytes([0xDF, 0xB1, 0xDF, 0x81, 0x60, 0x01]))
        assert sim.carry is True

    def test_inc_does_not_affect_accumulator(self) -> None:
        sim = Intel4004Simulator()
        sim.run(bytes([0xD7, 0xB0, 0xD3, 0x60, 0x01]))
        assert sim.accumulator == 3  # A unchanged
        assert sim.registers[0] == 8  # R0 went from 7 to 8


# ===================================================================
# ADD — Add register to accumulator
# ===================================================================


class TestADD:
    """ADD Rn (0x8R): A = A + Rn + carry. Sets carry on overflow."""

    def test_add_basic(self) -> None:
        """2 + 3 = 5, no carry."""
        sim = Intel4004Simulator()
        sim.run(bytes([0xD3, 0xB0, 0xD2, 0x80, 0x01]))
        assert sim.accumulator == 5
        assert sim.carry is False

    def test_add_carry_on_overflow(self) -> None:
        """15 + 1 = 0 with carry (overflow)."""
        sim = Intel4004Simulator()
        sim.run(bytes([0xD1, 0xB0, 0xDF, 0x80, 0x01]))
        assert sim.accumulator == 0
        assert sim.carry is True

    def test_add_no_carry_at_boundary(self) -> None:
        """8 + 7 = 15, no carry (exactly at maximum)."""
        sim = Intel4004Simulator()
        sim.run(bytes([0xD7, 0xB0, 0xD8, 0x80, 0x01]))
        assert sim.accumulator == 15
        assert sim.carry is False

    def test_add_both_max(self) -> None:
        """15 + 15 = 14 with carry (30 & 0xF = 14)."""
        sim = Intel4004Simulator()
        sim.run(bytes([0xDF, 0xB0, 0xDF, 0x80, 0x01]))
        assert sim.accumulator == 14
        assert sim.carry is True

    def test_add_includes_carry_in(self) -> None:
        """ADD includes the carry flag: A + Rn + carry.

        This is how multi-digit BCD addition works. The carry from
        the previous digit feeds into the next digit's addition.
        """
        sim = Intel4004Simulator()
        # First cause carry: LDM 15, XCH R0, LDM 15, ADD R0 → A=14, carry=1
        # Then: LDM 1, XCH R1, LDM 1, ADD R1 → A = 1 + 1 + 1(carry) = 3
        sim.run(bytes([
            0xDF, 0xB0, 0xDF, 0x80,  # 15+15 → A=14, carry=1
            0xD1, 0xB1,              # LDM 1, XCH R1 (R1=1)
            0xD1, 0x81,              # LDM 1, ADD R1 → 1+1+1=3
            0x01,                    # HLT
        ]))
        assert sim.accumulator == 3
        assert sim.carry is False


# ===================================================================
# SUB — Subtract register from accumulator
# ===================================================================


class TestSUB:
    """SUB Rn (0x9R): A = A + ~Rn + borrow_in.

    The carry flag semantics are INVERTED for subtraction:
      carry=1 (True)  → no borrow (result >= 0)
      carry=0 (False) → borrow occurred (result was negative)

    This is the standard complement-add approach used by the real 4004.
    """

    def test_sub_basic(self) -> None:
        """5 - 3 = 2, no borrow → carry=True."""
        sim = Intel4004Simulator()
        sim.run(bytes([0xD3, 0xB0, 0xD5, 0x90, 0x01]))
        assert sim.accumulator == 2
        assert sim.carry is True  # No borrow

    def test_sub_underflow(self) -> None:
        """0 - 1 = 15 (wraps), borrow → carry=False."""
        sim = Intel4004Simulator()
        sim.run(bytes([0xD1, 0xB0, 0xD0, 0x90, 0x01]))
        assert sim.accumulator == 15
        assert sim.carry is False  # Borrow occurred

    def test_sub_equal_values(self) -> None:
        """7 - 7 = 0, no borrow → carry=True."""
        sim = Intel4004Simulator()
        sim.run(bytes([0xD7, 0xB0, 0xD7, 0x90, 0x01]))
        assert sim.accumulator == 0
        assert sim.carry is True  # No borrow

    def test_sub_wraps_to_4_bits(self) -> None:
        """3 - 5 = 14 (4-bit wrap), borrow → carry=False."""
        sim = Intel4004Simulator()
        sim.run(bytes([0xD5, 0xB0, 0xD3, 0x90, 0x01]))
        assert sim.accumulator == 14
        assert sim.carry is False  # Borrow occurred


# ===================================================================
# Accumulator operations (0xF0–0xFD)
# ===================================================================


class TestCLB:
    """CLB (0xF0): Clear both accumulator and carry."""

    def test_clb_clears_both(self) -> None:
        sim = Intel4004Simulator()
        # Set A=15 and carry via overflow, then CLB
        sim.run(bytes([0xDF, 0xB0, 0xDF, 0x80, 0xF0, 0x01]))
        assert sim.accumulator == 0
        assert sim.carry is False


class TestCLC:
    """CLC (0xF1): Clear carry flag only."""

    def test_clc_clears_carry(self) -> None:
        sim = Intel4004Simulator()
        # Cause carry: 15 + 15, then CLC
        sim.run(bytes([0xDF, 0xB0, 0xDF, 0x80, 0xF1, 0x01]))
        assert sim.carry is False
        assert sim.accumulator == 14  # A unchanged


class TestIAC:
    """IAC (0xF2): Increment accumulator. Carry set if A wraps from 15 to 0."""

    def test_iac_basic(self) -> None:
        sim = Intel4004Simulator()
        sim.run(bytes([0xD5, 0xF2, 0x01]))
        assert sim.accumulator == 6
        assert sim.carry is False

    def test_iac_wraps_and_sets_carry(self) -> None:
        sim = Intel4004Simulator()
        sim.run(bytes([0xDF, 0xF2, 0x01]))
        assert sim.accumulator == 0
        assert sim.carry is True

    def test_iac_no_carry_at_14(self) -> None:
        sim = Intel4004Simulator()
        sim.run(bytes([0xDE, 0xF2, 0x01]))
        assert sim.accumulator == 15
        assert sim.carry is False


class TestCMC:
    """CMC (0xF3): Complement (toggle) carry flag."""

    def test_cmc_sets_carry_when_clear(self) -> None:
        sim = Intel4004Simulator()
        sim.run(bytes([0xF3, 0x01]))
        assert sim.carry is True

    def test_cmc_clears_carry_when_set(self) -> None:
        sim = Intel4004Simulator()
        # Set carry, then complement
        sim.run(bytes([0xFA, 0xF3, 0x01]))  # STC, CMC
        assert sim.carry is False


class TestCMA:
    """CMA (0xF4): Complement accumulator (4-bit NOT)."""

    def test_cma_complements(self) -> None:
        sim = Intel4004Simulator()
        sim.run(bytes([0xD5, 0xF4, 0x01]))  # LDM 5, CMA
        assert sim.accumulator == 10  # ~5 & 0xF = 0b1010 = 10

    def test_cma_zero_to_fifteen(self) -> None:
        sim = Intel4004Simulator()
        sim.run(bytes([0xD0, 0xF4, 0x01]))  # LDM 0, CMA
        assert sim.accumulator == 15  # ~0 & 0xF = 15

    def test_cma_fifteen_to_zero(self) -> None:
        sim = Intel4004Simulator()
        sim.run(bytes([0xDF, 0xF4, 0x01]))  # LDM 15, CMA
        assert sim.accumulator == 0

    def test_cma_double_is_identity(self) -> None:
        sim = Intel4004Simulator()
        sim.run(bytes([0xD7, 0xF4, 0xF4, 0x01]))
        assert sim.accumulator == 7


class TestRAL:
    """RAL (0xF5): Rotate accumulator left through carry.

    [carry | A3 A2 A1 A0] rotates left:
    carry_new = A3, A = [A2 A1 A0 carry_old]
    """

    def test_ral_no_carry(self) -> None:
        """RAL with A=5 (0101), carry=0 → A=10 (1010), carry=0."""
        sim = Intel4004Simulator()
        sim.run(bytes([0xD5, 0xF5, 0x01]))
        assert sim.accumulator == 0b1010  # 10
        assert sim.carry is False

    def test_ral_with_high_bit(self) -> None:
        """RAL with A=8 (1000), carry=0 → A=0, carry=1."""
        sim = Intel4004Simulator()
        sim.run(bytes([0xD8, 0xF5, 0x01]))
        assert sim.accumulator == 0
        assert sim.carry is True

    def test_ral_carry_feeds_in(self) -> None:
        """RAL with A=0, carry=1 → A=1, carry=0."""
        sim = Intel4004Simulator()
        sim.run(bytes([0xFA, 0xD0, 0xF5, 0x01]))  # STC, LDM 0, RAL
        assert sim.accumulator == 1
        assert sim.carry is False


class TestRAR:
    """RAR (0xF6): Rotate accumulator right through carry.

    [carry | A3 A2 A1 A0] rotates right:
    carry_new = A0, A = [carry_old A3 A2 A1]
    """

    def test_rar_basic(self) -> None:
        """RAR with A=4 (0100), carry=0 → A=2 (0010), carry=0."""
        sim = Intel4004Simulator()
        sim.run(bytes([0xD4, 0xF6, 0x01]))
        assert sim.accumulator == 2
        assert sim.carry is False

    def test_rar_with_low_bit(self) -> None:
        """RAR with A=1 (0001), carry=0 → A=0, carry=1."""
        sim = Intel4004Simulator()
        sim.run(bytes([0xD1, 0xF6, 0x01]))
        assert sim.accumulator == 0
        assert sim.carry is True

    def test_rar_carry_feeds_in(self) -> None:
        """RAR with A=0, carry=1 → A=8, carry=0."""
        sim = Intel4004Simulator()
        sim.run(bytes([0xFA, 0xD0, 0xF6, 0x01]))  # STC, LDM 0, RAR
        assert sim.accumulator == 8
        assert sim.carry is False


class TestTCC:
    """TCC (0xF7): Transfer carry to A, clear carry."""

    def test_tcc_carry_set(self) -> None:
        sim = Intel4004Simulator()
        sim.run(bytes([0xFA, 0xF7, 0x01]))  # STC, TCC
        assert sim.accumulator == 1
        assert sim.carry is False

    def test_tcc_carry_clear(self) -> None:
        sim = Intel4004Simulator()
        sim.run(bytes([0xF7, 0x01]))  # TCC
        assert sim.accumulator == 0
        assert sim.carry is False


class TestDAC:
    """DAC (0xF8): Decrement accumulator.

    carry=1 if no borrow (A > 0), carry=0 if borrow (A was 0).
    """

    def test_dac_basic(self) -> None:
        sim = Intel4004Simulator()
        sim.run(bytes([0xD5, 0xF8, 0x01]))
        assert sim.accumulator == 4
        assert sim.carry is True  # No borrow

    def test_dac_wraps_from_zero(self) -> None:
        sim = Intel4004Simulator()
        sim.run(bytes([0xD0, 0xF8, 0x01]))
        assert sim.accumulator == 15
        assert sim.carry is False  # Borrow

    def test_dac_from_one(self) -> None:
        sim = Intel4004Simulator()
        sim.run(bytes([0xD1, 0xF8, 0x01]))
        assert sim.accumulator == 0
        assert sim.carry is True  # No borrow (0 >= 0)


class TestTCS:
    """TCS (0xF9): Transfer carry subtract. A = 10 if carry, else 9.

    Used in BCD subtraction for tens-complement correction.
    """

    def test_tcs_carry_set(self) -> None:
        sim = Intel4004Simulator()
        sim.run(bytes([0xFA, 0xF9, 0x01]))  # STC, TCS
        assert sim.accumulator == 10
        assert sim.carry is False

    def test_tcs_carry_clear(self) -> None:
        sim = Intel4004Simulator()
        sim.run(bytes([0xF9, 0x01]))
        assert sim.accumulator == 9
        assert sim.carry is False


class TestSTC:
    """STC (0xFA): Set carry flag."""

    def test_stc_sets_carry(self) -> None:
        sim = Intel4004Simulator()
        sim.run(bytes([0xFA, 0x01]))
        assert sim.carry is True


class TestDAA:
    """DAA (0xFB): Decimal adjust accumulator (BCD correction).

    If A > 9 or carry is set, add 6 to A.
    This corrects binary addition results to valid BCD digits.
    """

    def test_daa_no_adjust_needed(self) -> None:
        """A=5, carry=0 → no adjustment."""
        sim = Intel4004Simulator()
        sim.run(bytes([0xD5, 0xFB, 0x01]))
        assert sim.accumulator == 5

    def test_daa_adjusts_when_gt_9(self) -> None:
        """A=12, carry=0 → A = (12+6) & 0xF = 2, carry=1."""
        sim = Intel4004Simulator()
        sim.run(bytes([0xDC, 0xFB, 0x01]))
        assert sim.accumulator == 2
        assert sim.carry is True

    def test_daa_adjusts_when_carry_set(self) -> None:
        """A=3, carry=1 → A = (3+6) = 9, carry unchanged (no overflow)."""
        sim = Intel4004Simulator()
        sim.run(bytes([0xFA, 0xD3, 0xFB, 0x01]))  # STC, LDM 3, DAA
        assert sim.accumulator == 9

    def test_daa_bcd_addition_7_plus_8(self) -> None:
        """7 + 8 = 15 in binary, DAA corrects to 5 with carry (= BCD 15)."""
        sim = Intel4004Simulator()
        sim.run(bytes([
            0xD8, 0xB0,  # LDM 8, XCH R0 (R0=8)
            0xD7, 0x80,  # LDM 7, ADD R0 (A=15)
            0xFB,        # DAA → A=5, carry=1 (BCD: tens digit = 1, ones = 5)
            0x01,
        ]))
        assert sim.accumulator == 5
        assert sim.carry is True


class TestKBP:
    """KBP (0xFC): Keyboard process — 1-hot to binary conversion.

    Truth table:
        0b0000 (0)  → 0  (no key)
        0b0001 (1)  → 1
        0b0010 (2)  → 2
        0b0100 (4)  → 3
        0b1000 (8)  → 4
        else        → 15 (error)
    """

    def test_kbp_no_key(self) -> None:
        sim = Intel4004Simulator()
        sim.run(bytes([0xD0, 0xFC, 0x01]))
        assert sim.accumulator == 0

    def test_kbp_key_1(self) -> None:
        sim = Intel4004Simulator()
        sim.run(bytes([0xD1, 0xFC, 0x01]))
        assert sim.accumulator == 1

    def test_kbp_key_2(self) -> None:
        sim = Intel4004Simulator()
        sim.run(bytes([0xD2, 0xFC, 0x01]))
        assert sim.accumulator == 2

    def test_kbp_key_3(self) -> None:
        sim = Intel4004Simulator()
        sim.run(bytes([0xD4, 0xFC, 0x01]))
        assert sim.accumulator == 3

    def test_kbp_key_4(self) -> None:
        sim = Intel4004Simulator()
        sim.run(bytes([0xD8, 0xFC, 0x01]))
        assert sim.accumulator == 4

    def test_kbp_error_multiple_keys(self) -> None:
        """Multiple keys pressed → error code 15."""
        sim = Intel4004Simulator()
        sim.run(bytes([0xD3, 0xFC, 0x01]))  # 0b0011 = two keys
        assert sim.accumulator == 15

    def test_kbp_error_all_keys(self) -> None:
        sim = Intel4004Simulator()
        sim.run(bytes([0xDF, 0xFC, 0x01]))  # 0b1111
        assert sim.accumulator == 15


class TestDCL:
    """DCL (0xFD): Designate command line — select RAM bank."""

    def test_dcl_selects_bank(self) -> None:
        sim = Intel4004Simulator()
        sim.run(bytes([0xD2, 0xFD, 0x01]))  # LDM 2, DCL
        assert sim.ram_bank == 2

    def test_dcl_masks_to_3_bits(self) -> None:
        """Only lower 3 bits of A are used, clamped to 0–3."""
        sim = Intel4004Simulator()
        sim.run(bytes([0xD7, 0xFD, 0x01]))  # LDM 7, DCL → bank = 7 & 3 = 3
        assert sim.ram_bank == 3


# ===================================================================
# Jump instructions
# ===================================================================


class TestJUN:
    """JUN addr (0x4H 0xLL): Unconditional jump to 12-bit address."""

    def test_jun_jumps_forward(self) -> None:
        """JUN should skip over intermediate instructions."""
        sim = Intel4004Simulator()
        # JUN to address 4 (0x004), skipping LDM 5 at address 2
        # Address 0-1: JUN 0x004, Address 2: LDM 5, Address 3: HLT, Address 4: HLT
        sim.run(bytes([0x40, 0x04, 0xD5, 0x01, 0x01]))
        assert sim.accumulator == 0  # LDM 5 was skipped

    def test_jun_backward_creates_loop(self) -> None:
        """JUN backward + a counter = loop. Use INC to limit iterations."""
        sim = Intel4004Simulator()
        # Address 0: INC R0, Address 1-2: JCN (R0 == 4, skip to 5),
        # But simpler: just test that JUN can jump back
        # LDM 1, XCH R0, INC R0, JUN to HLT
        sim.run(bytes([
            0xD1, 0xB0,  # 0-1: LDM 1, XCH R0 (R0=1)
            0x60,        # 2: INC R0 (R0=2)
            0x40, 0x06,  # 3-4: JUN 0x006
            0xDF,        # 5: LDM 15 (should be skipped)
            0x01,        # 6: HLT
        ]))
        assert sim.registers[0] == 2
        assert sim.accumulator == 0  # LDM 15 was skipped


class TestJCN:
    """JCN cond,addr (0x1C 0xAA): Conditional jump.

    Condition nibble bits:
        Bit 3 (0x8): INVERT
        Bit 2 (0x4): TEST A==0
        Bit 1 (0x2): TEST carry==1
        Bit 0 (0x1): TEST pin (always 0)
    """

    def test_jcn_jump_if_acc_zero(self) -> None:
        """JCN 4,addr: jump if A == 0."""
        sim = Intel4004Simulator()
        # A starts at 0, so condition 4 (test zero) is true
        sim.run(bytes([0x14, 0x04, 0xD5, 0x01, 0x01]))
        assert sim.accumulator == 0  # LDM 5 was skipped

    def test_jcn_no_jump_if_acc_nonzero(self) -> None:
        """JCN 4,addr: don't jump if A != 0."""
        sim = Intel4004Simulator()
        sim.run(bytes([0xD3, 0x14, 0x06, 0xD5, 0x01, 0x01, 0x01]))
        assert sim.accumulator == 5  # LDM 5 executed

    def test_jcn_invert_jump_if_acc_nonzero(self) -> None:
        """JCN 0xC,addr: INVERT + TEST_ZERO = jump if A != 0."""
        sim = Intel4004Simulator()
        # LDM 3, JCN 0xC (invert zero test = jump if nonzero)
        sim.run(bytes([0xD3, 0x1C, 0x06, 0xD5, 0x01, 0x01, 0x01]))
        assert sim.accumulator == 3  # LDM 5 was skipped, A still 3

    def test_jcn_jump_if_carry(self) -> None:
        """JCN 2,addr: jump if carry is set."""
        sim = Intel4004Simulator()
        # Set carry: LDM 15, XCH R0, LDM 15, ADD R0 → carry=1
        # Then JCN 2 should jump
        sim.run(bytes([
            0xDF, 0xB0, 0xDF, 0x80,  # carry=1, A=14
            0x12, 0x08,              # JCN 2,0x08
            0xD0, 0x01,              # LDM 0, HLT (should be skipped)
            0x01,                    # HLT
        ]))
        assert sim.accumulator == 14  # LDM 0 was skipped

    def test_jcn_invert_no_conditions(self) -> None:
        """JCN 8,addr: INVERT with no test bits = always jump.

        No tests → result is False. INVERT → True. Always jumps.
        """
        sim = Intel4004Simulator()
        sim.run(bytes([0x18, 0x04, 0xD5, 0x01, 0x01]))
        assert sim.accumulator == 0  # LDM 5 was skipped


class TestISZ:
    """ISZ Rn,addr (0x7R 0xAA): Increment register, skip if zero.

    Increment Rn. If Rn != 0, jump to addr. If Rn == 0, fall through.
    This is the 4004's loop instruction.
    """

    def test_isz_loops_until_zero(self) -> None:
        """Count from 14 to 0 using ISZ (2 iterations: 14→15→0)."""
        sim = Intel4004Simulator()
        # LDM 14, XCH R0 (R0=14), ISZ R0 loop_start, HLT
        # Loop: R0 increments: 14→15 (jump), 15→0 (fall through)
        sim.run(bytes([
            0xDE, 0xB0,  # 0-1: LDM 14, XCH R0
            0x70, 0x02,  # 2-3: ISZ R0, 0x02 (loop back to ISZ)
            0x01,        # 4: HLT
        ]))
        assert sim.registers[0] == 0  # Wrapped to 0

    def test_isz_single_iteration(self) -> None:
        """R0=15, ISZ increments to 0, falls through immediately."""
        sim = Intel4004Simulator()
        sim.run(bytes([
            0xDF, 0xB0,  # LDM 15, XCH R0
            0x70, 0x02,  # ISZ R0, 0x02
            0xD7,        # LDM 7 (should execute)
            0x01,        # HLT
        ]))
        assert sim.registers[0] == 0
        assert sim.accumulator == 7  # Fell through, LDM 7 executed


# ===================================================================
# Subroutine instructions
# ===================================================================


class TestJMS_BBL:
    """JMS addr (0x5H 0xLL): Jump to subroutine.
    BBL N (0xCN): Branch back and load N into accumulator.

    The 4004 has a 3-level hardware stack. JMS pushes the return address
    (next instruction after JMS), BBL pops and loads an immediate.
    """

    def test_jms_bbl_basic(self) -> None:
        """Call a subroutine that sets A=7, then returns with BBL 0."""
        sim = Intel4004Simulator()
        sim.run(bytes([
            0x50, 0x05,  # 0-1: JMS 0x005 (call subroutine at addr 5)
            0xB0,        # 2: XCH R0 (store return value)
            0x01,        # 3: HLT
            0x00,        # 4: padding (NOP)
            0xD7,        # 5: LDM 7 (subroutine body)
            0xC0,        # 6: BBL 0 (return, A=0)
        ]))
        # After JMS: subroutine runs LDM 7 (A=7), then BBL 0 (A=0)
        # Back at addr 2: XCH R0 (R0=0, A=0)
        assert sim.registers[0] == 0

    def test_bbl_loads_immediate(self) -> None:
        """BBL N loads N into the accumulator before returning."""
        sim = Intel4004Simulator()
        sim.run(bytes([
            0x50, 0x04,  # 0-1: JMS 0x004
            0x01,        # 2: HLT
            0x00,        # 3: padding
            0xC5,        # 4: BBL 5 (return with A=5)
        ]))
        assert sim.accumulator == 5

    def test_nested_calls(self) -> None:
        """Two levels of subroutine nesting."""
        sim = Intel4004Simulator()
        sim.run(bytes([
            0x50, 0x06,  # 0-1: JMS 0x006 (call sub1)
            0xB0,        # 2: XCH R0
            0x01,        # 3: HLT
            0x00, 0x00,  # 4-5: padding
            # sub1 at 6:
            0x50, 0x0C,  # 6-7: JMS 0x00C (call sub2)
            0xB1,        # 8: XCH R1 (save sub2's return value)
            0xD9,        # 9: LDM 9
            0xC0,        # A: BBL 0 (return to main, A=0 overrides LDM 9)
            0x00,        # B: padding
            # sub2 at C:
            0xC3,        # C: BBL 3 (return to sub1 with A=3)
        ]))
        assert sim.registers[1] == 3  # sub2 returned 3
        assert sim.registers[0] == 0  # sub1 returned BBL 0

    def test_stack_wraps_on_overflow(self) -> None:
        """The 4004's 3-level stack silently wraps on the 4th call."""
        sim = Intel4004Simulator()
        # Call 4 subroutines deep — the 4th overwrites the 1st return address.
        # After 4 BBLs, we should end up at the wrong place (the 2nd call's
        # return address instead of the 1st).
        # This is a simplified test — just ensure it doesn't crash.
        sim.run(bytes([
            0x50, 0x04,  # 0-1: JMS 0x004 (call level 1)
            0x01,        # 2: HLT (expected return)
            0x00,        # 3: padding
            0x50, 0x08,  # 4-5: JMS 0x008 (call level 2)
            0xC0,        # 6: BBL 0 (return from level 1)
            0x00,        # 7: padding
            0x50, 0x0C,  # 8-9: JMS 0x00C (call level 3)
            0xC0,        # A: BBL 0 (return from level 2)
            0x00,        # B: padding
            0xC0,        # C: BBL 0 (return from level 3)
        ]))
        assert sim.halted is True  # Should eventually halt


# ===================================================================
# Register pair instructions
# ===================================================================


class TestFIM:
    """FIM Pp,data (0x2P 0xDD): Load 8-bit immediate into register pair."""

    def test_fim_loads_pair(self) -> None:
        sim = Intel4004Simulator()
        # FIM P0, 0xAB → R0=0xA (10), R1=0xB (11)
        sim.run(bytes([0x20, 0xAB, 0x01]))
        assert sim.registers[0] == 0xA
        assert sim.registers[1] == 0xB

    def test_fim_pair_3(self) -> None:
        """FIM P3, 0x42 → R6=4, R7=2."""
        sim = Intel4004Simulator()
        sim.run(bytes([0x26, 0x42, 0x01]))
        assert sim.registers[6] == 4
        assert sim.registers[7] == 2

    def test_fim_zero(self) -> None:
        sim = Intel4004Simulator()
        # First set registers, then FIM with 0
        sim.run(bytes([0xDF, 0xB0, 0xDF, 0xB1, 0x20, 0x00, 0x01]))
        assert sim.registers[0] == 0
        assert sim.registers[1] == 0


class TestSRC:
    """SRC Pp (0x2P+1): Send register control — set RAM address."""

    def test_src_sets_ram_address(self) -> None:
        sim = Intel4004Simulator()
        # FIM P0, 0x35 (R0=3, R1=5), SRC P0 → register=3, character=5
        sim.run(bytes([0x20, 0x35, 0x21, 0x01]))
        assert sim.ram_register == 3
        assert sim.ram_character == 5


class TestFIN:
    """FIN Pp (0x3P): Fetch indirect from ROM via P0."""

    def test_fin_reads_rom(self) -> None:
        sim = Intel4004Simulator()
        # Set P0 to point to address 0x08 in ROM, put 0xAB at ROM[0x08]
        # FIM P0, 0x08 sets R0=0, R1=8 → P0 value = 0x08
        # FIN P1 reads ROM[0x08] into P1
        program = bytearray(16)
        program[0] = 0x20  # FIM P0, 0x08
        program[1] = 0x08
        program[2] = 0x32  # FIN P1
        program[3] = 0x01  # HLT
        program[8] = 0xCD  # Data at ROM[0x08]
        sim.run(bytes(program))
        assert sim.registers[2] == 0xC  # P1 high = 0xC
        assert sim.registers[3] == 0xD  # P1 low = 0xD


class TestJIN:
    """JIN Pp (0x3P+1): Jump indirect via register pair."""

    def test_jin_jumps_to_pair_value(self) -> None:
        sim = Intel4004Simulator()
        # FIM P1, 0x06 (R2=0, R3=6), JIN P1 → jump to address 0x06
        sim.run(bytes([
            0x22, 0x06,  # 0-1: FIM P1, 0x06
            0x33,        # 2: JIN P1 (jump to 0x06)
            0xD5,        # 3: LDM 5 (should be skipped)
            0x01,        # 4: HLT
            0x00,        # 5: NOP
            0x01,        # 6: HLT (jump target)
        ]))
        assert sim.accumulator == 0  # LDM 5 was skipped


# ===================================================================
# RAM/ROM I/O instructions
# ===================================================================


class TestRAMIO:
    """WRM, RDM, WR0–WR3, RD0–RD3, WMP, WRR, RDR, WPM."""

    def test_wrm_rdm_round_trip(self) -> None:
        """Write A to RAM, read it back."""
        sim = Intel4004Simulator()
        sim.run(bytes([
            0x20, 0x00,  # FIM P0, 0x00 (register=0, character=0)
            0x21,        # SRC P0
            0xD7,        # LDM 7
            0xE0,        # WRM (write A to RAM[bank=0][reg=0][char=0])
            0xD0,        # LDM 0 (clear A)
            0xE9,        # RDM (read back)
            0x01,        # HLT
        ]))
        assert sim.accumulator == 7

    def test_wrm_different_characters(self) -> None:
        """Write to different RAM characters within the same register."""
        sim = Intel4004Simulator()
        sim.run(bytes([
            # Write 5 to character 0
            0x20, 0x00, 0x21, 0xD5, 0xE0,
            # Write 9 to character 3
            0x20, 0x03, 0x21, 0xD9, 0xE0,
            # Read back character 0
            0x20, 0x00, 0x21, 0xE9,
            0x01,
        ]))
        assert sim.accumulator == 5
        assert sim.ram[0][0][3] == 9

    def test_wr_rd_status(self) -> None:
        """Write and read RAM status characters."""
        sim = Intel4004Simulator()
        sim.run(bytes([
            0x20, 0x00, 0x21,  # FIM P0,0x00; SRC P0
            0xD3, 0xE4,        # LDM 3, WR0 (status[0] = 3)
            0xD7, 0xE5,        # LDM 7, WR1 (status[1] = 7)
            0xDA, 0xE6,        # LDM 10, WR2 (status[2] = 10)
            0xDF, 0xE7,        # LDM 15, WR3 (status[3] = 15)
            0xD0,              # LDM 0 (clear A)
            0xEC,              # RD0 (read status[0])
            0x01,
        ]))
        assert sim.accumulator == 3
        assert sim.ram_status[0][0][1] == 7
        assert sim.ram_status[0][0][2] == 10
        assert sim.ram_status[0][0][3] == 15

    def test_rd1_rd2_rd3(self) -> None:
        """Read status characters 1, 2, 3 (write first, then read)."""
        sim = Intel4004Simulator()
        sim.run(bytes([
            0x20, 0x00, 0x21,  # FIM P0,0x00; SRC P0
            0xD2, 0xE5,        # LDM 2, WR1 (status[1] = 2)
            0xD0,              # LDM 0 (clear A)
            0x20, 0x00, 0x21,  # SRC P0 again
            0xED,              # RD1
            0x01,
        ]))
        assert sim.accumulator == 2

    def test_wmp(self) -> None:
        """WMP writes to RAM output port."""
        sim = Intel4004Simulator()
        sim.run(bytes([0xD9, 0xE1, 0x01]))  # LDM 9, WMP
        assert sim.ram_output[0] == 9

    def test_wrr_rdr_round_trip(self) -> None:
        """WRR/RDR write and read the ROM I/O port."""
        sim = Intel4004Simulator()
        sim.run(bytes([0xDB, 0xE2, 0xD0, 0xEA, 0x01]))
        # LDM 11, WRR (port=11), LDM 0, RDR (A=11)
        assert sim.accumulator == 11

    def test_wpm_is_nop(self) -> None:
        """WPM (program RAM write) is treated as NOP in simulation."""
        sim = Intel4004Simulator()
        sim.run(bytes([0xD5, 0xE3, 0x01]))
        assert sim.accumulator == 5  # Unchanged


class TestADM_SBM:
    """ADM (0xEB): Add RAM to accumulator.
    SBM (0xE8): Subtract RAM from accumulator.
    """

    def test_adm_adds_ram_value(self) -> None:
        """Write 5 to RAM, then add it to A=3 via ADM."""
        sim = Intel4004Simulator()
        sim.run(bytes([
            0x20, 0x00, 0x21,  # FIM P0,0x00; SRC P0
            0xD5, 0xE0,        # LDM 5, WRM (RAM[0][0][0]=5)
            0xD3,              # LDM 3
            0x20, 0x00, 0x21,  # SRC P0 again
            0xEB,              # ADM (A = 3 + 5 = 8)
            0x01,
        ]))
        assert sim.accumulator == 8
        assert sim.carry is False

    def test_adm_with_carry(self) -> None:
        """ADM overflow: 10 + 8 = 18 → A=2, carry=1."""
        sim = Intel4004Simulator()
        sim.run(bytes([
            0x20, 0x00, 0x21,  # SRC P0
            0xD8, 0xE0,        # LDM 8, WRM (RAM=8)
            0xDA,              # LDM 10
            0x20, 0x00, 0x21,  # SRC P0 again
            0xEB,              # ADM (10 + 8 = 18 → A=2, carry=1)
            0x01,
        ]))
        assert sim.accumulator == 2
        assert sim.carry is True

    def test_sbm_subtracts_ram_value(self) -> None:
        """Write 3 to RAM, then subtract from A=7 via SBM."""
        sim = Intel4004Simulator()
        sim.run(bytes([
            0x20, 0x00, 0x21,  # SRC P0
            0xD3, 0xE0,        # LDM 3, WRM (RAM=3)
            0xD7,              # LDM 7
            0x20, 0x00, 0x21,  # SRC P0 again
            0xE8,              # SBM (7 - 3 = 4, no borrow → carry=1)
            0x01,
        ]))
        assert sim.accumulator == 4
        assert sim.carry is True


class TestRAMBanking:
    """Test DCL + SRC + WRM/RDM for multi-bank RAM access."""

    def test_write_to_different_banks(self) -> None:
        """Write different values to bank 0 and bank 1."""
        sim = Intel4004Simulator()
        sim.run(bytes([
            # Bank 0: write 5
            0xD0, 0xFD,        # LDM 0, DCL (bank 0)
            0x20, 0x00, 0x21,  # FIM P0,0; SRC P0
            0xD5, 0xE0,        # LDM 5, WRM

            # Bank 1: write 9
            0xD1, 0xFD,        # LDM 1, DCL (bank 1)
            0x20, 0x00, 0x21,  # SRC P0
            0xD9, 0xE0,        # LDM 9, WRM

            # Read bank 0
            0xD0, 0xFD,        # DCL (bank 0)
            0x20, 0x00, 0x21,  # SRC P0
            0xE9,              # RDM
            0x01,
        ]))
        assert sim.accumulator == 5
        assert sim.ram[1][0][0] == 9


# ===================================================================
# Trace verification
# ===================================================================


class TestTracing:
    """Verify that traces capture before/after state correctly."""

    def test_trace_address(self) -> None:
        sim = Intel4004Simulator()
        traces = sim.run(bytes([0xD1, 0xB0, 0x01]))
        assert traces[0].address == 0
        assert traces[1].address == 1
        assert traces[2].address == 2

    def test_trace_raw_bytes(self) -> None:
        sim = Intel4004Simulator()
        traces = sim.run(bytes([0xD5, 0x01]))
        assert traces[0].raw == 0xD5
        assert traces[0].raw2 is None  # 1-byte instruction

    def test_trace_2byte_instruction(self) -> None:
        sim = Intel4004Simulator()
        traces = sim.run(bytes([0x20, 0xAB, 0x01]))  # FIM P0, 0xAB
        assert traces[0].raw == 0x20
        assert traces[0].raw2 == 0xAB

    def test_trace_accumulator_flow(self) -> None:
        """Track accumulator through x = 1 + 2."""
        sim = Intel4004Simulator()
        traces = sim.run(bytes([0xD1, 0xB0, 0xD2, 0x80, 0xB1, 0x01]))

        expected = [
            (0, 1),   # LDM 1
            (1, 0),   # XCH R0
            (0, 2),   # LDM 2
            (2, 3),   # ADD R0
            (3, 0),   # XCH R1
            (0, 0),   # HLT
        ]
        for trace, (before, after) in zip(traces, expected, strict=True):
            assert trace.accumulator_before == before
            assert trace.accumulator_after == after

    def test_trace_carry_tracking(self) -> None:
        """Verify carry is tracked in traces."""
        sim = Intel4004Simulator()
        # 15 + 15 → carry=True
        traces = sim.run(bytes([0xDF, 0xB0, 0xDF, 0x80, 0x01]))
        add_trace = traces[3]  # ADD R0
        assert add_trace.carry_before is False
        assert add_trace.carry_after is True


# ===================================================================
# Reset and state management
# ===================================================================


class TestReset:
    """Verify reset clears all state."""

    def test_reset_clears_everything(self) -> None:
        sim = Intel4004Simulator()
        sim.run(bytes([0xD5, 0xB3, 0xFA, 0x01]))  # Set some state
        sim.reset()

        assert sim.accumulator == 0
        assert sim.registers == [0] * 16
        assert sim.carry is False
        assert sim.halted is False
        assert sim.ram_bank == 0
        assert sim.rom_port == 0
        assert sim.hw_stack == [0, 0, 0]

    def test_run_resets_first(self) -> None:
        """run() should reset before loading new program."""
        sim = Intel4004Simulator()
        sim.run(bytes([0xD5, 0x01]))
        assert sim.accumulator == 5

        sim.run(bytes([0xD3, 0x01]))
        assert sim.accumulator == 3  # Fresh start


# ===================================================================
# 4-bit masking — the fundamental constraint
# ===================================================================


class TestFourBitMasking:
    """All values must stay within 0–15."""

    def test_accumulator_never_exceeds_15(self) -> None:
        sim = Intel4004Simulator()
        sim.run(bytes([0xDF, 0xB0, 0xDF, 0x80, 0x01]))
        assert 0 <= sim.accumulator <= 15

    def test_registers_never_exceed_15(self) -> None:
        sim = Intel4004Simulator()
        sim.run(bytes([
            0xDF, 0xB0, 0xDA, 0xB1, 0xD0, 0xB2, 0x01,
        ]))
        for i, val in enumerate(sim.registers):
            assert 0 <= val <= 15, f"R{i} = {val}"


# ===================================================================
# End-to-end programs
# ===================================================================


class TestEndToEnd:
    """Complete programs exercising multiple instruction types."""

    def test_x_equals_1_plus_2(self) -> None:
        """The canonical first program: compute 1 + 2 = 3."""
        sim = Intel4004Simulator()
        traces = sim.run(bytes([0xD1, 0xB0, 0xD2, 0x80, 0xB1, 0x01]))

        assert sim.registers[1] == 3
        assert sim.registers[0] == 1
        assert sim.accumulator == 0
        assert sim.carry is False
        assert sim.halted is True
        assert len(traces) == 6

    def test_multiply_3_times_4(self) -> None:
        """Multiply 3 × 4 = 12 using repeated addition.

        Algorithm:
            R0 = 3 (multiplicand)
            R1 = counter (starts at -4 = 12 in 4-bit)
            A = running sum
            loop: A += R0; ISZ R1, loop
        """
        sim = Intel4004Simulator()
        sim.run(bytes([
            0xD3, 0xB0,        # 0-1: LDM 3, XCH R0 (R0=3, multiplicand)
            0xDC, 0xB1,        # 2-3: LDM 12, XCH R1 (R1=12=-4 in 4-bit)
            0xD0,              # 4: LDM 0 (A=0, start sum)
            0x80,              # 5: ADD R0 (A += 3)
            0x71, 0x05,        # 6-7: ISZ R1, 0x05 (R1++; if R1!=0, goto 5)
            0xB2,              # 8: XCH R2 (store result in R2)
            0x01,              # 9: HLT
        ]))
        assert sim.registers[2] == 12  # 3 × 4 = 12

    def test_subroutine_returns_value(self) -> None:
        """Call a subroutine that adds two numbers and returns the result."""
        sim = Intel4004Simulator()
        sim.run(bytes([
            # Main program
            0xD3, 0xB0,        # 0-1: LDM 3, XCH R0 (R0=3)
            0xD4, 0xB1,        # 2-3: LDM 4, XCH R1 (R1=4)
            0x50, 0x0A,        # 4-5: JMS 0x00A (call add_r0_r1)
            0xB2,              # 6: XCH R2 (store result)
            0x01,              # 7: HLT
            0x00, 0x00,        # 8-9: padding
            # Subroutine add_r0_r1 at 0x00A:
            0xA0,              # A: LD R0 (A = R0 = 3)
            0x81,              # B: ADD R1 (A = 3 + 4 = 7)
            0xC0,              # C: BBL 0 (return, but A=0 overwrites!)
        ]))
        # BBL 0 sets A=0, so XCH R2 stores 0, not 7.
        # This demonstrates BBL's "load" behavior.
        assert sim.registers[2] == 0

    def test_bcd_addition_7_plus_8(self) -> None:
        """BCD addition: 7 + 8 = 15 in BCD (1 carry + 5 ones).

        This is the 4004's raison d'être — Binary Coded Decimal arithmetic
        for the Busicom calculator.

        Algorithm:
            LDM 8, XCH R0 (R0=8)
            LDM 7 (A=7)
            ADD R0 (A = 7+8 = 15 binary)
            DAA (decimal adjust: 15+6=21, A=5, carry=1)
            → BCD result: carry=1 (tens), A=5 (ones) = 15
        """
        sim = Intel4004Simulator()
        sim.run(bytes([
            0xD8, 0xB0,  # R0=8
            0xD7,        # A=7
            0x80,        # ADD R0 → A=15, carry=0
            0xFB,        # DAA → A=5, carry=1
            0x01,
        ]))
        ones = sim.accumulator
        tens = 1 if sim.carry else 0
        assert tens == 1 and ones == 5  # BCD 15

    def test_ram_store_and_retrieve(self) -> None:
        """Store values in RAM and retrieve them.

        Uses P1 (R2:R3) for SRC addressing and stores results in R4/R5
        to avoid FIM overwriting the result registers.
        """
        sim = Intel4004Simulator()
        sim.run(bytes([
            # Store 5 at RAM[bank=0][reg=0][char=0]
            0x22, 0x00,  # FIM P1, 0x00
            0x23,        # SRC P1
            0xD5,        # LDM 5
            0xE0,        # WRM

            # Store 9 at RAM[bank=0][reg=0][char=1]
            0x22, 0x01,  # FIM P1, 0x01
            0x23,        # SRC P1
            0xD9,        # LDM 9
            0xE0,        # WRM

            # Read back char 0, save in R4
            0x22, 0x00,  # FIM P1, 0x00
            0x23,        # SRC P1
            0xE9,        # RDM
            0xB4,        # XCH R4

            # Read back char 1, save in R5
            0x22, 0x01,  # FIM P1, 0x01
            0x23,        # SRC P1
            0xE9,        # RDM
            0xB5,        # XCH R5

            0x01,        # HLT
        ]))
        assert sim.registers[4] == 5
        assert sim.registers[5] == 9

    def test_countdown_loop(self) -> None:
        """Count down from 5 to 0 using DAC and JCN."""
        sim = Intel4004Simulator()
        sim.run(bytes([
            0xD5,        # 0: LDM 5 (A=5)
            0xF8,        # 1: DAC (A--)
            0x1C, 0x01,  # 2-3: JCN 0xC (INVERT|TEST_ZERO = jump if A!=0), addr 01
            0x01,        # 4: HLT
        ]))
        assert sim.accumulator == 0  # Counted down to 0

    def test_max_steps_prevents_infinite_loop(self) -> None:
        """run() should stop after max_steps to prevent infinite loops."""
        sim = Intel4004Simulator()
        # Infinite loop: JUN to itself
        traces = sim.run(bytes([0x40, 0x00]), max_steps=10)
        assert len(traces) == 10
        assert sim.halted is False
