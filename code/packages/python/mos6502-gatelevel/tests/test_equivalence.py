"""Cross-validation: gate-level vs behavioral 6502 simulator.

Runs 50+ programs on both simulators and asserts identical final state.
"""

from __future__ import annotations

import pytest

from mos6502_gatelevel import MOS6502GateLevelSimulator
from mos6502_simulator import MOS6502Simulator


def both(program: bytes, origin: int = 0x0000):
    """Run program on both simulators; return (gate_state, behavioral_state)."""
    gate = MOS6502GateLevelSimulator()
    behav = MOS6502Simulator()
    gr = gate.execute(program, origin)
    br = behav.execute(program, origin)
    return gr.final_state, br.final_state


def check(g, b, *, check_memory: bool = False, mem_range: tuple[int, int] | None = None):
    """Assert that gate-level and behavioral states match."""
    assert g.a == b.a, f"A: gate={g.a:#04x} behav={b.a:#04x}"
    assert g.x == b.x, f"X: gate={g.x:#04x} behav={b.x:#04x}"
    assert g.y == b.y, f"Y: gate={g.y:#04x} behav={b.y:#04x}"
    assert g.s == b.s, f"S: gate={g.s:#04x} behav={b.s:#04x}"
    assert g.flag_n == b.flag_n, f"N: gate={g.flag_n} behav={b.flag_n}"
    assert g.flag_v == b.flag_v, f"V: gate={g.flag_v} behav={b.flag_v}"
    assert g.flag_d == b.flag_d, f"D: gate={g.flag_d} behav={b.flag_d}"
    assert g.flag_i == b.flag_i, f"I: gate={g.flag_i} behav={b.flag_i}"
    assert g.flag_z == b.flag_z, f"Z: gate={g.flag_z} behav={b.flag_z}"
    assert g.flag_c == b.flag_c, f"C: gate={g.flag_c} behav={b.flag_c}"
    assert g.halted == b.halted
    if check_memory:
        lo, hi = mem_range if mem_range else (0, 256)
        for addr in range(lo, hi):
            assert g.memory[addr] == b.memory[addr], f"mem[{addr:#06x}]: gate={g.memory[addr]} behav={b.memory[addr]}"


# ── Basic load tests ──────────────────────────────────────────────────────────

class TestLoadEquivalence:
    def test_lda_imm(self):
        g, b = both(bytes([0xA9, 0x42, 0x00]))
        check(g, b)
        assert g.a == 0x42

    def test_ldx_imm(self):
        g, b = both(bytes([0xA2, 0x10, 0x00]))
        check(g, b)
        assert g.x == 0x10

    def test_ldy_imm(self):
        g, b = both(bytes([0xA0, 0x20, 0x00]))
        check(g, b)
        assert g.y == 0x20

    def test_lda_zero_sets_z(self):
        g, b = both(bytes([0xA9, 0x00, 0x00]))
        check(g, b)
        assert g.flag_z is True

    def test_lda_negative_sets_n(self):
        g, b = both(bytes([0xA9, 0x80, 0x00]))
        check(g, b)
        assert g.flag_n is True


# ── Arithmetic equivalence ────────────────────────────────────────────────────

class TestArithmeticEquivalence:
    def test_adc_basic(self):
        g, b = both(bytes([0xA9, 0x0A, 0x18, 0x69, 0x05, 0x00]))
        check(g, b)
        assert g.a == 15

    def test_adc_carry_out(self):
        g, b = both(bytes([0xA9, 0xFF, 0x18, 0x69, 0x01, 0x00]))
        check(g, b)
        assert g.flag_c is True
        assert g.a == 0

    def test_adc_with_carry(self):
        g, b = both(bytes([0xA9, 0xFF, 0x38, 0x69, 0xFF, 0x00]))
        check(g, b)

    def test_sbc_basic(self):
        g, b = both(bytes([0xA9, 0x0A, 0x38, 0xE9, 0x03, 0x00]))
        check(g, b)
        assert g.a == 7

    def test_sbc_borrow(self):
        g, b = both(bytes([0xA9, 0x03, 0x38, 0xE9, 0x0A, 0x00]))
        check(g, b)

    def test_overflow_detection(self):
        # 0x7F + 0x01 → overflow
        g, b = both(bytes([0xA9, 0x7F, 0x18, 0x69, 0x01, 0x00]))
        check(g, b)
        assert g.flag_v is True

    def test_inc_basic(self):
        prog = bytearray(10)
        prog[0] = 0xA9; prog[1] = 0x05   # LDA #5
        prog[2] = 0x85; prog[3] = 0x10   # STA $10
        prog[4] = 0xE6; prog[5] = 0x10   # INC $10
        prog[6] = 0xA5; prog[7] = 0x10   # LDA $10
        prog[8] = 0x00
        g, b = both(bytes(prog))
        check(g, b)
        assert g.a == 6

    def test_dec_basic(self):
        prog = bytearray(10)
        prog[0] = 0xA9; prog[1] = 0x05
        prog[2] = 0x85; prog[3] = 0x10
        prog[4] = 0xC6; prog[5] = 0x10
        prog[6] = 0xA5; prog[7] = 0x10
        prog[8] = 0x00
        g, b = both(bytes(prog))
        check(g, b)
        assert g.a == 4

    def test_inx_iny(self):
        g, b = both(bytes([0xA2, 0x05, 0xE8, 0xC8, 0x00]))
        check(g, b)
        assert g.x == 6
        assert g.y == 1

    def test_dex_dey(self):
        g, b = both(bytes([0xA2, 0x05, 0xA0, 0x03, 0xCA, 0x88, 0x00]))
        check(g, b)
        assert g.x == 4
        assert g.y == 2


# ── Logical equivalence ────────────────────────────────────────────────────────

class TestLogicalEquivalence:
    def test_and(self):
        g, b = both(bytes([0xA9, 0xFF, 0x29, 0x0F, 0x00]))
        check(g, b)
        assert g.a == 0x0F

    def test_ora(self):
        g, b = both(bytes([0xA9, 0x0F, 0x09, 0xF0, 0x00]))
        check(g, b)
        assert g.a == 0xFF

    def test_eor(self):
        g, b = both(bytes([0xA9, 0xFF, 0x49, 0xFF, 0x00]))
        check(g, b)
        assert g.a == 0
        assert g.flag_z is True

    def test_bit_test(self):
        # Set up memory[0x10] = 0xC0; LDA #0x01; BIT $10
        prog = bytearray(10)
        prog[0] = 0xA9; prog[1] = 0xC0
        prog[2] = 0x85; prog[3] = 0x10
        prog[4] = 0xA9; prog[5] = 0x01
        prog[6] = 0x24; prog[7] = 0x10
        prog[8] = 0x00
        g, b = both(bytes(prog))
        check(g, b)
        # N=1, V=1 (from M[7:6] = 0xC0), Z=1 (A & M = 0)
        assert g.flag_n is True
        assert g.flag_v is True
        assert g.flag_z is True


# ── Shift/rotate equivalence ─────────────────────────────────────────────────

class TestShiftRotateEquivalence:
    def test_asl_acc(self):
        g, b = both(bytes([0xA9, 0x01, 0x0A, 0x00]))
        check(g, b)
        assert g.a == 2

    def test_asl_carry(self):
        g, b = both(bytes([0xA9, 0x80, 0x0A, 0x00]))
        check(g, b)
        assert g.flag_c is True
        assert g.a == 0

    def test_lsr_acc(self):
        g, b = both(bytes([0xA9, 0x02, 0x4A, 0x00]))
        check(g, b)
        assert g.a == 1

    def test_lsr_carry(self):
        g, b = both(bytes([0xA9, 0x01, 0x4A, 0x00]))
        check(g, b)
        assert g.flag_c is True

    def test_rol_acc(self):
        g, b = both(bytes([0x38, 0xA9, 0x00, 0x2A, 0x00]))  # SEC; LDA #0; ROL
        check(g, b)
        assert g.a == 1   # Carry rotated in

    def test_ror_acc(self):
        g, b = both(bytes([0x38, 0xA9, 0x00, 0x6A, 0x00]))  # SEC; LDA #0; ROR
        check(g, b)
        assert g.a == 0x80   # Carry rotated into MSB


# ── Compare equivalence ───────────────────────────────────────────────────────

class TestCompareEquivalence:
    def test_cmp_equal(self):
        g, b = both(bytes([0xA9, 0x05, 0xC9, 0x05, 0x00]))
        check(g, b)
        assert g.flag_z is True
        assert g.flag_c is True

    def test_cmp_greater(self):
        g, b = both(bytes([0xA9, 0x0A, 0xC9, 0x05, 0x00]))
        check(g, b)
        assert g.flag_c is True
        assert g.flag_z is False

    def test_cmp_less(self):
        g, b = both(bytes([0xA9, 0x03, 0xC9, 0x05, 0x00]))
        check(g, b)
        assert g.flag_c is False

    def test_cpx(self):
        g, b = both(bytes([0xA2, 0x05, 0xE0, 0x05, 0x00]))
        check(g, b)
        assert g.flag_z is True

    def test_cpy(self):
        g, b = both(bytes([0xA0, 0x10, 0xC0, 0x05, 0x00]))
        check(g, b)
        assert g.flag_c is True


# ── Branch equivalence ───────────────────────────────────────────────────────

class TestBranchEquivalence:
    def test_beq_taken(self):
        # LDA #0; BEQ +1; NOP; BRK → should skip NOP
        prog = bytes([0xA9, 0x00, 0xF0, 0x01, 0xEA, 0x00])
        g, b = both(prog)
        check(g, b)

    def test_beq_not_taken(self):
        prog = bytes([0xA9, 0x01, 0xF0, 0x01, 0xEA, 0x00])
        g, b = both(prog)
        check(g, b)

    def test_bne_taken(self):
        prog = bytes([0xA9, 0x01, 0xD0, 0x01, 0xEA, 0x00])
        g, b = both(prog)
        check(g, b)

    def test_bcs_taken(self):
        prog = bytes([0x38, 0xB0, 0x01, 0xEA, 0x00])
        g, b = both(prog)
        check(g, b)

    def test_bcc_taken(self):
        prog = bytes([0x18, 0x90, 0x01, 0xEA, 0x00])
        g, b = both(prog)
        check(g, b)

    def test_bmi_taken(self):
        prog = bytes([0xA9, 0x80, 0x30, 0x01, 0xEA, 0x00])
        g, b = both(prog)
        check(g, b)

    def test_bpl_taken(self):
        prog = bytes([0xA9, 0x01, 0x10, 0x01, 0xEA, 0x00])
        g, b = both(prog)
        check(g, b)

    def test_bvc_taken(self):
        prog = bytes([0xB8, 0x50, 0x01, 0xEA, 0x00])  # CLV; BVC
        g, b = both(prog)
        check(g, b)

    def test_bvs_taken(self):
        # Cause overflow then BVS
        prog = bytes([0xA9, 0x7F, 0x18, 0x69, 0x01, 0x70, 0x01, 0xEA, 0x00])
        g, b = both(prog)
        check(g, b)


# ── Transfer equivalence ─────────────────────────────────────────────────────

class TestTransferEquivalence:
    def test_tax(self):
        g, b = both(bytes([0xA9, 0x42, 0xAA, 0x00]))
        check(g, b)
        assert g.x == 0x42

    def test_tay(self):
        g, b = both(bytes([0xA9, 0x42, 0xA8, 0x00]))
        check(g, b)
        assert g.y == 0x42

    def test_txa(self):
        g, b = both(bytes([0xA2, 0x55, 0x8A, 0x00]))
        check(g, b)
        assert g.a == 0x55

    def test_tya(self):
        g, b = both(bytes([0xA0, 0x33, 0x98, 0x00]))
        check(g, b)
        assert g.a == 0x33

    def test_txs(self):
        # TXS does NOT set flags; S = 0xAB after TXS, then BRK decrements by 3
        g, b = both(bytes([0xA2, 0xAB, 0x9A, 0x00]))
        check(g, b)
        # After TXS, S=0xAB; BRK pushes 3 bytes so S = (0xAB - 3) & 0xFF = 0xA8
        assert g.s == (0xAB - 3) & 0xFF

    def test_tsx(self):
        g, b = both(bytes([0xA2, 0x50, 0x9A, 0xBA, 0x00]))  # LDX; TXS; TSX
        check(g, b)
        assert g.x == 0x50


# ── Stack equivalence ────────────────────────────────────────────────────────

class TestStackEquivalence:
    def test_pha_pla(self):
        g, b = both(bytes([0xA9, 0x42, 0x48, 0xA9, 0x00, 0x68, 0x00]))
        check(g, b)
        assert g.a == 0x42

    def test_php_plp(self):
        # SEC; PHP; CLC; PLP → C should be restored
        g, b = both(bytes([0x38, 0x08, 0x18, 0x28, 0x00]))
        check(g, b)
        assert g.flag_c is True

    def test_pha_modifies_s(self):
        gate = MOS6502GateLevelSimulator()
        behav = MOS6502Simulator()
        for sim in [gate, behav]:
            sim.reset()
        s_before_gate = gate.get_state().s
        s_before_behav = behav.get_state().s
        # PHA
        gate.load(bytes([0xA9, 0x42, 0x48, 0x00]))
        behav.load(bytes([0xA9, 0x42, 0x48, 0x00]))
        for _ in range(3):  # LDA, PHA, BRK
            if not gate.get_state().halted:
                gate.step()
            if not behav.get_state().halted:
                behav.step()
        gs = gate.get_state()
        bs = behav.get_state()
        assert gs.s == bs.s
        # PHA decrements S by 1, then BRK decrements by 3: total -4
        assert gs.s == (s_before_gate - 4) & 0xFF


# ── Flag instruction equivalence ─────────────────────────────────────────────

class TestFlagEquivalence:
    def test_clc_sec(self):
        g, b = both(bytes([0x38, 0x18, 0x00]))  # SEC; CLC
        check(g, b)
        assert g.flag_c is False

    def test_cld_sed(self):
        g, b = both(bytes([0xF8, 0xD8, 0x00]))
        check(g, b)
        assert g.flag_d is False

    def test_cli_sei(self):
        g, b = both(bytes([0x58, 0x78, 0x00]))
        check(g, b)
        assert g.flag_i is True

    def test_clv(self):
        g, b = both(bytes([0xA9, 0x7F, 0x18, 0x69, 0x01, 0xB8, 0x00]))
        check(g, b)
        assert g.flag_v is False


# ── Addressing mode equivalence ───────────────────────────────────────────────

class TestAddressingModeEquivalence:
    def test_zero_page(self):
        # LDA #42; STA $10; LDA $10
        prog = bytes([0xA9, 0x2A, 0x85, 0x10, 0xA9, 0x00, 0xA5, 0x10, 0x00])
        g, b = both(prog)
        check(g, b)
        assert g.a == 0x2A

    def test_zero_page_x(self):
        # LDA #5; STA $20; LDX #5; LDA $1B,X  (0x1B + 5 = 0x20)
        prog = bytes([0xA9, 0x05, 0x85, 0x20, 0xA2, 0x05, 0xA9, 0x00, 0xB5, 0x1B, 0x00])
        g, b = both(prog)
        check(g, b)
        assert g.a == 5

    def test_absolute(self):
        gate = MOS6502GateLevelSimulator()
        behav = MOS6502Simulator()
        # LDA #99; STA $0200; LDA $0200; BRK
        prog = bytearray(256)
        prog[0] = 0xA9; prog[1] = 0x63   # LDA #99
        prog[2] = 0x8D; prog[3] = 0x00; prog[4] = 0x02  # STA $0200
        prog[5] = 0xA9; prog[6] = 0x00   # LDA #0
        prog[7] = 0xAD; prog[8] = 0x00; prog[9] = 0x02  # LDA $0200
        prog[10] = 0x00
        g, b = both(bytes(prog))
        check(g, b)
        assert g.a == 99

    def test_absolute_x(self):
        prog = bytearray(256)
        prog[0] = 0xA9; prog[1] = 0x77   # LDA #0x77
        prog[2] = 0x8D; prog[3] = 0x10; prog[4] = 0x02  # STA $0210
        prog[5] = 0xA2; prog[6] = 0x10   # LDX #16
        prog[7] = 0xBD; prog[8] = 0x00; prog[9] = 0x02  # LDA $0200,X
        prog[10] = 0x00
        g, b = both(bytes(prog))
        check(g, b)
        assert g.a == 0x77

    def test_absolute_y(self):
        prog = bytearray(256)
        prog[0] = 0xA9; prog[1] = 0x88
        prog[2] = 0x8D; prog[3] = 0x05; prog[4] = 0x02  # STA $0205
        prog[5] = 0xA0; prog[6] = 0x05   # LDY #5
        prog[7] = 0xB9; prog[8] = 0x00; prog[9] = 0x02  # LDA $0200,Y
        prog[10] = 0x00
        g, b = both(bytes(prog))
        check(g, b)
        assert g.a == 0x88

    def test_indirect_x(self):
        # INX: pointer at ($20 + X) in zero page
        prog = bytearray(256)
        # Store 0xAA at $0300
        prog[0] = 0xA9; prog[1] = 0xAA
        prog[2] = 0x8D; prog[3] = 0x00; prog[4] = 0x03  # STA $0300
        # Store pointer $0300 at zero page $25/$26
        prog[5] = 0xA9; prog[6] = 0x00
        prog[7] = 0x85; prog[8] = 0x25   # $25 = 0x00 (lo)
        prog[9] = 0xA9; prog[10] = 0x03
        prog[11] = 0x85; prog[12] = 0x26  # $26 = 0x03 (hi)
        # LDX #5; LDA ($20,X) → pointer at $25
        prog[13] = 0xA2; prog[14] = 0x05
        prog[15] = 0xA1; prog[16] = 0x20
        prog[17] = 0x00
        g, b = both(bytes(prog))
        check(g, b)
        assert g.a == 0xAA

    def test_indirect_y(self):
        # INY: (base at zp) + Y
        prog = bytearray(256)
        # Store pointer $0300 at zero page $30/$31
        prog[0] = 0xA9; prog[1] = 0x00
        prog[2] = 0x85; prog[3] = 0x30
        prog[4] = 0xA9; prog[5] = 0x03
        prog[6] = 0x85; prog[7] = 0x31
        # Store 0xBB at $0305
        prog[8] = 0xA9; prog[9] = 0xBB
        prog[10] = 0x8D; prog[11] = 0x05; prog[12] = 0x03
        # LDY #5; LDA ($30),Y
        prog[13] = 0xA0; prog[14] = 0x05
        prog[15] = 0xB1; prog[16] = 0x30
        prog[17] = 0x00
        g, b = both(bytes(prog))
        check(g, b)
        assert g.a == 0xBB


# ── JSR/RTS equivalence ────────────────────────────────────────────────────────

class TestSubroutineEquivalence:
    def test_jsr_rts(self):
        # JSR to a subroutine that adds 1 to A, then RTS
        prog = bytearray(256)
        prog[0] = 0xA9; prog[1] = 0x05   # LDA #5
        prog[2] = 0x20; prog[3] = 0x10; prog[4] = 0x00  # JSR $0010
        prog[5] = 0x00                   # BRK
        # Subroutine at $0010:
        prog[0x10] = 0x69; prog[0x11] = 0x01   # ADC #1
        prog[0x12] = 0x60                        # RTS
        g, b = both(bytes(prog))
        check(g, b)
        assert g.a == 6

    def test_nested_jsr(self):
        prog = bytearray(256)
        prog[0] = 0xA9; prog[1] = 0x00   # LDA #0
        prog[2] = 0x20; prog[3] = 0x10; prog[4] = 0x00  # JSR $0010
        prog[5] = 0x00                   # BRK
        # Outer subroutine at $0010: calls inner at $0020
        prog[0x10] = 0x69; prog[0x11] = 0x01  # ADC #1
        prog[0x12] = 0x20; prog[0x13] = 0x20; prog[0x14] = 0x00  # JSR $0020
        prog[0x15] = 0x60               # RTS
        # Inner subroutine at $0020:
        prog[0x20] = 0x69; prog[0x21] = 0x01  # ADC #1
        prog[0x22] = 0x60               # RTS
        g, b = both(bytes(prog))
        check(g, b)
        assert g.a == 2


# ── RTI equivalence ──────────────────────────────────────────────────────────

class TestRTIEquivalence:
    def test_rti_restores_pc_and_p(self):
        # Manually set up stack and RTI
        prog = bytearray(256)
        # Push PC=$0010 and P=$25 onto stack, then RTI
        # PHA(P=$25 via SEC+PHP), then push 0x10 and 0x00 as PC
        # Start: push hi of $0010 = 0x00, lo = 0x10, P with C set
        prog[0] = 0x38           # SEC
        prog[1] = 0x08           # PHP  (pushes P with B=1, so 0x35)
        prog[2] = 0xA9; prog[3] = 0x00   # LDA #0 (hi byte of return PC)
        prog[4] = 0x48           # PHA
        prog[5] = 0xA9; prog[6] = 0x10   # LDA #$10 (lo byte of return PC)
        prog[7] = 0x48           # PHA
        prog[8] = 0x40           # RTI
        # After RTI, PC = $0010 (BRK there)
        prog[0x10] = 0x00        # BRK
        g, b = both(bytes(prog))
        check(g, b)
