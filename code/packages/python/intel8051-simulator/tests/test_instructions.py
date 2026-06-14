"""Per-instruction tests for the Intel 8051 simulator.

Encoding constants (from spec 07p):
  MOV A, #imm       0x74 imm
  MOV A, dir        0xE5 dir
  MOV A, @Ri        0xE6+i
  MOV A, Rn         0xE8+n
  MOV Rn, A         0xF8+n
  MOV Rn, #imm      0x78+n imm
  MOV dir, A        0xF5 dir
  MOV dir, Rn       0x88+n dir
  MOV dir, dir2     0x85 src dst
  MOV dir, @Ri      0x86+i dir
  MOV dir, #imm     0x75 dir imm
  MOV @Ri, A        0xF6+i
  MOV @Ri, dir      0xA6+i dir
  MOV @Ri, #imm     0x76+i imm
  MOV DPTR, #imm16  0x90 hi lo
  MOVC A, @A+DPTR   0x93
  MOVC A, @A+PC     0x83
  MOVX A, @Ri       0xE2+i
  MOVX A, @DPTR     0xE0
  MOVX @Ri, A       0xF2+i
  MOVX @DPTR, A     0xF0
  PUSH dir          0xC0 dir
  POP dir           0xD0 dir
  XCH A, Rn         0xC8+n
  XCH A, dir        0xC5 dir
  XCH A, @Ri        0xC6+i
  XCHD A, @Ri       0xD6+i
  ADD A, Rn         0x28+n
  ADD A, dir        0x25 dir
  ADD A, @Ri        0x26+i
  ADD A, #imm       0x24 imm
  ADDC A, #imm      0x34 imm
  SUBB A, #imm      0x94 imm
  INC A             0x04
  INC Rn            0x08+n
  INC dir           0x05 dir
  INC DPTR          0xA3
  DEC A             0x14
  DEC Rn            0x18+n
  MUL AB            0xA4
  DIV AB            0x84
  DA A              0xD4
  ANL A, #imm       0x54 imm
  ORL A, #imm       0x44 imm
  XRL A, #imm       0x64 imm
  CLR A             0xE4
  CPL A             0xF4
  RL A              0x23
  RLC A             0x33
  RR A              0x03
  RRC A             0x13
  SWAP A            0xC4
  CLR C             0xC3
  CLR bit           0xC2 bit
  SETB C            0xD3
  SETB bit          0xD2 bit
  CPL C             0xB3
  ANL C, bit        0x82 bit
  ORL C, bit        0x72 bit
  MOV C, bit        0xA2 bit
  MOV bit, C        0x92 bit
  LJMP addr16       0x02 hi lo
  SJMP rel          0x80 rel
  JMP @A+DPTR       0x73
  JZ rel            0x60 rel
  JNZ rel           0x70 rel
  JC rel            0x40 rel
  JNC rel           0x50 rel
  JB bit, rel       0x20 bit rel
  JNB bit, rel      0x30 bit rel
  JBC bit, rel      0x10 bit rel
  CJNE A, #imm, rel 0xB4 imm rel
  CJNE Rn, #imm,rel 0xB8+n imm rel
  DJNZ Rn, rel      0xD8+n rel
  DJNZ dir, rel     0xD5 dir rel
  LCALL addr16      0x12 hi lo
  RET               0x22
  NOP               0x00
  HALT              0xA5  (sentinel)
"""

from __future__ import annotations

from intel8051_simulator import I8051Simulator
from intel8051_simulator.state import SFR_ACC, SFR_B, SFR_DPH, SFR_DPL, SFR_PSW, SFR_SP

HALT = bytes([0xA5])


def run(prog: bytes) -> I8051Simulator:
    """Helper: execute program and return simulator in final state."""
    sim = I8051Simulator()
    sim.execute(prog)
    return sim


# ── Data transfer ─────────────────────────────────────────────────────────────

class TestMOV:
    def test_mov_a_imm(self):
        sim = run(bytes([0x74, 0x42]) + HALT)   # MOV A, #0x42
        assert sim._iram[SFR_ACC] == 0x42

    def test_mov_a_r0(self):
        # MOV R0, #5; MOV A, R0
        prog = bytes([0x78, 0x05, 0xE8]) + HALT
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 0x05

    def test_mov_a_r3(self):
        prog = bytes([0x7B, 0x7F, 0xEB]) + HALT  # MOV R3, #0x7F; MOV A, R3
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 0x7F

    def test_mov_a_dir(self):
        sim = I8051Simulator()
        sim.load(bytes([0xE5, 0x30]) + HALT)   # MOV A, 0x30
        sim._iram[0x30] = 0xAB
        sim.step(); sim.step(); sim.step()
        assert sim._iram[SFR_ACC] == 0xAB

    def test_mov_a_at_r0(self):
        sim = I8051Simulator()
        # Set R0=0x30, set iram[0x30]=0x55, then MOV A, @R0
        sim.load(bytes([0x78, 0x30,   # MOV R0, #0x30
                        0xE6]) + HALT)  # MOV A, @R0
        sim._iram[0x30] = 0x55
        for _ in range(3): sim.step()
        assert sim._iram[SFR_ACC] == 0x55

    def test_mov_rn_a(self):
        # MOV A, #0xAB; MOV R5, A
        prog = bytes([0x74, 0xAB, 0xFD]) + HALT
        sim = run(prog)
        assert sim._rn(5) == 0xAB

    def test_mov_rn_imm(self):
        prog = bytes([0x7C, 0x99]) + HALT   # MOV R4, #0x99
        sim = run(prog)
        assert sim._rn(4) == 0x99

    def test_mov_rn_dir(self):
        sim = I8051Simulator()
        sim.load(bytes([0xAC, 0x30]) + HALT)   # MOV R4, 0x30
        sim._iram[0x30] = 0x12
        for _ in range(2): sim.step()
        assert sim._rn(4) == 0x12

    def test_mov_dir_a(self):
        # MOV A, #0xBC; MOV 0x40, A
        prog = bytes([0x74, 0xBC, 0xF5, 0x40]) + HALT
        sim = run(prog)
        assert sim._iram[0x40] == 0xBC

    def test_mov_dir_rn(self):
        # MOV R2, #0xDE; MOV 0x50, R2
        prog = bytes([0x7A, 0xDE, 0x8A, 0x50]) + HALT
        sim = run(prog)
        assert sim._iram[0x50] == 0xDE

    def test_mov_dir_dir(self):
        # MOV 0x50, #0x11; MOV 0x51, 0x50   (src=0x50, dst=0x51)
        prog = bytes([0x75, 0x50, 0x11,    # MOV 0x50, #0x11
                      0x85, 0x50, 0x51]) + HALT
        sim = run(prog)
        assert sim._iram[0x51] == 0x11

    def test_mov_dir_imm(self):
        prog = bytes([0x75, 0x35, 0xCC]) + HALT   # MOV 0x35, #0xCC
        sim = run(prog)
        assert sim._iram[0x35] == 0xCC

    def test_mov_at_ri_a(self):
        # Set R0=0x30; MOV A, #0xAA; MOV @R0, A
        prog = bytes([0x78, 0x30, 0x74, 0xAA, 0xF6]) + HALT
        sim = run(prog)
        assert sim._iram[0x30] == 0xAA

    def test_mov_at_ri_imm(self):
        # MOV R0, #0x32; MOV @R0, #0x77
        prog = bytes([0x78, 0x32, 0x76, 0x77]) + HALT
        sim = run(prog)
        assert sim._iram[0x32] == 0x77

    def test_mov_dptr_imm16(self):
        prog = bytes([0x90, 0x12, 0x34]) + HALT   # MOV DPTR, #0x1234
        sim = run(prog)
        assert sim._iram[SFR_DPH] == 0x12
        assert sim._iram[SFR_DPL] == 0x34


class TestMOVX:
    def test_movx_a_at_dptr(self):
        sim = I8051Simulator()
        sim.load(bytes([0x90, 0x10, 0x00,   # MOV DPTR, #0x1000
                        0xE0]) + HALT)        # MOVX A, @DPTR
        sim._xdata[0x1000] = 0x9F
        for _ in range(3): sim.step()
        assert sim._iram[SFR_ACC] == 0x9F

    def test_movx_at_dptr_a(self):
        prog = bytes([0x74, 0x55,             # MOV A, #0x55
                      0x90, 0x20, 0x00,       # MOV DPTR, #0x2000
                      0xF0]) + HALT           # MOVX @DPTR, A
        sim = run(prog)
        assert sim._xdata[0x2000] == 0x55

    def test_movx_a_at_ri(self):
        sim = I8051Simulator()
        sim.load(bytes([0x78, 0x08,    # MOV R0, #0x08
                        0xE2]) + HALT)  # MOVX A, @R0
        sim._xdata[0x08] = 0xBB
        for _ in range(3): sim.step()
        assert sim._iram[SFR_ACC] == 0xBB

    def test_movx_at_ri_a(self):
        prog = bytes([0x74, 0xCC,      # MOV A, #0xCC
                      0x78, 0x10,      # MOV R0, #0x10
                      0xF2]) + HALT    # MOVX @R0, A
        sim = run(prog)
        assert sim._xdata[0x10] == 0xCC


class TestMOVC:
    def test_movc_a_at_a_plus_dptr(self):
        # MOV A, #2; MOV DPTR, #0x100; MOVC A, @A+DPTR  → code[0x102]
        prog = bytes([0x74, 0x02,          # MOV A, #2
                      0x90, 0x01, 0x00,    # MOV DPTR, #0x100
                      0x93]) + HALT        # MOVC A, @A+DPTR
        sim = I8051Simulator()
        sim.load(prog)
        sim._code[0x0102] = 0x77
        for _ in range(4): sim.step()
        assert sim._iram[SFR_ACC] == 0x77

    def test_movc_a_at_a_plus_pc(self):
        # MOVC A, @A+PC reads code[PC_after_0x83 + A]
        # Encode: MOV A, #1; MOVC A, @A+PC → PC after 0x83 = 4; reads code[5]
        prog = bytearray([0x74, 0x01,   # 0x00: MOV A, #1  (2 bytes)
                          0x83,          # 0x02: MOVC A, @A+PC; PC becomes 0x03; EA=0x03+1=0x04
                          0xA5,          # 0x03: HALT (will halt)
                          0xFF])         # 0x04: lookup value
        # Wait: after MOVC fetch, PC=0x03; A=1; EA=0x03+1=0x04 → code[4]=0xFF
        prog[4] = 0xEE
        sim = I8051Simulator()
        sim.load(bytes(prog))
        sim.step()   # MOV A, #1
        sim.step()   # MOVC A, @A+PC
        assert sim._iram[SFR_ACC] == 0xEE


class TestPushPop:
    def test_push_pop_roundtrip(self):
        # MOV A, #0xAB; MOV 0x30, A; PUSH 0x30; POP 0x31
        prog = bytes([0x74, 0xAB,   # MOV A, #0xAB
                      0xF5, 0x30,   # MOV 0x30, A
                      0xC0, 0x30,   # PUSH 0x30
                      0xD0, 0x31]) + HALT
        sim = run(prog)
        assert sim._iram[0x31] == 0xAB
        assert sim._iram[SFR_SP] == 0x07   # SP returned to initial value

    def test_push_increments_sp(self):
        prog = bytes([0x75, 0x30, 0x42,   # MOV 0x30, #0x42
                      0xC0, 0x30]) + HALT  # PUSH 0x30
        sim = run(prog)
        assert sim._iram[SFR_SP] == 0x08   # 0x07 → 0x08


class TestXCH:
    def test_xch_a_rn(self):
        # MOV A, #0x11; MOV R2, #0x22; XCH A, R2
        prog = bytes([0x74, 0x11, 0x7A, 0x22, 0xCA]) + HALT
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 0x22
        assert sim._rn(2) == 0x11

    def test_xch_a_dir(self):
        # MOV A,#0xAA; MOV 0x30,#0xBB; XCH A,0x30 → A=0xBB, iram[0x30]=0xAA
        prog = bytes([0x74, 0xAA, 0x75, 0x30, 0xBB, 0xC5, 0x30]) + HALT
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 0xBB
        assert sim._iram[0x30] == 0xAA

    def test_xchd_nibble_swap(self):
        # MOV A, #0xAF; MOV R0, #0x30; MOV @R0, #0x5B; XCHD A, @R0
        # A.lo=F ↔ [0x30].lo=B → A=0xAB, [0x30]=0x5F
        prog = bytes([0x74, 0xAF,    # MOV A, #0xAF
                      0x78, 0x30,    # MOV R0, #0x30
                      0x76, 0x5B,    # MOV @R0, #0x5B
                      0xD6]) + HALT  # XCHD A, @R0
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 0xAB
        assert sim._iram[0x30] == 0x5F


# ── Arithmetic ────────────────────────────────────────────────────────────────

class TestADD:
    def test_add_a_rn_no_carry(self):
        prog = bytes([0x74, 0x10, 0x78, 0x20, 0x28]) + HALT  # A=0x10; R0=0x20; ADD A,R0
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 0x30
        assert not (sim._iram[SFR_PSW] & 0x80)   # CY=0

    def test_add_a_imm_carry(self):
        prog = bytes([0x74, 0xFF, 0x24, 0x01]) + HALT  # A=0xFF; ADD A,#1
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 0x00
        assert sim._iram[SFR_PSW] & 0x80   # CY=1

    def test_add_a_dir(self):
        # MOV 0x30,#5; MOV A,#0x10; ADD A,0x30 → A=0x15
        prog = bytes([0x75, 0x30, 0x05, 0x74, 0x10, 0x25, 0x30]) + HALT
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 0x15

    def test_addc_uses_carry(self):
        # A=0x01, CY=1; ADDC A, #0x01 → A=3
        sim = I8051Simulator()
        sim.load(bytes([0x74, 0x01, 0x34, 0x01]) + HALT)
        sim._iram[SFR_PSW] |= 0x80  # set CY before load (load resets — must set after)
        sim.load(bytes([0x74, 0x01, 0x34, 0x01]) + HALT)
        sim._iram[SFR_PSW] |= 0x80   # set CY after load
        sim.step(); sim.step(); sim.step()
        assert sim._iram[SFR_ACC] == 3

    def test_add_auxiliary_carry(self):
        # 0x08 + 0x08 = 0x10; AC = carry from bit 3 to 4 → 1
        prog = bytes([0x74, 0x08, 0x24, 0x08]) + HALT
        sim = run(prog)
        assert sim._iram[SFR_PSW] & 0x40   # AC=1

    def test_add_overflow(self):
        # 0x7F + 0x01 = 0x80; signed overflow (positive + positive = negative)
        prog = bytes([0x74, 0x7F, 0x24, 0x01]) + HALT
        sim = run(prog)
        assert sim._iram[SFR_PSW] & 0x04   # OV=1


class TestSUBB:
    def test_subb_basic(self):
        prog = bytes([0x74, 0x10, 0x94, 0x05]) + HALT  # A=0x10; SUBB A,#5 (CY=0)
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 0x0B
        assert not (sim._iram[SFR_PSW] & 0x80)   # no borrow

    def test_subb_borrow(self):
        prog = bytes([0x74, 0x00, 0x94, 0x01]) + HALT  # A=0; SUBB A,#1 → borrow
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 0xFF
        assert sim._iram[SFR_PSW] & 0x80   # CY=1 (borrow)

    def test_subb_uses_carry(self):
        sim = I8051Simulator()
        sim.load(bytes([0x74, 0x05, 0x94, 0x02]) + HALT)  # A=5; SUBB A,#2
        sim._iram[SFR_PSW] |= 0x80   # CY=1 (borrow in)
        sim.step(); sim.step(); sim.step()
        assert sim._iram[SFR_ACC] == 2  # 5 - 2 - 1 = 2


class TestINCDEC:
    def test_inc_a_no_flags(self):
        prog = bytes([0x74, 0x05, 0x04]) + HALT  # MOV A,#5; INC A
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 6
        assert sim._iram[SFR_PSW] == 0  # no flags changed

    def test_inc_a_wraps(self):
        prog = bytes([0x74, 0xFF, 0x04]) + HALT  # MOV A,#0xFF; INC A
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 0x00
        assert not (sim._iram[SFR_PSW] & 0x80)  # no CY set by INC

    def test_inc_rn(self):
        prog = bytes([0x7A, 0x09, 0x0A]) + HALT  # MOV R2,#9; INC R2
        sim = run(prog)
        assert sim._rn(2) == 10

    def test_dec_a(self):
        prog = bytes([0x74, 0x05, 0x14]) + HALT
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 4

    def test_dec_rn(self):
        prog = bytes([0x79, 0x03, 0x19]) + HALT  # MOV R1,#3; DEC R1
        sim = run(prog)
        assert sim._rn(1) == 2

    def test_inc_dptr(self):
        prog = bytes([0x90, 0x00, 0xFF, 0xA3]) + HALT  # MOV DPTR,#0xFF; INC DPTR
        sim = run(prog)
        assert sim._iram[SFR_DPH] == 0x01
        assert sim._iram[SFR_DPL] == 0x00

    def test_inc_dir(self):
        prog = bytes([0x75, 0x30, 0x0A, 0x05, 0x30]) + HALT  # MOV 0x30,#10; INC 0x30
        sim = run(prog)
        assert sim._iram[0x30] == 11

    def test_dec_dir(self):
        prog = bytes([0x75, 0x30, 0x05, 0x15, 0x30]) + HALT
        sim = run(prog)
        assert sim._iram[0x30] == 4


class TestMulDiv:
    def test_mul_ab(self):
        # A=0x12, B=0x10; MUL AB → product=0x0120; A=0x20, B=0x01
        prog = bytes([0x74, 0x12,    # MOV A, #0x12
                      0x75, SFR_B, 0x10,  # MOV B, #0x10
                      0xA4]) + HALT
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 0x20
        assert sim._iram[SFR_B]   == 0x01
        assert sim._iram[SFR_PSW] & 0x04   # OV=1 (B≠0 after, product>0xFF)
        assert not (sim._iram[SFR_PSW] & 0x80)  # CY=0 always

    def test_mul_small(self):
        # A=2, B=3; product=6; A=6, B=0; OV=0
        prog = bytes([0x74, 0x02,
                      0x75, SFR_B, 0x03,
                      0xA4]) + HALT
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 6
        assert sim._iram[SFR_B]   == 0
        assert not (sim._iram[SFR_PSW] & 0x04)   # OV=0

    def test_div_ab(self):
        # A=17, B=5; quotient=3, remainder=2
        prog = bytes([0x74, 17,
                      0x75, SFR_B, 5,
                      0x84]) + HALT
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 3
        assert sim._iram[SFR_B]   == 2
        assert not (sim._iram[SFR_PSW] & 0x04)   # OV=0
        assert not (sim._iram[SFR_PSW] & 0x80)   # CY=0

    def test_div_by_zero(self):
        # B=0; OV should be set
        prog = bytes([0x74, 0x10,
                      0x75, SFR_B, 0x00,
                      0x84]) + HALT
        sim = run(prog)
        assert sim._iram[SFR_PSW] & 0x04   # OV=1
        assert not (sim._iram[SFR_PSW] & 0x80)   # CY=0


class TestDA:
    def test_da_corrects_bcd(self):
        # 0x28 + 0x47 = 0x6F; DA → adds 6 to low nibble → 0x75 (BCD 75)
        prog = bytes([0x74, 0x28, 0x24, 0x47, 0xD4]) + HALT
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 0x75

    def test_da_with_carry(self):
        # 0x58 + 0x46 = 0x9E; DA → adds 0x66 → 0x04, CY=1 (BCD 104)
        prog = bytes([0x74, 0x58, 0x24, 0x46, 0xD4]) + HALT
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 0x04
        assert sim._iram[SFR_PSW] & 0x80   # CY=1


# ── Logic ─────────────────────────────────────────────────────────────────────

class TestLogic:
    def test_anl_a_imm(self):
        prog = bytes([0x74, 0xFF, 0x54, 0x0F]) + HALT  # A=0xFF; ANL A,#0x0F
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 0x0F

    def test_orl_a_imm(self):
        prog = bytes([0x74, 0x00, 0x44, 0xF0]) + HALT  # A=0; ORL A,#0xF0
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 0xF0

    def test_xrl_a_imm(self):
        prog = bytes([0x74, 0xFF, 0x64, 0x0F]) + HALT  # A=0xFF; XRL A,#0x0F
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 0xF0

    def test_clr_a(self):
        prog = bytes([0x74, 0xAB, 0xE4]) + HALT
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 0

    def test_cpl_a(self):
        prog = bytes([0x74, 0x0F, 0xF4]) + HALT
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 0xF0

    def test_anl_dir_a(self):
        # MOV 0x30,#0xFF; MOV A,#0x0F; ANL 0x30,A → 0x30=0x0F
        prog = bytes([0x75, 0x30, 0xFF, 0x74, 0x0F, 0x52, 0x30]) + HALT
        sim = run(prog)
        assert sim._iram[0x30] == 0x0F

    def test_orl_dir_imm(self):
        prog = bytes([0x75, 0x30, 0x0F, 0x43, 0x30, 0xF0]) + HALT
        sim = run(prog)
        assert sim._iram[0x30] == 0xFF

    def test_xrl_dir_a(self):
        # MOV 0x30,#0xFF; MOV A,#0xAA; XRL 0x30,A → 0x30=0x55
        prog = bytes([0x75, 0x30, 0xFF, 0x74, 0xAA, 0x62, 0x30]) + HALT
        sim = run(prog)
        assert sim._iram[0x30] == 0x55


class TestRotate:
    def test_rl_a(self):
        prog = bytes([0x74, 0x81, 0x23]) + HALT  # A=0x81; RL A → 0x03
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 0x03

    def test_rr_a(self):
        prog = bytes([0x74, 0x81, 0x03]) + HALT  # A=0x81; RR A → 0xC0
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 0xC0

    def test_rlc_a_no_carry(self):
        # A=0x40, CY=0; RLC → A=0x80, CY=0
        prog = bytes([0xC3, 0x74, 0x40, 0x33]) + HALT  # CLR C; MOV A,#0x40; RLC A
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 0x80
        assert not (sim._iram[SFR_PSW] & 0x80)

    def test_rlc_a_with_carry(self):
        # A=0x80, CY=0; RLC → A=0x00, CY=1
        prog = bytes([0xC3, 0x74, 0x80, 0x33]) + HALT
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 0x00
        assert sim._iram[SFR_PSW] & 0x80   # CY=1

    def test_rrc_a(self):
        # A=0x01, CY=0; RRC → A=0x00, CY=1
        prog = bytes([0xC3, 0x74, 0x01, 0x13]) + HALT
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 0x00
        assert sim._iram[SFR_PSW] & 0x80   # CY=1

    def test_swap_a(self):
        prog = bytes([0x74, 0xAB, 0xC4]) + HALT  # A=0xAB; SWAP → 0xBA
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 0xBA


# ── Bit manipulation ──────────────────────────────────────────────────────────

class TestBit:
    def test_setb_c(self):
        prog = bytes([0xD3]) + HALT
        sim = run(prog)
        assert sim._iram[SFR_PSW] & 0x80

    def test_clr_c(self):
        prog = bytes([0xD3, 0xC3]) + HALT  # SETB C; CLR C
        sim = run(prog)
        assert not (sim._iram[SFR_PSW] & 0x80)

    def test_cpl_c(self):
        prog = bytes([0xD3, 0xB3]) + HALT  # SETB C; CPL C
        sim = run(prog)
        assert not (sim._iram[SFR_PSW] & 0x80)

    def test_setb_bit_in_ram(self):
        # Bit 0x00 → byte 0x20, bit 0; SETB 0x00
        prog = bytes([0xD2, 0x00]) + HALT
        sim = run(prog)
        assert sim._iram[0x20] & 0x01

    def test_clr_bit_in_ram(self):
        sim = I8051Simulator()
        sim.load(bytes([0xC2, 0x00]) + HALT)
        sim._iram[0x20] = 0xFF
        sim.execute(bytes([0xC2, 0x00]) + HALT)
        assert not (sim._iram[0x20] & 0x01)

    def test_cpl_bit(self):
        prog = bytes([0xD2, 0x04, 0xB2, 0x04]) + HALT  # SETB bit4; CPL bit4
        sim = run(prog)
        # bit4 = byte 0x20, bit4 position → bit 4 of byte 0x20
        assert not (sim._iram[0x20] & (1 << 4))

    def test_mov_c_bit(self):
        # SETB bit3; MOV C, bit3 → CY=1
        prog = bytes([0xD2, 0x03, 0xA2, 0x03]) + HALT
        sim = run(prog)
        assert sim._iram[SFR_PSW] & 0x80

    def test_mov_bit_c(self):
        # SETB C; MOV bit5, C
        prog = bytes([0xD3, 0x92, 0x05]) + HALT
        sim = run(prog)
        assert sim._iram[0x20] & (1 << 5)

    def test_anl_c_bit_both_set(self):
        # SETB C; SETB bit0; ANL C,bit0 → CY stays 1
        prog = bytes([0xD3, 0xD2, 0x00, 0x82, 0x00]) + HALT
        sim = run(prog)
        assert sim._iram[SFR_PSW] & 0x80

    def test_anl_c_bit_bit_clear(self):
        # SETB C; CLR bit0; ANL C,bit0 → CY=0
        prog = bytes([0xD3, 0xC2, 0x00, 0x82, 0x00]) + HALT
        sim = run(prog)
        assert not (sim._iram[SFR_PSW] & 0x80)

    def test_orl_c_bit(self):
        # CLR C; SETB bit0; ORL C,bit0 → CY=1
        prog = bytes([0xC3, 0xD2, 0x00, 0x72, 0x00]) + HALT
        sim = run(prog)
        assert sim._iram[SFR_PSW] & 0x80

    def test_anl_c_notbit(self):
        # SETB C; SETB bit0; ANL C, /bit0 → CY = CY & ~bit0 = 1 & 0 = 0
        prog = bytes([0xD3, 0xD2, 0x00, 0xB0, 0x00]) + HALT
        sim = run(prog)
        assert not (sim._iram[SFR_PSW] & 0x80)

    def test_orl_c_notbit(self):
        # CLR C; CLR bit0; ORL C, /bit0 → CY = CY | ~bit0 = 0 | 1 = 1
        prog = bytes([0xC3, 0xC2, 0x00, 0xA0, 0x00]) + HALT
        sim = run(prog)
        assert sim._iram[SFR_PSW] & 0x80


# ── Branches ─────────────────────────────────────────────────────────────────

class TestBranch:
    def test_ljmp(self):
        # LJMP 0x0010 → then MOV A,#0x42 → HALT at 0x0013
        prog = bytearray(0x20)
        prog[0] = 0x02; prog[1] = 0x00; prog[2] = 0x10   # LJMP 0x0010
        prog[0x10] = 0x74; prog[0x11] = 0x42              # MOV A,#0x42
        prog[0x12] = 0xA5                                  # HALT
        sim = I8051Simulator().execute.__self__ if False else I8051Simulator()
        result = sim.execute(bytes(prog))
        assert result.ok
        assert sim._iram[SFR_ACC] == 0x42

    def test_sjmp_forward(self):
        # SJMP +2 (skip 2 bytes); NOP; NOP; MOV A,#0x55
        prog = bytes([0x80, 0x02,    # SJMP +2 (skip 2 bytes: 0x02 and 0x03)
                      0x74, 0x00,    # MOV A,#0 (skipped)
                      0x74, 0x55]) + HALT
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 0x55

    def test_sjmp_backward(self):
        # Start at 0x00; MOV A,#0; SJMP to 0x00 (offset=-4 from after sjmp)
        # After SJMP fetch PC=0x04; rel=-4 → 0x04 + (-4) = 0x00 → infinite loop
        # But with max_steps=5 it stops
        loop = bytes([0x74, 0x00,    # 0x00: MOV A,#0
                      0x80, 0xFC]) + HALT  # 0x02: SJMP -4 (loops)
        result = I8051Simulator().execute(loop, max_steps=5)
        assert not result.ok  # max_steps exceeded

    def test_jz_taken(self):
        prog = bytes([0xE4, 0x60, 0x02, 0x74, 0xFF]) + HALT  # CLR A; JZ +2 skip bad; HALT
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 0   # 0xFF was skipped

    def test_jz_not_taken(self):
        prog = bytes([0x74, 0x01, 0x60, 0x02, 0xE4]) + bytes([0x74, 0x55]) + HALT
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 0x55

    def test_jnz_taken(self):
        # A=1; JNZ +1 → skip CLR A (1 byte at 0x04) → land at MOV A,#0x55 (0x05)
        prog = bytes([0x74, 0x01,   # 0x00: MOV A,#1
                      0x70, 0x01,   # 0x02: JNZ +1 (skip 0x04: CLR A)
                      0xE4,         # 0x04: CLR A (skipped)
                      0x74, 0x55])  + HALT  # 0x05: MOV A,#0x55
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 0x55

    def test_jc_taken(self):
        # A=0xFF; ADD A,#1 (sets CY); JC +2 skip; MOV A,#0; → final A=0
        prog = bytes([0x74, 0xFF, 0x24, 0x01, 0x40, 0x02, 0x74, 0x00]) + HALT
        # JC taken → skip MOV A,#0 → A stays 0 from addition
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 0x00

    def test_jnc_taken(self):
        prog = bytes([0xC3, 0x50, 0x02, 0x74, 0xFF]) + HALT  # CLR C; JNC +2 skip; →HALT
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 0x00  # 0xFF was skipped

    def test_jb_taken(self):
        # SETB bit0; JB bit0, +2 (skip MOV A,#0xFF)
        prog = bytes([0xD2, 0x00, 0x20, 0x00, 0x02, 0x74, 0xFF]) + HALT
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 0x00

    def test_jnb_taken(self):
        # CLR bit0; JNB bit0, +2 (skip MOV A,#0xFF)
        prog = bytes([0xC2, 0x00, 0x30, 0x00, 0x02, 0x74, 0xFF]) + HALT
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 0x00

    def test_jbc_clears_bit_when_taken(self):
        # SETB bit0; JBC bit0, +2; MOV A,#0xFF; HALT — bit0 must be cleared after JBC
        prog = bytes([0xD2, 0x00, 0x10, 0x00, 0x02, 0x74, 0xFF]) + HALT
        sim = run(prog)
        assert not (sim._iram[0x20] & 0x01)
        assert sim._iram[SFR_ACC] == 0x00   # skip happened

    def test_cjne_a_imm_taken(self):
        # A=5; CJNE A,#10,+2 → taken (skip MOV A,#0xFF)
        prog = bytes([0x74, 0x05, 0xB4, 0x0A, 0x02, 0x74, 0xFF]) + HALT
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 0x05

    def test_cjne_a_imm_not_taken(self):
        # A=10; CJNE A,#10,+2 → not taken → execute MOV A,#0xFF
        prog = bytes([0x74, 0x0A, 0xB4, 0x0A, 0x02, 0x74, 0xFF]) + HALT
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 0xFF

    def test_cjne_sets_cy_when_less(self):
        # A=3; CJNE A,#5,rel → CY=1 (3<5)
        prog = bytes([0x74, 0x03, 0xB4, 0x05, 0x00]) + HALT  # rel=0 → stay
        sim = run(prog)
        assert sim._iram[SFR_PSW] & 0x80   # CY=1

    def test_cjne_rn_imm(self):
        # R0=3; CJNE R0,#3,+2 → not taken (equal) → fall through to MOV A,#0xAA
        prog = bytes([0x78, 0x03, 0xB8, 0x03, 0x02, 0x74, 0x00, 0x74, 0xAA]) + HALT
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 0xAA

    def test_djnz_rn(self):
        # R0=3; loop: DEC via DJNZ back to MOV A each iter; A accumulates
        # MOV R0,#3; Loop: INC A; DJNZ R0, Loop
        # loop is at offset 2 from start; DJNZ at offset 4
        # After DJNZ fetch (PC=6); rel = 2-6 = -4 signed → 0xFC
        prog = bytes([0x78, 0x03,    # 0x00: MOV R0,#3
                      0x04,          # 0x02: INC A
                      0xD8, 0xFD]) + HALT  # 0x03: DJNZ R0,-3 → 0x05+(-3)=0x02
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 3

    def test_djnz_dir(self):
        # MOV 0x30,#2; Loop(0x03): INC A; DJNZ 0x30, Loop
        # DJNZ dir at 0x04: after fetch PC=0x07; rel=-4 → 0xFC → target=0x03 ✓
        prog = bytes([0x75, 0x30, 0x02,    # 0x00: MOV 0x30, #2
                      0x04,                # 0x03: INC A  (loop top)
                      0xD5, 0x30, 0xFC]) + HALT  # 0x04: DJNZ 0x30, -4 (to 0x03)
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 2

    def test_jmp_at_a_plus_dptr(self):
        # MOV DPTR,#0x10; MOV A,#2; JMP @A+DPTR → PC=0x12; MOV A,#0x55 at 0x12
        prog = bytearray(0x20)
        prog[0] = 0x90; prog[1] = 0x00; prog[2] = 0x10   # MOV DPTR, #0x10
        prog[3] = 0x74; prog[4] = 0x02                    # MOV A, #2
        prog[5] = 0x73                                     # JMP @A+DPTR → 0x12
        prog[0x12] = 0x74; prog[0x13] = 0x55              # MOV A, #0x55
        prog[0x14] = 0xA5                                  # HALT
        sim = I8051Simulator()
        result = sim.execute(bytes(prog))
        assert result.ok
        assert sim._iram[SFR_ACC] == 0x55


# ── Subroutines ───────────────────────────────────────────────────────────────

class TestSubroutines:
    def test_lcall_ret(self):
        # at 0x00: MOV A,#0; LCALL 0x10; (A should become 0x42 from sub)
        # at 0x10: MOV A,#0x42; RET
        prog = bytearray(0x20)
        prog[0] = 0x74; prog[1] = 0x00   # MOV A, #0
        prog[2] = 0x12; prog[3] = 0x00; prog[4] = 0x10   # LCALL 0x0010
        prog[5] = 0xA5   # HALT
        prog[0x10] = 0x74; prog[0x11] = 0x42   # MOV A, #0x42
        prog[0x12] = 0x22   # RET
        sim = I8051Simulator()
        result = sim.execute(bytes(prog))
        assert result.ok
        assert sim._iram[SFR_ACC] == 0x42
        assert sim._iram[SFR_SP] == 0x07   # SP restored

    def test_nested_lcall_ret(self):
        # Outer call at 0x00; inner call at 0x10; leaf at 0x20
        prog = bytearray(0x30)
        prog[0] = 0x12; prog[1] = 0x00; prog[2] = 0x10   # LCALL 0x10
        prog[3] = 0xA5   # HALT
        prog[0x10] = 0x12; prog[0x11] = 0x00; prog[0x12] = 0x20  # LCALL 0x20
        prog[0x13] = 0x22   # RET from outer sub
        prog[0x20] = 0x74; prog[0x21] = 0x77   # MOV A, #0x77
        prog[0x22] = 0x22   # RET from leaf
        sim = I8051Simulator()
        result = sim.execute(bytes(prog))
        assert result.ok
        assert sim._iram[SFR_ACC] == 0x77
        assert sim._iram[SFR_SP] == 0x07

    def test_reti_acts_like_ret(self):
        prog = bytearray(0x20)
        prog[0] = 0x12; prog[1] = 0x00; prog[2] = 0x10   # LCALL 0x10
        prog[3] = 0xA5   # HALT
        prog[0x10] = 0x74; prog[0x11] = 0x55   # MOV A, #0x55
        prog[0x12] = 0x32   # RETI
        sim = I8051Simulator()
        result = sim.execute(bytes(prog))
        assert result.ok
        assert sim._iram[SFR_ACC] == 0x55


# ── Parity ────────────────────────────────────────────────────────────────────

class TestParity:
    def test_parity_one_bit(self):
        # A=0x01 (1 set bit → odd → P=1)
        prog = bytes([0x74, 0x01]) + HALT
        sim = run(prog)
        assert sim._iram[SFR_PSW] & 0x01   # P=1

    def test_parity_two_bits(self):
        # A=0x03 (2 set bits → even → P=0)
        prog = bytes([0x74, 0x03]) + HALT
        sim = run(prog)
        assert not (sim._iram[SFR_PSW] & 0x01)

    def test_parity_zero(self):
        prog = bytes([0xE4]) + HALT  # CLR A → P=0
        sim = run(prog)
        assert not (sim._iram[SFR_PSW] & 0x01)
