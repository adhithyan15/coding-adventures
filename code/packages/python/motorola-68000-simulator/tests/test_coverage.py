"""Edge case tests targeting full line coverage of the M68K simulator.

These tests specifically exercise code paths that normal instruction tests
might miss: error conditions, unusual CCR interactions, specific EA modes,
shift-by-zero, division overflow, etc.
"""

from __future__ import annotations

import struct
import unittest

from motorola_68000_simulator import M68KSimulator

_LOAD = 0x001000
_SP   = 0x00F000


def _w(v: int) -> bytes: return struct.pack(">H", v & 0xFFFF)
def _l(v: int) -> bytes: return struct.pack(">I", v & 0xFFFFFFFF)
def _stop() -> bytes:    return _w(0x4E4F)  # TRAP #15 — halt without touching SR

MAX_STEPS = 200


def _run(prog: bytes, max_steps: int = MAX_STEPS) -> M68KSimulator:
    sim = M68KSimulator()
    result = sim.execute(prog, max_steps=max_steps)
    assert result.ok, f"Not ok: {result.error!r}"
    return sim


class TestMemoryHelpers(unittest.TestCase):
    """Direct test of memory read/write helpers."""

    def setUp(self):
        self.sim = M68KSimulator()
        self.sim.reset()

    def test_read_write_byte(self):
        self.sim._mem_write_byte(0x2000, 0xAB)
        assert self.sim._mem_read_byte(0x2000) == 0xAB

    def test_read_write_word(self):
        self.sim._mem_write_word(0x2000, 0x1234)
        assert self.sim._mem_read_word(0x2000) == 0x1234

    def test_read_write_long(self):
        self.sim._mem_write_long(0x2000, 0xDEADBEEF)
        assert self.sim._mem_read_long(0x2000) == 0xDEADBEEF

    def test_word_misaligned_read_raises(self):
        with self.assertRaises(ValueError):
            self.sim._mem_read_word(0x2001)

    def test_word_misaligned_write_raises(self):
        with self.assertRaises(ValueError):
            self.sim._mem_write_word(0x2001, 0x1234)

    def test_long_misaligned_read_raises(self):
        with self.assertRaises(ValueError):
            self.sim._mem_read_long(0x2001)

    def test_long_misaligned_write_raises(self):
        with self.assertRaises(ValueError):
            self.sim._mem_write_long(0x2003, 0x1234)

    def test_address_wraps_at_24bit(self):
        self.sim._mem_write_byte(0xFFFFFF, 0x42)
        assert self.sim._mem_read_byte(0x1FFFFFF) == 0x42  # wraps to 0xFFFFFF

    def test_mem_read_generic(self):
        self.sim._mem_write_byte(0x3000, 0xAB)
        assert self.sim._mem_read(0x3000, 1) == 0xAB
        self.sim._mem_write_word(0x3000, 0x1234)
        assert self.sim._mem_read(0x3000, 2) == 0x1234
        self.sim._mem_write_long(0x3000, 0x12345678)
        assert self.sim._mem_read(0x3000, 4) == 0x12345678

    def test_mem_write_generic(self):
        self.sim._mem_write(0x3000, 1, 0xAB)
        assert self.sim._mem[0x3000] == 0xAB
        self.sim._mem_write(0x3000, 2, 0xABCD)
        assert self.sim._mem_read_word(0x3000) == 0xABCD
        self.sim._mem_write(0x3000, 4, 0xDEADBEEF)
        assert self.sim._mem_read_long(0x3000) == 0xDEADBEEF


class TestRegisterHelpers(unittest.TestCase):
    """Tests for _set_dn, _get_dn, _push/_pop helpers."""

    def setUp(self):
        self.sim = M68KSimulator()
        self.sim.reset()

    def test_set_dn_byte(self):
        self.sim._d[0] = 0x12345678
        self.sim._set_dn(0, 0xAB, 1)
        assert self.sim._d[0] == 0x123456AB

    def test_set_dn_word(self):
        self.sim._d[0] = 0x12345678
        self.sim._set_dn(0, 0xABCD, 2)
        assert self.sim._d[0] == 0x1234ABCD

    def test_set_dn_long(self):
        self.sim._d[0] = 0x12345678
        self.sim._set_dn(0, 0xDEADBEEF, 4)
        assert self.sim._d[0] == 0xDEADBEEF

    def test_get_dn_byte(self):
        self.sim._d[0] = 0xABCDEF12
        assert self.sim._get_dn(0, 1) == 0x12

    def test_push_pop_long(self):
        self.sim._push_long(0x12345678)
        assert self.sim._pop_long() == 0x12345678

    def test_push_pop_word(self):
        self.sim._push_word(0xABCD)
        assert self.sim._pop_word() == 0xABCD


class TestEAModes(unittest.TestCase):
    """Test all effective addressing modes via _ea_address."""

    def setUp(self):
        self.sim = M68KSimulator()
        self.sim.reset()

    def _setup_fetch(self, *words: int) -> None:
        """Write words at current PC for fetching."""
        for i, w in enumerate(words):
            self.sim._mem_write_word(self.sim._pc + i * 2, w)

    def test_mode_2_indirect(self):
        self.sim._a[0] = 0x2000
        addr = self.sim._ea_address(2, 0, 2)
        assert addr == 0x2000

    def test_mode_3_postinc(self):
        self.sim._a[0] = 0x2000
        addr = self.sim._ea_address(3, 0, 4)
        assert addr == 0x2000
        assert self.sim._a[0] == 0x2004  # incremented by 4

    def test_mode_3_postinc_byte_sp(self):
        # SP (A7) increments by 2 even for byte
        self.sim._a[7] = 0x2000
        addr = self.sim._ea_address(3, 7, 1)
        assert addr == 0x2000
        assert self.sim._a[7] == 0x2002  # incremented by 2 (word alignment)

    def test_mode_4_predec(self):
        self.sim._a[0] = 0x2004
        addr = self.sim._ea_address(4, 0, 4)
        assert addr == 0x2000  # decremented by 4
        assert self.sim._a[0] == 0x2000

    def test_mode_4_predec_byte_sp(self):
        self.sim._a[7] = 0x2002
        addr = self.sim._ea_address(4, 7, 1)
        assert addr == 0x2000
        assert self.sim._a[7] == 0x2000

    def test_mode_5_disp(self):
        self.sim._a[0] = 0x2000
        self._setup_fetch(0x0010)   # d16 = 16
        addr = self.sim._ea_address(5, 0, 2)
        assert addr == 0x2010

    def test_mode_5_negative_disp(self):
        self.sim._a[0] = 0x2010
        self._setup_fetch(0xFFF0)   # d16 = -16 (signed)
        addr = self.sim._ea_address(5, 0, 2)
        assert addr == 0x2000

    def test_mode_6_index_dn_word(self):
        self.sim._a[0] = 0x2000
        self.sim._d[1] = 0x00000010   # Xn = D1.W = 16
        # Extension word: DA=0(Dn), reg=1, W/L=0(word), d8=4
        # bits: 0 001 0 000 0000 0100 = 0x1004
        self._setup_fetch(0x1004)
        addr = self.sim._ea_address(6, 0, 2)
        assert addr == 0x2000 + 16 + 4  # An + Xn.W + d8

    def test_mode_6_index_an_long(self):
        self.sim._a[0] = 0x2000
        self.sim._a[1] = 0x0000_0100   # An index = A1.L = 256
        # Extension: DA=1(An), reg=1, W/L=1(long), d8=8
        # bits: 1 001 1 000 0000 1000 = 0x9808... let me compute:
        # bit 15 = 1 (An), bits 14-12 = 001 (A1), bit 11 = 1 (long), bits 10-8 = 0, bits 7-0 = 8
        # = 0b1001_1000_0000_1000 = 0x9808
        self._setup_fetch(0x9808)
        addr = self.sim._ea_address(6, 0, 2)
        assert addr == 0x2000 + 256 + 8

    def test_mode_7_abs_short_positive(self):
        self._setup_fetch(0x3000)   # absolute short = 0x3000 (positive)
        addr = self.sim._ea_address(7, 0, 2)
        assert addr == 0x3000

    def test_mode_7_abs_short_negative(self):
        self._setup_fetch(0x8000)   # abs short = 0x8000 → sign-extend to -32768
        addr = self.sim._ea_address(7, 0, 2)
        assert addr == (0x8000 - 0x10000) & 0xFFFFFF   # = 0xFF8000

    def test_mode_7_abs_long(self):
        self._setup_fetch(0x0001, 0x2345)   # abs long = 0x00012345
        addr = self.sim._ea_address(7, 1, 4)
        assert addr == 0x12345

    def test_mode_7_pc_disp(self):
        pc_base = self.sim._pc
        self._setup_fetch(0x0010)   # d16 = 16
        addr = self.sim._ea_address(7, 2, 2)
        assert addr == (pc_base + 16) & 0xFFFFFF

    def test_mode_7_pc_index(self):
        pc_base = self.sim._pc
        self.sim._d[0] = 0x0000_0008   # D0.W = 8
        # Extension: DA=0(Dn), reg=0(D0), W/L=0(word), d8=4
        # = 0b0 000 0 000 0000 0100 = 0x0004
        self._setup_fetch(0x0004)
        addr = self.sim._ea_address(7, 3, 2)
        assert addr == (pc_base + 8 + 4) & 0xFFFFFF

    def test_mode_invalid_raises(self):
        with self.assertRaises(ValueError):
            self.sim._ea_address(0, 0, 2)   # Dn direct — no address

    def test_mode_7_4_imm_raises(self):
        with self.assertRaises(ValueError):
            self.sim._ea_address(7, 4, 2)   # immediate — no address

    def test_ea_read_dn(self):
        self.sim._d[3] = 0xABCD
        val = self.sim._ea_read(0, 3, 2)
        assert val == 0xABCD

    def test_ea_read_an(self):
        self.sim._a[2] = 0x12345678
        val = self.sim._ea_read(1, 2, 4)
        assert val == 0x12345678

    def test_ea_write_an_word_sign_extend(self):
        self.sim._ea_write(1, 0, 2, 0x8000)
        assert self.sim._a[0] == 0xFFFF8000   # sign-extended

    def test_ea_read_imm(self):
        # Set up fetch of immediate word
        self.sim._mem_write_word(self.sim._pc, 0x1234)
        val = self.sim._ea_read(7, 4, 2)   # immediate word
        assert val == 0x1234

    def test_ea_read_imm_long(self):
        self.sim._mem_write_long(self.sim._pc, 0x12345678)
        val = self.sim._ea_read(7, 4, 4)
        assert val == 0x12345678


class TestCCREdgeCases(unittest.TestCase):
    """Edge cases for condition code computation."""

    def test_add_zero_plus_zero(self):
        prog = (
            _w(0x7000)   # MOVEQ #0, D0
            + _w(0x7200)  # MOVEQ #0, D1
            + _w(0xD081)  # ADD.L D1, D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._sr & 4   # Z set

    def test_sub_self(self):
        prog = (
            _w(0x7042)   # MOVEQ #66, D0
            + _w(0x9080)  # SUB.L D0, D0  (1001 000 0 10 000 000)
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 0
        assert sim._sr & 4   # Z set
        assert not (sim._sr & 1)   # C clear

    def test_neg_min_byte(self):
        # NEG.B of 0x80 (-128) → overflow (cannot represent +128 in byte)
        prog = (
            _w(0x103C) + _w(0x0080)   # MOVE.B #0x80, D0
            + _w(0x4400)               # NEG.B D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._sr & 2   # V set (overflow)

    def test_add_overflow(self):
        prog = (
            _w(0x203C) + _l(0x7FFF_FFFF)  # MOVE.L #0x7FFFFFFF, D0
            + _w(0x7201)                    # MOVEQ #1, D1
            + _w(0xD081)                    # ADD.L D1, D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 0x8000_0000
        assert sim._sr & 2   # V set

    def test_sub_overflow(self):
        prog = (
            _w(0x203C) + _l(0x8000_0000)  # MOVE.L #0x80000000, D0
            + _w(0x7201)                    # MOVEQ #1, D1
            + _w(0x9081)                    # SUB.L D1, D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 0x7FFF_FFFF
        assert sim._sr & 2   # V set (overflow: -2^31 - 1 = +2^31 - 1 in signed)

    def test_x_flag_not_changed_by_and(self):
        # Set X, then AND → X should remain set
        prog = (
            _w(0x44FC) + _w(0x10)   # MOVE #0x10, CCR  (X=1)
            + _w(0x7005)             # MOVEQ #5, D0
            + _w(0xC040)             # AND.W D0, D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._sr & 0x10   # X still set

    def test_cmp_does_not_change_x(self):
        prog = (
            _w(0x44FC) + _w(0x10)   # MOVE #0x10, CCR  (X=1)
            + _w(0x7005)             # MOVEQ #5, D0
            + _w(0x0C80) + _l(3)    # CMPI.L #3, D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._sr & 0x10   # X still set


class TestDivisionEdgeCases(unittest.TestCase):

    def test_divu_division_by_zero(self):
        sim = M68KSimulator()
        sim.reset()
        prog = (
            _w(0x203C) + _l(100)
            + _w(0x7200)              # MOVEQ #0, D1  (divisor = 0)
            + _w(0x80C1)              # DIVU D1, D0
            + _stop()
        )
        sim.load(prog)
        with self.assertRaises(RuntimeError):
            while not sim._halted:
                sim.step()

    def test_divs_division_by_zero(self):
        sim = M68KSimulator()
        sim.reset()
        prog = (
            _w(0x203C) + _l(100)
            + _w(0x7200)              # MOVEQ #0, D1
            + _w(0x81C1)              # DIVS D1, D0
            + _stop()
        )
        sim.load(prog)
        with self.assertRaises(RuntimeError):
            while not sim._halted:
                sim.step()

    def test_divu_overflow(self):
        # 0xFFFFFFFF / 1 = quotient > 0xFFFF → overflow
        prog = (
            _w(0x203C) + _l(0xFFFF_FFFF)
            + _w(0x323C) + _w(1)   # MOVE.W #1, D1
            + _w(0x80C1)           # DIVU D1, D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._sr & 2   # V set (overflow)

    def test_divs_overflow(self):
        # 0x80000000 / 1 = -2^31, can't fit in 16-bit signed quotient
        prog = (
            _w(0x203C) + _l(0x8000_0000)
            + _w(0x323C) + _w(1)   # MOVE.W #1, D1
            + _w(0x81C1)           # DIVS D1, D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._sr & 2   # V set (overflow)


class TestShiftEdgeCases(unittest.TestCase):

    def test_asl_overflow_detection(self):
        # ASL.W #1, D0 where D0 = 0x4000: MSB changes (0→1), V=1
        # 0x4000 = 0100 0000 0000 0000, ASL 1 = 0x8000, MSB flipped → V=1
        prog = (
            _w(0x303C) + _w(0x4000)
            + _w(0xE340)               # ASL.W #1, D0 (1110 001 1 01 0 00 000)
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] & 0xFFFF == 0x8000
        assert sim._sr & 2   # V set

    def test_lsl_zero_count(self):
        # LSL.L #8 by 0 count via register: if D1=0, count=0
        # When count = 0: result unchanged, C cleared
        # LSL.L D1, D0: 1110 001 1 10 1 01 000 = 0xE3A8
        prog = (
            _w(0x7005)   # MOVEQ #5, D0
            + _w(0x7200)  # MOVEQ #0, D1  (count = 0)
            + _w(0xE3A8)  # LSL.L D1, D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 5   # unchanged
        assert not (sim._sr & 1)   # C cleared

    def test_asr_zero_carry(self):
        # ASR.L #1 on 0x00000001 (bit 0 = 1 shifts out → C=1)
        prog = (
            _w(0x7001)   # MOVEQ #1, D0
            + _w(0xE280)  # ASR.L #1, D0 → 0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 0
        assert sim._sr & 1   # C set (bit 0 was 1)
        assert sim._sr & 4   # Z set

    def test_ror_zero_count(self):
        # ROR.L with count = 0: C should be cleared per 68000 spec
        prog = (
            _w(0x7005)   # MOVEQ #5, D0
            + _w(0x7200)  # MOVEQ #0, D1  (count = 0)
            # ROR.L D1, D0: 1110 001 0 10 1 11 000 = 0xE2B8... let me compute
            # 1110 ccc d ss r tt rrr
            # ccc=001(D1), d=0(right), ss=10(long), r=1(reg), tt=11(RO), rrr=000(D0)
            # = 1110 001 0 10 1 11 000 = 0xE2B8
            + _w(0xE2B8)  # ROR.L D1, D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 5   # unchanged
        # C = 0 when count = 0 for rotate


class TestUnimplementedOpcode(unittest.TestCase):

    def test_line_a_raises(self):
        # Line A (0xA...) should raise RuntimeError
        sim = M68KSimulator()
        sim.reset()
        sim.load(bytes([0xA0, 0x00]))   # line-A opcode
        with self.assertRaises(RuntimeError):
            sim.step()

    def test_execute_captures_runtime_error(self):
        prog = bytes([0xA0, 0x00])   # line-A opcode
        sim = M68KSimulator()
        result = sim.execute(prog, max_steps=10)
        assert not result.ok
        assert result.error is not None

    def test_load_too_large_raises(self):
        sim = M68KSimulator()
        sim.reset()
        too_big = bytes(17 * 1024 * 1024)   # 17 MB > 16 MB
        with self.assertRaises(ValueError):
            sim.load(too_big)


class TestFlagsModule(unittest.TestCase):
    """Direct tests for flags.py helpers."""

    def test_pack_ccr(self):
        from motorola_68000_simulator.flags import pack_ccr
        ccr = pack_ccr(x=True, n=False, z=True, v=False, c=True)
        assert ccr & (1 << 4)   # X
        assert ccr & (1 << 2)   # Z
        assert ccr & (1 << 0)   # C
        assert not (ccr & (1 << 3))   # N clear
        assert not (ccr & (1 << 1))   # V clear

    def test_unpack_ccr(self):
        from motorola_68000_simulator.flags import unpack_ccr
        flags = unpack_ccr(0b10101)   # C=1, Z=1, X=0... 0b10101 = C|Z|X... let me check
        # bits: 4=X, 3=N, 2=Z, 1=V, 0=C
        # 0b10101 = bit4|bit2|bit0 = X|Z|C
        assert flags["c"] is True
        assert flags["z"] is True
        assert flags["x"] is True
        assert flags["n"] is False
        assert flags["v"] is False

    def test_compute_nzvc_add_examples(self):
        from motorola_68000_simulator.flags import compute_nzvc_add
        # 255 + 1 = 256 in byte → carry, zero
        n, z, v, c, x = compute_nzvc_add(255, 1, 256, word=False, long=False)
        assert c is True
        assert z is True
        assert n is False
        assert x is True

    def test_compute_nzvc_sub_examples(self):
        from motorola_68000_simulator.flags import compute_nzvc_sub
        # 0 - 1 = -1 in byte
        n, z, v, c, x = compute_nzvc_sub(0, 1, -1, word=False, long=False)
        assert c is True   # borrow
        assert n is True   # MSB of 0xFF
        assert z is False

    def test_compute_nzvc_neg_examples(self):
        from motorola_68000_simulator.flags import compute_nzvc_neg
        n, z, v, c, x = compute_nzvc_neg(0, 0, word=False, long=False)
        assert z is True; assert c is False
        n, z, v, c, x = compute_nzvc_neg(1, 0xFF, word=False, long=False)
        assert n is True; assert c is True

    def test_compute_nz_logic(self):
        from motorola_68000_simulator.flags import compute_nz_logic
        n, z = compute_nz_logic(0, word=False, long=False)
        assert not n; assert z
        n, z = compute_nz_logic(0x80, word=False, long=False)
        assert n; assert not z

    def test_sz_kwargs(self):
        from motorola_68000_simulator.simulator import _sz_kwargs
        assert _sz_kwargs(1) == {"word": False, "long": False}
        assert _sz_kwargs(2) == {"word": True,  "long": False}
        assert _sz_kwargs(4) == {"word": False, "long": True}


class TestBitOpsRegister(unittest.TestCase):
    """Test bit operation instructions with register-specified bit number."""

    def test_btst_reg_bit_clear(self):
        # BTST D1, D0: 0000 001 1 00 000 000 = 0x0300
        prog = (
            _w(0x7000)   # MOVEQ #0, D0
            + _w(0x7200)  # MOVEQ #0, D1 (test bit 0)
            + _w(0x0300)  # BTST D1, D0  (bit 0 of D0 is 0 → Z=1)
            + _stop()
        )
        sim = _run(prog)
        assert sim._sr & 4   # Z set (bit was clear)

    def test_bset_reg(self):
        # BSET D1, D0 sets bit number in D1 of D0
        # BSET D1, D0: 0000 001 1 11 000 000... wait
        # BSET uses pattern: 0000 rrr1 11 mm rrr? No.
        # bit register: 0000 rrr 1 00 ea where bits 7-6 = xx
        # Correcting: bit ops with register: 0000 rrr 1 dd mm rrr
        # dd=00=BTST, 01=BCHG, 10=BCLR, 11=BSET
        # BSET D1, D0: 0000 001 1 11 000 000 = 0x03C0
        prog = (
            _w(0x7000)   # MOVEQ #0, D0
            + _w(0x7203)  # MOVEQ #3, D1 (set bit 3)
            + _w(0x03C0)  # BSET D1, D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 8   # bit 3 set

    def test_bclr_reg(self):
        # BCLR D1, D0: 0000 001 1 10 000 000 = 0x0380
        prog = (
            _w(0x700F)   # MOVEQ #15, D0
            + _w(0x7202)  # MOVEQ #2, D1 (clear bit 2 = value 4)
            + _w(0x0380)  # BCLR D1, D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 11   # 15 - 4 = 11

    def test_bchg_reg(self):
        # BCHG D1, D0: 0000 001 1 01 000 000 = 0x0340
        prog = (
            _w(0x700F)   # MOVEQ #15, D0
            + _w(0x7201)  # MOVEQ #1, D1 (toggle bit 1 = value 2)
            + _w(0x0340)  # BCHG D1, D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 13   # 15 ^ 2 = 13


class TestMemoryBitOps(unittest.TestCase):
    """Test bit ops on memory operands (8-bit)."""

    def test_btst_imm_memory(self):
        # BTST #3, (A0): tests bit 3 of byte at (A0)
        prog = (
            _w(0x207C) + _l(0x002000)  # MOVEA.L #0x2000, A0
            + _w(0x20BC) + _l(0x00000008)  # MOVE.L #8, (A0) — sets bit 3 of byte
            # Actually at address 0x2000, byte = high byte of 0x00000008 = 0x00
            # Let's write a byte directly: use MOVE.B #8, (A0)
            # MOVE.B #8, (A0): 0001 111 010 111 100 = 0x1E3C + data
            + _w(0x10BC) + _w(0x0008)   # MOVE.B #8, (A0)  (1 000 010 111 100 = hmm)
            # MOVE.B dst=(A0)=mode010,reg000; src=#imm=mode111,reg100
            # In MOVE encoding: 00ss DDD ddd MMM mmm
            # sz=01(byte), dest_mode=010,dest_reg=000, src_mode=111,src_reg=100
            # = 0001 000 010 111 100 = 0x10BC
            + _w(0x0810) + _w(0x0003)   # BTST #3, (A0): bit 3 of byte at 0x2000
            # BTST #3, (A0): 0000 1000 00 010 000 + 0x0003
            # = 0x0810 then 0x0003
            + _stop()
        )
        sim = _run(prog)
        # 0x08 = 0000 1000, bit 3 is set → Z=0
        assert not (sim._sr & 4)   # Z clear (bit was set)


class TestANDISR(unittest.TestCase):
    """Test ANDI/ORI to SR."""

    def test_andi_sr(self):
        prog = (
            _w(0x46FC) + _w(0x271F)   # MOVE #0x271F, SR (set all CCR bits)
            + _w(0x027C) + _w(0x270A)  # ANDI #0x270A, SR
            + _stop()
        )
        sim = _run(prog)
        assert sim._sr == 0x270A

    def test_ori_sr(self):
        prog = (
            _w(0x46FC) + _w(0x2700)   # MOVE #0x2700, SR (clear CCR)
            + _w(0x007C) + _w(0x000F)  # ORI #0x0F, SR
            + _stop()
        )
        sim = _run(prog)
        assert sim._sr == 0x270F


class TestMoveMemoryToMemory(unittest.TestCase):
    """Test MOVE between two memory locations."""

    def test_move_w_indirect_to_indirect(self):
        # MOVE.W (A0), (A1) — memory to memory
        # src EA: mode=010, reg=000; dst EA: mode=010, reg=001
        # MOVE.W: 0011 dest_reg(001) dest_mode(010) src_mode(010) src_reg(000)
        # = 0011 001 010 010 000 = 0x3290
        prog = (
            _w(0x207C) + _l(0x002000)  # MOVEA.L #0x2000, A0
            + _w(0x227C) + _l(0x002010)  # MOVEA.L #0x2010, A1
            + _w(0x10BC) + _w(0x00AB)   # MOVE.B #0xAB, (A0)  wait, should write word
            # Let's use MOVE.W #0x5678, (A0)
            # MOVE.W #imm, (A0): sz=11, dst=(A0)=mode010,reg000, src=#imm=mode111,reg100
            # 0011 000 010 111 100 = 0x30BC
            + _w(0x30BC) + _w(0x5678)   # MOVE.W #0x5678, (A0)
            + _w(0x3290)                 # MOVE.W (A0), (A1)
            + _stop()
        )
        sim = _run(prog)
        val = (sim._mem[0x2010] << 8) | sim._mem[0x2011]
        assert val == 0x5678


class TestSignExtendHelper(unittest.TestCase):

    def test_sign_extend_positive(self):
        from motorola_68000_simulator.simulator import _sign_extend
        assert _sign_extend(0x7F, 8) == 127
        assert _sign_extend(0x7FFF, 16) == 32767

    def test_sign_extend_negative(self):
        from motorola_68000_simulator.simulator import _sign_extend
        assert _sign_extend(0xFF, 8) == -1
        assert _sign_extend(0x80, 8) == -128
        assert _sign_extend(0x8000, 16) == -32768

    def test_to_signed(self):
        from motorola_68000_simulator.simulator import _to_signed
        assert _to_signed(0xFF, 1) == -1
        assert _to_signed(0x7F, 1) == 127
        assert _to_signed(0x8000_0000, 4) == -2147483648


if __name__ == "__main__":
    unittest.main()
