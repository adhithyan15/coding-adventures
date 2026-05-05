"""Edge-case and coverage tests for the Intel 8051 simulator.

Tests that specifically target:
  - Flags module (add8_flags, sub8_flags, da_flags, parity)
  - State module (properties, constants)
  - Boundary conditions (SP wrap, bit addressing extremes)
  - Register bank switching
  - Indirect addressing edge cases
  - Unknown opcode raises ValueError
"""

from __future__ import annotations

import pytest

from intel8051_simulator import I8051Simulator
from intel8051_simulator.flags import add8_flags, da_flags, sub8_flags
from intel8051_simulator.state import (
    SFR_ACC,
    SFR_B,
    SFR_DPH,
    SFR_DPL,
    SFR_PSW,
    SFR_SP,
)

HALT = bytes([0xA5])


# ── Flags module unit tests ───────────────────────────────────────────────────

class TestAdd8Flags:
    def test_no_overflow_no_carry(self):
        result, cy, ac, ov, p = add8_flags(0x10, 0x20)
        assert result == 0x30
        assert cy == 0
        assert ac == 0
        assert ov == 0

    def test_unsigned_carry(self):
        result, cy, ac, ov, p = add8_flags(0xFF, 0x01)
        assert result == 0x00
        assert cy == 1

    def test_auxiliary_carry(self):
        _, cy, ac, ov, p = add8_flags(0x08, 0x08)
        assert ac == 1
        assert cy == 0

    def test_signed_overflow_pos_pos(self):
        # 0x7F + 0x01 = 0x80 (positive + positive = negative)
        _, cy, ac, ov, p = add8_flags(0x7F, 0x01)
        assert ov == 1
        assert cy == 0

    def test_signed_overflow_neg_neg(self):
        # 0x80 + 0x80 = 0x00 with carry (negative + negative = positive)
        result, cy, ac, ov, p = add8_flags(0x80, 0x80)
        assert result == 0x00
        assert cy == 1
        assert ov == 1

    def test_carry_in_addc(self):
        result, cy, ac, ov, p = add8_flags(0x01, 0x01, 1)
        assert result == 3

    def test_carry_triggers_carry_out(self):
        # 0xFF + 0x00 + 1 = 0x100 → result=0, cy=1
        result, cy, ac, ov, p = add8_flags(0xFF, 0x00, 1)
        assert result == 0x00
        assert cy == 1

    def test_parity_odd_bits(self):
        # 0x01 has 1 set bit → odd → p=1
        _, cy, ac, ov, p = add8_flags(0x00, 0x01)
        assert p == 1

    def test_parity_even_bits(self):
        # 0x03 has 2 set bits → even → p=0
        _, cy, ac, ov, p = add8_flags(0x00, 0x03)
        assert p == 0


class TestSub8Flags:
    def test_no_borrow(self):
        # 0x10 - 0x05 = 0x0B; no borrow out (cy=0)
        # AC: low nibble 0x0 < 0x5, so a borrow from bit4 occurs → AC=1
        result, cy, ac, ov, p = sub8_flags(0x10, 0x05)
        assert result == 0x0B
        assert cy == 0
        assert ac == 1   # auxiliary borrow: low nibble 0 < 5

    def test_borrow(self):
        result, cy, ac, ov, p = sub8_flags(0x00, 0x01)
        assert result == 0xFF
        assert cy == 1

    def test_borrow_propagated(self):
        # 0x05 - 0x05 - 1 = -1 → borrow
        result, cy, ac, ov, p = sub8_flags(0x05, 0x05, 1)
        assert result == 0xFF
        assert cy == 1

    def test_aux_borrow(self):
        # 0x10 - 0x01 → low nibble: 0 < 1 → AC=1
        _, cy, ac, ov, p = sub8_flags(0x10, 0x01)
        assert ac == 1

    def test_signed_overflow_sub(self):
        # 0x80 - 0x01 = 0x7F; negative - positive = positive → OV=1
        _, cy, ac, ov, p = sub8_flags(0x80, 0x01)
        assert ov == 1


class TestDaFlags:
    def test_no_adjust_needed(self):
        # 0x35 is valid BCD (3, 5) and sum was clean
        result, new_cy, new_p = da_flags(0x35, 0, 0)
        assert result == 0x35

    def test_low_nibble_correction(self):
        # 0x0A → low nibble >9 → add 6 → 0x10
        result, new_cy, new_p = da_flags(0x0A, 0, 0)
        assert result == 0x10

    def test_high_nibble_correction(self):
        # 0xA0 → high nibble >9 → add 0x60 → 0x00, CY=1
        result, new_cy, new_p = da_flags(0xA0, 0, 0)
        assert result == 0x00
        assert new_cy == 1

    def test_ac_triggers_low_correction(self):
        result, new_cy, new_p = da_flags(0x09, 0, 1)   # AC=1 → add 6 → 0x0F
        assert result == 0x0F

    def test_cy_triggers_high_correction(self):
        result, new_cy, new_p = da_flags(0x10, 1, 0)   # CY_in=1 → add 0x60
        assert result == 0x70
        assert new_cy == 1


# ── State properties ──────────────────────────────────────────────────────────

class TestI8051StateProperties:
    def test_acc_property(self):
        sim = I8051Simulator()
        sim._iram[SFR_ACC] = 0xAB
        state = sim.get_state()
        assert state.acc == 0xAB

    def test_b_property(self):
        sim = I8051Simulator()
        sim._iram[SFR_B] = 0xCD
        state = sim.get_state()
        assert state.b == 0xCD

    def test_sp_property(self):
        sim = I8051Simulator()
        state = sim.get_state()
        assert state.sp == 0x07

    def test_dptr_property(self):
        sim = I8051Simulator()
        sim._iram[SFR_DPH] = 0x12
        sim._iram[SFR_DPL] = 0x34
        state = sim.get_state()
        assert state.dptr == 0x1234

    def test_psw_property(self):
        sim = I8051Simulator()
        sim._iram[SFR_PSW] = 0x80
        state = sim.get_state()
        assert state.psw == 0x80
        assert state.cy

    def test_ov_property(self):
        sim = I8051Simulator()
        sim._iram[SFR_PSW] = 0x04
        state = sim.get_state()
        assert state.ov

    def test_parity_property(self):
        sim = I8051Simulator()
        sim._iram[SFR_PSW] = 0x01
        state = sim.get_state()
        assert state.parity

    def test_bank_property_default(self):
        sim = I8051Simulator()
        state = sim.get_state()
        assert state.bank == 0

    def test_bank_property_bank2(self):
        sim = I8051Simulator()
        sim._iram[SFR_PSW] = 0x10   # RS1=1, RS0=0 → bank 2
        state = sim.get_state()
        assert state.bank == 2


# ── Register bank switching ───────────────────────────────────────────────────

class TestRegisterBanks:
    def test_bank0_r0_at_0x00(self):
        sim = I8051Simulator()
        # Bank 0 (default): R0 = iram[0x00]
        sim._iram[0x00] = 0xAA
        assert sim._rn(0) == 0xAA

    def test_bank1_r0_at_0x08(self):
        sim = I8051Simulator()
        sim._iram[SFR_PSW] = 0x08   # RS0=1 → bank 1
        sim._iram[0x08] = 0xBB
        assert sim._rn(0) == 0xBB

    def test_bank2_r0_at_0x10(self):
        sim = I8051Simulator()
        sim._iram[SFR_PSW] = 0x10   # RS1=1 → bank 2
        sim._iram[0x10] = 0xCC
        assert sim._rn(0) == 0xCC

    def test_bank3_r7_at_0x1f(self):
        sim = I8051Simulator()
        sim._iram[SFR_PSW] = 0x18   # RS1=RS0=1 → bank 3
        sim._iram[0x1F] = 0xDD
        assert sim._rn(7) == 0xDD

    def test_switch_bank_via_mov_psw(self):
        # MOV PSW, #0x08 (bank 1); then use R0
        prog = bytes([
            0x75, SFR_PSW, 0x08,   # MOV PSW, #0x08 → bank 1
            0x78, 0x42,             # MOV R0, #0x42 (bank 1, iram[0x08])
            0xA5,
        ])
        sim = I8051Simulator()
        sim.execute(prog)
        assert sim._iram[0x08] == 0x42   # bank 1 R0


# ── Bit addressing ────────────────────────────────────────────────────────────

class TestBitAddressing:
    def test_ram_bit_0(self):
        """Bit 0x00 → byte 0x20, bit 0."""
        sim = I8051Simulator()
        assert sim._bit_addr(0x00) == (0x20, 0)

    def test_ram_bit_7(self):
        """Bit 0x07 → byte 0x20, bit 7."""
        sim = I8051Simulator()
        assert sim._bit_addr(0x07) == (0x20, 7)

    def test_ram_bit_8(self):
        """Bit 0x08 → byte 0x21, bit 0."""
        sim = I8051Simulator()
        assert sim._bit_addr(0x08) == (0x21, 0)

    def test_ram_bit_127(self):
        """Bit 0x7F → byte 0x2F, bit 7 (last RAM bit)."""
        sim = I8051Simulator()
        assert sim._bit_addr(0x7F) == (0x2F, 7)

    def test_sfr_bit_0x80(self):
        """Bit 0x80 → SFR byte 0x80 (P0), bit 0."""
        sim = I8051Simulator()
        assert sim._bit_addr(0x80) == (0x80, 0)

    def test_sfr_acc_bit_0xe0(self):
        """Bit 0xE0 → SFR 0xE0 (ACC), bit 0."""
        sim = I8051Simulator()
        assert sim._bit_addr(0xE0) == (0xE0, 0)

    def test_write_and_read_bit(self):
        sim = I8051Simulator()
        sim._write_bit(0x05, 1)
        assert sim._read_bit(0x05) == 1
        sim._write_bit(0x05, 0)
        assert sim._read_bit(0x05) == 0

    def test_write_sfr_bit_updates_psw(self):
        """Writing bit 0xD7 (PSW.7 = CY) via bit write."""
        sim = I8051Simulator()
        sim._write_bit(0xD7, 1)
        assert sim._iram[SFR_PSW] & 0x80


# ── Indirect addressing boundaries ───────────────────────────────────────────

class TestIndirectAddressing:
    def test_indirect_at_0x7f_ok(self):
        sim = I8051Simulator()
        sim._iram[0x00] = 0x7F   # R0 = 0x7F (max valid indirect address)
        sim._iram[0x7F] = 0xAB
        assert sim._indirect_read(0) == 0xAB

    def test_indirect_at_0x80_raises(self):
        sim = I8051Simulator()
        sim._iram[0x00] = 0x80   # R0 = 0x80 (invalid on 8051)
        with pytest.raises(ValueError, match="0x80"):
            sim._indirect_read(0)

    def test_indirect_write_at_0x80_raises(self):
        sim = I8051Simulator()
        sim._iram[0x00] = 0x80
        with pytest.raises(ValueError):
            sim._indirect_write(0, 0x42)


# ── Stack behavior ────────────────────────────────────────────────────────────

class TestStack:
    def test_push_increases_sp(self):
        sim = I8051Simulator()
        sim._push8(0xAB)
        assert sim._iram[SFR_SP] == 0x08
        assert sim._iram[0x08] == 0xAB

    def test_pop_decreases_sp(self):
        sim = I8051Simulator()
        sim._push8(0xCD)
        val = sim._pop8()
        assert val == 0xCD
        assert sim._iram[SFR_SP] == 0x07

    def test_push_pc_pop_pc(self):
        sim = I8051Simulator()
        sim._pc = 0x1234
        sim._push_pc()
        sim._pc = 0x0000
        sim._pop_pc()
        assert sim._pc == 0x1234

    def test_sp_wraps_on_overflow(self):
        """SP wraps around 0xFF → 0x00 (bytearray, 8-bit)."""
        sim = I8051Simulator()
        sim._iram[SFR_SP] = 0xFF
        sim._push8(0xAA)
        assert sim._iram[SFR_SP] == 0x00


# ── Parity recomputation ──────────────────────────────────────────────────────

class TestParityRecomputation:
    def test_parity_updated_after_mov_a(self):
        """Parity must be updated whenever ACC changes."""
        sim = I8051Simulator()
        sim.execute(bytes([0x74, 0x01]) + HALT)   # A=1 → 1 bit → P=1
        assert sim._iram[SFR_PSW] & 0x01

    def test_parity_updated_after_anl(self):
        sim = I8051Simulator()
        sim.execute(bytes([0x74, 0x03, 0x54, 0x01]) + HALT)  # A=3; ANL A,#1 → A=1 (P=1)
        assert sim._iram[SFR_PSW] & 0x01

    def test_parity_updated_via_direct_write_to_acc(self):
        """MOV SFR_ACC, A should update parity via _direct_write."""
        sim = I8051Simulator()
        prog = bytes([0x74, 0x03,             # MOV A, #3  (P=0)
                      0xF5, SFR_ACC]) + HALT   # MOV dir(ACC), A  — redundant but valid
        sim.execute(prog)
        assert not (sim._iram[SFR_PSW] & 0x01)  # 0x03 has 2 bits → P=0


# ── Unknown opcode ────────────────────────────────────────────────────────────

class TestUnknownOpcode:
    def test_illegal_indirect_in_execute_returns_error(self):
        """Indirect address ≥ 0x80 should surface as an error in ExecutionResult.

        0x01 (AJMP) IS defined on 8051.  We test the error path by setting R0
        to an invalid indirect address (0x80) and executing MOV A, @R0.
        """
        # MOV R0, #0x80; MOV A, @R0 → illegal indirect addr
        result = I8051Simulator().execute(bytes([0x78, 0x80, 0xE6]) + HALT)
        assert not result.ok
        assert result.error is not None

    def test_illegal_indirect_in_step_raises(self):
        """Step raises ValueError when @Ri points to ≥ 0x80."""
        sim = I8051Simulator()
        sim.load(bytes([0x78, 0x80,   # MOV R0, #0x80
                        0xE6]) + HALT)  # MOV A, @R0
        sim.step()   # MOV R0, #0x80
        with pytest.raises(ValueError, match="0x80"):
            sim.step()   # MOV A, @R0 → should raise


# ── DEC dir and INC @Ri forms ─────────────────────────────────────────────────

class TestDecIncForms:
    def test_dec_at_ri(self):
        # MOV R0, #0x30; MOV @R0, #0x10 (via 0x76 #0x10); DEC @R0
        prog = bytes([0x78, 0x30, 0x76, 0x10, 0x16]) + HALT
        sim = I8051Simulator()
        sim.execute(prog)
        assert sim._iram[0x30] == 0x0F

    def test_inc_at_ri(self):
        prog = bytes([0x78, 0x30, 0x76, 0x10, 0x06]) + HALT
        sim = I8051Simulator()
        sim.execute(prog)
        assert sim._iram[0x30] == 0x11


# ── CPL bit ───────────────────────────────────────────────────────────────────

class TestCplBit:
    def test_cpl_bit_in_sfr_acc(self):
        """CPL bit 0xE0 toggles ACC bit 0 and updates parity."""
        sim = I8051Simulator()
        sim.load(bytes([0x74, 0x00,   # CLR A
                        0xB2, 0xE0]) + HALT)  # CPL bit 0xE0 (ACC.0)
        sim.execute(bytes([0x74, 0x00, 0xB2, 0xE0]) + HALT)
        assert sim._iram[SFR_ACC] & 0x01   # bit 0 should be 1
        assert sim._iram[SFR_PSW] & 0x01   # parity updated (1 set bit → P=1)
