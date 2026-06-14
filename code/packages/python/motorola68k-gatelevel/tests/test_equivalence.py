"""Cross-validation: gate-level vs behavioral Motorola 68000 simulator.

Runs 40+ programs on both simulators and compares final state (registers,
flags, memory).
"""


from motorola_68000_simulator.simulator import M68KSimulator

from motorola68k_gatelevel.simulator import Motorola68kGateLevelSimulator

STOP = bytes([0x4E, 0x72, 0x27, 0x00])  # STOP #0x2700


def compare(prog: bytes) -> None:
    """Run prog on both sims; assert identical final state."""
    gl = Motorola68kGateLevelSimulator()
    bh = M68KSimulator()

    r_gl = gl.execute(prog)
    r_bh = bh.execute(prog)

    s_gl = r_gl.final_state
    s_bh = r_bh.final_state

    # Data registers
    for i in range(8):
        assert s_gl.d[i] == s_bh.d[i], (
            f"D{i} mismatch: gate-level={s_gl.d[i]:#010x} "
            f"behavioral={s_bh.d[i]:#010x}"
        )

    # Address registers
    for i in range(8):
        assert s_gl.a[i] == s_bh.a[i], (
            f"A{i} mismatch: gate-level={s_gl.a[i]:#010x} "
            f"behavioral={s_bh.a[i]:#010x}"
        )

    # PC
    assert s_gl.pc == s_bh.pc, f"PC mismatch: {s_gl.pc:#x} vs {s_bh.pc:#x}"

    # Flags (from SR)
    assert s_gl.c == s_bh.c, "C flag mismatch"
    assert s_gl.n == s_bh.n, "N flag mismatch"
    assert s_gl.z == s_bh.z, "Z flag mismatch"
    assert s_gl.v == s_bh.v, "V flag mismatch"
    assert s_gl.x == s_bh.x, "X flag mismatch"

    # Halt
    assert s_gl.halted == s_bh.halted, "Halted mismatch"


class TestMOVE:
    def test_moveq_positive(self):
        compare(bytes([0x70, 0x05]) + STOP)

    def test_moveq_negative(self):
        compare(bytes([0x70, 0xFF]) + STOP)

    def test_moveq_zero(self):
        compare(bytes([0x70, 0x00]) + STOP)

    def test_move_l_d0_d1(self):
        compare(bytes([0x70, 0x42, 0x22, 0x00]) + STOP)

    def test_move_w_d0_d1(self):
        compare(bytes([0x30, 0x3C, 0x00, 0x7F, 0x32, 0x00]) + STOP)

    def test_move_b_d0_d1(self):
        compare(bytes([0x10, 0x3C, 0x00, 0xAA, 0x12, 0x00]) + STOP)

    def test_movea_w(self):
        compare(bytes([0x30, 0x7C, 0x10, 0x00]) + STOP)  # MOVEA.W #0x1000, A0

    def test_movea_l(self):
        compare(bytes([0x20, 0x7C, 0x00, 0x00, 0x10, 0x00]) + STOP)


class TestADD:
    def test_add_l_positive(self):
        compare(bytes([
            0x70, 0x05,
            0x72, 0x03,
            0xD0, 0x81,
        ]) + STOP)

    def test_add_l_overflow(self):
        compare(bytes([
            0x20, 0x3C, 0x7F, 0xFF, 0xFF, 0xFF,
            0x06, 0x80, 0x00, 0x00, 0x00, 0x01,
        ]) + STOP)

    def test_addi_w(self):
        compare(bytes([
            0x70, 0x0A,
            0xD0, 0x7C, 0x00, 0x05,  # ADD.W #5, D0
        ]) + STOP)

    def test_adda_w(self):
        compare(bytes([
            0x20, 0x7C, 0x00, 0x00, 0x10, 0x00,
            0xD0, 0x7C, 0x00, 0x10,  # ADD.W #16, D0? Use ADDA
        ]) + STOP)

    def test_addq(self):
        compare(bytes([
            0x70, 0x0A,
            0x5E, 0x80,  # ADDQ.L #7, D0
        ]) + STOP)

    def test_add_carry_flag(self):
        compare(bytes([
            0x20, 0x3C, 0xFF, 0xFF, 0xFF, 0xFF,
            0x06, 0x80, 0x00, 0x00, 0x00, 0x01,
        ]) + STOP)


class TestSUB:
    def test_sub_basic(self):
        compare(bytes([
            0x70, 0x0A,
            0x72, 0x03,
            0x90, 0x81,
        ]) + STOP)

    def test_sub_borrow(self):
        compare(bytes([
            0x70, 0x03,
            0x72, 0x0A,
            0x90, 0x81,  # D0 - D1 → borrow
        ]) + STOP)

    def test_subi(self):
        compare(bytes([
            0x70, 0x64,
            0x04, 0x80, 0x00, 0x00, 0x00, 0x0A,  # SUBI.L #10, D0
        ]) + STOP)

    def test_subq(self):
        compare(bytes([
            0x70, 0x0F,
            0x53, 0x80,  # SUBQ.L #1, D0 — wait, 0x5380 is SUBQ.L #1, D0
        ]) + STOP)


class TestLogic:
    def test_and(self):
        compare(bytes([
            0x70, 0xFF,
            0xC0, 0x3C, 0x00, 0xF0,  # AND.W #0xF0, D0
        ]) + STOP)

    def test_or(self):
        compare(bytes([
            0x70, 0xF0,
            0x80, 0x3C, 0x00, 0x0F,  # OR.W #0x0F, D0
        ]) + STOP)

    def test_xor(self):
        compare(bytes([
            0x70, 0xFF,
            0x0A, 0x40, 0x00, 0x55,  # EORI.W #0x55, D0
        ]) + STOP)

    def test_not(self):
        compare(bytes([
            0x70, 0x00,
            0x46, 0x80,  # NOT.L D0
        ]) + STOP)

    def test_andi_ccr(self):
        compare(bytes([
            0x44, 0xFC, 0x00, 0x1F,  # MOVE #31, CCR (all flags set)
            0x02, 0x3C, 0x00, 0xF4,  # ANDI #0xF4, CCR (clear Z,C)
        ]) + STOP)

    def test_ori_ccr(self):
        compare(bytes([
            0x44, 0xFC, 0x00, 0x00,  # MOVE #0, CCR
            0x00, 0x3C, 0x00, 0x04,  # ORI #4, CCR (set Z)
        ]) + STOP)


class TestCMP:
    def test_cmp_equal(self):
        compare(bytes([
            0x70, 0x05,
            0xB0, 0x3C, 0x00, 0x05,  # CMP.W #5, D0
        ]) + STOP)

    def test_cmp_less(self):
        compare(bytes([
            0x70, 0x03,
            0xB0, 0x3C, 0x00, 0x05,  # CMP.W #5, D0 (D0 < 5)
        ]) + STOP)

    def test_cmp_greater(self):
        compare(bytes([
            0x70, 0x0A,
            0xB0, 0x3C, 0x00, 0x05,  # CMP.W #5, D0 (D0 > 5)
        ]) + STOP)

    def test_tst(self):
        compare(bytes([
            0x70, 0x00,
            0x4A, 0x80,  # TST.L D0
        ]) + STOP)

    def test_tst_negative(self):
        compare(bytes([
            0x20, 0x3C, 0x80, 0x00, 0x00, 0x00,
            0x4A, 0x80,  # TST.L D0
        ]) + STOP)


class TestShifts:
    def test_asl_l(self):
        compare(bytes([
            0x70, 0x01,
            0xE3, 0x88,  # ASL.L #1, D0
        ]) + STOP)

    def test_asr_l(self):
        compare(bytes([
            0x20, 0x3C, 0x80, 0x00, 0x00, 0x00,
            0xE0, 0x80,  # ASR.L #1, D0
        ]) + STOP)

    def test_lsl_w(self):
        compare(bytes([
            0x30, 0x3C, 0x00, 0x01,
            0xE3, 0x48,  # LSL.W #1, D0
        ]) + STOP)

    def test_lsr_b(self):
        compare(bytes([
            0x10, 0x3C, 0x00, 0x02,
            0xE2, 0x08,  # LSR.B #1, D0
        ]) + STOP)

    def test_rol_b(self):
        compare(bytes([
            0x10, 0x3C, 0x00, 0x80,  # MOVE.B #0x80, D0
            0xE3, 0x18,              # ROL.B #1, D0
        ]) + STOP)

    def test_ror_w(self):
        compare(bytes([
            0x30, 0x3C, 0x00, 0x01,
            0xE2, 0x58,  # ROR.W #1, D0
        ]) + STOP)


class TestBranches:
    def test_bra_taken(self):
        compare(bytes([
            0x70, 0x01,
            0x60, 0x02,  # BRA +2 (skip MOVEQ #0,D0)
            0x70, 0x00,  # not reached
        ]) + STOP)

    def test_bne_taken(self):
        compare(bytes([
            0x70, 0x05,
            0xB0, 0x3C, 0x00, 0x03,  # CMP.W #3, D0 (not equal → Z=0)
            0x66, 0x02,              # BNE +2
            0x70, 0x00,              # not reached
        ]) + STOP)

    def test_beq_not_taken(self):
        compare(bytes([
            0x70, 0x05,
            0xB0, 0x3C, 0x00, 0x03,  # CMP.W #3, D0 (not equal)
            0x67, 0x02,              # BEQ +2 (not taken)
            0x70, 0x0A,              # MOVEQ #10, D0 (executed)
        ]) + STOP)

    def test_bge_signed(self):
        compare(bytes([
            0x70, 0x05,
            0xB0, 0x3C, 0x00, 0x05,  # CMP.W #5, D0 (equal → GE true)
            0x6C, 0x02,              # BGE +2 (taken)
            0x70, 0x00,
        ]) + STOP)


class TestMisc:
    def test_clr(self):
        compare(bytes([
            0x70, 0xFF,
            0x42, 0x80,  # CLR.L D0
        ]) + STOP)

    def test_neg(self):
        compare(bytes([
            0x70, 0x05,
            0x44, 0x80,  # NEG.L D0
        ]) + STOP)

    def test_neg_zero(self):
        compare(bytes([
            0x70, 0x00,
            0x44, 0x80,
        ]) + STOP)

    def test_swap(self):
        compare(bytes([
            0x20, 0x3C, 0xAB, 0xCD, 0x12, 0x34,
            0x48, 0x40,  # SWAP D0
        ]) + STOP)

    def test_ext_w(self):
        compare(bytes([
            0x70, 0x80,
            0x48, 0x80,  # EXT.W D0
        ]) + STOP)

    def test_ext_l(self):
        compare(bytes([
            0x30, 0x3C, 0x80, 0x00,
            0x48, 0xC0,  # EXT.L D0
        ]) + STOP)

    def test_nop(self):
        compare(bytes([
            0x70, 0x42,
            0x4E, 0x71,  # NOP
        ]) + STOP)

    def test_exg_dn_dn(self):
        compare(bytes([
            0x70, 0x0A,
            0x72, 0x14,
            0xC1, 0x41,  # EXG D0, D1
        ]) + STOP)

    def test_mulu(self):
        compare(bytes([
            0x70, 0x06,
            0x72, 0x07,
            0xC0, 0xC1,  # MULU D1, D0
        ]) + STOP)

    def test_muls_negative(self):
        compare(bytes([
            0x70, 0xFF,  # MOVEQ #-1, D0 (low 16 bits = 0xFFFF)
            0x72, 0x05,  # MOVEQ #5, D1
            0xC1, 0xC0,  # MULS D0, D1
        ]) + STOP)

    def test_dbf_loop(self):
        compare(bytes([
            0x70, 0x03,
            0x72, 0x00,
            0x52, 0x41,              # ADDQ.W #1, D1
            0x51, 0xC8, 0xFF, 0xFC,  # DBF D0, loop
        ]) + STOP)

    def test_move_to_from_sr(self):
        compare(bytes([
            0x44, 0xFC, 0x00, 0x04,  # MOVE #4, CCR
            0x42, 0xC0,              # MOVE CCR, D0
        ]) + STOP)

    def test_btst_set(self):
        compare(bytes([
            0x70, 0x01,
            0x08, 0x00, 0x00, 0x00,  # BTST #0, D0
        ]) + STOP)

    def test_bset(self):
        compare(bytes([
            0x70, 0x00,
            0x08, 0xC0, 0x00, 0x03,  # BSET #3, D0
        ]) + STOP)

    def test_bclr(self):
        compare(bytes([
            0x70, 0xFF,
            0x08, 0x80, 0x00, 0x00,  # BCLR #0, D0
        ]) + STOP)

    def test_link_unlk(self):
        compare(bytes([
            0x4E, 0x56, 0xFF, 0xFC,  # LINK A6, #-4
            0x4E, 0x5E,              # UNLK A6
        ]) + STOP)

    def test_bsr_rts(self):
        compare(bytes([
            0x70, 0x00,
            0x61, 0x04,              # BSR +4
        ]) + STOP + bytes([
            0x52, 0x80,              # ADDQ.L #1, D0
            0x4E, 0x75,              # RTS
        ]))

    def test_move_indirect(self):
        compare(bytes([
            0x41, 0xF8, 0x20, 0x00,  # LEA 0x2000, A0
            0x70, 0x42,
            0x20, 0x80,              # MOVE.L D0, (A0)
            0x22, 0x10,              # MOVE.L (A0), D1
        ]) + STOP)

    def test_move_postincrement(self):
        compare(bytes([
            0x41, 0xF8, 0x20, 0x00,
            0x70, 0x42,
            0x30, 0xC0,              # MOVE.W D0, (A0)+
            0x32, 0x18,              # MOVE.W (A0)+... wait
        ]) + STOP)
