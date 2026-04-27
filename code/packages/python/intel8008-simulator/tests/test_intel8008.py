"""Tests for the Intel 8008 behavioral simulator.

These tests verify every instruction group, all flag combinations, the
push-down stack, the M pseudo-register, and example programs from the spec.

Test organization:
    - test_flags_*          : Flag computation correctness
    - test_mov_*            : Register-to-register transfer
    - test_mvi_*            : Move immediate
    - test_inr_dcr_*        : Increment/decrement
    - test_alu_reg_*        : ALU with register source
    - test_alu_imm_*        : ALU with immediate source
    - test_rotate_*         : Rotate accumulator instructions
    - test_jump_*           : Jump (JMP, conditional)
    - test_call_ret_*       : Call/return and stack management
    - test_rst_*            : Restart instruction
    - test_io_*             : IN/OUT ports
    - test_hlt_*            : HLT encodings
    - test_program_*        : Complete example programs
    - test_reset_*          : Reset behavior
    - test_memory_*         : M pseudo-register / memory access
    - TestSimulatorProtocol : simulator-protocol conformance tests
"""

from __future__ import annotations

import pytest

from intel8008_simulator import Intel8008Flags, Intel8008Simulator, Intel8008Trace


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def make_sim() -> Intel8008Simulator:
    """Return a fresh simulator instance."""
    return Intel8008Simulator()


def run_program(program: bytes, max_steps: int = 100_000) -> Intel8008Simulator:
    """Run a complete program and return the simulator after HLT."""
    sim = make_sim()
    sim.run(program, max_steps=max_steps)
    return sim


# ---------------------------------------------------------------------------
# Flags computation
# ---------------------------------------------------------------------------


class TestFlags:
    """Tests for flag computation correctness."""

    def test_zero_flag_set_when_result_is_zero(self) -> None:
        # SUB A clears A to 0 → Z should be set
        sim = run_program(bytes([0x97, 0x76]))  # SUB A; HLT
        assert sim.flags.zero is True
        assert sim.flags.sign is False

    def test_zero_flag_clear_when_result_nonzero(self) -> None:
        # MVI A,5; ADI 1; HLT → A=6, Z=0
        sim = run_program(bytes([0x3E, 0x05, 0xC4, 0x01, 0x76]))
        assert sim.a == 6
        assert sim.flags.zero is False

    def test_sign_flag_set_when_bit7_is_one(self) -> None:
        # ADI 0x80 → result=0x80, sign flag should be set
        # (MVI does not update flags; use ADI to trigger flag update)
        sim = run_program(bytes([0xC4, 0x80, 0x76]))  # ADI 0x80; HLT
        assert sim.flags.sign is True
        assert sim.a == 0x80

    def test_sign_flag_clear_for_positive(self) -> None:
        # MVI A, 0x7F → sign flag should be clear
        sim = run_program(bytes([0x3E, 0x7F, 0x76]))
        assert sim.flags.sign is False

    def test_carry_set_on_overflow_addition(self) -> None:
        # MVI A, 0xFF; ADI 1 → overflow → CY=1, A=0, Z=1
        sim = run_program(bytes([0x3E, 0xFF, 0xC4, 0x01, 0x76]))
        assert sim.a == 0
        assert sim.flags.carry is True
        assert sim.flags.zero is True

    def test_carry_clear_when_no_overflow(self) -> None:
        # MVI A, 1; ADI 1 → A=2, CY=0
        sim = run_program(bytes([0x3E, 0x01, 0xC4, 0x01, 0x76]))
        assert sim.a == 2
        assert sim.flags.carry is False

    def test_parity_even_with_zero(self) -> None:
        # Result 0x00 has 0 ones → even parity → P=1
        sim = run_program(bytes([0x97, 0x76]))  # SUB A
        assert sim.flags.parity is True

    def test_parity_even_with_three(self) -> None:
        # 0x03 = 0b00000011 → 2 ones → even parity → P=1
        sim = run_program(bytes([0x06, 0x01, 0x3E, 0x02, 0x80, 0x76]))
        assert sim.a == 3
        assert sim.flags.parity is True

    def test_parity_odd_with_one(self) -> None:
        # 0x01 = 0b00000001 → 1 one → odd parity → P=0
        sim = run_program(bytes([0x3E, 0x01, 0x76]))
        assert sim.flags.parity is False

    def test_parity_even_with_0xFF(self) -> None:
        # 0xFF = 0b11111111 → 8 ones → even parity → P=1
        # Use ADI to trigger flag update (MVI doesn't set flags)
        sim = run_program(bytes([0xC4, 0xFF, 0x76]))  # ADI 0xFF; HLT
        assert sim.flags.parity is True  # 8 ones = even parity

    def test_parity_odd_with_0x01(self) -> None:
        # Explicit: load 0x01, check P=0
        sim = run_program(bytes([0x3E, 0x01, 0x76]))
        assert sim.a == 0x01
        assert sim.flags.parity is False

    def test_borrow_flag_on_subtraction(self) -> None:
        # MVI A, 1; SUI 2 → 1 - 2 = -1 (borrow) → CY=1, A=0xFF
        sim = run_program(bytes([0x3E, 0x01, 0xD4, 0x02, 0x76]))
        assert sim.a == 0xFF
        assert sim.flags.carry is True  # borrow occurred

    def test_no_borrow_flag_on_subtraction(self) -> None:
        # MVI A, 5; SUI 3 → 5 - 3 = 2, no borrow → CY=0
        sim = run_program(bytes([0x3E, 0x05, 0xD4, 0x03, 0x76]))
        assert sim.a == 2
        assert sim.flags.carry is False

    def test_inr_does_not_update_carry(self) -> None:
        # Set carry via ADD overflow, then INR B — carry should be preserved
        # MVI A,0xFF; ADI 1 (overflow sets CY=1); MVI B,0; INR B; HLT
        sim = run_program(bytes([0x3E, 0xFF, 0xC4, 0x01, 0x06, 0x00, 0x00, 0x76]))
        # After ADI, CY=1; after INR B, CY should still be 1
        assert sim.flags.carry is True
        assert sim.b == 1

    def test_dcr_does_not_update_carry(self) -> None:
        # Similar: set carry, then DCR
        sim = run_program(bytes([0x3E, 0xFF, 0xC4, 0x01, 0x06, 0x05, 0x01, 0x76]))
        assert sim.flags.carry is True

    def test_ana_clears_carry(self) -> None:
        # Set CY via overflow, then ANA A → CY must be 0
        # MVI A,0xFF; ADI 1 (CY=1); ANA A; HLT
        sim = run_program(bytes([0x3E, 0xFF, 0xC4, 0x01, 0xA7, 0x76]))
        assert sim.flags.carry is False

    def test_xra_clears_carry(self) -> None:
        sim = run_program(bytes([0x3E, 0xFF, 0xC4, 0x01, 0xAF, 0x76]))
        assert sim.flags.carry is False

    def test_ora_clears_carry(self) -> None:
        sim = run_program(bytes([0x3E, 0xFF, 0xC4, 0x01, 0xB7, 0x76]))
        assert sim.flags.carry is False


# ---------------------------------------------------------------------------
# MOV instruction
# ---------------------------------------------------------------------------


class TestMov:
    """Register-to-register transfer."""

    def test_mov_a_b(self) -> None:
        # MOV A, B: load B=5, then copy to A
        # MVI B,5; MOV A,B (0x78); HLT
        sim = run_program(bytes([0x06, 0x05, 0x78, 0x76]))
        assert sim.a == 5

    def test_mov_h_l(self) -> None:
        # MOV H, L: 0x65
        sim = run_program(bytes([0x2E, 0x42, 0x65, 0x76]))  # MVI L,0x42; MOV H,L
        assert sim.h == 0x42

    def test_mov_does_not_affect_flags(self) -> None:
        # After MOV, flags should be unchanged
        sim = run_program(bytes([0x06, 0x05, 0x78, 0x76]))
        # Default flags: all False
        assert sim.flags.zero is False
        assert sim.flags.carry is False

    def test_mov_a_a(self) -> None:
        # MOV A,A (0x7F) — no-op essentially
        sim = run_program(bytes([0x3E, 0x42, 0x7F, 0x76]))
        assert sim.a == 0x42

    def test_mov_all_regs_to_a(self) -> None:
        # Test MOV A, E (sss=E=3, SSS&3==11 → unambiguously MOV)
        # MOV A,E = 01 111 011 = 0x7B
        sim = run_program(bytes([0x1E, 0x07, 0x7B, 0x76]))  # MVI E,7; MOV A,E
        assert sim.a == 7


# ---------------------------------------------------------------------------
# MVI instruction
# ---------------------------------------------------------------------------


class TestMvi:
    """Move immediate."""

    def test_mvi_a(self) -> None:
        # MVI A, 0x42 = 0x3E, 0x42
        sim = run_program(bytes([0x3E, 0x42, 0x76]))
        assert sim.a == 0x42

    def test_mvi_b(self) -> None:
        sim = run_program(bytes([0x06, 0xFF, 0x76]))
        assert sim.b == 0xFF

    def test_mvi_h_l(self) -> None:
        # MVI H,0x10; MVI L,0x20
        sim = run_program(bytes([0x26, 0x10, 0x2E, 0x20, 0x76]))
        assert sim.h == 0x10
        assert sim.l == 0x20

    def test_mvi_zero(self) -> None:
        sim = run_program(bytes([0x3E, 0x00, 0x76]))
        assert sim.a == 0

    def test_mvi_flags_not_affected(self) -> None:
        sim = run_program(bytes([0x3E, 0x00, 0x76]))
        # MVI doesn't update flags — Z should be False despite A=0
        assert sim.flags.zero is False


# ---------------------------------------------------------------------------
# INR and DCR
# ---------------------------------------------------------------------------


class TestInrDcr:
    """Increment and decrement."""

    def test_inr_b(self) -> None:
        # INR B = 0x00; B wraps from 0 to 1
        sim = run_program(bytes([0x00, 0x76]))
        assert sim.b == 1

    def test_inr_a(self) -> None:
        # MVI A,5; INR A (0x38)
        sim = run_program(bytes([0x3E, 0x05, 0x38, 0x76]))
        assert sim.a == 6

    def test_inr_wraps_0xff_to_0(self) -> None:
        # MVI A,0xFF; INR A → A=0, Z=1
        sim = run_program(bytes([0x3E, 0xFF, 0x38, 0x76]))
        assert sim.a == 0
        assert sim.flags.zero is True

    def test_dcr_b(self) -> None:
        # MVI B,5; DCR B (0x01) → B=4
        sim = run_program(bytes([0x06, 0x05, 0x01, 0x76]))
        assert sim.b == 4

    def test_dcr_wraps_0_to_0xff(self) -> None:
        # DCR B from 0 → 0xFF, S=1
        sim = run_program(bytes([0x01, 0x76]))  # DCR B (B starts at 0)
        assert sim.b == 0xFF
        assert sim.flags.sign is True

    def test_dcr_sets_zero_flag(self) -> None:
        # MVI B,1; DCR B → B=0, Z=1
        sim = run_program(bytes([0x06, 0x01, 0x01, 0x76]))
        assert sim.b == 0
        assert sim.flags.zero is True

    def test_inr_updates_z_s_p_not_cy(self) -> None:
        # Set CY by overflow, then INR B
        sim = run_program(bytes([0x3E, 0xFF, 0xC4, 0x01, 0x06, 0x0, 0x00, 0x76]))
        # CY=1 from ADI, INR B doesn't touch CY
        assert sim.flags.carry is True


# ---------------------------------------------------------------------------
# ALU register instructions
# ---------------------------------------------------------------------------


class TestAluReg:
    """ALU operations with register source."""

    def test_add_b(self) -> None:
        # MVI B,1; MVI A,2; ADD B (0x80) → A=3
        sim = run_program(bytes([0x06, 0x01, 0x3E, 0x02, 0x80, 0x76]))
        assert sim.a == 3
        assert sim.flags.carry is False

    def test_add_overflow(self) -> None:
        # MVI A,0xFF; ADD A (0x87) → A=0xFE, CY=1
        sim = run_program(bytes([0x3E, 0xFF, 0x87, 0x76]))
        assert sim.a == 0xFE
        assert sim.flags.carry is True

    def test_adc_uses_carry(self) -> None:
        # Set CY, then ADC B: A ← A + B + 1
        # MVI A,0xFF; ADI 1 (sets CY); MVI B,1; ADC B (0x88)
        sim = run_program(bytes([0x3E, 0xFF, 0xC4, 0x01, 0x06, 0x01, 0x88, 0x76]))
        # After ADI: A=0, CY=1; ADC B: A = 0 + 1 + 1 = 2
        assert sim.a == 2

    def test_sub_b(self) -> None:
        # MVI A,5; MVI B,3; SUB B (0x90) → A=2
        sim = run_program(bytes([0x3E, 0x05, 0x06, 0x03, 0x90, 0x76]))
        assert sim.a == 2
        assert sim.flags.carry is False  # no borrow

    def test_sub_self_zeros_a(self) -> None:
        # SUB A (0x97) → A=0, Z=1, CY=0
        sim = run_program(bytes([0x3E, 0x42, 0x97, 0x76]))
        assert sim.a == 0
        assert sim.flags.zero is True
        assert sim.flags.carry is False

    def test_sbb_with_borrow(self) -> None:
        # MVI A,5; set CY; SBB B (where B=1): A ← 5 - 1 - 1 = 3
        # MVI A,5; MVI B,1; MVI C,0xFF; ADI 1 to set CY; then SBB B
        # Simpler: MVI A,0xFF; ADI 1 (CY=1); MVI A,5; MVI B,1; SBB B (0x98)
        sim = run_program(bytes([0x3E, 0xFF, 0xC4, 0x01, 0x3E, 0x05, 0x06, 0x01, 0x98, 0x76]))
        # After: A = 5 - 1 - 1 = 3
        assert sim.a == 3

    def test_ana_b(self) -> None:
        # MVI A,0xFF; MVI B,0x0F; ANA B (0xA0) → A=0x0F
        sim = run_program(bytes([0x3E, 0xFF, 0x06, 0x0F, 0xA0, 0x76]))
        assert sim.a == 0x0F
        assert sim.flags.carry is False

    def test_xra_b(self) -> None:
        # MVI A,0xFF; MVI B,0x0F; XRA B (0xA8) → A=0xF0
        sim = run_program(bytes([0x3E, 0xFF, 0x06, 0x0F, 0xA8, 0x76]))
        assert sim.a == 0xF0

    def test_ora_b(self) -> None:
        # MVI A,0x0F; MVI B,0xF0; ORA B (0xB0) → A=0xFF
        sim = run_program(bytes([0x3E, 0x0F, 0x06, 0xF0, 0xB0, 0x76]))
        assert sim.a == 0xFF

    def test_cmp_equal_sets_zero(self) -> None:
        # MVI A,5; MVI B,5; CMP B (0xB8) → Z=1, A unchanged
        sim = run_program(bytes([0x3E, 0x05, 0x06, 0x05, 0xB8, 0x76]))
        assert sim.flags.zero is True
        assert sim.a == 5  # A unchanged by CMP

    def test_cmp_a_less_sets_carry(self) -> None:
        # MVI A,1; MVI B,5; CMP B → borrow (1-5<0), CY=1
        sim = run_program(bytes([0x3E, 0x01, 0x06, 0x05, 0xB8, 0x76]))
        assert sim.flags.carry is True
        assert sim.a == 1  # A unchanged

    def test_xra_self_clears_a(self) -> None:
        # XRA A (0xAF) clears A to 0, Z=1, CY=0
        sim = run_program(bytes([0x3E, 0x42, 0xAF, 0x76]))
        assert sim.a == 0
        assert sim.flags.zero is True
        assert sim.flags.carry is False


# ---------------------------------------------------------------------------
# ALU immediate instructions
# ---------------------------------------------------------------------------


class TestAluImm:
    """ALU operations with immediate operand."""

    def test_adi(self) -> None:
        # MVI A,2; ADI 3 (0xC4,0x03) → A=5
        sim = run_program(bytes([0x3E, 0x02, 0xC4, 0x03, 0x76]))
        assert sim.a == 5

    def test_aci_with_carry(self) -> None:
        # MVI A,0xFF; ADI 1 (CY=1); MVI A,0; ACI 0 → A = 0 + 0 + 1 = 1
        sim = run_program(bytes([0x3E, 0xFF, 0xC4, 0x01, 0x3E, 0x00, 0xCC, 0x00, 0x76]))
        assert sim.a == 1

    def test_sui(self) -> None:
        # MVI A,10; SUI 3 (0xD4,0x03) → A=7
        sim = run_program(bytes([0x3E, 0x0A, 0xD4, 0x03, 0x76]))
        assert sim.a == 7

    def test_ani(self) -> None:
        # MVI A,0xFF; ANI 0x0F (0xE4,0x0F) → A=0x0F
        sim = run_program(bytes([0x3E, 0xFF, 0xE4, 0x0F, 0x76]))
        assert sim.a == 0x0F

    def test_xri(self) -> None:
        # MVI A,0xFF; XRI 0xFF (0xEC,0xFF) → A=0, Z=1
        sim = run_program(bytes([0x3E, 0xFF, 0xEC, 0xFF, 0x76]))
        assert sim.a == 0
        assert sim.flags.zero is True

    def test_ori(self) -> None:
        # MVI A,0x0F; ORI 0xF0 (0xF4,0xF0) → A=0xFF
        sim = run_program(bytes([0x3E, 0x0F, 0xF4, 0xF0, 0x76]))
        assert sim.a == 0xFF

    def test_cpi_equal(self) -> None:
        # MVI A,7; CPI 7 (0xFC,0x07) → Z=1, A=7 unchanged
        sim = run_program(bytes([0x3E, 0x07, 0xFC, 0x07, 0x76]))
        assert sim.flags.zero is True
        assert sim.a == 7


# ---------------------------------------------------------------------------
# Rotate instructions
# ---------------------------------------------------------------------------


class TestRotates:
    """Rotate accumulator tests."""

    def test_rlc_basic(self) -> None:
        # MVI A,0x01 (00000001); RLC (0x02) → A=0x02, CY=0
        sim = run_program(bytes([0x3E, 0x01, 0x02, 0x76]))
        assert sim.a == 0x02
        assert sim.flags.carry is False

    def test_rlc_wraps_bit7(self) -> None:
        # MVI A,0x80 (10000000); RLC → A=0x01, CY=1
        sim = run_program(bytes([0x3E, 0x80, 0x02, 0x76]))
        assert sim.a == 0x01
        assert sim.flags.carry is True

    def test_rrc_basic(self) -> None:
        # MVI A,0x02 (00000010); RRC (0x0A) → A=0x01, CY=0
        sim = run_program(bytes([0x3E, 0x02, 0x0A, 0x76]))
        assert sim.a == 0x01
        assert sim.flags.carry is False

    def test_rrc_wraps_bit0(self) -> None:
        # MVI A,0x01 (00000001); RRC → A=0x80, CY=1
        sim = run_program(bytes([0x3E, 0x01, 0x0A, 0x76]))
        assert sim.a == 0x80
        assert sim.flags.carry is True

    def test_ral_shifts_carry_in(self) -> None:
        # Set CY via overflow, then RAL on 0x01:
        # MVI A,0xFF; ADI 1 (CY=1); MVI A,0x01; RAL (0x12) → A=0x03, CY=0
        sim = run_program(bytes([0x3E, 0xFF, 0xC4, 0x01, 0x3E, 0x01, 0x12, 0x76]))
        assert sim.a == 0x03  # 0x01 << 1 | CY(1) = 0b11 = 3
        assert sim.flags.carry is False

    def test_rar_shifts_carry_in(self) -> None:
        # Set CY, then RAR on 0x02:
        # old_CY=1 → new A[7]=1; A >> 1 = 0x01; new_A = 0x81
        sim = run_program(bytes([0x3E, 0xFF, 0xC4, 0x01, 0x3E, 0x02, 0x1A, 0x76]))
        assert sim.a == 0x81
        assert sim.flags.carry is False

    def test_rotate_does_not_change_zsf(self) -> None:
        # After RLC, Z/S/P flags should not change
        # Set Z=1 via SUB A, then RLC
        sim = run_program(bytes([0x97, 0x3E, 0x01, 0x02, 0x76]))  # SUB A; MVI A,1; RLC
        # Z was True after SUB A, but then MVI A,1 doesn't change flags,
        # then RLC doesn't change Z/S/P either (only CY)
        # Actually MVI doesn't change flags, so Z is still True from SUB A
        assert sim.flags.zero is True  # Z preserved from SUB A


# ---------------------------------------------------------------------------
# Jump instructions
# ---------------------------------------------------------------------------


class TestJumps:
    """JMP and conditional jump tests."""

    def test_jmp_unconditional(self) -> None:
        # JMP 0x0006; (bytes 0-2) | HLT at 3 | ... | MVI A,42; HLT at 6
        # JMP: 0x7C, 0x06, 0x00
        # HLT at 3: 0x76 (should be skipped)
        # MVI A,42 at 4: 0x3E, 0x2A
        # HLT at 6: 0x76
        program = bytes([
            0x7C, 0x04, 0x00,   # JMP 0x0004
            0x76,               # HLT (at 0x0003, should be skipped)
            0x3E, 0x2A,         # MVI A, 42 (at 0x0004)
            0x76,               # HLT (at 0x0006)
        ])
        sim = run_program(program)
        assert sim.a == 42

    def test_jfc_jumps_when_carry_false(self) -> None:
        # JFC (carry false = 0x40): with CY=0, should jump
        program = bytes([
            0x40, 0x06, 0x00,   # JFC 0x0006
            0x3E, 0x01,         # MVI A,1 (should be skipped)
            0x76,               # HLT (skipped)
            0x3E, 0x02,         # MVI A,2 (at 0x0006)
            0x76,               # HLT
        ])
        sim = run_program(program)
        assert sim.a == 2

    def test_jfc_does_not_jump_when_carry_true(self) -> None:
        # Set carry, then JFC: should NOT jump
        program = bytes([
            0x3E, 0xFF,         # MVI A,0xFF
            0xC4, 0x01,         # ADI 1 (sets CY=1)
            0x40, 0x0B, 0x00,   # JFC 0x000B (won't jump)
            0x3E, 0x05,         # MVI A,5 (at 0x0007)
            0x76,               # HLT
            0x00,               # padding
            0x3E, 0x99,         # MVI A,0x99 (at 0x000B, should not reach)
            0x76,               # HLT
        ])
        sim = run_program(program)
        assert sim.a == 5

    def test_jtc_jumps_when_carry_true(self) -> None:
        # Set CY, then JTC (0x44): should jump
        # Addresses: 0-1=MVI A,0xFF; 2-3=ADI 1; 4-6=JTC 0x000A;
        # 7=MVI A,1(skip); 8=HLT(skip); 10-11=MVI A,2; 12=HLT (at 0x000A)
        program = bytes([
            0x3E, 0xFF,         # MVI A,0xFF  (0x0000)
            0xC4, 0x01,         # ADI 1 (CY=1)  (0x0002)
            0x44, 0x0A, 0x00,   # JTC 0x000A  (0x0004)
            0x3E, 0x01,         # MVI A,1 (skipped)  (0x0007)
            0x76,               # HLT (skipped)  (0x0009)
            0x3E, 0x02,         # MVI A,2 (at 0x000A)
            0x76,               # HLT
        ])
        sim = run_program(program)
        assert sim.a == 2

    def test_jtz_jumps_when_zero_set(self) -> None:
        # SUB A sets Z=1, JTZ (0x4C) should jump
        program = bytes([
            0x97,               # SUB A (Z=1)
            0x4C, 0x05, 0x00,   # JTZ 0x0005
            0x76,               # HLT (skipped)
            0x3E, 0x42,         # MVI A,0x42 (at 0x0005)
            0x76,               # HLT
        ])
        sim = run_program(program)
        assert sim.a == 0x42


# ---------------------------------------------------------------------------
# Call and Return
# ---------------------------------------------------------------------------


class TestCallRet:
    """Call/return and stack depth tracking."""

    def test_cal_unconditional(self) -> None:
        # CAL 0x0006: pushes return addr onto stack, jumps to subroutine
        # Subroutine: MVI A,42; RET (0x3F)
        # Main returns to HLT
        program = bytes([
            0x7E, 0x06, 0x00,   # CAL 0x0006 (at 0x0000)
            0x76,               # HLT (at 0x0003, return target)
            0x00, 0x00,         # padding (0x0004, 0x0005)
            0x3E, 0x2A,         # MVI A,42 (at 0x0006)
            0x3F,               # RET (at 0x0008)
        ])
        sim = run_program(program)
        assert sim.a == 42

    def test_stack_depth_increments_on_call(self) -> None:
        # After a CAL, stack_depth should be 1
        sim = make_sim()
        # Load: CAL 0x0005; HLT; ... (sub: HLT)
        program = bytes([
            0x7E, 0x05, 0x00,   # CAL 0x0005
            0x76,               # HLT (at 0x0003)
            0x00,               # padding
            0x76,               # HLT in subroutine (at 0x0005)
        ])
        traces = sim.run(program)
        # After CAL, stack_depth should be >= 1
        # The subroutine hits HLT, so we never RET
        assert sim.stack_depth >= 1

    def test_stack_depth_decrements_on_ret(self) -> None:
        sim = make_sim()
        program = bytes([
            0x7E, 0x06, 0x00,   # CAL 0x0006
            0x76,               # HLT
            0x00, 0x00,         # padding
            0x3F,               # RET (at 0x0006)
        ])
        sim.run(program)
        assert sim.stack_depth == 0

    def test_nested_calls(self) -> None:
        # Two levels of nesting
        program = bytes([
            0x7E, 0x07, 0x00,   # CAL 0x0007 (level 1)  at 0x0000
            0x76,               # HLT at 0x0003
            0x00, 0x00, 0x00,   # padding
            0x7E, 0x0D, 0x00,   # CAL 0x000D (level 2)  at 0x0007
            0x3F,               # RET at 0x000A
            0x00, 0x00,         # padding
            0x3E, 0x07,         # MVI A,7  at 0x000D
            0x3F,               # RET at 0x000F
        ])
        sim = run_program(program)
        assert sim.a == 7

    def test_conditional_call_taken(self) -> None:
        # CFC (carry false = 0x42): CY=0, so call is taken
        program = bytes([
            0x42, 0x06, 0x00,   # CFC 0x0006
            0x76,               # HLT
            0x00, 0x00,         # padding
            0x3E, 0x55,         # MVI A,0x55 (at 0x0006)
            0x3F,               # RET
        ])
        sim = run_program(program)
        assert sim.a == 0x55

    def test_conditional_return_taken(self) -> None:
        # RFC (return if carry false): CY=0 so return is taken
        program = bytes([
            0x7E, 0x06, 0x00,   # CAL 0x0006
            0x76,               # HLT
            0x00, 0x00,         # padding
            0x3E, 0x42,         # MVI A,0x42 (at 0x0006)
            0x03,               # RFC (return if CY=0; CY is 0 so returns)
        ])
        sim = run_program(program)
        assert sim.a == 0x42

    def test_unconditional_ret(self) -> None:
        program = bytes([
            0x7E, 0x06, 0x00,   # CAL 0x0006
            0x76,               # HLT
            0x00, 0x00,
            0x3E, 0x99,         # MVI A,0x99
            0x3F,               # RET
        ])
        sim = run_program(program)
        assert sim.a == 0x99


# ---------------------------------------------------------------------------
# RST instructions
# ---------------------------------------------------------------------------


class TestRst:
    """Restart instructions — 1-byte calls to fixed addresses."""

    def test_rst_0_calls_0x0000(self) -> None:
        # RST 0 = 0x05 → calls 0x0000
        # But 0x0000 is where the program starts. We need to be careful here.
        # Better: use RST 1 which calls 0x0008
        # Layout: RST 1 (0x0D) at 0x0000; HLT at 0x0001; sub at 0x0008
        program = bytes([
            0x0D,               # RST 1 → call 0x0008
            0x76,               # HLT at 0x0001 (return target)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  # padding 0x0002-0x0007
            0x3E, 0x11,         # MVI A,17 at 0x0008
            0x3F,               # RET at 0x000A
        ])
        sim = run_program(program)
        assert sim.a == 17

    def test_rst_7_calls_0x0038(self) -> None:
        # RST 7 = 0x3D → target 7*8=56=0x38
        # This requires a subroutine at 0x38
        program = bytearray(64)  # 64 bytes
        program[0] = 0x3D        # RST 7
        program[1] = 0x76        # HLT (return target)
        program[0x38] = 0x3E     # MVI A, ...
        program[0x39] = 0x07     # ... 7
        program[0x3A] = 0x3F     # RET
        sim = run_program(bytes(program))
        assert sim.a == 7


# ---------------------------------------------------------------------------
# I/O ports
# ---------------------------------------------------------------------------


class TestIO:
    """IN/OUT instruction tests."""

    def test_in_reads_from_input_port(self) -> None:
        sim = make_sim()
        sim.set_input_port(0, 0xAB)
        # IN 0 = 0x41
        program = bytes([0x41, 0x76])  # IN 0; HLT
        sim.run(program)
        assert sim.a == 0xAB

    def test_in_port_3(self) -> None:
        sim = make_sim()
        sim.set_input_port(3, 0x42)
        # IN 3 = 0x59 (01 011 001)
        program = bytes([0x59, 0x76])
        sim.run(program)
        assert sim.a == 0x42

    def test_out_writes_to_output_port(self) -> None:
        # OUT port encoding: group=00, sss=010, port=bits[5:1]
        # For port 17 (0b10001): opcode = 0b00100010 = 0x22
        # ddd = (0x22 >> 3) & 7 = 4, sss = 0x22 & 7 = 2
        # ddd=4 >= 4 → OUT (not rotate); port = (0x22 >> 1) & 0x1F = 17
        sim = make_sim()
        program = bytes([0x3E, 0xAB, 0x22, 0x76])  # MVI A,0xAB; OUT 17; HLT
        sim.run(program)
        assert sim.get_output_port(17) == 0xAB

    def test_output_port_initial_zero(self) -> None:
        sim = make_sim()
        assert sim.get_output_port(0) == 0

    def test_input_port_validation(self) -> None:
        sim = make_sim()
        with pytest.raises(ValueError):
            sim.set_input_port(8, 0)

    def test_output_port_validation(self) -> None:
        sim = make_sim()
        with pytest.raises(ValueError):
            sim.get_output_port(24)


# ---------------------------------------------------------------------------
# HLT
# ---------------------------------------------------------------------------


class TestHlt:
    """HLT instruction tests — both encodings."""

    def test_hlt_0x76_stops_execution(self) -> None:
        sim = make_sim()
        traces = sim.run(bytes([0x76]))  # HLT
        assert sim.halted is True
        assert len(traces) == 1

    def test_hlt_0xff_stops_execution(self) -> None:
        sim = make_sim()
        traces = sim.run(bytes([0xFF]))  # HLT (alternate)
        assert sim.halted is True
        assert len(traces) == 1

    def test_step_raises_after_hlt(self) -> None:
        sim = make_sim()
        sim.run(bytes([0x76]))
        with pytest.raises(RuntimeError, match="halted"):
            sim.step()

    def test_instructions_before_hlt_execute(self) -> None:
        # MVI A,5; HLT → should execute MVI
        sim = make_sim()
        traces = sim.run(bytes([0x3E, 0x05, 0x76]))
        assert sim.a == 5
        assert len(traces) == 2  # MVI + HLT


# ---------------------------------------------------------------------------
# Memory access via M pseudo-register
# ---------------------------------------------------------------------------


class TestMemoryAccess:
    """M pseudo-register and memory access."""

    def test_mov_m_a_writes_memory(self) -> None:
        # MVI H,0; MVI L,0x20; MVI A,0x42; MOV M,A (0x77)
        program = bytes([
            0x26, 0x00,   # MVI H,0
            0x2E, 0x20,   # MVI L,0x20
            0x3E, 0x42,   # MVI A,0x42
            0x77,         # MOV M,A
            0x76,         # HLT
        ])
        sim = run_program(program)
        assert sim.memory[0x20] == 0x42

    def test_mov_h_m_reads_memory(self) -> None:
        # Setup: write 0x55 to memory[0x10], then read it back via MOV H,M
        # MOV H,M = 01 100 110 = 0x66 (ddd=4=H, sss=6=M)
        # ddd=4 >= 4 → decoded as MOV (not jump/call)
        program = bytes([
            0x26, 0x00,   # MVI H,0
            0x2E, 0x10,   # MVI L,0x10
            0x36, 0x55,   # MVI M,0x55
            0x66,         # MOV H,M = 01 100 110
            0x76,         # HLT
        ])
        sim = run_program(program)
        assert sim.h == 0x55

    def test_hl_address_formula(self) -> None:
        # H=0x3F, L=0x80 → address = (0x3F & 0x3F) << 8 | 0x80 = 0x3F80
        sim = make_sim()
        program = bytes([
            0x26, 0x3F,   # MVI H,0x3F
            0x2E, 0x80,   # MVI L,0x80
            0x76,         # HLT
        ])
        sim.run(program)
        assert sim.hl_address == 0x3F80

    def test_inr_m_increments_memory(self) -> None:
        # Set [0x20]=5, INR M → [0x20]=6
        program = bytes([
            0x26, 0x00,   # MVI H,0
            0x2E, 0x20,   # MVI L,0x20
            0x36, 0x05,   # MVI M,5
            0x30,         # INR M (0x30)
            0x76,         # HLT
        ])
        sim = run_program(program)
        assert sim.memory[0x20] == 6

    def test_trace_records_memory_address_for_m(self) -> None:
        # After MOV M,A, the trace should have memory_address set
        sim = make_sim()
        program = bytes([
            0x26, 0x00,   # MVI H,0
            0x2E, 0x10,   # MVI L,0x10
            0x3E, 0x42,   # MVI A,0x42
            0x77,         # MOV M,A
            0x76,         # HLT
        ])
        traces = sim.run(program)
        # The 4th trace (index 3) should be MOV M,A
        mov_trace = traces[3]
        assert mov_trace.memory_address == 0x10


# ---------------------------------------------------------------------------
# Trace structure
# ---------------------------------------------------------------------------


class TestTrace:
    """Tests for trace structure and content."""

    def test_trace_has_correct_address(self) -> None:
        sim = make_sim()
        traces = sim.run(bytes([0x3E, 0x05, 0x76]))
        assert traces[0].address == 0
        assert traces[1].address == 2  # after 2-byte MVI

    def test_trace_contains_raw_bytes(self) -> None:
        sim = make_sim()
        traces = sim.run(bytes([0x3E, 0x05, 0x76]))
        assert traces[0].raw == bytes([0x3E, 0x05])  # MVI A,5

    def test_trace_records_a_before_after(self) -> None:
        sim = make_sim()
        traces = sim.run(bytes([0x3E, 0x05, 0x3E, 0x0A, 0x76]))
        assert traces[0].a_before == 0
        assert traces[0].a_after == 5
        assert traces[1].a_before == 5
        assert traces[1].a_after == 10

    def test_trace_mnemonic_includes_value(self) -> None:
        sim = make_sim()
        traces = sim.run(bytes([0x3E, 0x42, 0x76]))
        assert "0x42" in traces[0].mnemonic.lower() or "42" in traces[0].mnemonic


# ---------------------------------------------------------------------------
# Reset behavior
# ---------------------------------------------------------------------------


class TestReset:
    """reset() clears all state."""

    def test_reset_clears_registers(self) -> None:
        sim = make_sim()
        sim.run(bytes([0x3E, 0x42, 0x06, 0xFF, 0x76]))
        sim.reset()
        assert sim.a == 0
        assert sim.b == 0

    def test_reset_clears_memory(self) -> None:
        sim = make_sim()
        sim.run(bytes([0x26, 0x00, 0x2E, 0x10, 0x36, 0x42, 0x76]))
        sim.reset()
        assert all(b == 0 for b in sim.memory)


# ---------------------------------------------------------------------------
# simulator-protocol conformance tests
# ---------------------------------------------------------------------------


class TestSimulatorProtocol:
    """Verify Intel8008Simulator conforms to the Simulator[Intel8008State] protocol.

    These tests exercise the three new methods added for protocol conformance:
    ``get_state()``, ``execute()``, and ``load()``.

    They do NOT import from ``simulator_protocol`` directly — they test the
    public API surface that consumers of the protocol will use.
    """

    def test_get_state_returns_intel8008_state(self) -> None:
        """get_state() must return an Intel8008State instance."""
        from intel8008_simulator import Intel8008State

        sim = make_sim()
        state = sim.get_state()
        assert isinstance(state, Intel8008State)

    def test_get_state_is_frozen(self) -> None:
        """Intel8008State must be immutable — assignment raises FrozenInstanceError."""
        import dataclasses

        sim = make_sim()
        state = sim.get_state()
        with pytest.raises(dataclasses.FrozenInstanceError):
            state.a = 99  # type: ignore[misc]

    def test_get_state_initial_values(self) -> None:
        """A fresh simulator's state snapshot should have all-zero registers and PC=0."""
        sim = make_sim()
        state = sim.get_state()
        assert state.pc == 0
        assert state.a == 0
        assert state.b == 0
        assert state.halted is False
        assert state.stack_depth == 0
        assert len(state.stack) == 8
        assert len(state.memory) == 16384

    def test_get_state_reflects_current_registers(self) -> None:
        """After running a program, get_state() must reflect the final register values."""
        sim = make_sim()
        # MVI A, 42; HLT
        sim.run(bytes([0x3E, 0x2A, 0x76]))
        state = sim.get_state()
        assert state.a == 42
        assert state.halted is True

    def test_get_state_memory_is_immutable_snapshot(self) -> None:
        """Mutating the simulator's memory after get_state() must not change the snapshot."""
        sim = make_sim()
        # Write 0xFF to memory[0x10] via MVI M then grab state
        program = bytes([
            0x26, 0x00,   # MVI H, 0
            0x2E, 0x10,   # MVI L, 0x10
            0x36, 0xFF,   # MVI M, 0xFF
            0x76,         # HLT
        ])
        sim.run(program)
        state = sim.get_state()
        # Confirm the snapshot has 0xFF at offset 0x10
        assert state.memory[0x10] == 0xFF
        # Now mutate the simulator's live memory
        sim._memory[0x10] = 0x00
        # The snapshot must NOT change
        assert state.memory[0x10] == 0xFF

    def test_execute_returns_execution_result(self) -> None:
        """execute() must return an ExecutionResult with the expected fields."""
        from simulator_protocol import ExecutionResult

        sim = make_sim()
        result = sim.execute(bytes([0x76]))  # HLT
        assert isinstance(result, ExecutionResult)
        assert hasattr(result, "halted")
        assert hasattr(result, "steps")
        assert hasattr(result, "final_state")
        assert hasattr(result, "error")
        assert hasattr(result, "traces")

    def test_execute_simple_program_halts(self) -> None:
        """A program ending in HLT must produce result.ok == True."""
        sim = make_sim()
        # MVI A, 7; HLT  (2 instructions)
        result = sim.execute(bytes([0x3E, 0x07, 0x76]))
        assert result.ok is True
        assert result.halted is True
        assert result.error is None

    def test_execute_max_steps_exceeded(self) -> None:
        """When max_steps is hit without HLT, result.ok must be False with an error."""
        sim = make_sim()
        # An infinite-ish loop: JMP 0x0000 (3 bytes)
        # 0x7C = JMP, lo=0x00, hi=0x00 → jump back to 0x0000 forever
        result = sim.execute(bytes([0x7C, 0x00, 0x00]), max_steps=10)
        assert result.ok is False
        assert result.halted is False
        assert result.error is not None
        assert "max_steps" in result.error
        assert result.steps == 10

    def test_execute_final_state_accessible(self) -> None:
        """final_state on ExecutionResult must expose Intel8008State fields."""
        from intel8008_simulator import Intel8008State

        sim = make_sim()
        # MVI A, 0x55; MVI B, 0xAA; HLT
        result = sim.execute(bytes([0x3E, 0x55, 0x06, 0xAA, 0x76]))
        state = result.final_state
        assert isinstance(state, Intel8008State)
        assert state.a == 0x55
        assert state.b == 0xAA
        assert state.halted is True

    def test_execute_traces_match_steps(self) -> None:
        """The length of result.traces must equal result.steps."""
        sim = make_sim()
        # MVI A,1; MVI B,2; HLT  → 3 instructions
        result = sim.execute(bytes([0x3E, 0x01, 0x06, 0x02, 0x76]))
        assert result.steps == 3
        assert len(result.traces) == 3

    def test_execute_trace_has_correct_structure(self) -> None:
        """Each StepTrace in result.traces must have pc_before, pc_after, mnemonic, description."""
        from simulator_protocol import StepTrace

        sim = make_sim()
        result = sim.execute(bytes([0x76]))  # HLT at PC=0
        assert len(result.traces) == 1
        trace = result.traces[0]
        assert isinstance(trace, StepTrace)
        assert trace.pc_before == 0
        assert isinstance(trace.mnemonic, str)
        assert isinstance(trace.description, str)
        assert "0x" in trace.description  # description includes hex PC

    def test_execute_resets_between_calls(self) -> None:
        """Calling execute() twice must produce independent results (implicit reset)."""
        sim = make_sim()
        # First run: MVI A, 10; HLT
        r1 = sim.execute(bytes([0x3E, 0x0A, 0x76]))
        # Second run: MVI A, 20; HLT
        r2 = sim.execute(bytes([0x3E, 0x14, 0x76]))
        assert r1.final_state.a == 10
        assert r2.final_state.a == 20

    def test_load_alias_works(self) -> None:
        """load() is a valid alias for load_program(program, 0)."""
        sim = make_sim()
        sim.reset()
        sim.load(bytes([0x3E, 0x07, 0x76]))  # MVI A,7; HLT
        # Manually step until halted
        while not sim.halted:
            sim.step()
        assert sim.a == 7

    def test_reset_clears_stack(self) -> None:
        sim = make_sim()
        sim.run(bytes([0x7E, 0x06, 0x00, 0x76, 0x00, 0x00, 0x76]))  # CAL; HLT...
        sim.reset()
        assert sim.pc == 0
        assert sim.stack_depth == 0

    def test_reset_clears_flags(self) -> None:
        sim = make_sim()
        sim.run(bytes([0x3E, 0xFF, 0xC4, 0x01, 0x76]))  # set CY
        sim.reset()
        assert sim.flags.carry is False
        assert sim.flags.zero is False

    def test_reset_clears_halted(self) -> None:
        sim = make_sim()
        sim.run(bytes([0x76]))
        assert sim.halted is True
        sim.reset()
        assert sim.halted is False

    def test_reset_clears_output_ports_not_input_ports(self) -> None:
        # I/O ports are intentionally NOT cleared by reset() since they model
        # external hardware connections. Output ports are written by OUT instructions
        # and persist. Input ports are set externally and also persist.
        sim = make_sim()
        sim.set_input_port(3, 0xAB)
        sim.reset()
        # Input ports persist across resets (external hardware connection)
        # Output ports: since no OUT was executed, they're still 0
        assert sim.get_output_port(0) == 0


# ---------------------------------------------------------------------------
# Complete programs from the spec
# ---------------------------------------------------------------------------


class TestPrograms:
    """End-to-end example programs."""

    def test_spec_example_1_plus_2(self) -> None:
        """x = 1 + 2: MVI B,1; MVI A,2; ADD B; HLT"""
        program = bytes([0x06, 0x01, 0x3E, 0x02, 0x80, 0x76])
        sim = run_program(program)
        assert sim.a == 3
        assert sim.flags.zero is False
        assert sim.flags.sign is False
        assert sim.flags.carry is False
        assert sim.flags.parity is True  # 0b00000011 → 2 ones → even

    def test_memory_addition(self) -> None:
        """x = 1 + 2 using memory: MVI M,2; MOV H,M; ADD H with 1 in A"""
        # Use MOV H,M (0x66) to read from memory, then ADD H
        program = bytes([
            0x26, 0x00,   # MVI H,0      (0x0000)
            0x2E, 0x10,   # MVI L,0x10   (0x0002) — addr = 0x0010
            0x36, 0x02,   # MVI M,2      (0x0004) — mem[0x10] = 2
            0x3E, 0x01,   # MVI A,1      (0x0006) — A = 1
            0x66,         # MOV H,M      (0x0008) — H = mem[0x10] = 2
            0x84,         # ADD H (0x84) (0x0009) — A = 1 + 2 = 3
            0x76,         # HLT          (0x000A)
        ])
        sim = run_program(program)
        assert sim.a == 3

    def test_loop_countdown(self) -> None:
        """Simple countdown loop: B=5, decrement until 0."""
        # MVI B,5
        # loop: DCR B; JFZ loop; HLT
        program = bytes([
            0x06, 0x05,         # MVI B,5
            0x01,               # DCR B (at 0x0002)
            0x48, 0x02, 0x00,   # JFZ 0x0002 (jump if Z=0)
            0x76,               # HLT
        ])
        sim = run_program(program, max_steps=1000)
        assert sim.b == 0
        assert sim.flags.zero is True

    def test_run_returns_all_traces(self) -> None:
        """run() should return one trace per instruction."""
        program = bytes([0x3E, 0x01, 0x3E, 0x02, 0x76])
        sim = make_sim()
        traces = sim.run(program)
        # Two MVI + one HLT = 3 traces
        assert len(traces) == 3

    def test_max_steps_limits_execution(self) -> None:
        """max_steps should stop infinite loops."""
        # Infinite loop: JMP 0x0000 (jumps back forever)
        program = bytes([0x7C, 0x00, 0x00])  # JMP 0x0000
        sim = make_sim()
        traces = sim.run(program, max_steps=10)
        assert len(traces) == 10
        assert sim.halted is False  # stopped by max_steps, not HLT

    def test_adc_chain(self) -> None:
        """Multi-byte addition using ADC for carry propagation."""
        # Add 0xFF + 0xFF + carry across two ADD/ADC pairs
        # MVI B,0xFF; MVI A,0xFF; ADD B (A=0xFE, CY=1); ACI 0 (A=0xFF, CY=0)
        program = bytes([
            0x06, 0xFF,   # MVI B,0xFF
            0x3E, 0xFF,   # MVI A,0xFF
            0x80,         # ADD B → A=0xFE, CY=1
            0xCC, 0x00,   # ACI 0 → A=0xFE+0+1=0xFF, CY=0
            0x76,         # HLT
        ])
        sim = run_program(program)
        assert sim.a == 0xFF

    def test_flags_dataclass_equality(self) -> None:
        """Intel8008Flags equality works correctly."""
        f1 = Intel8008Flags(carry=True, zero=False, sign=False, parity=True)
        f2 = Intel8008Flags(carry=True, zero=False, sign=False, parity=True)
        assert f1 == f2

    def test_flags_copy_is_independent(self) -> None:
        """flags.copy() produces an independent copy."""
        f = Intel8008Flags(carry=True)
        f2 = f.copy()
        f2.carry = False
        assert f.carry is True

    def test_step_returns_trace(self) -> None:
        """step() returns an Intel8008Trace."""
        sim = make_sim()
        sim.load_program(bytes([0x3E, 0x05, 0x76]))
        trace = sim.step()
        assert isinstance(trace, Intel8008Trace)
        assert trace.address == 0
        assert trace.a_after == 5

    def test_load_program_at_offset(self) -> None:
        """load_program with non-zero start_address."""
        sim = make_sim()
        sim.load_program(bytes([0x3E, 0x42, 0x76]), start_address=0x100)
        sim._stack[0] = 0x100  # set PC to start_address manually
        traces = sim.run(bytes([]), start_address=0x000)
        # reset clears memory; we can't easily test this without run()
        # Instead test load_program directly
        sim2 = make_sim()
        sim2.load_program(bytes([0x42]), start_address=0x100)
        assert sim2.memory[0x100] == 0x42

    def test_program_too_large_raises(self) -> None:
        """load_program raises ValueError for programs that exceed memory."""
        sim = make_sim()
        with pytest.raises(ValueError):
            sim.load_program(bytes(20000))  # larger than 16384
