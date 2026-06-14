"""Branch-coverage edge-case tests for PDP11Simulator.

These tests target specific paths not covered by the primary test modules:
mode 5/7 addressing, odd-address errors, byte-autoincrement PC/SP rules,
flag helpers, state properties, and the flags module doctests.
"""

from __future__ import annotations

import pytest

from pdp11_simulator import PDP11Simulator, PDP11State
from pdp11_simulator.flags import (
    compute_c_sub,
    compute_v_sub,
    nzvc_add,
    nzvc_logic,
    nzvc_sub,
    pack_psw,
)
from pdp11_simulator.state import INIT_SP, LOAD_ADDR, MEM_SIZE, PC, SP


def _w(value: int) -> bytes:
    return bytes([value & 0xFF, (value >> 8) & 0xFF])

HALT = _w(0x0000)

def _op(mode, reg): return (mode << 3) | reg
def _mov(sm, sr, dm, dr): return _w(0x1000 | (_op(sm, sr) << 6) | _op(dm, dr))
def _add(sm, sr, dm, dr): return _w(0x6000 | (_op(sm, sr) << 6) | _op(dm, dr))
def _clr(mode, reg):      return _w(0x0A00 | _op(mode, reg))
def _sob(reg, off):       return _w(0x7E00 | (reg << 6) | (off & 0x3F))
def _rts(reg):            return _w(0x0080 | reg)   # octal 000 200
def _jsr(link, mode, dst): return _w(0x0800 | (link << 6) | _op(mode, dst))


# ── State / flags module tests ────────────────────────────────────────────────

class TestFlagsModule:
    def test_pack_psw_all_set(self):
        assert pack_psw(True, True, True, True) == 0b1111

    def test_pack_psw_all_clear(self):
        assert pack_psw(False, False, False, False) == 0

    def test_nzvc_add_zero_result(self):
        n, z, v, c = nzvc_add(0xFF, 0x01, word=False)
        assert not n and z and not v and c

    def test_nzvc_sub_equal(self):
        n, z, v, c = nzvc_sub(5, 5, word=True)
        assert z and not n and not v and not c

    def test_nzvc_logic_msb_set(self):
        n, z, v, c = nzvc_logic(0x8000, word=True)
        assert n and not z and not v and not c

    def test_compute_v_sub_no_overflow(self):
        assert not compute_v_sub(5, 3, 2, word=False)

    def test_compute_c_sub_no_borrow(self):
        assert not compute_c_sub(10, 5)


class TestStateProperties:
    def _state(self, psw: int) -> PDP11State:
        return PDP11State(r=(0,)*8, psw=psw, halted=False, memory=tuple([0]*65536))

    def test_n_flag(self):
        assert self._state(0b1000).n
        assert not self._state(0b0111).n

    def test_z_flag(self):
        assert self._state(0b0100).z
        assert not self._state(0b1011).z

    def test_v_flag(self):
        assert self._state(0b0010).v
        assert not self._state(0b1101).v

    def test_c_flag(self):
        assert self._state(0b0001).c
        assert not self._state(0b1110).c

    def test_state_constants(self):
        assert MEM_SIZE == 65536
        assert LOAD_ADDR == 0x1000
        assert INIT_SP == 0xF000
        assert SP == 6
        assert PC == 7


class TestMemoryErrors:
    def test_odd_word_read_raises(self):
        sim = PDP11Simulator()
        sim.reset()
        with pytest.raises(ValueError, match="Odd address"):
            sim._read_word(0x1001)

    def test_odd_word_write_raises(self):
        sim = PDP11Simulator()
        sim.reset()
        with pytest.raises(ValueError, match="Odd address"):
            sim._write_word(0x1003, 0x1234)


class TestMode5AddressAutoDecrDeferred:
    def test_mode5_autodec_deferred(self):
        """Mode 5 (@-(Rn)): R-=2; EA=M[R]; operand=M[EA]."""
        # Set up:
        # M[0x2000] = 0x2100 (pointer stored here)
        # M[0x2100] = 0xBEEF (data)
        # R1 = 0x2002 (autodecrement → 0x2000)
        # MOV @-(R1), R0 → R1=0x2000; EA=M[0x2000]=0x2100; R0=M[0x2100]=0xBEEF
        sim = PDP11Simulator()
        sim.reset()
        sim._mem[0x2000] = 0x00; sim._mem[0x2001] = 0x21  # 0x2100 LE
        sim._mem[0x2100] = 0xEF; sim._mem[0x2101] = 0xBE  # 0xBEEF LE
        sim._r[1] = 0x2002

        # Load: MOV @-(R1), R0 = mode 5/R1 src, mode 0/R0 dst
        sim._mem[LOAD_ADDR]   = 0x00 | _op(5, 1)  # low byte
        sim._mem[LOAD_ADDR+1] = 0x14              # high byte: 0x1000 | (5<<9)|(1<<6) ...
        # Let me compute correctly:
        # MOV src dst = 0x1000 | (src_field << 6) | dst_field
        # src_field = _op(5,1) = 0b101_001 = 0x29
        # dst_field = _op(0,0) = 0
        # iw = 0x1000 | (0x29 << 6) | 0 = 0x1000 | 0xA40 = 0x1A40
        iw = 0x1000 | (_op(5, 1) << 6) | _op(0, 0)
        sim._mem[LOAD_ADDR]   = iw & 0xFF
        sim._mem[LOAD_ADDR+1] = (iw >> 8) & 0xFF
        sim._mem[LOAD_ADDR+2] = 0x00   # HALT
        sim._mem[LOAD_ADDR+3] = 0x00

        sim.step()   # MOV @-(R1), R0
        assert sim._r[0] == 0xBEEF
        assert sim._r[1] == 0x2000


class TestMode7IndexDeferred:
    def test_mode7_index_deferred(self):
        """Mode 7 (@X(Rn)): EA = M[Rn + disp]; R0 = M[EA]."""
        # R1 = 0x2000, disp = 4 → ptr_addr = 0x2004
        # M[0x2004] = 0x2200 (points to data)
        # M[0x2200] = 0x1234
        sim = PDP11Simulator()
        sim.reset()
        sim._mem[0x2004] = 0x00; sim._mem[0x2005] = 0x22  # 0x2200
        sim._mem[0x2200] = 0x34; sim._mem[0x2201] = 0x12  # 0x1234
        sim._r[1] = 0x2000

        # MOV @4(R1), R0 = mode 7/R1 src, mode 0/R0 dst, followed by disp=4
        iw = 0x1000 | (_op(7, 1) << 6) | _op(0, 0)
        sim._mem[LOAD_ADDR]   = iw & 0xFF
        sim._mem[LOAD_ADDR+1] = (iw >> 8) & 0xFF
        sim._mem[LOAD_ADDR+2] = 4    # displacement low
        sim._mem[LOAD_ADDR+3] = 0    # displacement high
        sim._mem[LOAD_ADDR+4] = 0    # HALT
        sim._mem[LOAD_ADDR+5] = 0

        sim.step()
        assert sim._r[0] == 0x1234


class TestByteAutoIncrPCRule:
    def test_byte_autoinc_sp_always_2(self):
        """MOVB (SP)+, R0 should increment SP by 2 (not 1) because SP=R6."""
        # Place a byte value at the stack and pop it with MOVB
        sim = PDP11Simulator()
        sim.reset()
        sim._r[6] = 0xF000
        sim._mem[0xF000] = 0x42   # byte value at SP
        sim._mem[0xF001] = 0x00

        # MOVB (R6)+, R0  = mode 2/R6 src, mode 0/R0 dst  (byte version = 0x9000 base)
        iw = 0x9000 | (_op(2, 6) << 6) | _op(0, 0)
        sim._mem[LOAD_ADDR]   = iw & 0xFF
        sim._mem[LOAD_ADDR+1] = (iw >> 8) & 0xFF
        sim._mem[LOAD_ADDR+2] = 0
        sim._mem[LOAD_ADDR+3] = 0

        sp_before = sim._r[6]
        sim.step()
        # SP should advance by 2 (not 1)
        assert sim._r[6] == sp_before + 2

    def test_byte_autoinc_normal_reg_steps_by_1(self):
        """MOVB (R0)+, R1 should increment R0 by 1 (byte step) for non-SP/PC registers."""
        sim = PDP11Simulator()
        sim.reset()
        sim._r[0] = 0x2000
        sim._mem[0x2000] = 0x55

        # MOVB (R0)+, R1 = mode2/R0 src, mode0/R1 dst (byte)
        iw = 0x9000 | (_op(2, 0) << 6) | _op(0, 1)
        sim._mem[LOAD_ADDR]   = iw & 0xFF
        sim._mem[LOAD_ADDR+1] = (iw >> 8) & 0xFF
        sim._mem[LOAD_ADDR+2] = 0
        sim._mem[LOAD_ADDR+3] = 0

        sim.step()
        assert sim._r[0] == 0x2001   # incremented by 1 only


class TestSOBEdgeCases:
    def test_sob_at_zero_no_branch(self):
        """SOB with R already 0 after decrement should not branch."""
        sim = PDP11Simulator()
        sim._r[1] = 1   # will decrement to 0
        sim._r[7] = LOAD_ADDR

        iw = 0x7E00 | (1 << 6) | 5   # SOB R1, offset=5
        sim._mem[LOAD_ADDR]   = iw & 0xFF
        sim._mem[LOAD_ADDR+1] = (iw >> 8) & 0xFF
        sim._mem[LOAD_ADDR+2] = 0   # HALT
        sim._mem[LOAD_ADDR+3] = 0

        sim.step()   # SOB: R1 → 0, no branch
        assert sim._r[1] == 0
        assert sim._r[7] == LOAD_ADDR + 2   # falls through to HALT


class TestExecuteErrorPath:
    def test_execute_with_bad_opcode(self):
        """execute() catches ValueError from bad opcode and sets error."""
        sim = PDP11Simulator()
        # JMP mode=0 (illegal) then HALT
        iw = 0x0040 | _op(0, 0)  # JMP mode 0 (illegal)
        prog = bytes([iw & 0xFF, (iw >> 8) & 0xFF]) + HALT
        result = sim.execute(prog)
        assert not result.ok
        assert result.error is not None


class TestRTSVariants:
    def test_rts_with_link_register_not_pc(self):
        """RTS R0: PC ← R0; R0 ← (SP)+."""
        sim = PDP11Simulator()
        sim.reset()
        # Set R0 = return target (0x2000)
        # Push 0xABCD onto stack as the old R0 value
        sim._r[0]  = 0x2000     # return address in R0
        sim._r[6]  = 0xEFFE     # SP - 2 already
        sim._mem[0xEFFE] = 0xCD; sim._mem[0xEFFF] = 0xAB   # 0xABCD on stack

        # Place RTS R0 at 0x1000
        sim._mem[LOAD_ADDR]   = 0x80   # RTS R0 = 0x0080
        sim._mem[LOAD_ADDR+1] = 0x00

        sim._r[7] = LOAD_ADDR
        sim.step()   # RTS R0

        assert sim._r[7] == 0x2000   # PC ← old R0
        assert sim._r[0] == 0xABCD   # R0 ← popped value
        assert sim._r[6] == 0xF000   # SP restored


class TestINCDECByteVariants:
    def test_incb_memory(self):
        sim = PDP11Simulator()
        sim.reset()
        sim._mem[0x2000] = 0x0A
        sim._r[1] = 0x2000

        iw = 0x8A80 | _op(1, 1)  # INCB (R1)
        sim._mem[LOAD_ADDR]   = iw & 0xFF
        sim._mem[LOAD_ADDR+1] = (iw >> 8) & 0xFF
        sim._mem[LOAD_ADDR+2] = 0
        sim._mem[LOAD_ADDR+3] = 0
        sim._r[7] = LOAD_ADDR

        sim.step()
        assert sim._mem[0x2000] == 0x0B

    def test_decb_memory(self):
        sim = PDP11Simulator()
        sim.reset()
        sim._mem[0x2000] = 0x0A
        sim._r[1] = 0x2000

        iw = 0x8AC0 | _op(1, 1)  # DECB (R1)
        sim._mem[LOAD_ADDR]   = iw & 0xFF
        sim._mem[LOAD_ADDR+1] = (iw >> 8) & 0xFF
        sim._mem[LOAD_ADDR+2] = 0
        sim._mem[LOAD_ADDR+3] = 0
        sim._r[7] = LOAD_ADDR

        sim.step()
        assert sim._mem[0x2000] == 0x09


class TestAdcSbcByteVariants:
    def test_adcb_with_carry(self):
        sim = PDP11Simulator()
        sim.reset()
        sim._r[0] = 0x000A
        sim._psw  = 0b0001   # C=1

        iw = 0x8B40 | _op(0, 0)  # ADCB R0
        sim._mem[LOAD_ADDR]   = iw & 0xFF
        sim._mem[LOAD_ADDR+1] = (iw >> 8) & 0xFF
        sim._mem[LOAD_ADDR+2] = 0
        sim._mem[LOAD_ADDR+3] = 0
        sim._r[7] = LOAD_ADDR

        sim.step()
        assert (sim._r[0] & 0xFF) == 0x0B

    def test_sbcb_with_carry(self):
        sim = PDP11Simulator()
        sim.reset()
        sim._r[0] = 0x000A
        sim._psw  = 0b0001   # C=1

        iw = 0x8B80 | _op(0, 0)  # SBCB R0
        sim._mem[LOAD_ADDR]   = iw & 0xFF
        sim._mem[LOAD_ADDR+1] = (iw >> 8) & 0xFF
        sim._mem[LOAD_ADDR+2] = 0
        sim._mem[LOAD_ADDR+3] = 0
        sim._r[7] = LOAD_ADDR

        sim.step()
        assert (sim._r[0] & 0xFF) == 0x09


class TestTstbCmpb:
    def test_tstb_memory(self):
        sim = PDP11Simulator()
        sim.reset()
        sim._mem[0x2000] = 0x00
        sim._r[1] = 0x2000

        iw = 0x8BC0 | _op(1, 1)  # TSTB (R1)
        sim._mem[LOAD_ADDR]   = iw & 0xFF
        sim._mem[LOAD_ADDR+1] = (iw >> 8) & 0xFF
        sim._mem[LOAD_ADDR+2] = 0
        sim._mem[LOAD_ADDR+3] = 0
        sim._r[7] = LOAD_ADDR

        sim.step()
        assert sim._psw & 0b0100   # Z=1

    def test_cmpb(self):
        sim = PDP11Simulator()
        sim.reset()
        sim._r[0] = 0x0005
        sim._r[1] = 0x0005

        # CMPB R0, R1 = 0xA000 | (src << 6) | dst
        iw = 0xA000 | (_op(0, 0) << 6) | _op(0, 1)
        sim._mem[LOAD_ADDR]   = iw & 0xFF
        sim._mem[LOAD_ADDR+1] = (iw >> 8) & 0xFF
        sim._mem[LOAD_ADDR+2] = 0
        sim._mem[LOAD_ADDR+3] = 0
        sim._r[7] = LOAD_ADDR

        sim.step()
        assert sim._psw & 0b0100   # Z=1 (equal)


class TestRolRorByteVariants:
    def test_rolb_register(self):
        sim = PDP11Simulator()
        sim.reset()
        sim._r[0] = 0x0001
        sim._psw  = 0b0000

        iw = 0x8C40 | _op(0, 0)  # ROLB R0  (octal 106100)
        sim._mem[LOAD_ADDR]   = iw & 0xFF
        sim._mem[LOAD_ADDR+1] = (iw >> 8) & 0xFF
        sim._mem[LOAD_ADDR+2] = 0
        sim._mem[LOAD_ADDR+3] = 0
        sim._r[7] = LOAD_ADDR

        sim.step()
        assert (sim._r[0] & 0xFF) == 0x02
        assert not (sim._psw & 1)   # C=0 (old bit7 of 0x01 = 0)

    def test_asrb_sign_fill(self):
        sim = PDP11Simulator()
        sim.reset()
        sim._r[0] = 0x0080   # byte = 0x80 = -128

        iw = 0x8C80 | _op(0, 0)  # ASRB R0  (octal 106200)
        sim._mem[LOAD_ADDR]   = iw & 0xFF
        sim._mem[LOAD_ADDR+1] = (iw >> 8) & 0xFF
        sim._mem[LOAD_ADDR+2] = 0
        sim._mem[LOAD_ADDR+3] = 0
        sim._r[7] = LOAD_ADDR

        sim.step()
        # ASRB 0x80: sign bit preserved → 0xC0
        assert (sim._r[0] & 0xFF) == 0xC0

    def test_aslb(self):
        sim = PDP11Simulator()
        sim.reset()
        sim._r[0] = 0x0040

        iw = 0x8CC0 | _op(0, 0)  # ASLB R0  (octal 106300)
        sim._mem[LOAD_ADDR]   = iw & 0xFF
        sim._mem[LOAD_ADDR+1] = (iw >> 8) & 0xFF
        sim._mem[LOAD_ADDR+2] = 0
        sim._mem[LOAD_ADDR+3] = 0
        sim._r[7] = LOAD_ADDR

        sim.step()
        assert (sim._r[0] & 0xFF) == 0x80
        assert not (sim._psw & 1)   # C=0 (old bit7 of 0x40 = 0)


class TestBicBisBitByteVariants:
    def test_bicb(self):
        sim = PDP11Simulator()
        sim.reset()
        sim._r[0] = 0x00FF
        sim._r[1] = 0x000F

        # BICB R1, R0: R0 = R0 & ~R1 (byte) = 0xFF & ~0x0F = 0xF0
        iw = 0xC000 | (_op(0, 1) << 6) | _op(0, 0)
        sim._mem[LOAD_ADDR]   = iw & 0xFF
        sim._mem[LOAD_ADDR+1] = (iw >> 8) & 0xFF
        sim._mem[LOAD_ADDR+2] = 0
        sim._mem[LOAD_ADDR+3] = 0
        sim._r[7] = LOAD_ADDR

        sim.step()
        assert (sim._r[0] & 0xFF) == 0xF0

    def test_bisb(self):
        sim = PDP11Simulator()
        sim.reset()
        sim._r[0] = 0x000F
        sim._r[1] = 0x00F0

        # BISB R1, R0: R0 = R0 | R1 = 0xFF
        iw = 0xD000 | (_op(0, 1) << 6) | _op(0, 0)
        sim._mem[LOAD_ADDR]   = iw & 0xFF
        sim._mem[LOAD_ADDR+1] = (iw >> 8) & 0xFF
        sim._mem[LOAD_ADDR+2] = 0
        sim._mem[LOAD_ADDR+3] = 0
        sim._r[7] = LOAD_ADDR

        sim.step()
        assert (sim._r[0] & 0xFF) == 0xFF

    def test_bitb(self):
        sim = PDP11Simulator()
        sim.reset()
        sim._r[0] = 0x00FF
        sim._r[1] = 0x0000

        # BITB R1, R0: test = 0xFF & 0x00 = 0 → Z=1
        iw = 0xB000 | (_op(0, 1) << 6) | _op(0, 0)
        sim._mem[LOAD_ADDR]   = iw & 0xFF
        sim._mem[LOAD_ADDR+1] = (iw >> 8) & 0xFF
        sim._mem[LOAD_ADDR+2] = 0
        sim._mem[LOAD_ADDR+3] = 0
        sim._r[7] = LOAD_ADDR

        sim.step()
        assert sim._psw & 0b0100   # Z=1


class TestNegbComb:
    def test_negb_zero(self):
        sim = PDP11Simulator()
        sim.reset()
        sim._r[0] = 0x0000

        iw = 0x8B00 | _op(0, 0)  # NEGB R0
        sim._mem[LOAD_ADDR]   = iw & 0xFF
        sim._mem[LOAD_ADDR+1] = (iw >> 8) & 0xFF
        sim._mem[LOAD_ADDR+2] = 0
        sim._mem[LOAD_ADDR+3] = 0
        sim._r[7] = LOAD_ADDR

        sim.step()
        assert (sim._r[0] & 0xFF) == 0
        assert not (sim._psw & 1)   # C=0

    def test_comb_register(self):
        sim = PDP11Simulator()
        sim.reset()
        sim._r[0] = 0x00F0

        iw = 0x8A40 | _op(0, 0)  # COMB R0
        sim._mem[LOAD_ADDR]   = iw & 0xFF
        sim._mem[LOAD_ADDR+1] = (iw >> 8) & 0xFF
        sim._mem[LOAD_ADDR+2] = 0
        sim._mem[LOAD_ADDR+3] = 0
        sim._r[7] = LOAD_ADDR

        sim.step()
        assert (sim._r[0] & 0xFF) == 0x0F
        assert sim._psw & 1   # C=1 always for COM
