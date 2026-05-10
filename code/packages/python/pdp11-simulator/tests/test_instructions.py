"""Per-instruction correctness tests for PDP11Simulator.

Each test targets a specific instruction or addressing mode.  Programs are
assembled by hand using well-documented encodings.

PDP-11 instruction encoding quick reference:
  MOV  src, dst  = 0001 sss sss ddd ddd  (bits 15-12=0x1, src=11:6, dst=5:0)
  ADD  src, dst  = 0110 sss sss ddd ddd  (bits 15-12=0x6)
  SUB  src, dst  = 1110 sss sss ddd ddd  (bits 15-12=0xE)
  CMP  src, dst  = 0010 sss sss ddd ddd  (bits 15-12=0x2)
  BIS  src, dst  = 0101 sss sss ddd ddd  (bits 15-12=0x5)
  BIC  src, dst  = 0100 sss sss ddd ddd  (bits 15-12=0x4)
  BIT  src, dst  = 0011 sss sss ddd ddd  (bits 15-12=0x3)
  MOVB src, dst  = 1001 sss sss ddd ddd  (bit 15=1, bits 14-12=0x1)
  CLR  dst       = 0000 1010 00 mmm rrr  = 0x0A00 | (mode<<3) | reg
  COM  dst       = 0000 1010 01 mmm rrr  = 0x0A40 ...
  INC  dst       = 0000 1010 10 mmm rrr  = 0x0A80 ...
  DEC  dst       = 0000 1010 11 mmm rrr  = 0x0AC0 ...
  NEG  dst       = 0000 1011 00 mmm rrr  = 0x0B00 ...
  ADC  dst       = 0000 1011 01 mmm rrr  = 0x0B40 ...
  SBC  dst       = 0000 1011 10 mmm rrr  = 0x0B80 ...
  TST  dst       = 0000 1011 11 mmm rrr  = 0x0BC0 ...
  ROR  dst       = 0110 0000 00 mmm rrr  = 0x6000 ...
  ROL  dst       = 0110 0000 01 mmm rrr  = 0x6040 ...
  ASR  dst       = 0110 0000 10 mmm rrr  = 0x6080 ...
  ASL  dst       = 0110 0000 11 mmm rrr  = 0x60C0 ...
  SWAB dst       = 0000 0000 11 mmm rrr  = 0x00C0 ...
  BR   offset    = 0000 0001 oooooooo    = 0x0100 | offset
  BNE  offset    = 0000 0010 oooooooo
  BEQ  offset    = 0000 0011 oooooooo
  BGE  offset    = 0000 0100 oooooooo
  BLT  offset    = 0000 0101 oooooooo
  BGT  offset    = 0000 0110 oooooooo
  BLE  offset    = 0000 0111 oooooooo
  BPL  offset    = 1000 0000 oooooooo    = 0x8000 | offset
  BMI  offset    = 1000 0001 oooooooo
  BHI  offset    = 1000 0010 oooooooo
  BLOS offset    = 1000 0011 oooooooo
  BVC  offset    = 1000 0100 oooooooo
  BVS  offset    = 1000 0101 oooooooo
  BCC  offset    = 1000 0110 oooooooo
  BCS  offset    = 1000 0111 oooooooo
  JMP  dst       = 0000 0000 01 mmm rrr  = 0x0040 | (mode<<3) | reg
  JSR  reg, dst  = 0000 1000 rrr mmm ddd = 0x0800 | (reg<<6) | (mode<<3) | dst
  RTS  reg       = 0000 0000 1000 0 rrr  = 0x0200 | reg
  SOB  reg, off  = 0111 11 rrr oooooo    = 0x7E00 | (reg<<6) | offset
  HALT           = 0x0000
  NOP            = 0x00A0
"""

from __future__ import annotations

import pytest

from pdp11_simulator import PDP11Simulator, PDP11State


def _w(value: int) -> bytes:
    """Pack a 16-bit integer as little-endian word bytes."""
    return bytes([value & 0xFF, (value >> 8) & 0xFF])

HALT = _w(0x0000)
NOP  = _w(0x00A0)

def _run(prog: bytes) -> PDP11State:
    """Run a program and return the final state."""
    sim = PDP11Simulator()
    result = sim.execute(prog)
    assert result.ok, f"Simulation error: {result.error}"
    return result.final_state


# ── Helper: encode operand field (mode, reg) as 6-bit field ──────────────────
def _op(mode: int, reg: int) -> int:
    return (mode << 3) | reg

def _mov(src_mode, src_reg, dst_mode, dst_reg) -> bytes:
    """Encode MOV src, dst."""
    return _w(0x1000 | (_op(src_mode, src_reg) << 6) | _op(dst_mode, dst_reg))

def _add(src_mode, src_reg, dst_mode, dst_reg) -> bytes:
    """Encode ADD src, dst."""
    return _w(0x6000 | (_op(src_mode, src_reg) << 6) | _op(dst_mode, dst_reg))

def _sub(src_mode, src_reg, dst_mode, dst_reg) -> bytes:
    """Encode SUB src, dst."""
    return _w(0xE000 | (_op(src_mode, src_reg) << 6) | _op(dst_mode, dst_reg))

def _cmp(src_mode, src_reg, dst_mode, dst_reg) -> bytes:
    """Encode CMP src, dst."""
    return _w(0x2000 | (_op(src_mode, src_reg) << 6) | _op(dst_mode, dst_reg))

def _bit(src_mode, src_reg, dst_mode, dst_reg) -> bytes:
    return _w(0x3000 | (_op(src_mode, src_reg) << 6) | _op(dst_mode, dst_reg))

def _bic(src_mode, src_reg, dst_mode, dst_reg) -> bytes:
    return _w(0x4000 | (_op(src_mode, src_reg) << 6) | _op(dst_mode, dst_reg))

def _bis(src_mode, src_reg, dst_mode, dst_reg) -> bytes:
    return _w(0x5000 | (_op(src_mode, src_reg) << 6) | _op(dst_mode, dst_reg))

def _movb(src_mode, src_reg, dst_mode, dst_reg) -> bytes:
    return _w(0x9000 | (_op(src_mode, src_reg) << 6) | _op(dst_mode, dst_reg))

def _clr(mode, reg)   -> bytes: return _w(0x0A00 | _op(mode, reg))
def _com(mode, reg)   -> bytes: return _w(0x0A40 | _op(mode, reg))
def _inc(mode, reg)   -> bytes: return _w(0x0A80 | _op(mode, reg))
def _dec(mode, reg)   -> bytes: return _w(0x0AC0 | _op(mode, reg))
def _neg(mode, reg)   -> bytes: return _w(0x0B00 | _op(mode, reg))
def _adc(mode, reg)   -> bytes: return _w(0x0B40 | _op(mode, reg))
def _sbc(mode, reg)   -> bytes: return _w(0x0B80 | _op(mode, reg))
def _tst(mode, reg)   -> bytes: return _w(0x0BC0 | _op(mode, reg))
def _ror(mode, reg)   -> bytes: return _w(0x0C00 | _op(mode, reg))  # octal 006000
def _rol(mode, reg)   -> bytes: return _w(0x0C40 | _op(mode, reg))  # octal 006100
def _asr(mode, reg)   -> bytes: return _w(0x0C80 | _op(mode, reg))  # octal 006200
def _asl(mode, reg)   -> bytes: return _w(0x0CC0 | _op(mode, reg))  # octal 006300
def _swab(mode, reg)  -> bytes: return _w(0x00C0 | _op(mode, reg))
def _clrb(mode, reg)  -> bytes: return _w(0x8A00 | _op(mode, reg))
def _comb(mode, reg)  -> bytes: return _w(0x8A40 | _op(mode, reg))
def _incb(mode, reg)  -> bytes: return _w(0x8A80 | _op(mode, reg))
def _decb(mode, reg)  -> bytes: return _w(0x8AC0 | _op(mode, reg))
def _negb(mode, reg)  -> bytes: return _w(0x8B00 | _op(mode, reg))
def _adcb(mode, reg)  -> bytes: return _w(0x8B40 | _op(mode, reg))
def _sbcb(mode, reg)  -> bytes: return _w(0x8B80 | _op(mode, reg))
def _tstb(mode, reg)  -> bytes: return _w(0x8BC0 | _op(mode, reg))
def _rorb(mode, reg)  -> bytes: return _w(0x8C00 | _op(mode, reg))  # octal 106000
def _rolb(mode, reg)  -> bytes: return _w(0x8C40 | _op(mode, reg))  # octal 106100
def _asrb(mode, reg)  -> bytes: return _w(0x8C80 | _op(mode, reg))  # octal 106200
def _aslb(mode, reg)  -> bytes: return _w(0x8CC0 | _op(mode, reg))  # octal 106300

def _br(offset)   -> bytes: return _w(0x0100 | (offset & 0xFF))
def _bne(offset)  -> bytes: return _w(0x0200 | (offset & 0xFF))
def _beq(offset)  -> bytes: return _w(0x0300 | (offset & 0xFF))
def _bge(offset)  -> bytes: return _w(0x0400 | (offset & 0xFF))
def _blt(offset)  -> bytes: return _w(0x0500 | (offset & 0xFF))
def _bgt(offset)  -> bytes: return _w(0x0600 | (offset & 0xFF))
def _ble(offset)  -> bytes: return _w(0x0700 | (offset & 0xFF))
def _bpl(offset)  -> bytes: return _w(0x8000 | (offset & 0xFF))
def _bmi(offset)  -> bytes: return _w(0x8100 | (offset & 0xFF))
def _bhi(offset)  -> bytes: return _w(0x8200 | (offset & 0xFF))
def _blos(offset) -> bytes: return _w(0x8300 | (offset & 0xFF))
def _bvc(offset)  -> bytes: return _w(0x8400 | (offset & 0xFF))
def _bvs(offset)  -> bytes: return _w(0x8500 | (offset & 0xFF))
def _bcc(offset)  -> bytes: return _w(0x8600 | (offset & 0xFF))
def _bcs(offset)  -> bytes: return _w(0x8700 | (offset & 0xFF))

def _jmp(mode, reg) -> bytes: return _w(0x0040 | _op(mode, reg))
def _jsr(link, mode, dst_reg) -> bytes:
    return _w(0x0800 | (link << 6) | _op(mode, dst_reg))
def _rts(reg) -> bytes: return _w(0x0080 | reg)   # octal 000 200
def _sob(reg, offset) -> bytes: return _w(0x7E00 | (reg << 6) | (offset & 0x3F))


# ── MOV tests ─────────────────────────────────────────────────────────────────

class TestMOV:
    def test_mov_imm_to_r0(self):
        # MOV #42, R0: src=mode2/R7 (immediate), dst=mode0/R0
        prog = _mov(2, 7, 0, 0) + _w(42) + HALT
        s = _run(prog)
        assert s.r[0] == 42

    def test_mov_r0_to_r1(self):
        # Load R0=10 via immediate, then MOV R0, R1
        prog = _mov(2, 7, 0, 0) + _w(10) + _mov(0, 0, 0, 1) + HALT
        s = _run(prog)
        assert s.r[1] == 10

    def test_mov_clears_v_flag(self):
        # MOV should always clear V
        sim = PDP11Simulator()
        sim._psw = 0b0010   # V=1
        sim.load(_mov(2, 7, 0, 0) + _w(1) + HALT)
        sim.execute(_mov(2, 7, 0, 0) + _w(1) + HALT)
        state = sim.get_state()
        assert not state.v

    def test_mov_negative_sets_n(self):
        # MOV #0x8000, R0 → N=1
        prog = _mov(2, 7, 0, 0) + _w(0x8000) + HALT
        s = _run(prog)
        assert s.n
        assert not s.z

    def test_mov_zero_sets_z(self):
        prog = _mov(2, 7, 0, 0) + _w(0) + HALT
        s = _run(prog)
        assert s.z
        assert not s.n

    def test_mov_to_memory(self):
        # MOV R0, (R1): R0=0x1234, R1 points to some address
        # Set up: MOV #0x1234, R0; MOV #0x2000, R1; MOV R0, (R1); HALT
        prog = (
            _mov(2, 7, 0, 0) + _w(0x1234)   # MOV #0x1234, R0
            + _mov(2, 7, 0, 1) + _w(0x2000) # MOV #0x2000, R1
            + _mov(0, 0, 1, 1)               # MOV R0, (R1)
            + HALT
        )
        s = _run(prog)
        word_at_2000 = s.memory[0x2000] | (s.memory[0x2001] << 8)
        assert word_at_2000 == 0x1234

    def test_mov_from_memory(self):
        # Store 0x5678 at 0x2000, then MOV (R1), R0
        sim = PDP11Simulator()
        sim.reset()
        sim._mem[0x2000] = 0x78
        sim._mem[0x2001] = 0x56
        prog = (
            _mov(2, 7, 0, 1) + _w(0x2000)  # MOV #0x2000, R1
            + _mov(1, 1, 0, 0)              # MOV (R1), R0
            + HALT
        )
        sim.load(prog)
        # Restore the memory data since load() zeros memory
        sim._mem[0x2000] = 0x78
        sim._mem[0x2001] = 0x56
        # Use step-by-step
        sim.step(); sim.step(); sim.step()
        state = sim.get_state()
        assert state.r[0] == 0x5678


class TestMOVB:
    def test_movb_imm_to_memory(self):
        # MOVB #0xAB, (R1): store byte at R1
        prog = (
            _mov(2, 7, 0, 1) + _w(0x2000)   # MOV #0x2000, R1
            + _movb(2, 7, 1, 1) + _w(0xAB)  # MOVB #0xAB, (R1)
            + HALT
        )
        s = _run(prog)
        assert s.memory[0x2000] == 0xAB

    def test_movb_sign_extends_to_register(self):
        # MOVB #0xFF, R0 → R0 should be 0xFFFF (sign extended)
        prog = _movb(2, 7, 0, 0) + _w(0x00FF) + HALT
        s = _run(prog)
        assert s.r[0] == 0xFFFF

    def test_movb_positive_no_extend(self):
        # MOVB #0x7F, R0 → R0 = 0x007F
        prog = _movb(2, 7, 0, 0) + _w(0x007F) + HALT
        s = _run(prog)
        assert s.r[0] == 0x007F


# ── ADD tests ─────────────────────────────────────────────────────────────────

class TestADD:
    def test_add_basic(self):
        # ADD R0, R1: R0=3, R1=4 → R1=7
        prog = (
            _mov(2, 7, 0, 0) + _w(3)   # MOV #3, R0
            + _mov(2, 7, 0, 1) + _w(4) # MOV #4, R1
            + _add(0, 0, 0, 1)          # ADD R0, R1
            + HALT
        )
        s = _run(prog)
        assert s.r[1] == 7
        assert not s.n and not s.z and not s.v and not s.c

    def test_add_sets_carry(self):
        # 0xFFFF + 1 = 0x10000 → C=1, Z=1
        prog = (
            _mov(2, 7, 0, 0) + _w(0xFFFF)
            + _mov(2, 7, 0, 1) + _w(1)
            + _add(0, 0, 0, 1)
            + HALT
        )
        s = _run(prog)
        assert s.r[1] == 0
        assert s.z
        assert s.c

    def test_add_sets_overflow(self):
        # 0x7FFF + 1 = 0x8000 → V=1 (positive + positive = negative)
        prog = (
            _mov(2, 7, 0, 0) + _w(0x7FFF)
            + _mov(2, 7, 0, 1) + _w(1)
            + _add(0, 0, 0, 1)
            + HALT
        )
        s = _run(prog)
        assert s.r[1] == 0x8000
        assert s.v
        assert s.n

    def test_add_sets_negative(self):
        # ADD results in negative number
        prog = (
            _mov(2, 7, 0, 0) + _w(0x8000)  # -32768
            + _mov(2, 7, 0, 1) + _w(0xFFFF) # -1
            + _add(0, 0, 0, 1)
            + HALT
        )
        s = _run(prog)
        assert s.r[1] == 0x7FFF   # wraps
        assert s.c


# ── SUB tests ─────────────────────────────────────────────────────────────────

class TestSUB:
    def test_sub_basic(self):
        # SUB R0, R1: dst=R1=10, src=R0=3 → R1 = 10-3 = 7
        prog = (
            _mov(2, 7, 0, 0) + _w(3)
            + _mov(2, 7, 0, 1) + _w(10)
            + _sub(0, 0, 0, 1)
            + HALT
        )
        s = _run(prog)
        assert s.r[1] == 7
        assert not s.c

    def test_sub_borrow(self):
        # 3 - 5 = -2 (0xFFFE); C=1 (borrow)
        prog = (
            _mov(2, 7, 0, 0) + _w(5)
            + _mov(2, 7, 0, 1) + _w(3)
            + _sub(0, 0, 0, 1)
            + HALT
        )
        s = _run(prog)
        assert s.r[1] == 0xFFFE
        assert s.c
        assert s.n

    def test_sub_overflow(self):
        # 0x8000 - 1 = 0x7FFF → V=1 (neg minus pos = pos? no: neg minus pos)
        # Actually: -32768 - 1 should underflow to +32767
        prog = (
            _mov(2, 7, 0, 0) + _w(1)
            + _mov(2, 7, 0, 1) + _w(0x8000)
            + _sub(0, 0, 0, 1)   # R1 = 0x8000 - 1 = 0x7FFF
            + HALT
        )
        s = _run(prog)
        assert s.r[1] == 0x7FFF
        assert s.v


# ── CMP tests ─────────────────────────────────────────────────────────────────

class TestCMP:
    def test_cmp_equal(self):
        prog = (
            _mov(2, 7, 0, 0) + _w(5)
            + _mov(2, 7, 0, 1) + _w(5)
            + _cmp(0, 0, 0, 1)   # CMP R0, R1 = R0 - R1 = 5-5=0
            + HALT
        )
        s = _run(prog)
        assert s.z
        assert not s.n and not s.c

    def test_cmp_less_than(self):
        # CMP R0, R1 = R0 - R1; if R0=3, R1=5 → 3-5=-2 → N=1, C=1
        prog = (
            _mov(2, 7, 0, 0) + _w(3)
            + _mov(2, 7, 0, 1) + _w(5)
            + _cmp(0, 0, 0, 1)
            + HALT
        )
        s = _run(prog)
        assert s.n
        assert s.c
        assert not s.z

    def test_cmp_does_not_modify_registers(self):
        prog = (
            _mov(2, 7, 0, 0) + _w(7)
            + _mov(2, 7, 0, 1) + _w(3)
            + _cmp(0, 0, 0, 1)
            + HALT
        )
        s = _run(prog)
        assert s.r[0] == 7   # untouched
        assert s.r[1] == 3   # untouched


# ── BIT / BIC / BIS tests ─────────────────────────────────────────────────────

class TestBitOps:
    def test_bit_nonzero(self):
        # BIT #0xFF00, R0 with R0=0x0F0F → 0xFF00 & 0x0F0F = 0x0F00 → N=0, Z=0
        prog = (
            _mov(2, 7, 0, 0) + _w(0x0F0F)
            + _bit(2, 7, 0, 0) + _w(0xFF00)
            + HALT
        )
        s = _run(prog)
        assert not s.z
        assert not s.v
        assert s.r[0] == 0x0F0F   # BIT doesn't modify dst

    def test_bit_zero(self):
        # BIT #0xFF00, R0 with R0=0x00FF → result=0 → Z=1
        prog = (
            _mov(2, 7, 0, 0) + _w(0x00FF)
            + _bit(2, 7, 0, 0) + _w(0xFF00)
            + HALT
        )
        s = _run(prog)
        assert s.z

    def test_bic_clears_bits(self):
        # BIC #0x000F, R0 with R0=0x00FF → R0 = 0x00FF & ~0x000F = 0x00F0
        prog = (
            _mov(2, 7, 0, 0) + _w(0x00FF)
            + _bic(2, 7, 0, 0) + _w(0x000F)
            + HALT
        )
        s = _run(prog)
        assert s.r[0] == 0x00F0

    def test_bis_sets_bits(self):
        # BIS #0x00F0, R0 with R0=0x000F → R0 = 0x00FF
        prog = (
            _mov(2, 7, 0, 0) + _w(0x000F)
            + _bis(2, 7, 0, 0) + _w(0x00F0)
            + HALT
        )
        s = _run(prog)
        assert s.r[0] == 0x00FF


# ── Single-operand tests ──────────────────────────────────────────────────────

class TestCLR:
    def test_clr_register(self):
        prog = _mov(2, 7, 0, 0) + _w(0x1234) + _clr(0, 0) + HALT
        s = _run(prog)
        assert s.r[0] == 0
        assert s.z and not s.n and not s.v and not s.c

    def test_clrb_memory(self):
        prog = (
            _mov(2, 7, 0, 1) + _w(0x2000)
            + _mov(2, 7, 0, 0) + _w(0x00AB)
            + _mov(0, 0, 1, 1)              # MOV R0, (R1) — write 0x00AB to 0x2000
            + _clrb(1, 1)                   # CLRB (R1) — zero the byte at 0x2000
            + HALT
        )
        s = _run(prog)
        assert s.memory[0x2000] == 0


class TestCOM:
    def test_com_register(self):
        prog = _mov(2, 7, 0, 0) + _w(0x0F0F) + _com(0, 0) + HALT
        s = _run(prog)
        assert s.r[0] == 0xF0F0
        assert s.c   # COM always sets C

    def test_com_zero_result(self):
        prog = _mov(2, 7, 0, 0) + _w(0xFFFF) + _com(0, 0) + HALT
        s = _run(prog)
        assert s.r[0] == 0
        assert s.z


class TestINCDEC:
    def test_inc(self):
        prog = _mov(2, 7, 0, 0) + _w(5) + _inc(0, 0) + HALT
        s = _run(prog)
        assert s.r[0] == 6

    def test_inc_overflow(self):
        # 0x7FFF + 1 = 0x8000 → V=1
        prog = _mov(2, 7, 0, 0) + _w(0x7FFF) + _inc(0, 0) + HALT
        s = _run(prog)
        assert s.r[0] == 0x8000
        assert s.v

    def test_inc_does_not_change_c(self):
        sim = PDP11Simulator()
        # load() resets PSW; set C=1 after loading
        sim.load(_mov(2, 7, 0, 0) + _w(5) + _inc(0, 0) + HALT)
        sim._psw = 0b0001   # C=1 (set after load so reset doesn't clear it)
        sim.step(); sim.step(); sim.step()
        assert sim._psw & 1   # C unchanged

    def test_dec(self):
        prog = _mov(2, 7, 0, 0) + _w(5) + _dec(0, 0) + HALT
        s = _run(prog)
        assert s.r[0] == 4

    def test_dec_underflow_overflow(self):
        # 0x8000 - 1 = 0x7FFF → V=1
        prog = _mov(2, 7, 0, 0) + _w(0x8000) + _dec(0, 0) + HALT
        s = _run(prog)
        assert s.r[0] == 0x7FFF
        assert s.v


class TestNEG:
    def test_neg_positive(self):
        prog = _mov(2, 7, 0, 0) + _w(5) + _neg(0, 0) + HALT
        s = _run(prog)
        assert s.r[0] == 0xFFFB   # -5 = 0xFFFB
        assert s.c   # C=1 (result != 0)

    def test_neg_zero(self):
        prog = _clr(0, 0) + _neg(0, 0) + HALT
        s = _run(prog)
        assert s.r[0] == 0
        assert not s.c   # C=0 when result=0

    def test_neg_most_negative(self):
        # NEG 0x8000 = 0x8000; V=1
        prog = _mov(2, 7, 0, 0) + _w(0x8000) + _neg(0, 0) + HALT
        s = _run(prog)
        assert s.r[0] == 0x8000
        assert s.v


class TestTST:
    def test_tst_positive(self):
        prog = _mov(2, 7, 0, 0) + _w(42) + _tst(0, 0) + HALT
        s = _run(prog)
        assert not s.n and not s.z and not s.v and not s.c

    def test_tst_negative(self):
        prog = _mov(2, 7, 0, 0) + _w(0x8000) + _tst(0, 0) + HALT
        s = _run(prog)
        assert s.n and not s.z

    def test_tst_zero(self):
        prog = _clr(0, 0) + _tst(0, 0) + HALT
        s = _run(prog)
        assert s.z and not s.n


class TestSWAB:
    def test_swab(self):
        prog = _mov(2, 7, 0, 0) + _w(0x1234) + _swab(0, 0) + HALT
        s = _run(prog)
        assert s.r[0] == 0x3412

    def test_swab_n_from_low_byte(self):
        # SWAB 0x00FF → 0xFF00; N from low byte of result = 0x00 → N=0
        prog = _mov(2, 7, 0, 0) + _w(0x00FF) + _swab(0, 0) + HALT
        s = _run(prog)
        assert s.r[0] == 0xFF00
        assert not s.n   # N from low byte = 0x00

    def test_swab_n_from_low_byte_set(self):
        # SWAB 0xFF00 → 0x00FF; low byte = 0xFF → N=1
        prog = _mov(2, 7, 0, 0) + _w(0xFF00) + _swab(0, 0) + HALT
        s = _run(prog)
        assert s.r[0] == 0x00FF
        assert s.n   # N from low byte = 0xFF → bit 7 set


class TestADCSBC:
    def test_adc_with_carry(self):
        sim = PDP11Simulator()
        sim.load(_mov(2, 7, 0, 0) + _w(5) + _adc(0, 0) + HALT)
        sim._psw = 0b0001   # C=1 (set after load)
        sim.step(); sim.step(); sim.step()
        assert sim._r[0] == 6

    def test_adc_without_carry(self):
        prog = _mov(2, 7, 0, 0) + _w(5) + _adc(0, 0) + HALT
        s = _run(prog)
        assert s.r[0] == 5   # 5 + 0 = 5

    def test_sbc_with_carry(self):
        sim = PDP11Simulator()
        sim.load(_mov(2, 7, 0, 0) + _w(5) + _sbc(0, 0) + HALT)
        sim._psw = 0b0001   # C=1 (set after load)
        sim.step(); sim.step(); sim.step()
        assert sim._r[0] == 4   # 5 - 1 = 4

    def test_sbc_without_carry(self):
        prog = _mov(2, 7, 0, 0) + _w(5) + _sbc(0, 0) + HALT
        s = _run(prog)
        assert s.r[0] == 5   # 5 - 0 = 5


# ── Shift and rotate tests ────────────────────────────────────────────────────

class TestShifts:
    def test_asr_word(self):
        # ASR 0x0010 → 0x0008; C=0
        prog = _mov(2, 7, 0, 0) + _w(0x0010) + _asr(0, 0) + HALT
        s = _run(prog)
        assert s.r[0] == 0x0008
        assert not s.c

    def test_asr_preserves_sign(self):
        # ASR 0x8000 → 0xC000 (sign bit replicated); C=0 (bit0=0)
        prog = _mov(2, 7, 0, 0) + _w(0x8000) + _asr(0, 0) + HALT
        s = _run(prog)
        assert s.r[0] == 0xC000
        assert not s.c

    def test_asr_carry_from_bit0(self):
        # ASR 0x0001 → 0x0000; C=1
        prog = _mov(2, 7, 0, 0) + _w(0x0001) + _asr(0, 0) + HALT
        s = _run(prog)
        assert s.r[0] == 0
        assert s.c
        assert s.z

    def test_asl_word(self):
        prog = _mov(2, 7, 0, 0) + _w(0x0001) + _asl(0, 0) + HALT
        s = _run(prog)
        assert s.r[0] == 0x0002
        assert not s.c

    def test_asl_carry_from_msb(self):
        # ASL 0x8000 → 0x0000; C=1
        prog = _mov(2, 7, 0, 0) + _w(0x8000) + _asl(0, 0) + HALT
        s = _run(prog)
        assert s.r[0] == 0
        assert s.c
        assert s.z

    def test_ror_with_carry(self):
        sim = PDP11Simulator()
        # ROR 0x0002 with C=1 → 0x8001; new C=0
        sim.load(_mov(2, 7, 0, 0) + _w(0x0002) + _ror(0, 0) + HALT)
        sim._psw = 0b0001   # C=1 (set after load)
        sim.step(); sim.step(); sim.step()
        assert sim._r[0] == 0x8001
        assert not sim._psw & 1   # C=0 (old bit0 was 0)

    def test_ror_without_carry(self):
        prog = _mov(2, 7, 0, 0) + _w(0x0001) + _ror(0, 0) + HALT
        s = _run(prog)
        assert s.r[0] == 0x0000
        assert s.c   # bit0 of original = 1 → new C=1

    def test_rol_basic(self):
        prog = _mov(2, 7, 0, 0) + _w(0x0001) + _rol(0, 0) + HALT
        s = _run(prog)
        assert s.r[0] == 0x0002
        assert not s.c

    def test_rol_msb_to_carry(self):
        prog = _mov(2, 7, 0, 0) + _w(0x8000) + _rol(0, 0) + HALT
        s = _run(prog)
        assert s.r[0] == 0x0000
        assert s.c


# ── Branch tests ──────────────────────────────────────────────────────────────

class TestBranches:
    """Test all 15 branch conditions and both taken/not-taken paths."""

    def _branch_test(self, branch_instr: bytes, taken: bool, setup: bytes = b"") -> PDP11State:
        """
        Layout at 0x1000:
          setup (variable length)
          branch over HALT (or fallthrough):
            HALT          ← if branch NOT taken, lands here
            NOP           ← 2 bytes of dead code
            HALT          ← reached when branch IS taken
        """
        # If taken, branch skips 1 word (offset=1) over the first HALT
        # If not taken, fall through to first HALT
        # The branch offset=1 means: PC_after_fetch + 2*1 = branch_target
        #   PC after fetch is at (start_of_branch + 2)
        #   target = start_of_branch + 2 + 2 = start_of_branch + 4 = skip first HALT
        prog = setup + branch_instr + HALT + NOP + HALT
        s = _run(prog)
        if taken:
            # Should reach second HALT
            return s
        else:
            # Should reach first HALT
            return s

    def test_br_always_taken(self):
        # BR over next instruction
        prog = _br(1) + NOP + _mov(2, 7, 0, 0) + _w(99) + HALT
        # BR offset=1: skip 1 word (NOP), land on MOV
        s = _run(prog)
        assert s.r[0] == 99

    def test_bne_taken_when_z_clear(self):
        # CMP sets Z=0 → BNE taken
        prog = (
            _mov(2, 7, 0, 0) + _w(3)
            + _mov(2, 7, 0, 1) + _w(5)
            + _cmp(0, 0, 0, 1)          # 3 - 5 ≠ 0 → Z=0
            + _bne(1)                   # skip HALT
            + HALT
            + _mov(2, 7, 0, 2) + _w(1) # R2 = 1 (reachable)
            + HALT
        )
        s = _run(prog)
        assert s.r[2] == 1

    def test_bne_not_taken_when_z_set(self):
        # CMP equal → Z=1 → BNE NOT taken
        prog = (
            _mov(2, 7, 0, 0) + _w(5)
            + _mov(2, 7, 0, 1) + _w(5)
            + _cmp(0, 0, 0, 1)          # Z=1
            + _bne(2)                   # NOT taken
            + _mov(2, 7, 0, 2) + _w(42) # R2 = 42 (fallthrough)
            + HALT
        )
        s = _run(prog)
        assert s.r[2] == 42   # fell through

    def test_beq_taken(self):
        prog = (
            _clr(0, 0)
            + _tst(0, 0)
            + _beq(1)
            + HALT
            + _mov(2, 7, 0, 1) + _w(7)
            + HALT
        )
        s = _run(prog)
        assert s.r[1] == 7

    def test_bpl_taken_when_positive(self):
        prog = (
            _mov(2, 7, 0, 0) + _w(1)
            + _tst(0, 0)
            + _bpl(1)
            + HALT
            + _mov(2, 7, 0, 1) + _w(1)
            + HALT
        )
        s = _run(prog)
        assert s.r[1] == 1

    def test_bmi_taken_when_negative(self):
        prog = (
            _mov(2, 7, 0, 0) + _w(0x8000)
            + _tst(0, 0)
            + _bmi(1)
            + HALT
            + _mov(2, 7, 0, 1) + _w(1)
            + HALT
        )
        s = _run(prog)
        assert s.r[1] == 1

    def test_blt_taken_when_signed_less(self):
        # CMP #5, R0 with R0=3: CMP src=5, dst=3; CMP does src-dst = 5-3 = 2 ≥ 0
        # Actually CMP R0, R1 with R0=5, R1=7: 5-7 = -2 → N=1, V=0 → N^V=1 → BLT taken
        prog = (
            _mov(2, 7, 0, 0) + _w(5)
            + _mov(2, 7, 0, 1) + _w(7)
            + _cmp(0, 0, 0, 1)           # 5 - 7 = -2 → N=1
            + _blt(1)                    # N^V = 1 → taken
            + HALT
            + _mov(2, 7, 0, 2) + _w(1)
            + HALT
        )
        s = _run(prog)
        assert s.r[2] == 1

    def test_bge_taken_when_greater_equal(self):
        prog = (
            _mov(2, 7, 0, 0) + _w(7)
            + _mov(2, 7, 0, 1) + _w(5)
            + _cmp(0, 0, 0, 1)           # 7-5 = 2 → N=0, V=0 → N^V=0 → BGE taken
            + _bge(1)
            + HALT
            + _mov(2, 7, 0, 2) + _w(1)
            + HALT
        )
        s = _run(prog)
        assert s.r[2] == 1

    def test_bhi_taken(self):
        # BHI: C=0 AND Z=0
        prog = (
            _mov(2, 7, 0, 0) + _w(7)
            + _mov(2, 7, 0, 1) + _w(5)
            + _cmp(0, 0, 0, 1)   # 7-5=2 → C=0, Z=0
            + _bhi(1)
            + HALT
            + _mov(2, 7, 0, 2) + _w(1)
            + HALT
        )
        s = _run(prog)
        assert s.r[2] == 1

    def test_blos_taken(self):
        # BLOS: C=1 OR Z=1  (lower or same unsigned)
        prog = (
            _mov(2, 7, 0, 0) + _w(5)
            + _mov(2, 7, 0, 1) + _w(7)
            + _cmp(0, 0, 0, 1)   # 5-7 → C=1
            + _blos(1)
            + HALT
            + _mov(2, 7, 0, 2) + _w(1)
            + HALT
        )
        s = _run(prog)
        assert s.r[2] == 1

    def test_bcc_taken_when_carry_clear(self):
        prog = _clr(0, 0) + _tst(0, 0) + _bcc(1) + HALT + _mov(2, 7, 0, 1) + _w(1) + HALT
        s = _run(prog)
        assert s.r[1] == 1

    def test_bcs_taken_when_carry_set(self):
        # NEG 1 → C=1
        prog = (
            _mov(2, 7, 0, 0) + _w(1)
            + _neg(0, 0)
            + _bcs(1)
            + HALT
            + _mov(2, 7, 0, 1) + _w(1)
            + HALT
        )
        s = _run(prog)
        assert s.r[1] == 1

    def test_bvc_taken_when_no_overflow(self):
        prog = _clr(0, 0) + _tst(0, 0) + _bvc(1) + HALT + _mov(2, 7, 0, 1) + _w(1) + HALT
        s = _run(prog)
        assert s.r[1] == 1

    def test_bvs_taken_when_overflow(self):
        # 0x7FFF + 1 → V=1
        prog = (
            _mov(2, 7, 0, 0) + _w(0x7FFF)
            + _mov(2, 7, 0, 1) + _w(1)
            + _add(0, 0, 0, 1)
            + _bvs(1)
            + HALT
            + _mov(2, 7, 0, 2) + _w(1)
            + HALT
        )
        s = _run(prog)
        assert s.r[2] == 1

    def test_bgt_taken(self):
        prog = (
            _mov(2, 7, 0, 0) + _w(10)
            + _mov(2, 7, 0, 1) + _w(5)
            + _cmp(0, 0, 0, 1)   # 10-5=5 > 0
            + _bgt(1)
            + HALT
            + _mov(2, 7, 0, 2) + _w(1)
            + HALT
        )
        s = _run(prog)
        assert s.r[2] == 1

    def test_ble_taken_when_zero(self):
        prog = (
            _mov(2, 7, 0, 0) + _w(5)
            + _mov(2, 7, 0, 1) + _w(5)
            + _cmp(0, 0, 0, 1)   # Z=1
            + _ble(1)
            + HALT
            + _mov(2, 7, 0, 2) + _w(1)
            + HALT
        )
        s = _run(prog)
        assert s.r[2] == 1


# ── JMP tests ─────────────────────────────────────────────────────────────────

class TestJMP:
    def test_jmp_register_deferred(self):
        # JMP (R1): R1 = address of HALT, so jump there
        # Target = load_addr + offset where HALT lives
        # Layout: MOV #addr, R1 | JMP (R1) | MOV #99, R0 (skipped) | HALT
        # HALT is at offset 8 from start (3 words: MOV#, R1, JMP, then HALT)
        # Actually: MOV #addr, R1 = 4 bytes + JMP(R1) = 2 bytes → HALT at 0x1006
        prog = (
            _mov(2, 7, 0, 1) + _w(0x1006)  # MOV #0x1006, R1  (4 bytes at 0x1000)
            + _jmp(1, 1)                    # JMP (R1)         (2 bytes at 0x1004)
            + _mov(2, 7, 0, 0) + _w(99)    # MOV #99, R0 — skipped (4 bytes at 0x1006? no)
            + HALT                          # at 0x1006
        )
        # Wait: HALT is at 0x1006, but MOV occupies 0x1006-0x1009
        # Let me recalculate:
        # 0x1000: MOV #0x??, R1  (opcode 2 bytes + imm 2 bytes = 4 bytes)
        # 0x1004: JMP (R1)       (2 bytes)
        # 0x1006: MOV #99, R0    (4 bytes) — SKIP TARGET
        # 0x100A: HALT
        # We want to jump OVER MOV to HALT at 0x100A
        prog = (
            _mov(2, 7, 0, 1) + _w(0x100A)  # MOV #0x100A, R1
            + _jmp(1, 1)                    # JMP (R1)
            + _mov(2, 7, 0, 0) + _w(99)    # SKIPPED
            + HALT                          # at 0x100A
        )
        s = _run(prog)
        assert s.r[0] == 0   # skipped

    def test_jmp_mode0_raises(self):
        sim = PDP11Simulator()
        sim.load(_jmp(0, 0) + HALT)
        with pytest.raises(ValueError):
            sim.step()


# ── JSR / RTS tests ───────────────────────────────────────────────────────────

class TestJSR:
    def test_jsr_pc_and_rts(self):
        # Layout:
        # 0x1000: MOV #10, R0     (4 bytes)
        # 0x1004: JSR PC, sub     (2 bytes — sub is relative or absolute)
        # We use absolute addressing: JSR PC, @#sub_addr
        # 0x1006: immediate = address of subroutine
        # 0x1008: ADD R0, R0      (2 bytes) — skipped by JSR? no, executed after return
        # 0x100A: HALT
        # sub at 0x100C:
        # 0x100C: ADD #1, R0? No, let's do: ADD R0, R0 (double R0)
        # Actually let's do:
        # sub: ADD R0, R0 | RTS PC | back: HALT
        # 0x1000: MOV #5, R0      (4 bytes: 0x1000-0x1003)
        # 0x1004: JSR PC, @#sub   (2 bytes: 0x1004; immediate 0x1006: 2 bytes = sub addr)
        # Wait, JSR with mode 3 (absolute @#) takes an immediate word.
        # 0x1004: JSR opcode word; 0x1006: sub address; → PC = sub_addr after JSR
        # 0x1008: HALT  ← return address pushed by JSR
        # sub at 0x100C:
        # 0x100C: ADD R0, R0
        # 0x100E: RTS PC
        # Total: 16 bytes of program

        # Layout:
        # 0x1000: MOV #5, R0         (4 bytes: opcode+immediate)
        # 0x1004: JSR PC, @#sub_addr (4 bytes: opcode+absolute address word)
        # 0x1008: HALT               ← return address; execution resumes here after RTS
        # sub at 0x100A:
        # 0x100A: ADD R0, R0         (R0 becomes 10)
        # 0x100C: RTS PC             (return to 0x1008 = HALT)
        sub_addr = 0x100A
        prog = (
            _mov(2, 7, 0, 0) + _w(5)          # 0x1000: MOV #5, R0
            + _jsr(7, 3, 7) + _w(sub_addr)    # 0x1004: JSR PC, @#0x100A
            + HALT                              # 0x1008: return here
            + _add(0, 0, 0, 0)                 # 0x100A: ADD R0, R0 (R0 = 10)
            + _rts(7)                           # 0x100C: RTS PC
        )
        s = _run(prog)
        assert s.r[0] == 10

    def test_jsr_mode0_raises(self):
        sim = PDP11Simulator()
        sim.load(_jsr(7, 0, 0) + HALT)
        with pytest.raises(ValueError):
            sim.step()


# ── SOB tests ─────────────────────────────────────────────────────────────────

class TestSOB:
    def test_sob_loop(self):
        # Loop R2 times: ADD R0, R0 (double R0) each iteration
        # Starting R0=1, R2=3 → R0 = 1 * 2^3 = 8
        # Layout:
        # 0x1000: MOV #1, R0   (4 bytes)
        # 0x1004: MOV #3, R2   (4 bytes)
        # 0x1008: ADD R0, R0   (2 bytes)  ← loop body
        # 0x100A: SOB R2, 1    (2 bytes) → branch to 0x100A - 2 = 0x1008 when R2>0
        # 0x100C: HALT
        # 0x1000: MOV #1, R0         (4 bytes)
        # 0x1004: MOV #3, R2         (4 bytes)
        # 0x1008: ADD R0, R0         (2 bytes) ← loop body
        # 0x100A: SOB R2, offset=2   (2 bytes); PC after fetch = 0x100C
        #   branch target = 0x100C - 2*2 = 0x1008 ✓
        # 0x100C: HALT
        prog = (
            _mov(2, 7, 0, 0) + _w(1)   # MOV #1, R0
            + _mov(2, 7, 0, 2) + _w(3) # MOV #3, R2
            + _add(0, 0, 0, 0)          # ADD R0, R0 (doubles R0)  ← 0x1008
            + _sob(2, 2)                # SOB R2, 2 → target 0x100C - 4 = 0x1008 ✓
            + HALT
        )
        s = _run(prog)
        assert s.r[0] == 8    # 1 → 2 → 4 → 8

    def test_sob_no_branch_at_zero(self):
        # R2 starts at 1: decrement to 0 → no branch → fall through
        prog = (
            _mov(2, 7, 0, 2) + _w(1)  # MOV #1, R2
            + _sob(2, 1)               # SOB R2 → R2=0, no branch
            + HALT
        )
        s = _run(prog)
        assert s.r[2] == 0


# ── Addressing mode tests ─────────────────────────────────────────────────────

class TestAddressingModes:
    def test_mode0_register_direct(self):
        # MOV R0, R1 (both mode 0)
        prog = _mov(2, 7, 0, 0) + _w(42) + _mov(0, 0, 0, 1) + HALT
        s = _run(prog)
        assert s.r[1] == 42

    def test_mode1_register_deferred(self):
        # MOV (R1), R0: R1 points to address containing 0x1234
        # We'll put 0x1234 at 0x2000
        prog = (
            _mov(2, 7, 0, 1) + _w(0x2000)   # R1 = 0x2000
            + _mov(2, 7, 1, 1) + _w(0x1234) # MOV #0x1234, (R1) — write to 0x2000
            + _mov(1, 1, 0, 0)               # MOV (R1), R0
            + HALT
        )
        s = _run(prog)
        assert s.r[0] == 0x1234

    def test_mode2_autoincrement(self):
        # MOV (R1)+, R0: read from R1, then R1 += 2
        prog = (
            _mov(2, 7, 0, 1) + _w(0x2000)
            + _mov(2, 7, 1, 1) + _w(0xABCD)
            + _mov(2, 1, 0, 0)               # MOV (R1)+, R0
            + HALT
        )
        s = _run(prog)
        assert s.r[0] == 0xABCD
        assert s.r[1] == 0x2002   # incremented by 2

    def test_mode4_autodecrement(self):
        # MOV -(R1), R0: R1 -= 2, then read from R1
        # Set up R1=0x2002, write 0x5678 to 0x2000
        prog = (
            _mov(2, 7, 0, 1) + _w(0x2004)
            + _mov(2, 7, 1, 1) + _w(0x5678)  # write 0x5678 to 0x2004 wait...
        )
        # Let's be more explicit:
        # 1. Set R1 = 0x2002
        # 2. Write 0x9ABC to M[0x2000] via MOV #val, (R1) after setting R1=0x2000
        # 3. Set R1 = 0x2002
        # 4. MOV -(R1), R0 → R1=0x2000, read M[0x2000] = 0x9ABC
        prog = (
            _mov(2, 7, 0, 1) + _w(0x2000)  # R1 = 0x2000
            + _mov(2, 7, 1, 1) + _w(0x9ABC) # M[0x2000] = 0x9ABC; R1 stays 0x2000
        )
        # Actually MOV (dst mode 1) doesn't auto-increment; need to think
        # Use: MOV #val, @#addr (absolute addressing)
        # mode 3/R7 = absolute addressing: EA = M[PC]; PC+=2
        prog = (
            _mov(2, 7, 3, 7) + _w(0x9ABC) + _w(0x2000)  # MOV #0x9ABC, @#0x2000
            + _mov(2, 7, 0, 1) + _w(0x2002)              # R1 = 0x2002
            + _mov(4, 1, 0, 0)                             # MOV -(R1), R0
            + HALT
        )
        s = _run(prog)
        assert s.r[0] == 0x9ABC
        assert s.r[1] == 0x2000   # decremented to 0x2000

    def test_mode6_index(self):
        # MOV X(R1), R0: EA = R1 + next_word
        # R1 = 0x2000, X = 4 → EA = 0x2004
        prog = (
            _mov(2, 7, 3, 7) + _w(0x1111) + _w(0x2004)  # M[0x2004] = 0x1111
            + _mov(2, 7, 0, 1) + _w(0x2000)              # R1 = 0x2000
            + _mov(6, 1, 0, 0) + _w(4)                    # MOV 4(R1), R0
            + HALT
        )
        s = _run(prog)
        assert s.r[0] == 0x1111

    def test_mode3_autoincrement_deferred(self):
        # MOV @(R1)+, R0: EA = M[R1]; R1 += 2; R0 = M[EA]
        # R1 → 0x2000; M[0x2000] = 0x2004 (pointer); M[0x2004] = 0xABCD
        prog = (
            _mov(2, 7, 3, 7) + _w(0x2004) + _w(0x2000)  # M[0x2000] = 0x2004
            + _mov(2, 7, 3, 7) + _w(0xABCD) + _w(0x2004) # M[0x2004] = 0xABCD
            + _mov(2, 7, 0, 1) + _w(0x2000)               # R1 = 0x2000
            + _mov(3, 1, 0, 0)                              # MOV @(R1)+, R0
            + HALT
        )
        s = _run(prog)
        assert s.r[0] == 0xABCD
        assert s.r[1] == 0x2002   # autoincremented past the pointer word


# ── Byte instructions ─────────────────────────────────────────────────────────

class TestByteInstructions:
    def test_clrb_register(self):
        prog = _mov(2, 7, 0, 0) + _w(0xABCD) + _clrb(0, 0) + HALT
        s = _run(prog)
        # CLRB on register writes 0 to low byte: 0xABCD → 0xAB00
        assert s.r[0] == 0xAB00

    def test_incb_register(self):
        prog = _mov(2, 7, 0, 0) + _w(0x00FE) + _incb(0, 0) + HALT
        s = _run(prog)
        assert s.r[0] == 0x00FF

    def test_asrb_byte(self):
        prog = _mov(2, 7, 0, 0) + _w(0x0010) + _asrb(0, 0) + HALT
        s = _run(prog)
        assert (s.r[0] & 0xFF) == 0x08

    def test_rorb_byte(self):
        prog = _mov(2, 7, 0, 0) + _w(0x0001) + _rorb(0, 0) + HALT
        s = _run(prog)
        assert (s.r[0] & 0xFF) == 0
        assert s.c

    def test_negb_byte(self):
        prog = _mov(2, 7, 0, 0) + _w(0x0001) + _negb(0, 0) + HALT
        s = _run(prog)
        assert (s.r[0] & 0xFF) == 0xFF


# ── HALT and NOP ──────────────────────────────────────────────────────────────

class TestHaltNop:
    def test_halt_sets_halted(self):
        sim = PDP11Simulator()
        result = sim.execute(HALT)
        assert result.final_state.halted

    def test_nop_is_no_op(self):
        prog = NOP + NOP + HALT
        s = _run(prog)
        # All registers zero (except SP and PC)
        for i in range(6):
            assert s.r[i] == 0

    def test_unknown_opcode_raises(self):
        sim = PDP11Simulator()
        # 0x0800 with mode=0 is JSR with mode 0 which raises ValueError
        sim.load(_jsr(7, 0, 0) + HALT)
        with pytest.raises(ValueError):
            sim.step()
