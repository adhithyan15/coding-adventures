"""Multi-instruction program tests for the Motorola 68000 simulator.

Tests here exercise complete programs — sequences of multiple instructions
working together to compute a result.  They verify that the simulator's
decode/execute loop handles instruction chaining, memory access patterns,
and subroutine call/return correctly.
"""

from __future__ import annotations

import struct
import unittest

from motorola_68000_simulator import M68KSimulator


def _w(v: int) -> bytes: return struct.pack(">H", v & 0xFFFF)
def _l(v: int) -> bytes: return struct.pack(">I", v & 0xFFFFFFFF)
def _stop() -> bytes:    return _w(0x4E4F)  # TRAP #15 — halt without touching SR


def _run(prog: bytes) -> M68KSimulator:
    sim = M68KSimulator()
    result = sim.execute(prog)
    assert result.ok, f"Program failed: {result.error!r}"
    return sim


class TestSumLoop(unittest.TestCase):
    """Sum numbers 1..N using a DBF loop."""

    def test_sum_1_to_5(self):
        # D0 = loop counter (4 for 5 iters), D1 = sum, D2 = current addend (1..5)
        # ADD.W D2, D1: 1101 001 0 01 000 010 = 0xD242  (D1 += D2)
        # Layout:
        #   load+0:  MOVEQ #4, D0   load+2: MOVEQ #0, D1   load+4: MOVEQ #1, D2
        #   load+6:  ADD.W D2, D1  ← loop start
        #   load+8:  ADDQ.W #1, D2
        #   load+10: DBF D0 opword  load+12: ext (pc_before_ext)
        #   target = load+12 + (-6) = load+6 ✓
        prog = (
            _w(0x7004)              # MOVEQ #4, D0
            + _w(0x7200)            # MOVEQ #0, D1
            + _w(0x7401)            # MOVEQ #1, D2
            + _w(0xD242)            # ADD.W D2, D1  ← load+6
            + _w(0x5242)            # ADDQ.W #1, D2 ← load+8
            + _w(0x51C8) + _w(0xFFFA)  # DBF D0, #-6 → target=load+6 ✓
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[1] == 15   # 1+2+3+4+5

    def test_sum_1_to_10(self):
        prog = (
            _w(0x7009)              # MOVEQ #9, D0  (loop 10 times)
            + _w(0x7200)            # MOVEQ #0, D1
            + _w(0x7401)            # MOVEQ #1, D2
            + _w(0xD242)            # ADD.W D2, D1  (0xD242 = D1 += D2)
            + _w(0x5242)            # ADDQ.W #1, D2
            + _w(0x51C8) + _w(0xFFFA)  # DBF D0, #-6
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[1] == 55   # 1+2+...+10


class TestFibonacci(unittest.TestCase):
    """Compute Fibonacci(n) iteratively."""

    def test_fib_0(self):
        # fib(0) = 0  — BEQ must come right after MOVEQ #0, D0 before
        # any other MOVEQ that would clobber the Z flag.
        # Layout:
        #   load+0: MOVEQ #0, D0  → Z=1   (2 bytes)
        #   load+2: BEQ opword            (2 bytes)
        #   load+4: BEQ ext = 0x0010     (2 bytes); PC after ext = load+6
        #   target = load+6 + 0x10 = load+22 = STOP ✓
        #   load+6:  MOVEQ #0, D1
        #   load+8:  MOVEQ #1, D2
        #   load+10: MOVE.L D1, D3  ← loop start
        #   load+12: ADD.L D2, D3
        #   load+14: MOVE.L D2, D1
        #   load+16: MOVE.L D3, D2
        #   load+18: SUBQ.L #1, D0
        #   load+20: BNE #-12 (8-bit disp, no ext) → target=load+22-12=load+10 ✓
        #   load+22: STOP
        prog = (
            _w(0x7000)              # MOVEQ #0, D0  (n=0) → Z=1
            + _w(0x6700) + _w(0x0010)  # BEQ #16 → jump to STOP if n==0
            + _w(0x7200)            # MOVEQ #0, D1  (a=0)
            + _w(0x7401)            # MOVEQ #1, D2  (b=1)
            + _w(0x2601)            # MOVE.L D1, D3  ← loop start (load+10)
            + _w(0xD682)            # ADD.L D2, D3
            + _w(0x2202)            # MOVE.L D2, D1
            + _w(0x2403)            # MOVE.L D3, D2
            + _w(0x5380)            # SUBQ.L #1, D0
            + _w(0x66F4)            # BNE #-12 → target=load+22-12=load+10 ✓
            + _stop()               # STOP at load+22
        )
        sim = _run(prog)
        assert sim._d[1] == 0   # D1 never modified (BEQ taken), so stays 0

    def test_fib_6(self):
        # fib(6) = 8: 0,1,1,2,3,5,8
        prog = (
            _w(0x7006)              # MOVEQ #6, D0
            + _w(0x7200)            # MOVEQ #0, D1
            + _w(0x7401)            # MOVEQ #1, D2
            + _w(0x6700) + _w(0x000E)  # BEQ end
            + _w(0x2601)            # MOVE.L D1, D3  ← loop start (offset 10 from load)
            + _w(0xD682)            # ADD.L D2, D3
            + _w(0x2202)            # MOVE.L D2, D1
            + _w(0x2403)            # MOVE.L D3, D2
            + _w(0x5380)            # SUBQ.L #1, D0
            + _w(0x66F4)            # BNE -12 (back to offset 10)
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[1] == 8


class TestSubroutineChain(unittest.TestCase):
    """Test subroutine call chaining (nested BSR/RTS)."""

    def test_nested_calls(self):
        # main: BSR double → double: ADD.L D0,D0 + RTS
        # D0 starts at 5 → double multiplies by 2 → 10
        # Layout at load (0x1000):
        # 0x1000: MOVEQ #5, D0       (2 bytes)
        # 0x1002: BSR #2             (2 bytes) → 0x1004+2=0x1006 (double)
        # 0x1004: TRAP #15           (2 bytes) — halts after RTS returns here
        # 0x1006: ADD.L D0, D0       (2 bytes) [double = D0*2]
        # 0x1008: RTS                (2 bytes)
        prog = (
            _w(0x7005)              # MOVEQ #5, D0
            + _w(0x6102)            # BSR #2 → target: 0x1004+2=0x1006
            + _stop()               # TRAP #15 at 0x1004 (2 bytes)
            + _w(0xD080)            # ADD.L D0, D0 at 0x1006
            + _w(0x4E75)            # RTS at 0x1008
        )
        sim = _run(prog)
        assert sim._d[0] == 10   # 5 * 2


class TestMemoryOps(unittest.TestCase):
    """Test memory read/write patterns."""

    def test_copy_array(self):
        # Copy 4 words from src (0x2000) to dst (0x3000)
        # D0 = counter (3), A0 = src, A1 = dst
        # Loop: MOVE.W (A0)+, (A1)+; DBF D0, back
        src_data = [0x1111, 0x2222, 0x3333, 0x4444]
        prog = bytearray(0x3010)  # big enough

        instr = (
            _w(0x207C) + _l(0x2000)   # MOVEA.L #0x2000, A0
            + _w(0x227C) + _l(0x3000)  # MOVEA.L #0x3000, A1
            + _w(0x7003)               # MOVEQ #3, D0  (copy 4 times)
            + _w(0x32D8)               # MOVE.W (A0)+, (A1)+  ← loop start (offset 14 = 0x0E)
            + _w(0x51C8) + _w(0xFFFC)  # DBF D0, #-4 → ext at offset 18=0x12, target=0x12-4=0x0E ✓
            + _stop()
        )
        for i, b in enumerate(instr):
            prog[i] = b
        # Write source data at 0x2000 - 0x1000 = offset 0x1000 from load
        for i, w in enumerate(src_data):
            prog[0x1000 + i * 2] = (w >> 8) & 0xFF
            prog[0x1000 + i * 2 + 1] = w & 0xFF

        sim = M68KSimulator()
        result = sim.execute(bytes(prog))
        assert result.ok
        # Read destination
        state = result.final_state
        for i, expected in enumerate(src_data):
            addr = 0x3000 + i * 2
            actual = (state.memory[addr] << 8) | state.memory[addr + 1]
            assert actual == expected, f"dst[{i}] = {actual:#06x}, expected {expected:#06x}"

    def test_stack_push_pop(self):
        # Push D0, D1, D2 onto stack (using MOVE -(SP)) then pop in reverse
        prog = (
            _w(0x7011)              # MOVEQ #17, D0
            + _w(0x7233)            # MOVEQ #51, D1
            + _w(0x7455)            # MOVEQ #85, D2
            # Push: MOVE.L Dn, -(A7)  → predecrement A7 (SP)
            # MOVE.L D0, -(A7): 0010 111 100 000 000 = 0x2F00
            + _w(0x2F00)            # MOVE.L D0, -(SP)
            + _w(0x2F01)            # MOVE.L D1, -(SP)
            + _w(0x2F02)            # MOVE.L D2, -(SP)
            # Pop in reverse (MOVE.L (A7)+, Dn):
            # MOVE.L (A7)+, D0: 0010 000 011 011 111 = 0x201F
            + _w(0x201F)            # MOVE.L (SP)+, D0 → D0 = D2 = 85
            + _w(0x221F)            # MOVE.L (SP)+, D1 → D1 = D1_orig = 51
            + _w(0x241F)            # MOVE.L (SP)+, D2 → D2 = D0_orig = 17
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 85
        assert sim._d[1] == 51
        assert sim._d[2] == 17


class TestAbsoluteAddressing(unittest.TestCase):
    """Test absolute short and long addressing modes."""

    def test_move_to_abs_long(self):
        # MOVE.L #0xCAFE, (0x3000).L
        # MOVE.L #n, (abs).L: 0010 111 001 111 001  then 0x00003000 then #n
        # Wait: MOVE.L #imm, (abs).L
        # dst = mode=111,reg=001 (abs.L): in the dest field of MOVE
        # MOVE encoding: 00ss DDD ddd MMM mmm
        # MOVE.L(sz=10): 0010 dest_reg dest_mode src_mode src_reg
        # Dest is (abs).L: dest_mode=111(reversed=001), dest_reg=001
        # Actually in MOVE encoding, dest field is [dest_reg][dest_mode] (reversed!)
        # So dest (abs).L: dest_mode=111=7,dest_reg=001=1 → in opword: bits 11-9=001, bits 8-6=111
        # src = #imm: src_mode=111=7, src_reg=100=4 → bits 5-3=111, bits 2-0=100
        # Opword: 0010 001 111 111 100 = 0x23FC
        prog = (
            _w(0x23FC) + _l(0xCAFE) + _l(0x003000)  # MOVE.L #0xCAFE, 0x003000
            + _stop()
        )
        sim = _run(prog)
        addr = 0x3000
        val = (sim._mem[addr] << 24 | sim._mem[addr+1] << 16
               | sim._mem[addr+2] << 8 | sim._mem[addr+3])
        assert val == 0xCAFE

    def test_move_from_abs_short(self):
        # Write value at 0x1800, then read back via absolute short addressing
        prog = (
            _w(0x23FC) + _l(0x1234) + _l(0x001800)  # MOVE.L #0x1234, 0x001800
            + _w(0x2038) + _w(0x1800)                 # MOVE.L 0x1800.W, D0
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 0x1234


class TestPCRelative(unittest.TestCase):
    """Test PC-relative addressing (d16(PC) mode)."""

    def test_move_from_pc_relative(self):
        # MOVE.L d16(PC), D0 — reads a value from nearby memory
        # Opcode: MOVE.L d16(PC), D0
        # src EA: mode=111, reg=010 (d16(PC)) → bits 5-3=111, bits 2-0=010 = 0b111010
        # MOVE.L(sz=10): 0010 000 000 111 010 + d16
        # = 0x203A + d16 displacement
        # At opword position (load), after opword (2 bytes), PC = load+2
        # Extension word at load+2, PC_before_ext = load+2
        # Target = load+2 + d16
        # Let's put the data 2 words after the extension:
        # data at load+4, displacement = 4-0 = 4 (since PC_base = load+2)
        prog = (
            _w(0x203A) + _w(0x0002)   # MOVE.L d16(PC), D0; d16=2, target=PC+2=load+2+2=load+4
            + _l(0x12345678)           # data at load+4
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 0x12345678


class TestAddressRegisterOps(unittest.TestCase):
    """Test operations involving address registers."""

    def test_adda_subq_chain(self):
        # Build an address: start at 0x2000, add 0x100, subtract 4
        prog = (
            _w(0x207C) + _l(0x2000)   # MOVEA.L #0x2000, A0
            + _w(0xD0FC) + _w(0x0100)  # ADDA.W #0x100, A0  (1101 000 0 11 111 100 = 0xD0FC)
            + _w(0x5188)               # SUBQ.L #8, A0 (0101 000 1 10 001 000 = 0x5188... wait)
            + _stop()
        )
        # Actually SUBQ.L #8, A0: 0101 000 1 10 001 000 = 0x5188
        # SUBQ format: 0101 ddd1 ss mm rrr, for An direct (mode=001, not flagged)
        # #8 encoded as 000, sub=1, sz=10(long), mode=001(An), reg=000(A0)
        # = 0101 000 1 10 001 000 = 0x5188
        prog = (
            _w(0x207C) + _l(0x2000)   # MOVEA.L #0x2000, A0
            + _w(0xD0FC) + _w(0x0100)  # ADDA.W #0x100, A0  → A0 = 0x2100
            + _w(0x5188)               # SUBQ.L #8, A0  → A0 = 0x20F8... wait
            # Actually: ADDA.W with #imm:
            # 0xD0FC = 1101 000 0 11 111 100:
            #   dn=0 (A0), dir_bit=0, sz_code=3, mode=7, reg=4 (immediate)
            #   → ADDA.W #imm, A0 ✓
            # SUBQ.L #8, A0 = 0101 000 1 10 001 000 = 0x5188
            + _stop()
        )
        sim = _run(prog)
        # 0x2000 + 0x100 = 0x2100; - 8 = 0x20F8
        assert sim._a[0] == 0x20F8


class TestROXLROXR(unittest.TestCase):
    """Test rotate-through-extend instructions."""

    def test_roxl_basic(self):
        # ROXL.L #1, D0 with X=0: shifts left, bit 31 → C, X → bit 0
        # ROXL.L #1, D0: 1110 001 1 10 0 10 000 = 0xE390
        prog = (
            _w(0x44FC) + _w(0x00)   # MOVE #0, CCR  (X=0)
            + _w(0x7002)             # MOVEQ #2, D0
            + _w(0xE390)             # ROXL.L #1, D0
            + _stop()
        )
        sim = _run(prog)
        # 2 = 0b10, ROXL 1: X(0) into bit 0, bit 31 → C
        # 0b10 << 1 | 0 = 0b100 = 4, C = 0
        assert sim._d[0] == 4
        assert not (sim._sr & 1)   # C = 0 (MSB of 2 was 0)

    def test_roxr_with_x_set(self):
        # ROXR.L #1, D0 with X=1: X goes into MSB, bit 0 → C/X
        # ROXR.L #1, D0: 1110 001 0 10 0 10 000 = 0xE290
        prog = (
            _w(0x44FC) + _w(0x10)   # MOVE #0x10, CCR  (X=1)
            + _w(0x7002)             # MOVEQ #2, D0 (0b10)
            + _w(0xE290)             # ROXR.L #1, D0
            + _stop()
        )
        sim = _run(prog)
        # ROXR 1: X(1)→bit 31, shift right, bit 0 of 0b10 = 0 → C
        # result = 0x80000001, C = 0
        assert sim._d[0] == 0x8000_0001
        assert not (sim._sr & 1)   # C = 0 (bit 0 of 0b10 was 0)


class TestNOP(unittest.TestCase):

    def test_nop_no_effect(self):
        prog = (
            _w(0x7005)   # MOVEQ #5, D0
            + _w(0x4E71)  # NOP
            + _w(0x4E71)  # NOP
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 5

    def test_reset_no_effect(self):
        prog = (
            _w(0x7007)   # MOVEQ #7, D0
            + _w(0x4E70)  # RESET
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[0] == 7


class TestTRAP(unittest.TestCase):

    def test_trap_records_number(self):
        # TRAP #5 → D7 = 5 (simulator stub)
        prog = (
            _w(0x4E45)   # TRAP #5
            + _stop()
        )
        sim = _run(prog)
        assert sim._d[7] == 5

    def test_trap_15_halts(self):
        # TRAP #15 should halt (alternative halt mechanism)
        prog = bytes([0x4E, 0x4F])   # TRAP #15
        sim = M68KSimulator()
        result = sim.execute(prog)
        assert result.halted


if __name__ == "__main__":
    unittest.main()
