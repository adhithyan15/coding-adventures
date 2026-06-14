"""Additional coverage tests to hit all instruction paths in the simulator and decoder."""

import pytest
from intel8051_simulator.state import SFR_B

from intel8051_gatelevel import Intel8051GateLevelSimulator

HALT = 0xA5


@pytest.fixture
def sim():
    return Intel8051GateLevelSimulator()


class TestMOVFamilyCoverage:
    """Exercise all MOV addressing modes."""

    def test_mov_a_dir(self, sim):
        prog = bytes([0x75, 0x30, 0x42,  # MOV 0x30, #0x42
                      0xE5, 0x30,         # MOV A, dir(0x30)
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0x42

    def test_mov_a_at_r0(self, sim):
        # Setup: R0=0x30, mem[0x30]=0x99, MOV A,@R0
        prog = bytes([0x78, 0x30,         # MOV R0, #0x30
                      0x75, 0x30, 0x99,   # MOV 0x30, #0x99
                      0xE6,               # MOV A, @R0
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0x99

    def test_mov_at_r0_a(self, sim):
        prog = bytes([0x78, 0x30,         # MOV R0, #0x30
                      0x74, 0xAB,         # MOV A, #0xAB
                      0xF6,               # MOV @R0, A
                      0xE4,               # CLR A
                      0xE6,               # MOV A, @R0 (read back)
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0xAB

    def test_mov_at_r1_dir(self, sim):
        prog = bytes([0x79, 0x30,         # MOV R1, #0x30
                      0x75, 0x31, 0xCD,   # MOV 0x31, #0xCD
                      0xA7, 0x31,         # MOV @R1, dir(0x31)
                      0xE7,               # MOV A, @R1
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0xCD

    def test_mov_at_r0_imm(self, sim):
        prog = bytes([0x78, 0x30,         # MOV R0, #0x30
                      0x76, 0x77,         # MOV @R0, #0x77
                      0xE6,               # MOV A, @R0
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0x77

    def test_mov_dir_at_r0(self, sim):
        prog = bytes([0x78, 0x30,         # MOV R0, #0x30
                      0x75, 0x30, 0xBE,   # MOV 0x30, #0xBE
                      0x86, 0x31,         # MOV dir(0x31), @R0
                      0xE5, 0x31,         # MOV A, dir(0x31)
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0xBE

    def test_mov_rn_dir(self, sim):
        prog = bytes([0x75, 0x30, 0x55,   # MOV 0x30, #0x55
                      0xA8, 0x30,         # MOV R0, dir(0x30)
                      0xE8,               # MOV A, R0
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0x55

    def test_movx_at_ri_a(self, sim):
        # Write to XDATA via @R1
        prog = bytes([0x79, 0x05,         # MOV R1, #5
                      0x74, 0xDE,         # MOV A, #0xDE
                      0xF3,               # MOVX @R1, A
                      0x74, 0x00,         # CLR A (sort of)
                      0xE3,               # MOVX A, @R1
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0xDE


class TestArithmeticCoverage:
    """Cover all ADD/ADDC/SUBB addressing modes."""

    def test_add_a_rn_all(self, sim):
        # ADD A, R3
        prog = bytes([0x74, 0x10,  # MOV A, #0x10
                      0x7B, 0x05,  # MOV R3, #5
                      0x2B,        # ADD A, R3 → 0x15
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0x15

    def test_add_a_at_ri(self, sim):
        prog = bytes([0x78, 0x30,         # MOV R0, #0x30
                      0x75, 0x30, 0x10,   # MOV 0x30, #0x10
                      0x74, 0x05,         # MOV A, #5
                      0x26,               # ADD A, @R0 → 0x15
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0x15

    def test_addc_a_at_ri(self, sim):
        prog = bytes([0xD3,               # SETB C → CY=1
                      0x78, 0x30,         # MOV R0, #0x30
                      0x75, 0x30, 0x05,   # MOV 0x30, #5
                      0x74, 0x05,         # MOV A, #5
                      0x36,               # ADDC A, @R0 → 5+5+1=11
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 11

    def test_addc_a_dir(self, sim):
        prog = bytes([0xD3,               # SETB C
                      0x75, 0x30, 0x03,   # MOV 0x30, #3
                      0x74, 0x05,         # MOV A, #5
                      0x35, 0x30,         # ADDC A, dir(0x30) → 5+3+1=9
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 9

    def test_subb_a_at_ri(self, sim):
        prog = bytes([0xC3,               # CLR C
                      0x78, 0x30,         # MOV R0, #0x30
                      0x75, 0x30, 0x03,   # MOV 0x30, #3
                      0x74, 0x0A,         # MOV A, #10
                      0x96,               # SUBB A, @R0 → 10-3=7
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 7

    def test_subb_a_dir(self, sim):
        prog = bytes([0xC3,
                      0x75, 0x30, 0x04,   # MOV 0x30, #4
                      0x74, 0x0C,         # MOV A, #12
                      0x95, 0x30,         # SUBB A, dir(0x30) → 12-4=8
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 8

    def test_inc_at_ri(self, sim):
        prog = bytes([0x78, 0x30,
                      0x75, 0x30, 0x10,   # mem[0x30] = 0x10
                      0x06,               # INC @R0
                      0xE6,               # MOV A, @R0
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0x11

    def test_dec_at_ri(self, sim):
        prog = bytes([0x78, 0x30,
                      0x75, 0x30, 0x10,
                      0x16,               # DEC @R0
                      0xE6,
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0x0F


class TestLogicalCoverage:
    """Cover all ANL/ORL/XRL addressing modes."""

    def test_anl_a_at_ri(self, sim):
        prog = bytes([0x78, 0x30,
                      0x75, 0x30, 0x0F,
                      0x74, 0xFF,
                      0x56,               # ANL A, @R0 → 0x0F
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0x0F

    def test_anl_a_dir(self, sim):
        prog = bytes([0x75, 0x30, 0xF0,
                      0x74, 0xFF,
                      0x55, 0x30,         # ANL A, dir(0x30) → 0xF0
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0xF0

    def test_anl_dir_a(self, sim):
        prog = bytes([0x75, 0x30, 0xFF,
                      0x74, 0x0F,
                      0x52, 0x30,         # ANL dir(0x30), A → 0x0F
                      0xE5, 0x30,
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0x0F

    def test_anl_dir_imm(self, sim):
        prog = bytes([0x75, 0x30, 0xFF,
                      0x53, 0x30, 0xAA,   # ANL dir(0x30), #0xAA → 0xAA
                      0xE5, 0x30,
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0xAA

    def test_orl_a_at_ri(self, sim):
        prog = bytes([0x78, 0x30,
                      0x75, 0x30, 0x0F,
                      0x74, 0xF0,
                      0x46,               # ORL A, @R0 → 0xFF
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0xFF

    def test_orl_a_dir(self, sim):
        prog = bytes([0x75, 0x30, 0x0F,
                      0x74, 0xF0,
                      0x45, 0x30,         # ORL A, dir → 0xFF
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0xFF

    def test_orl_dir_a(self, sim):
        prog = bytes([0x75, 0x30, 0x0F,
                      0x74, 0xF0,
                      0x42, 0x30,         # ORL dir(0x30), A → 0xFF
                      0xE5, 0x30,
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0xFF

    def test_orl_dir_imm(self, sim):
        prog = bytes([0x75, 0x30, 0x0F,
                      0x43, 0x30, 0xF0,   # ORL dir(0x30), #0xF0 → 0xFF
                      0xE5, 0x30,
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0xFF

    def test_xrl_a_at_ri(self, sim):
        prog = bytes([0x78, 0x30,
                      0x75, 0x30, 0xFF,
                      0x74, 0xFF,
                      0x66,               # XRL A, @R0 → 0x00
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0x00

    def test_xrl_a_dir(self, sim):
        prog = bytes([0x75, 0x30, 0xAA,
                      0x74, 0x55,
                      0x65, 0x30,         # XRL A, dir(0x30) → 0xFF
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0xFF

    def test_xrl_dir_a(self, sim):
        prog = bytes([0x75, 0x30, 0x55,
                      0x74, 0xAA,
                      0x62, 0x30,         # XRL dir(0x30), A → 0xFF
                      0xE5, 0x30,
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0xFF

    def test_xrl_dir_imm(self, sim):
        prog = bytes([0x75, 0x30, 0x55,
                      0x63, 0x30, 0xAA,   # XRL dir(0x30), #0xAA → 0xFF
                      0xE5, 0x30,
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0xFF


class TestBranchCoverage:
    """Cover all branch instructions."""

    def test_jz_not_taken(self, sim):
        prog = bytes([0x74, 0x01,   # MOV A, #1 (not zero)
                      0x60, 0x02,   # JZ +2 (not taken)
                      0x74, 0x42,   # MOV A, #0x42 (executed)
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0x42

    def test_jnz_not_taken(self, sim):
        prog = bytes([0x74, 0x00,   # MOV A, #0 (zero)
                      0x70, 0x02,   # JNZ +2 (not taken)
                      0x74, 0x42,   # MOV A, #0x42 (executed)
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0x42

    def test_jc_not_taken(self, sim):
        prog = bytes([0xC3,         # CLR C
                      0x40, 0x02,   # JC +2 (not taken)
                      0x74, 0x42,
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0x42

    def test_jnc_taken(self, sim):
        prog = bytes([0xC3,         # CLR C
                      0x50, 0x02,   # JNC +2 (taken, skip MOV)
                      0x74, 0xDE,   # (skipped)
                      0x74, 0x42,   # MOV A, #0x42 (reached)
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0x42

    def test_jb_not_taken(self, sim):
        prog = bytes([0xC3,               # CLR C (CY=0, so CY bit clear)
                      0x20, 0xD7, 0x01,   # JB CY, +1 (not taken)
                      0x74, 0x42,
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0x42

    def test_jnb_taken(self, sim):
        prog = bytes([0xC3,               # CLR C
                      0x30, 0xD7, 0x02,   # JNB CY, +2 (taken, skip)
                      0x74, 0xDE,         # (skipped)
                      0x74, 0x42,         # (reached)
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0x42

    def test_jbc_clears_bit(self, sim):
        # JBC: jump if bit set, then clear it.
        # Encoding: 0x10, bit_addr, rel8
        # After fetching the 3-byte JBC instruction starting at PC=1,
        # PC=4.  With rel=+2 the branch lands at PC=6 (the MOV A,#0x42),
        # skipping the 2-byte MOV A,#0xDE at offsets 4-5.
        prog = bytes([0xD3,               # SETB C                   (PC 0→1)
                      0x10, 0xD7, 0x02,   # JBC CY, +2 → jump to 6  (PC 1→4, then 6)
                      0x74, 0xDE,         # MOV A,#0xDE  [skipped]   (offsets 4-5)
                      0x74, 0x42,         # MOV A,#0x42  [reached]   (offsets 6-7)
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0x42
        assert not r.final_state.cy  # CY cleared by JBC

    def test_djnz_dir(self, sim):
        # DJNZ dir,rel: offsets 0-1=MOV A,#0  2-4=MOV dir,#3
        #               5=INC A  6-8=DJNZ dir  9=HALT
        # After fetching DJNZ dir at offsets 6,7,8, PC=9.
        # rel = 0xFC = -4 → jump to PC=5 (INC A).
        # Sequence: ACC=0, counter=3
        #   INC A→1, DJNZ dir (3→2, jump), INC A→2, DJNZ dir (2→1, jump),
        #   INC A→3, DJNZ dir (1→0, no jump), HALT → ACC=3.
        prog = bytes([0x74, 0x00,         # MOV A, #0         (0-1)
                      0x75, 0x30, 0x03,   # MOV 0x30, #3      (2-4)
                      # loop at offset 5:
                      0x04,               # INC A             (5)
                      0xD5, 0x30, 0xFC,   # DJNZ 0x30, -4     (6-8)
                      HALT])              #                   (9)
        r = sim.execute(prog)
        assert r.final_state.acc == 3  # incremented 3 times

    def test_cjne_rn_imm_not_taken(self, sim):
        prog = bytes([0x78, 0x05,         # MOV R0, #5
                      0xB8, 0x05, 0x02,   # CJNE R0, #5, +2 (not taken, equal)
                      0x74, 0x42,         # MOV A, #0x42 (executed)
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0x42

    def test_cjne_at_ri_imm(self, sim):
        prog = bytes([0x78, 0x30,         # MOV R0, #0x30
                      0x75, 0x30, 0x0A,   # MOV 0x30, #0x0A
                      # loop: compare @R0 with #5, branch if != 5
                      0xB6, 0x0A, 0x01,   # CJNE @R0, #0x0A, +1 (not taken)
                      0x74, 0x42,
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0x42

    def test_acall_ret(self, sim):
        prog = bytearray(0x20)
        # Main: ACALL sub (page 0), then MOV A, #0x55, HALT
        prog[0] = 0x11   # ACALL page0 (bits[7:5]=000, byte2=address)
        prog[1] = 0x10   # target = 0x0010
        prog[2] = 0x74
        prog[3] = 0x55
        prog[4] = HALT
        # Sub at 0x10
        prog[0x10] = 0x74
        prog[0x11] = 0xAB  # MOV A, #0xAB
        prog[0x12] = 0x22   # RET
        r = sim.execute(bytes(prog))
        assert r.final_state.acc == 0x55

    def test_reti(self, sim):
        prog = bytearray(0x20)
        prog[0] = 0x12
        prog[1] = 0x00
        prog[2] = 0x10  # LCALL 0x0010
        prog[3] = 0x74
        prog[4] = 0x99  # MOV A, #0x99
        prog[5] = HALT
        prog[0x10] = 0x32   # RETI (same as RET for behavioral)
        r = sim.execute(bytes(prog))
        assert r.final_state.acc == 0x99

    def test_sjmp_backward(self, sim):
        # Test backward branching using a DJNZ countdown loop.
        # prog3 layout: offsets 0-1=MOV A,#0  2-3=MOV R0,#5
        #               4=INC A  5-6=DJNZ R0,-3→offset 4  7=HALT
        prog3 = bytes([
            0x74, 0x00,   # MOV A, #0
            0x78, 0x05,   # MOV R0, #5
            # loop (offset 4):
            0x04,         # INC A
            0xD8, 0xFD,   # DJNZ R0, -3 (to offset 4)
            HALT,
        ])
        r = sim.execute(prog3)
        assert r.final_state.acc == 5

    def test_ajmp(self, sim):
        prog = bytearray(0x20)
        prog[0] = 0x01   # AJMP page0, byte2 = target
        prog[1] = 0x10   # target = (0<<8) | 0x10 = 0x0010
        prog[2] = 0x74
        prog[3] = 0xDE   # (should be skipped)
        prog[0x10] = 0x74
        prog[0x11] = 0x42
        prog[0x12] = HALT
        r = sim.execute(bytes(prog))
        assert r.final_state.acc == 0x42

    def test_jmp_a_plus_dptr(self, sim):
        prog = bytearray(0x20)
        prog[0] = 0x90
        prog[1] = 0x00
        prog[2] = 0x10  # MOV DPTR, #0x10
        prog[3] = 0x74
        prog[4] = 0x02  # MOV A, #2 (offset into table)
        prog[5] = 0x73   # JMP @A+DPTR → 0x12
        # Target table at 0x10: [0x01, 0x21, 0x41] = AJMP bytes
        prog[0x12] = 0x74
        prog[0x13] = 0xBB  # MOV A, #0xBB
        prog[0x14] = HALT
        r = sim.execute(bytes(prog))
        assert r.final_state.acc == 0xBB


class TestBitOpsCoverage:
    """Cover all bit operations."""

    def test_cpl_bit(self, sim):
        prog = bytes([0xC3,         # CLR C (CY=0)
                      0xB2, 0xD7,   # CPL bit(0xD7=CY) → CY=1
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.cy

    def test_anl_c_bit(self, sim):
        prog = bytes([0xD3,         # SETB C (CY=1)
                      0x82, 0xD7,   # ANL C, CY → C = 1 AND 1 = 1
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.cy

    def test_anl_c_bit_clears(self, sim):
        prog = bytes([0xD3,         # SETB C (CY=1)
                      0xC3,         # Wait, CLR C first then test
                      0xD3,         # SETB C again
                      # ANL C, /CY: C = C AND NOT(CY) = 1 AND NOT(1) = 0
                      0xB0, 0xD7,   # ANL C, /bit(CY) → C = C AND NOT(CY) = 0
                      HALT])
        r = sim.execute(prog)
        assert not r.final_state.cy

    def test_orl_c_bit_from_zero(self, sim):
        prog = bytes([0xC3,         # CLR C
                      0xD2, 0xE0,   # SETB ACC.0 (bit 0xE0)
                      0x72, 0xE0,   # ORL C, ACC.0 → C = 0 OR 1 = 1
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.cy

    def test_orl_c_not_bit(self, sim):
        prog = bytes([0xC3,         # CLR C
                      0xC2, 0xE0,   # CLR ACC.0 (ensure bit 0 clear)
                      0xA0, 0xE0,   # ORL C, /ACC.0 → C = 0 OR NOT(0) = 1
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.cy

    def test_mov_c_bit(self, sim):
        prog = bytes([0xC3,         # CLR C
                      0xD2, 0xE0,   # SETB ACC.0
                      0xA2, 0xE0,   # MOV C, ACC.0 → CY = 1
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.cy

    def test_mov_bit_c(self, sim):
        prog = bytes([0xD3,         # SETB C
                      0x92, 0xE0,   # MOV ACC.0, C → ACC bit 0 = 1
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.iram[0xE0] & 0x01 == 1

    def test_setb_clr_bit(self, sim):
        prog = bytes([0xD2, 0x20,   # SETB bit(0x20) — bit 0 of byte 0x20
                      0xC2, 0x20,   # CLR bit(0x20)
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.iram[0x20] == 0

    def test_cpl_c(self, sim):
        prog = bytes([0xD3,   # SETB C
                      0xB3,   # CPL C → CY=0
                      HALT])
        r = sim.execute(prog)
        assert not r.final_state.cy


class TestExchangeCoverage:
    """Cover XCH and XCHD."""

    def test_xch_a_rn(self, sim):
        prog = bytes([0x74, 0xAA,   # MOV A, #0xAA
                      0x79, 0x55,   # MOV R1, #0x55
                      0xC9,         # XCH A, R1
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0x55

    def test_xchd_a_at_ri(self, sim):
        prog = bytes([0x78, 0x30,         # MOV R0, #0x30
                      0x74, 0xAB,         # MOV A, #0xAB
                      0xF6,               # MOV @R0, A (mem[0x30] = 0xAB)
                      0x74, 0xCD,         # MOV A, #0xCD
                      0xD6,               # XCHD A, @R0 → swap lower nibbles: A=CB, mem[0x30]=AD
                      HALT])
        r = sim.execute(prog)
        # A was 0xCD, @R0 was 0xAB
        # Lower nibble swap: A.lo=D, @R0.lo=B
        # New A = 0xCB, new @R0 = 0xAD
        assert r.final_state.acc == 0xCB

    def test_xch_a_at_r1(self, sim):
        prog = bytes([0x79, 0x30,         # MOV R1, #0x30
                      0x75, 0x30, 0x77,   # MOV 0x30, #0x77
                      0x74, 0x42,         # MOV A, #0x42
                      0xC7,               # XCH A, @R1
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0x77

    def test_xch_a_dir(self, sim):
        prog = bytes([0x75, 0x30, 0x77,
                      0x74, 0x42,
                      0xC5, 0x30,         # XCH A, dir(0x30) → A=0x77
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0x77


class TestIncDecCoverage:
    """Cover all INC/DEC variants."""

    def test_inc_dir(self, sim):
        prog = bytes([0x75, 0x30, 0x0F,
                      0x05, 0x30,   # INC dir(0x30)
                      0xE5, 0x30,
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0x10

    def test_dec_dir(self, sim):
        prog = bytes([0x75, 0x30, 0x10,
                      0x15, 0x30,   # DEC dir(0x30)
                      0xE5, 0x30,
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0x0F


class TestMiscCoverage:
    """Miscellaneous coverage tests."""

    def test_rlc_chain(self, sim):
        # RLC through carry: shift left with carry
        prog = bytes([0xC3,   # CLR C (CY=0)
                      0x74, 0x40,  # MOV A, #0x40 = 01000000
                      0x33,        # RLC A: bit7=0→CY, bit0=oldCY=0 → A=0x80, CY=0
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0x80
        assert not r.final_state.cy

    def test_rrc_chain(self, sim):
        prog = bytes([0xD3,   # SETB C
                      0x74, 0x02,  # MOV A, #0x02 = 00000010
                      0x13,        # RRC A: bit0=0→CY, CY=1→bit7 → A=0x81, CY=0
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0x81

    def test_pop_sfr(self, sim):
        # PUSH and POP SFR
        prog = bytes([0x75, SFR_B, 0x42,     # MOV B, #0x42
                      0xC0, SFR_B,            # PUSH B
                      0x75, SFR_B, 0x00,      # MOV B, #0 (corrupt)
                      0xD0, SFR_B,            # POP B → 0x42
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.b == 0x42

    def test_movc_a_plus_pc(self, sim):
        # MOVC A, @A+PC: table immediately after HALT
        # PC after MOVC fetch = position of next byte
        prog = bytearray(20)
        prog[0] = 0x74
        prog[1] = 0x03  # MOV A, #3
        prog[2] = 0x83                      # MOVC A, @A+PC  (PC=3 after this)
        prog[3] = HALT
        # Table: prog[3+A] = prog[3+3] = prog[6]
        prog[6] = 0xBE
        r = sim.execute(bytes(prog))
        assert r.final_state.acc == 0xBE

    def test_indirect_addr_too_high_step(self, sim):
        # @Ri with value > 0x7F is invalid on base 8051.
        # step() propagates ValueError; execute() stores it as error string.
        prog = bytes([0x78, 0x80,   # MOV R0, #0x80 (illegal for indirect)
                      0xE6,         # MOV A, @R0 → ValueError on step
                      HALT])
        sim.load(prog)
        sim.step()  # MOV R0, #0x80
        with pytest.raises(ValueError, match="Indirect address"):
            sim.step()  # MOV A, @R0

    def test_execute_captures_indirect_error(self, sim):
        # execute() catches the ValueError and stores it in result.error.
        prog = bytes([0x78, 0x80,   # MOV R0, #0x80 (illegal for indirect)
                      0xE6,         # MOV A, @R0 → error captured
                      HALT])
        r = sim.execute(prog)
        assert r.error is not None
        assert "Indirect address" in r.error

    def test_dec_rn_all(self, sim):
        prog = bytes([0x7C, 0x10,   # MOV R4, #0x10
                      0x1C,         # DEC R4
                      0xEC,         # MOV A, R4
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0x0F

    def test_addc_rn(self, sim):
        prog = bytes([0xD3,   # SETB C
                      0x74, 0x05,  # MOV A, #5
                      0x7E, 0x03,  # MOV R6, #3
                      0x3E,        # ADDC A, R6 → 5+3+1=9
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 9

    def test_subb_rn(self, sim):
        prog = bytes([0xC3,   # CLR C
                      0x74, 0x0A,  # MOV A, #10
                      0x79, 0x03,  # MOV R1, #3
                      0x99,        # SUBB A, R1 → 10-3=7
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 7

    def test_anl_a_rn(self, sim):
        prog = bytes([0x74, 0xFF,
                      0x7B, 0x0F,  # MOV R3, #0x0F
                      0x5B,        # ANL A, R3 → 0x0F
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0x0F

    def test_orl_a_rn(self, sim):
        prog = bytes([0x74, 0xF0,
                      0x78, 0x0F,  # MOV R0, #0x0F
                      0x48,        # ORL A, R0 → 0xFF
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0xFF

    def test_xrl_a_rn(self, sim):
        prog = bytes([0x74, 0xAA,
                      0x78, 0xFF,  # MOV R0, #0xFF
                      0x68,        # XRL A, R0 → 0x55
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0x55

    def test_cjne_a_dir(self, sim):
        # CJNE A, dir, rel: compare A with memory location
        prog = bytes([0x75, 0x30, 0x42,   # MOV 0x30, #0x42
                      0x74, 0x42,         # MOV A, #0x42
                      0xB5, 0x30, 0x01,   # CJNE A, dir(0x30), +1 — not taken (equal)
                      0x74, 0x99,         # MOV A, #0x99 (executed)
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0x99

    def test_djnz_rn_r7(self, sim):
        prog = bytes([0x74, 0x00,   # MOV A, #0
                      0x7F, 0x03,   # MOV R7, #3
                      # loop:
                      0x04,         # INC A
                      0xDF, 0xFD,   # DJNZ R7, -3
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 3

    def test_xch_a_r7(self, sim):
        prog = bytes([0x74, 0x11,
                      0x7F, 0x22,   # MOV R7, #0x22
                      0xCF,         # XCH A, R7
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0x22

    def test_mov_rn_a_r3(self, sim):
        prog = bytes([0x74, 0x77,
                      0xFB,         # MOV R3, A
                      0xE4,         # CLR A
                      0xEB,         # MOV A, R3
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0x77

    def test_inc_rn_r5(self, sim):
        prog = bytes([0x7D, 0x09,   # MOV R5, #9
                      0x0D,         # INC R5
                      0xED,         # MOV A, R5
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 10

    def test_dec_rn_r7(self, sim):
        prog = bytes([0x7F, 0x0A,   # MOV R7, #10
                      0x1F,         # DEC R7
                      0xEF,         # MOV A, R7
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 9

    def test_add_a_r7(self, sim):
        prog = bytes([0x74, 0x10,
                      0x7F, 0x07,
                      0x2F,         # ADD A, R7 → 0x17
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0x17

    def test_at_r1_variant(self, sim):
        prog = bytes([0x79, 0x30,         # MOV R1, #0x30
                      0x75, 0x30, 0x99,
                      0xE7,               # MOV A, @R1
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0x99

    def test_mov_at_r1_imm(self, sim):
        prog = bytes([0x79, 0x30,
                      0x77, 0x42,         # MOV @R1, #0x42
                      0xE7,
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0x42

    def test_mov_at_r0_dir(self, sim):
        prog = bytes([0x78, 0x30,
                      0x75, 0x31, 0x55,   # MOV 0x31, #0x55
                      0xA6, 0x31,         # MOV @R0, dir(0x31) → mem[0x30]=0x55
                      0xE6,               # MOV A, @R0
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 0x55

    def test_subb_rn_r7(self, sim):
        prog = bytes([0xC3,
                      0x74, 0x20,
                      0x7F, 0x05,
                      0x9F,               # SUBB A, R7 → 32-5=27
                      HALT])
        r = sim.execute(prog)
        assert r.final_state.acc == 27
