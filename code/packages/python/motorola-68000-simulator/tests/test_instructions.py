"""Unit tests for individual Motorola 68000 instructions.

Each test method exercises one instruction form in isolation, verifying
both the result and the CCR (condition codes).

Encoding cheat-sheet used throughout:
  MOVEQ #n, Dn   : 0x7000 | (dn<<9) | (n & 0xFF)
  STOP #sr       : 0x4E72 <sr_word>
  ADD.L Dm, Dn   : 0xD080 | (dn<<9) | dm   (sz=10, dir=0 → bits 8-6 = 010)
                   Actually: 1101 Dn 0 10 000 Dm
  SUB.L Dm, Dn   : 0x9080 | (dn<<9) | dm
  AND.L Dm, Dn   : 0xC080 | (dn<<9) | dm
  OR.L  Dm, Dn   : 0x8080 | (dn<<9) | dm
  EOR.L Dn, Dm   : 0xB180 | (dn<<9) | dm   (EOR always Dn→EA, dir=1)
  CMP.L Dm, Dn   : 0xB080 | (dn<<9) | dm   (CMP dir=0)
  MOVE.L Dm, Dn  : 0x2000 | (dn<<9) | dm   (size=10, dst_mode=000, src_mode=000)
  MOVE.W Dm, Dn  : 0x3000 | (dn<<9) | dm
  MOVE.B Dm, Dn  : 0x1000 | (dn<<9) | dm
  CLR.L  Dn      : 0x4280 | dn
  TST.L  Dn      : 0x4A80 | dn
  NEG.L  Dn      : 0x4480 | dn
  NOT.L  Dn      : 0x4680 | dn
  SWAP   Dn      : 0x4840 | dn
  EXT.W  Dn      : 0x4880 | dn
  EXT.L  Dn      : 0x48C0 | dn
  NOP            : 0x4E71
  RTS            : 0x4E75
  BRA #d8        : 0x6000 | (d8 & 0xFF)   (short form; d8 relative to PC after opword)
  BSR #d8        : 0x6100 | (d8 & 0xFF)
  JSR (An)       : 0x4E90 | an
  JMP (An)       : 0x4ED0 | an
  LEA d16(An),Am : 0x41E8 | (am<<9) ...   # see tests
  LINK A6, #d16  : 0x4E56 <d16_signed>
  UNLK A6        : 0x4E5E
"""

from __future__ import annotations

import struct
import unittest

from motorola_68000_simulator import M68KSimulator

_LOAD = 0x001000   # program load address
_SP   = 0x00F000   # initial stack pointer


def _w(value: int) -> bytes:
    """Pack a 16-bit big-endian word."""
    return struct.pack(">H", value & 0xFFFF)


def _l(value: int) -> bytes:
    """Pack a 32-bit big-endian long."""
    return struct.pack(">I", value & 0xFFFFFFFF)


def _stop() -> bytes:
    """TRAP #15 — halts simulation without modifying SR."""
    return _w(0x4E4F)


def _run(prog: bytes) -> M68KSimulator:
    """Execute prog and return simulator with final state."""
    sim = M68KSimulator()
    result = sim.execute(prog)
    assert result.ok, f"Program failed: {result.error!r}, traces={result.traces[-3:]}"
    return sim


# ─────────────────────────────────────────────────────────────────────────────
# MOVEQ
# ─────────────────────────────────────────────────────────────────────────────

class TestMOVEQ(unittest.TestCase):

    def test_moveq_positive(self):
        prog = _w(0x7005) + _stop()      # MOVEQ #5, D0
        sim = _run(prog)
        assert sim._d[0] == 5
        assert not sim._sr & 4            # Z clear
        assert not sim._sr & 8            # N clear

    def test_moveq_zero(self):
        prog = _w(0x7000) + _stop()      # MOVEQ #0, D0
        sim = _run(prog)
        assert sim._d[0] == 0
        assert sim._sr & 4               # Z set

    def test_moveq_negative(self):
        # MOVEQ #-1 (0xFF sign-extended to 0xFFFFFFFF)
        prog = _w(0x70FF) + _stop()
        sim = _run(prog)
        assert sim._d[0] == 0xFFFF_FFFF
        assert sim._sr & 8               # N set
        assert not sim._sr & 4           # Z clear

    def test_moveq_max_positive(self):
        prog = _w(0x707F) + _stop()      # MOVEQ #127, D0
        sim = _run(prog)
        assert sim._d[0] == 127
        assert not sim._sr & 8           # N clear

    def test_moveq_min_negative(self):
        prog = _w(0x7080) + _stop()      # MOVEQ #-128 (0x80)
        sim = _run(prog)
        assert sim._d[0] == 0xFFFF_FF80  # sign-extended
        assert sim._sr & 8               # N set

    def test_moveq_clears_v_c(self):
        # First create a state with V=1, C=1
        prog = (
            _w(0x70FF)  # MOVEQ #-1, D0 → sets N, clears C/V
            + _w(0x7005)  # MOVEQ #5, D0 → should clear N, V, C
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 5
        assert not (sim._sr & 0x0F)   # N/Z/V/C all clear (only Z bit... wait Z=0 too)
        # Actually Z is also clear since 5 != 0
        assert not (sim._sr & 3)   # V and C clear
        assert not (sim._sr & 8)   # N clear


# ─────────────────────────────────────────────────────────────────────────────
# MOVE (data registers)
# ─────────────────────────────────────────────────────────────────────────────

class TestMOVE(unittest.TestCase):

    def test_move_l_reg_to_reg(self):
        prog = (
            _w(0x7042)      # MOVEQ #66, D0
            + _w(0x2200)    # MOVE.L D0, D1  (0x2000 | (1<<9) | 0 = 0x2200)
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[1] == 66

    def test_move_w_reg_to_reg(self):
        # Set D0 = 0x12345678, then MOVE.W D0, D1 → D1 should get 0x5678
        prog = (
            _w(0x303C) + _w(0x5678)    # MOVE.W #0x5678, D0
            + _w(0x3200)                 # MOVE.W D0, D1  (0x3000 | (1<<9) | 0)
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[1] == 0x5678
        assert not (sim._sr & 4)       # not zero
        assert not (sim._sr & 8)       # not negative (bit 15 clear)

    def test_move_b_reg_to_reg(self):
        prog = (
            _w(0x103C) + _w(0x00AB)    # MOVE.B #0xAB, D0
            + _w(0x1200)                 # MOVE.B D0, D1  (0x1000 | (1<<9) | 0)
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[1] == 0xAB
        assert sim._sr & 8             # N set (bit 7 of 0xAB = 1)

    def test_move_sets_z_flag(self):
        prog = (
            _w(0x7000)   # MOVEQ #0, D0
            + _w(0x2200)  # MOVE.L D0, D1
            + _stop()
        )
        sim = _run(prog)
        assert sim._sr & 4             # Z set

    def test_move_imm_word_to_reg(self):
        prog = _w(0x303C) + _w(0xFFFF) + _stop()   # MOVE.W #-1, D0
        sim = _run(prog)
        assert sim._d[0] == 0xFFFF
        assert sim._sr & 8             # N set (bit 15 of 0xFFFF)

    def test_move_imm_long_to_reg(self):
        prog = _w(0x203C) + _l(0xDEAD_BEEF) + _stop()   # MOVE.L #0xDEADBEEF, D0
        sim = _run(prog)
        assert sim._d[0] == 0xDEAD_BEEF

    def test_move_to_memory_and_back(self):
        # MOVE.L #42, (A0)  then read back
        # LEA 0x2000, A0 first
        # MOVE.L #42, (A0)   → write to 0x2000
        # MOVE.L (A0), D1   → read back
        prog = (
            _w(0x207C) + _l(0x002000)  # MOVEA.L #0x2000, A0
            + _w(0x20BC) + _l(42)       # MOVE.L #42, (A0)
            + _w(0x2210)                # MOVE.L (A0), D1  (0x2000|(1<<9)|0b010_000)
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[1] == 42

    def test_move_postinc(self):
        # MOVE.L (A0)+, D1 should read and increment A0 by 4
        prog = (
            _w(0x207C) + _l(0x002000)  # MOVEA.L #0x2000, A0
            + _w(0x20BC) + _l(0xCAFE)  # MOVE.L #0xCAFE, (A0)
            + _w(0x2218)                # MOVE.L (A0)+, D1  (mode=011=postinc)
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[1] == 0xCAFE
        assert sim._a[0] == 0x002004   # incremented by 4

    def test_move_predec(self):
        # MOVE.W -(A1), D0 using predecrement addressing.
        # A1 starts at 0x2004; predecrement reduces by 2 → reads from 0x2002.
        # We write 0x5678 at 0x2002 beforehand using MOVE.W #0x5678, (A0).
        #
        # MOVEA.L #imm, A1:  sz=10, dst_reg=1, dst_mode=001(An), src=111/100(#imm)
        #   = 0010 001 001 111 100 = 0x227C
        # MOVE.W -(A1), D0:  sz=11, dst_reg=0, dst_mode=000(Dn), src_mode=100, src_reg=1
        #   = 0011 000 000 100 001 = 0x3021
        prog = (
            _w(0x207C) + _l(0x002002)   # MOVEA.L #0x2002, A0
            + _w(0x227C) + _l(0x002004)  # MOVEA.L #0x2004, A1
            + _w(0x30BC) + _w(0x5678)   # MOVE.W #0x5678, (A0)  → mem[0x2002]=0x5678
            + _w(0x3021)                 # MOVE.W -(A1), D0  (A1→0x2002, reads 0x5678)
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 0x5678
        assert sim._a[1] == 0x2002   # decremented by 2


# ─────────────────────────────────────────────────────────────────────────────
# MOVEA
# ─────────────────────────────────────────────────────────────────────────────

class TestMOVEA(unittest.TestCase):

    def test_movea_l_imm(self):
        prog = _w(0x207C) + _l(0x5000) + _stop()   # MOVEA.L #0x5000, A0
        sim = _run(prog)
        assert sim._a[0] == 0x5000

    def test_movea_w_sign_extends(self):
        # MOVEA.W #-1, A0 → A0 = 0xFFFFFFFF
        prog = _w(0x307C) + _w(0xFFFF) + _stop()   # MOVEA.W #-1, A0
        sim = _run(prog)
        assert sim._a[0] == 0xFFFF_FFFF

    def test_movea_does_not_set_flags(self):
        prog = (
            _w(0x44FC) + _w(0x04)    # MOVE #4, CCR  (Z=1)
            + _w(0x207C) + _l(0x5000)  # MOVEA.L #0x5000, A0  (should not touch flags)
            + _stop()
        )
        sim = _run(prog)
        assert sim._sr & 4   # Z still set


# ─────────────────────────────────────────────────────────────────────────────
# ADD / ADDI / ADDQ
# ─────────────────────────────────────────────────────────────────────────────

class TestADD(unittest.TestCase):

    def test_add_l_no_carry(self):
        prog = (
            _w(0x7005)   # MOVEQ #5, D0
            + _w(0x7203)  # MOVEQ #3, D1
            + _w(0xD081)  # ADD.L D1, D0  (1101 000 0 10 000 001)
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 8
        assert not (sim._sr & 1)   # C clear

    def test_add_l_carry(self):
        prog = (
            _w(0x203C) + _l(0xFFFF_FFFF)  # MOVE.L #0xFFFFFFFF, D0
            + _w(0x7201)                    # MOVEQ #1, D1
            + _w(0xD081)                    # ADD.L D1, D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 0             # wraps to 0
        assert sim._sr & 1                # C set
        assert sim._sr & 4                # Z set

    def test_add_w_overflow(self):
        # 0x7FFF + 1 = 0x8000 → signed overflow
        prog = (
            _w(0x303C) + _w(0x7FFF)    # MOVE.W #0x7FFF, D0
            + _w(0x5040)               # ADDQ.W #8... nope. Let's use ADDI
            + _stop()
        )
        prog = (
            _w(0x303C) + _w(0x7FFF)    # MOVE.W #0x7FFF, D0
            + _w(0x0640) + _w(0x0001)  # ADDI.W #1, D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] & 0xFFFF == 0x8000
        assert sim._sr & 2             # V set
        assert sim._sr & 8             # N set

    def test_addi_b(self):
        prog = (
            _w(0x103C) + _w(0x0001)   # MOVE.B #1, D0
            + _w(0x0600) + _w(0x0002)  # ADDI.B #2, D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] & 0xFF == 3

    def test_addi_l(self):
        prog = (
            _w(0x7000)                 # MOVEQ #0, D0
            + _w(0x0680) + _l(100)    # ADDI.L #100, D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 100

    def test_addq(self):
        # ADDQ.W #3, D0  — 0101 011 0 01 000 000 = 0x5640
        prog = (
            _w(0x303C) + _w(10)    # MOVE.W #10, D0
            + _w(0x5640)           # ADDQ.W #3, D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 13

    def test_addq_8_encoded_as_0(self):
        # ADDQ.W #8, D0 — data field = 000 (encoded as 8)
        prog = (
            _w(0x303C) + _w(1)    # MOVE.W #1, D0
            + _w(0x5040)          # ADDQ.W #8, D0 (0101 000 0 01 000 000)
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 9

    def test_adda_l(self):
        # ADDA.L D1, A0
        prog = (
            _w(0x207C) + _l(0x1000)   # MOVEA.L #0x1000, A0
            + _w(0x7202)               # MOVEQ #2, D1
            + _w(0xD1C1)               # ADDA.L D1, A0  (1101 000 1 11 000 001)
            + _stop()
        )
        sim = _run(prog)
        assert sim._a[0] == 0x1002

    def test_adda_w_sign_extends(self):
        # ADDA.W #-1 (0xFFFF), A0 — should sign-extend to -1 then add
        prog = (
            _w(0x207C) + _l(0x2000)   # MOVEA.L #0x2000, A0
            + _w(0x30FC) + _w(0xFFFF)  # MOVE.W #0xFFFF, (A0)+ (store -1 at 0x2000)
            + _w(0x207C) + _l(0x2000)  # MOVEA.L #0x2000, A0  (reset)
            + _w(0xD0D0)               # ADDA.W (A0), A0 (1101 000 0 11 010 000)
            + _stop()
        )
        # 0x2000 + sign_extend(0xFFFF→-1) = 0x1FFF
        sim = _run(prog)
        assert sim._a[0] == 0x1FFF


# ─────────────────────────────────────────────────────────────────────────────
# SUB / SUBI / SUBQ
# ─────────────────────────────────────────────────────────────────────────────

class TestSUB(unittest.TestCase):

    def test_sub_l(self):
        prog = (
            _w(0x700A)   # MOVEQ #10, D0
            + _w(0x7203)  # MOVEQ #3, D1
            + _w(0x9081)  # SUB.L D1, D0  (1001 000 0 10 000 001)
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 7
        assert not (sim._sr & 1)   # C clear

    def test_sub_l_borrow(self):
        prog = (
            _w(0x7001)               # MOVEQ #1, D0
            + _w(0x7203)              # MOVEQ #3, D1
            + _w(0x9081)              # SUB.L D1, D0 → 1 - 3 = -2
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 0xFFFF_FFFE  # -2 unsigned = 0xFFFFFFFE
        assert sim._sr & 1            # C set (borrow)
        assert sim._sr & 8            # N set

    def test_subi_w(self):
        prog = (
            _w(0x303C) + _w(10)      # MOVE.W #10, D0
            + _w(0x0440) + _w(3)     # SUBI.W #3, D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 7

    def test_subq(self):
        # SUBQ.W #1, D0 — 0101 001 1 01 000 000 = 0x5340
        prog = (
            _w(0x303C) + _w(5)    # MOVE.W #5, D0
            + _w(0x5340)          # SUBQ.W #1, D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 4

    def test_suba_l(self):
        prog = (
            _w(0x207C) + _l(0x3000)   # MOVEA.L #0x3000, A0
            + _w(0x7204)               # MOVEQ #4, D1
            + _w(0x91C1)               # SUBA.L D1, A0  (1001 000 1 11 000 001)
            + _stop()
        )
        sim = _run(prog)
        assert sim._a[0] == 0x2FFC

    def test_suba_w_sign_extends(self):
        # SUBA.W D1, A0: subtract sign-extended D1.W from A0.L
        # SUBA.W D0, A0: 1001 000 0 11 000 000 = 0x90C0
        # (dn=0=A0, dir_bit=0=SUBA.W, sz_code=3=11, mode=0=Dn, reg=0=D0)
        prog = (
            _w(0x207C) + _l(0x2000)   # MOVEA.L #0x2000, A0
            + _w(0x303C) + _w(0xFFFE)  # MOVE.W #0xFFFE (-2), D0
            + _w(0x90C0)               # SUBA.W D0, A0
            + _stop()
        )
        # A0 = 0x2000, D0.W = 0xFFFE → sign-extended to -2
        # A0 - (-2) = 0x2000 + 2 = 0x2002
        sim = _run(prog)
        assert sim._a[0] == 0x2002


# ─────────────────────────────────────────────────────────────────────────────
# AND / ANDI / OR / ORI / EOR / EORI
# ─────────────────────────────────────────────────────────────────────────────

class TestLogic(unittest.TestCase):

    def test_and_l(self):
        prog = (
            _w(0x203C) + _l(0xFF00FF00)  # MOVE.L #0xFF00FF00, D0
            + _w(0x223C) + _l(0x0F0F0F0F)  # MOVE.L #0x0F0F0F0F, D1
            + _w(0xC081)                  # AND.L D1, D0  (1100 000 0 10 000 001)
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 0x0F000F00

    def test_andi_w(self):
        prog = (
            _w(0x303C) + _w(0xF0F0)    # MOVE.W #0xF0F0, D0
            + _w(0x0240) + _w(0x00FF)  # ANDI.W #0x00FF, D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 0x00F0

    def test_or_l(self):
        prog = (
            _w(0x203C) + _l(0xF0F0F0F0)  # MOVE.L #0xF0F0F0F0, D0
            + _w(0x223C) + _l(0x0F0F0F0F)  # MOVE.L #0x0F0F0F0F, D1
            + _w(0x8081)                   # OR.L D1, D0  (1000 000 0 10 000 001)
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 0xFFFF_FFFF

    def test_ori_b(self):
        prog = (
            _w(0x103C) + _w(0x00F0)    # MOVE.B #0xF0, D0
            + _w(0x0000) + _w(0x000F)  # ORI.B #0x0F, D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] & 0xFF == 0xFF

    def test_eor_l(self):
        # EOR.L D1, D0: 1011 001 1 10 000 000 = 0xB380... wait
        # EOR Dn, <ea>: dir_bit=1, 1011 rrr 1 ss ea
        # EOR.L D1, D0: 1011 001 1 10 000 000 = 0xB382... let me compute
        # 1011 = B, 001 = D1(dn), 1 = dir, 10 = long, 000 = mode Dn, 000 = D0(reg)
        # = 1011 001 1 10 000 000 = 0b1011_001_1_10_000_000 = 0xB380
        prog = (
            _w(0x203C) + _l(0xAAAA_AAAA)  # MOVE.L #0xAAAAAAAA, D0
            + _w(0x223C) + _l(0x5555_5555)  # MOVE.L #0x55555555, D1
            + _w(0xB380)                    # EOR.L D1, D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 0xFFFF_FFFF

    def test_eori_l(self):
        prog = (
            _w(0x203C) + _l(0xFFFF_FFFF)
            + _w(0x0A80) + _l(0xFFFF_FFFF)  # EORI.L #0xFFFFFFFF, D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 0
        assert sim._sr & 4   # Z set

    def test_not_l(self):
        prog = (
            _w(0x7000)              # MOVEQ #0, D0
            + _w(0x4680)           # NOT.L D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 0xFFFF_FFFF
        assert sim._sr & 8   # N set

    def test_logic_clears_v_c(self):
        # After AND, V and C must be 0
        prog = (
            _w(0x303C) + _w(0x7FFF)    # MOVE.W #0x7FFF, D0
            + _w(0x0640) + _w(0x0001)  # ADDI.W #1, D0  → sets V
            + _w(0xC040)               # AND.W D0, D0  (no-op but clears V/C)
            + _stop()
        )
        # Actually AND.W D0, D0 would be (1100 000 0 01 000 000) = 0xC040
        sim = _run(prog)
        assert not (sim._sr & 3)   # V and C both cleared


# ─────────────────────────────────────────────────────────────────────────────
# CMP / CMPI / CMPA
# ─────────────────────────────────────────────────────────────────────────────

class TestCMP(unittest.TestCase):

    def test_cmp_equal(self):
        prog = (
            _w(0x7005)  # MOVEQ #5, D0
            + _w(0xB07C) + _w(5)   # CMPI.W #5, D0 — wait, that's wrong opcode
            + _stop()
        )
        # CMP.L #5, D0: CMPI.L #5, D0 = 0x0C80 then long
        prog = (
            _w(0x7005)
            + _w(0x0C80) + _l(5)   # CMPI.L #5, D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._sr & 4    # Z set
        assert not (sim._sr & 1)   # C clear

    def test_cmp_less_than(self):
        # D0 = 3, CMP.L #5, D0 → 3-5 < 0 → C set, N set
        prog = (
            _w(0x7003)
            + _w(0x0C80) + _l(5)   # CMPI.L #5, D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._sr & 1    # C set (borrow: 3 < 5)
        assert not (sim._sr & 4)   # Z clear

    def test_cmp_greater_than(self):
        prog = (
            _w(0x700A)           # MOVEQ #10, D0
            + _w(0x0C80) + _l(5)  # CMPI.L #5, D0
            + _stop()
        )
        sim = _run(prog)
        assert not (sim._sr & 1)   # C clear (no borrow: 10 >= 5)
        assert not (sim._sr & 4)   # Z clear

    def test_cmp_does_not_modify_register(self):
        prog = (
            _w(0x7005)
            + _w(0x0C80) + _l(3)
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 5   # D0 unchanged

    def test_cmpa_l(self):
        prog = (
            _w(0x207C) + _l(0x2000)   # MOVEA.L #0x2000, A0
            + _w(0x203C) + _l(0x2000)  # MOVE.L #0x2000, D0
            + _w(0xB1C0)               # CMPA.L D0, A0  (1011 000 1 11 000 000)
            + _stop()
        )
        sim = _run(prog)
        assert sim._sr & 4    # Z set (equal)


# ─────────────────────────────────────────────────────────────────────────────
# CLR / NEG / TST
# ─────────────────────────────────────────────────────────────────────────────

class TestUnary(unittest.TestCase):

    def test_clr_l(self):
        prog = (
            _w(0x203C) + _l(0xDEAD)  # MOVE.L #0xDEAD, D0
            + _w(0x4280)              # CLR.L D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 0
        assert sim._sr & 4    # Z set
        assert not (sim._sr & 1)   # C clear

    def test_clr_w(self):
        prog = (
            _w(0x303C) + _w(0xFFFF)  # MOVE.W #0xFFFF, D0
            + _w(0x4240)              # CLR.W D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 0

    def test_neg_l_positive(self):
        prog = (
            _w(0x7005)  # MOVEQ #5, D0
            + _w(0x4480)  # NEG.L D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 0xFFFF_FFFB   # -5 in unsigned 32-bit
        assert sim._sr & 8   # N set
        assert sim._sr & 1   # C set (result non-zero → carry)

    def test_neg_l_zero(self):
        prog = (
            _w(0x7000)   # MOVEQ #0, D0
            + _w(0x4480)  # NEG.L D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 0
        assert sim._sr & 4    # Z set
        assert not (sim._sr & 1)   # C clear (NEG 0 → no borrow)

    def test_tst_zero(self):
        prog = (
            _w(0x7000)   # MOVEQ #0, D0
            + _w(0x4A80)  # TST.L D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._sr & 4    # Z set

    def test_tst_negative(self):
        prog = (
            _w(0x203C) + _l(0x8000_0000)  # MOVE.L #0x80000000, D0
            + _w(0x4A80)                   # TST.L D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._sr & 8   # N set
        assert not (sim._sr & 4)   # Z clear


# ─────────────────────────────────────────────────────────────────────────────
# SWAP / EXT
# ─────────────────────────────────────────────────────────────────────────────

class TestSwapExt(unittest.TestCase):

    def test_swap(self):
        prog = (
            _w(0x203C) + _l(0x1234_5678)  # MOVE.L #0x12345678, D0
            + _w(0x4840)                   # SWAP D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 0x5678_1234

    def test_swap_flags(self):
        # SWAP of 0x00001234 → 0x12340000 → N=0, Z=0
        prog = (
            _w(0x203C) + _l(0x0000_1234)
            + _w(0x4840)  # SWAP D0
            + _stop()
        )
        sim = _run(prog)
        assert not (sim._sr & 8)   # N clear
        assert not (sim._sr & 4)   # Z clear

    def test_ext_w(self):
        # EXT.W D0: sign-extend byte 0xFF (-1) to word
        prog = (
            _w(0x103C) + _w(0x00FF)   # MOVE.B #0xFF, D0
            + _w(0x4880)               # EXT.W D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] & 0xFFFF == 0xFFFF
        assert sim._sr & 8   # N set

    def test_ext_l(self):
        # EXT.L D0: sign-extend word 0x8000 (-32768) to long
        prog = (
            _w(0x303C) + _w(0x8000)   # MOVE.W #0x8000, D0
            + _w(0x48C0)               # EXT.L D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 0xFFFF_8000
        assert sim._sr & 8   # N set


# ─────────────────────────────────────────────────────────────────────────────
# Multiply and divide
# ─────────────────────────────────────────────────────────────────────────────

class TestMulDiv(unittest.TestCase):

    def test_mulu(self):
        # MULU D1, D0: 1100 000 0 11 000 001 = 0xC0C1
        prog = (
            _w(0x303C) + _w(12)    # MOVE.W #12, D0
            + _w(0x323C) + _w(7)   # MOVE.W #7, D1
            + _w(0xC0C1)           # MULU D1, D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 84

    def test_muls_negative(self):
        # MULS D1, D0: 1100 000 1 11 000 001 = 0xC1C1
        # -2 × 3 = -6 (0xFFFFFFFA)
        prog = (
            _w(0x303C) + _w(0xFFFE)  # MOVE.W #-2 (0xFFFE), D0
            + _w(0x323C) + _w(3)     # MOVE.W #3, D1
            + _w(0xC1C1)             # MULS D1, D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 0xFFFF_FFFA   # -6 in unsigned

    def test_divu_basic(self):
        # DIVU D1, D0: 1000 000 0 11 000 001 = 0x80C1
        # 100 / 7 = 14 remainder 2
        prog = (
            _w(0x203C) + _l(100)   # MOVE.L #100, D0
            + _w(0x323C) + _w(7)   # MOVE.W #7, D1
            + _w(0x80C1)           # DIVU D1, D0
            + _stop()
        )
        sim = _run(prog)
        quotient  = sim._d[0] & 0xFFFF
        remainder = (sim._d[0] >> 16) & 0xFFFF
        assert quotient == 14
        assert remainder == 2

    def test_divs_negative(self):
        # DIVS D1, D0: 1000 000 1 11 000 001 = 0x81C1
        # -7 / 2 = -3 remainder -1
        prog = (
            _w(0x203C) + _l(0xFFFF_FFF9)  # MOVE.L #-7 (unsigned), D0
            + _w(0x323C) + _w(2)           # MOVE.W #2, D1
            + _w(0x81C1)                   # DIVS D1, D0
            + _stop()
        )
        sim = _run(prog)
        quotient  = _sign_extend_word(sim._d[0] & 0xFFFF)
        remainder = _sign_extend_word((sim._d[0] >> 16) & 0xFFFF)
        assert quotient == -3
        assert remainder == -1


def _sign_extend_word(v: int) -> int:
    return v if v < 0x8000 else v - 0x10000


# ─────────────────────────────────────────────────────────────────────────────
# Shifts and rotates
# ─────────────────────────────────────────────────────────────────────────────

class TestShifts(unittest.TestCase):

    def test_asl_l_by_1(self):
        # ASL.L #1, D0: 1110 001 1 10 0 00 000 = 0xE380
        prog = (
            _w(0x7002)   # MOVEQ #2, D0
            + _w(0xE380)  # ASL.L #1, D0  → 4
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 4

    def test_asr_l_by_1(self):
        # ASR.L #1, D0: 1110 001 0 10 0 00 000 = 0xE280
        prog = (
            _w(0x7008)   # MOVEQ #8, D0
            + _w(0xE280)  # ASR.L #1, D0  → 4
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 4

    def test_asr_sign_fill(self):
        # ASR.L #1 on 0x80000000 (negative) → 0xC0000000 (sign bit replicated)
        prog = (
            _w(0x203C) + _l(0x8000_0000)
            + _w(0xE280)   # ASR.L #1, D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 0xC000_0000
        # ASR shifts right: the bit shifted OUT is bit 0 of the source.
        # 0x80000000 >> 1 (with sign) = 0xC0000000; bit 0 of source = 0 → C = 0.
        assert not (sim._sr & 1)   # C clear (bit 0 of source was 0)

    def test_lsl_l_by_2(self):
        # LSL.L #2, D0: 1110 010 1 10 0 01 000 = 0xE588
        prog = (
            _w(0x7001)   # MOVEQ #1, D0
            + _w(0xE588)  # LSL.L #2, D0  → 4
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 4

    def test_lsr_l_by_1(self):
        # LSR.L #1, D0: 1110 001 0 10 0 01 000 = 0xE288
        prog = (
            _w(0x700C)   # MOVEQ #12, D0
            + _w(0xE288)  # LSR.L #1, D0  → 6
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 6

    def test_lsr_clears_v(self):
        prog = (
            _w(0x700F)   # MOVEQ #15, D0
            + _w(0xE288)  # LSR.L #1, D0
            + _stop()
        )
        sim = _run(prog)
        assert not (sim._sr & 2)   # V clear (LSR always clears V)

    def test_rol_w_by_1(self):
        # ROL.W #1, D0: 1110 001 1 01 0 11 000 = 0xE358  (sz=01=word, dir=1, type=11=RO, imm=1)
        # Actually: 1110 ccc d ss r tt rrr
        # ccc=001(count=1), d=1(left), ss=01(word), r=0(imm), tt=11(RO), rrr=000(D0)
        # = 1110 001 1 01 0 11 000 = 0xE358
        prog = (
            _w(0x303C) + _w(0x8001)   # MOVE.W #0x8001, D0
            + _w(0xE358)               # ROL.W #1, D0
            + _stop()
        )
        sim = _run(prog)
        # 0x8001 ROL 1 = 0x0003 (bit 15→bit 0, bit 0→bit 1... wait)
        # ROL: shifts left, MSB wraps to bit 0
        # 0x8001 = 1000_0000_0000_0001
        # ROL 1:  0000_0000_0000_0011 = 0x0003, C = 1 (the old MSB)
        assert sim._d[0] & 0xFFFF == 0x0003
        assert sim._sr & 1   # C set (old MSB was 1)

    def test_ror_w_by_1(self):
        # ROR.W #1, D0: 1110 001 0 01 0 11 000 = 0xE258
        prog = (
            _w(0x303C) + _w(0x8001)
            + _w(0xE258)   # ROR.W #1, D0
            + _stop()
        )
        sim = _run(prog)
        # 0x8001 ROR 1: bit 0 wraps to bit 15
        # 0x8001 = 1000_0000_0000_0001, ROR 1 = 1100_0000_0000_0000 = 0xC000
        assert sim._d[0] & 0xFFFF == 0xC000

    def test_shift_by_register(self):
        # ASL D1, D0: shift D0 left by amount in D1
        # 1110 001 1 10 1 00 000: ccc=001(D1), d=1(left), sz=10(long), r=1(reg), tt=00(AS), rrr=000(D0)
        # = 1110 001 1 10 1 00 000 = 0xE3A0
        prog = (
            _w(0x7001)   # MOVEQ #1, D0  (value to shift)
            + _w(0x7203)  # MOVEQ #3, D1  (shift count)
            + _w(0xE3A0)  # ASL.L D1, D0  → 1 << 3 = 8
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 8


# ─────────────────────────────────────────────────────────────────────────────
# Branches
# ─────────────────────────────────────────────────────────────────────────────

class TestBranches(unittest.TestCase):

    def test_bra_forward(self):
        # BRA: skip MOVE.L #99, D0 (6 bytes) then execute MOVEQ #1, D0
        # BRA opword at load+0; PC after opword = load+2; target = load+2+6 = load+8
        # MOVE.L #99, D0 occupies load+2..load+7 (opword 2B + immediate long 4B)
        # MOVEQ #1, D0 at load+8; STOP at load+10
        prog = (
            _w(0x6006)              # BRA #6  (skip 6 bytes)
            + _w(0x203C) + _l(99)  # MOVE.L #99, D0  [6 bytes, skipped]
            + _w(0x7001)            # MOVEQ #1, D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 1

    def test_bra_wide(self):
        # BRA with 16-bit displacement
        # 0x6000 (disp=0 → 16-bit extension follows)
        prog = (
            _w(0x6000) + _w(0x0006)  # BRA #6 (16-bit)  [skips next 6 bytes]
            + _w(0x7063)              # MOVEQ #99, D0 (2 bytes)
            + _w(0x7063)              # MOVEQ #99, D0 (2 bytes; total 4)
            + _w(0x7063)              # MOVEQ #99, D0 (2 bytes; total 6)
            + _w(0x7001)              # MOVEQ #1, D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 1

    def test_beq_taken(self):
        prog = (
            _w(0x7000)              # MOVEQ #0, D0 → Z=1
            + _w(0x6706)            # BEQ #6  (skips next 6 bytes)
            + _w(0x203C) + _l(99)  # MOVE.L #99, D0 (6 bytes, skipped)
            + _w(0x7001)            # MOVEQ #1, D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 1

    def test_beq_not_taken(self):
        prog = (
            _w(0x7001)              # MOVEQ #1, D0 → Z=0
            + _w(0x6704)            # BEQ #4 (would skip MOVEQ #99, D0)
            + _w(0x7063)            # MOVEQ #99, D0  [NOT skipped]
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 99

    def test_bne_taken(self):
        # BNE opword at load+2; PC after opword = load+4;
        # displacement 2 → target = load+4+2 = load+6 (MOVEQ #2)
        # load+4: MOVEQ #99 (2 bytes, skipped); load+6: MOVEQ #2
        prog = (
            _w(0x7001)              # MOVEQ #1, D0 → Z=0
            + _w(0x6602)            # BNE #2  (taken: Z=0, skips MOVEQ #99)
            + _w(0x7063)            # MOVEQ #99, D0  [skipped]
            + _w(0x7002)            # MOVEQ #2, D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 2

    def test_bsr_and_rts(self):
        # Main: BSR sub  → calls sub  → RTS back → STOP
        # Layout at load (0x1000):
        #   0x1000: BSR #4 (jump to 0x1000+2+4 = 0x1006)
        #   0x1002: MOVEQ #1, D0    [runs after return]
        #   0x1004: STOP
        #   0x1006: MOVEQ #42, D1   [subroutine body]
        #   0x1008: RTS
        prog = (
            _w(0x6104)              # BSR #4 → call to load+2+4=load+6
            + _w(0x7001)            # MOVEQ #1, D0  [runs after return]
            + _stop()               # 0x1004–0x1005
            + _w(0x722A)            # MOVEQ #42, D1  [subroutine at 0x1006]
            + _w(0x4E75)            # RTS
        )
        sim = _run(prog)
        assert sim._d[1] == 42   # subroutine ran
        assert sim._d[0] == 1    # continuation ran

    def test_bcc_conditions(self):
        # Test BCC (carry clear) taken when C=0
        # BCC opword at load+2; PC after opword = load+4;
        # displacement 2 → target = load+4+2 = load+6 (MOVEQ #2)
        prog = (
            _w(0x7005)   # MOVEQ #5, D0  (C=0)
            + _w(0x6402)  # BCC #2 (taken: C=0, skips MOVEQ #99)
            + _w(0x7063)  # MOVEQ #99, D0 (skipped)
            + _w(0x7002)  # MOVEQ #2, D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 2

    def test_blt_taken(self):
        # BLT: N != V (signed less than)
        # 0 - 5 = -5 → N=1, V=0 → N != V → taken
        # BLT opword at load+8; PC after opword = load+10;
        # displacement 2 → target = load+12 (MOVEQ #7)
        prog = (
            _w(0x7000)              # MOVEQ #0, D0
            + _w(0x0C80) + _l(5)   # CMPI.L #5, D0  → N=1,V=0 → LT
            + _w(0x6D02)            # BLT #2 (taken, skips MOVEQ #99)
            + _w(0x7063)            # MOVEQ #99, D0 (skipped)
            + _w(0x7007)            # MOVEQ #7, D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 7


# ─────────────────────────────────────────────────────────────────────────────
# DBcc
# ─────────────────────────────────────────────────────────────────────────────

class TestDBcc(unittest.TestCase):

    def test_dbf_loop(self):
        # DBF D0, #disp — loops D0+1 times (condition always false → always dec/branch)
        # D0 = 2: loop runs 3 times (counter goes 2→1→0→-1, fall through)
        # Layout at load:
        #   load+0:  MOVEQ #2, D0   (2 bytes) — 0x7002
        #   load+2:  MOVEQ #0, D2   (2 bytes)
        #   load+4:  ADDQ.W #2, D2  (2 bytes) ← loop start
        #   load+6:  DBF D0 opword  (2 bytes) — 0x51C8
        #   load+8:  DBF extension  (2 bytes) ← pc_before_ext
        # target = load+8 + (-4) = load+4 ✓
        prog = (
            _w(0x7002)              # MOVEQ #2, D0  (loop counter, D0 not D1!)
            + _w(0x7400)            # MOVEQ #0, D2  (accumulator)
            + _w(0x5442)            # ADDQ.W #2, D2  (add 2 per iter)
            + _w(0x51C8) + _w(0xFFFC)  # DBF D0, #-4 → target=load+8-4=load+4
            + _stop()
        )
        sim = _run(prog)
        # 3 iterations × +2 = 6
        assert sim._d[2] == 6

    def test_dbt_never_branches(self):
        # DBT: condition = T (always true) → condition never false → never dec/branch
        prog = (
            _w(0x7003)              # MOVEQ #3, D0
            + _w(0x50C8) + _w(0xFFFC)  # DBT D0, #-4  (never taken)
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 3   # D0 unchanged (DBT always falls through)


# ─────────────────────────────────────────────────────────────────────────────
# LINK / UNLK
# ─────────────────────────────────────────────────────────────────────────────

class TestLinkUnlk(unittest.TestCase):

    def test_link_unlk(self):
        # execute() calls reset() first, so post-reset state is:
        #   A6 = 0, A7 (SP) = 0x00F000
        # LINK A6, #-8: push old A6 (0) onto stack, set A6=SP, SP-=8
        # UNLK A6: SP = A6 (restores SP), A6 = pop() (restores 0)
        # Net result: A6 and SP both restored to their post-reset values.
        prog = (
            _w(0x4E56) + _w(0xFFF8)   # LINK A6, #-8
            + _w(0x4E5E)               # UNLK A6
            + _stop()
        )
        sim = M68KSimulator()
        sim.execute(prog)
        state = sim.get_state()
        assert state.a6 == 0           # A6 restored to post-reset value
        assert state.a7 == 0x00F000    # SP restored to post-reset value


# ─────────────────────────────────────────────────────────────────────────────
# LEA / PEA
# ─────────────────────────────────────────────────────────────────────────────

class TestLeaPea(unittest.TestCase):

    def test_lea_disp(self):
        # LEA 8(A0), A1: A1 = A0 + 8
        # LEA d16(An),Am: 0100 am 1 11 101 an  + d16
        # am=1 (A1), an=0 (A0): 0100 001 1 11 101 000 = 0x43E8
        prog = (
            _w(0x207C) + _l(0x2000)   # MOVEA.L #0x2000, A0
            + _w(0x43E8) + _w(8)       # LEA 8(A0), A1
            + _stop()
        )
        sim = _run(prog)
        assert sim._a[1] == 0x2008

    def test_pea(self):
        # PEA (A0): pushes A0 onto stack
        prog = (
            _w(0x207C) + _l(0x3000)   # MOVEA.L #0x3000, A0
            + _w(0x4850)               # PEA (A0)
            + _stop()
        )
        sim = _run(prog)
        # SP decremented by 4, value at new SP = 0x3000
        top = sim._mem_read_long(sim._a[7])
        assert top == 0x3000


# ─────────────────────────────────────────────────────────────────────────────
# JMP / JSR
# ─────────────────────────────────────────────────────────────────────────────

class TestJmpJsr(unittest.TestCase):

    def test_jmp_indirect(self):
        # JMP (A0): jump to address in A0
        # JMP (An): 0100 1110 11 010 an = 0x4ED0 | an
        # Place a MOVEQ #42, D0 + STOP at 0x2000
        # We need instructions at both load (0x1000) and at 0x2000.
        # 0x2000 is 0x1000 bytes past load, so allocate 4098 bytes:
        prog = bytearray(4098)
        instr = _w(0x207C) + _l(0x2000) + _w(0x4ED0)
        prog[:len(instr)] = instr
        # At offset 0x1000 from load (= physical 0x2000):
        target_instr = _w(0x702A) + _stop()   # MOVEQ #42, D0; STOP
        prog[0x1000:0x1000 + len(target_instr)] = target_instr

        sim = M68KSimulator()
        result = sim.execute(bytes(prog))
        assert result.ok
        assert result.final_state.d0 == 42

    def test_jsr_and_rts(self):
        # JSR (A0): pushes return address, jumps to A0
        prog = bytearray(4098)
        # At 0x1000 (offset 0): MOVEA.L #0x2000, A0; JSR (A0); MOVEQ #1, D0; STOP
        instr = _w(0x207C) + _l(0x2000) + _w(0x4E90) + _w(0x7001) + _stop()
        prog[:len(instr)] = instr
        # Subroutine at 0x2000 (offset 0x1000): MOVEQ #77, D1; RTS
        sub = _w(0x724D) + _w(0x4E75)
        prog[0x1000:0x1000 + len(sub)] = sub

        sim = M68KSimulator()
        result = sim.execute(bytes(prog))
        assert result.ok
        assert result.final_state.d0 == 1
        assert result.final_state.d1 == 77


# ─────────────────────────────────────────────────────────────────────────────
# ADDX / SUBX (extended arithmetic)
# ─────────────────────────────────────────────────────────────────────────────

class TestExtendedArith(unittest.TestCase):

    def test_addx_no_carry_in(self):
        # ADDX.L D1, D0 with X=0: same as ADD.L
        # 1101 000 1 10 00 0 001 = 0xD181
        prog = (
            _w(0x44FC) + _w(0x00)   # MOVE #0, CCR  (clear X)
            + _w(0x7003)             # MOVEQ #3, D0
            + _w(0x7204)             # MOVEQ #4, D1
            + _w(0xD181)             # ADDX.L D1, D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 7

    def test_addx_with_carry_in(self):
        # ADDX.L D1, D0 with X=1: adds an extra 1
        prog = (
            _w(0x44FC) + _w(0x10)   # MOVE #0x10, CCR  (set X)
            + _w(0x7003)             # MOVEQ #3, D0
            + _w(0x7204)             # MOVEQ #4, D1
            + _w(0xD181)             # ADDX.L D1, D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 8   # 3 + 4 + 1(X) = 8

    def test_subx_with_borrow(self):
        # SUBX.L D1, D0 with X=1: D0 = D0 - D1 - 1
        # 1001 000 1 10 00 0 001 = 0x9181
        prog = (
            _w(0x44FC) + _w(0x10)   # MOVE #0x10, CCR  (set X)
            + _w(0x700A)             # MOVEQ #10, D0
            + _w(0x7203)             # MOVEQ #3, D1
            + _w(0x9181)             # SUBX.L D1, D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 6   # 10 - 3 - 1 = 6


# ─────────────────────────────────────────────────────────────────────────────
# NEG with X semantics / NEGX
# ─────────────────────────────────────────────────────────────────────────────

class TestNEGX(unittest.TestCase):

    def test_negx_zero(self):
        # NEGX.L D0 with X=0 and D0=0: 0-0-0 = 0, C=0, Z preserved
        prog = (
            _w(0x44FC) + _w(0x04)   # MOVE #4, CCR (Z=1, X=0)
            + _w(0x7000)             # MOVEQ #0, D0  (sets Z=1 again)
            + _w(0x4080)             # NEGX.L D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 0
        # NEGX: Z not set if result≠0; Z preserved if result=0 and was already Z
        assert sim._sr & 4   # Z maintained


# ─────────────────────────────────────────────────────────────────────────────
# RTR (return and restore CCR)
# ─────────────────────────────────────────────────────────────────────────────

class TestRTR(unittest.TestCase):

    def test_rtr_restores_ccr(self):
        # RTR pops CCR word then PC; verify CCR is restored from stack.
        # RTR pops word for CCR then long for PC — set up the stack manually
        # so we control the exact CCR/PC values without the BSR complexity.
        sim = M68KSimulator()
        sim.reset()
        sim.load(b"")
        # Manually set up: push CCR word (0x0004 = Z=1) then push return PC (0x1010)
        # Note: RTR pops word for CCR then long for PC
        sim._push_long(0x001010)   # return address
        sim._push_word(0x0004)     # CCR word (Z=1)
        sim._mem[0x1010] = 0x4E   # STOP high byte
        sim._mem[0x1011] = 0x72
        sim._mem[0x1012] = 0x27
        sim._mem[0x1013] = 0x00
        sim._pc = _LOAD
        sim._mem[_LOAD]     = 0x4E
        sim._mem[_LOAD + 1] = 0x77   # RTR
        sim.step()
        assert sim._sr & 4   # Z set (from popped CCR)
        assert sim._pc == 0x001010


# ─────────────────────────────────────────────────────────────────────────────
# Scc
# ─────────────────────────────────────────────────────────────────────────────

class TestScc(unittest.TestCase):

    def test_seq_true(self):
        # SEQ D0 when Z=1 → D0.B = 0xFF
        # SEQ Dn: 0101 0111 11 000 dn = 0x57C0 | dn
        prog = (
            _w(0x7000)   # MOVEQ #0, D0 → Z=1
            + _w(0x57C0)  # SEQ D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] & 0xFF == 0xFF

    def test_seq_false(self):
        prog = (
            _w(0x7001)   # MOVEQ #1, D0 → Z=0
            + _w(0x57C0)  # SEQ D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] & 0xFF == 0x00


# ─────────────────────────────────────────────────────────────────────────────
# EXG
# ─────────────────────────────────────────────────────────────────────────────

class TestEXG(unittest.TestCase):

    def test_exg_dn_dm(self):
        # EXG D0, D1: 1100 000 1 01000 001 = 0xC141
        prog = (
            _w(0x7005)   # MOVEQ #5, D0
            + _w(0x720A)  # MOVEQ #10, D1
            + _w(0xC141)  # EXG D0, D1
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 10
        assert sim._d[1] == 5

    def test_exg_an_am(self):
        # EXG A0, A1: 1100 000 1 01001 001 = 0xC149
        prog = (
            _w(0x207C) + _l(0x1000)   # MOVEA.L #0x1000, A0
            + _w(0x227C) + _l(0x2000)  # MOVEA.L #0x2000, A1
            + _w(0xC149)               # EXG A0, A1
            + _stop()
        )
        sim = _run(prog)
        assert sim._a[0] == 0x2000
        assert sim._a[1] == 0x1000

    def test_exg_dn_an(self):
        # EXG D0, A1: 1100 000 1 10001 001 = 0xC189
        prog = (
            _w(0x7042)                 # MOVEQ #66, D0
            + _w(0x227C) + _l(0x9999)  # MOVEA.L #0x9999, A1
            + _w(0xC189)               # EXG D0, A1
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 0x9999
        assert sim._a[1] == 66


# ─────────────────────────────────────────────────────────────────────────────
# MOVE SR / CCR
# ─────────────────────────────────────────────────────────────────────────────

class TestSRCCR(unittest.TestCase):

    def test_move_sr_to_dn(self):
        prog = (
            _w(0x44FC) + _w(0x0005)   # MOVE #5, CCR  (C=1, V=0, Z=0, N=0... wait: 5 = 101 = C|V)
            # Actually CCR bits: 4=X,3=N,2=Z,1=V,0=C. 5 = 101 = X=0,N=0,Z=1,V=0,C=1... no
            # 5 = 0b00101: bit2=Z=1, bit0=C=1
            + _w(0x40C0)               # MOVE SR, D0
            + _stop()
        )
        sim = _run(prog)
        # SR = (system_byte | 5) = 0x2700 | 0x05 = 0x2705
        assert sim._d[0] & 0xFFFF == 0x2705

    def test_move_ccr_to_dn(self):
        prog = (
            _w(0x44FC) + _w(0x001C)   # MOVE #0x1C, CCR (X=N=Z=1, V=C=0... let me compute)
            # 0x1C = 0b11100: X=1,N=1,Z=1,V=0,C=0
            + _w(0x42C0)               # MOVE CCR, D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] & 0x1F == 0x1C

    def test_move_imm_sr(self):
        prog = (
            _w(0x46FC) + _w(0x2715)   # MOVE #0x2715, SR
            + _stop()
        )
        sim = _run(prog)
        # SR has X (0x10) | Z (0x04) | C (0x01) = 0x15 in CCR
        assert sim._sr == 0x2715


# ─────────────────────────────────────────────────────────────────────────────
# Bit operations
# ─────────────────────────────────────────────────────────────────────────────

class TestBitOps(unittest.TestCase):

    def test_btst_set(self):
        prog = (
            _w(0x203C) + _l(0x00000004)   # MOVE.L #4 (bit 2 set), D0
            + _w(0x0800) + _w(0x0002)     # BTST #2, D0
            + _stop()
        )
        sim = _run(prog)
        assert not (sim._sr & 4)   # Z clear (bit WAS set, so Z=0)

    def test_btst_clear(self):
        prog = (
            _w(0x203C) + _l(0x00000000)
            + _w(0x0800) + _w(0x0002)     # BTST #2, D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._sr & 4   # Z set (bit was clear)

    def test_bset_imm(self):
        prog = (
            _w(0x7000)                    # MOVEQ #0, D0
            + _w(0x08C0) + _w(0x0003)     # BSET #3, D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 8   # bit 3 set

    def test_bclr_imm(self):
        prog = (
            _w(0x700F)                    # MOVEQ #15, D0  (bits 0-3 set)
            + _w(0x0880) + _w(0x0002)     # BCLR #2, D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 11   # bit 2 cleared: 15 - 4 = 11

    def test_bchg_imm(self):
        prog = (
            _w(0x700F)
            + _w(0x0840) + _w(0x0001)     # BCHG #1, D0  (toggles bit 1)
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 13   # 15 ^ 2 = 13


# ─────────────────────────────────────────────────────────────────────────────
# ORI/ANDI/EORI to CCR
# ─────────────────────────────────────────────────────────────────────────────

class TestImmCCR(unittest.TestCase):

    def test_ori_ccr(self):
        prog = (
            _w(0x44FC) + _w(0x00)   # MOVE #0, CCR
            + _w(0x003C) + _w(0x05)  # ORI #5, CCR  (set C and Z)
            + _stop()
        )
        sim = _run(prog)
        assert sim._sr & 5 == 5   # C and Z set

    def test_andi_ccr(self):
        prog = (
            _w(0x44FC) + _w(0x1F)   # MOVE #0x1F, CCR  (all set)
            + _w(0x023C) + _w(0x14)  # ANDI #0x14, CCR  (keep X and Z only)
            + _stop()
        )
        sim = _run(prog)
        assert sim._sr & 0x1F == 0x14

    def test_eori_ccr(self):
        prog = (
            _w(0x44FC) + _w(0x0F)   # MOVE #0x0F, CCR
            + _w(0x0A3C) + _w(0x0F)  # EORI #0x0F, CCR  → 0
            + _stop()
        )
        sim = _run(prog)
        assert (sim._sr & 0x1F) == 0


if __name__ == "__main__":
    unittest.main()
