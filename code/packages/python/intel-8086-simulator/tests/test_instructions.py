"""Test suite: per-instruction unit tests for X86Simulator.

Each instruction class gets its own test class.  Tests use short programs
that set up state, execute the instruction under test, and check results.

Encoding cheat sheet (used throughout)
---------------------------------------
MOV AX, imm16:  B8 lo hi
MOV BX, imm16:  BB lo hi
MOV CX, imm16:  B9 lo hi
MOV DX, imm16:  BA lo hi
MOV SP, imm16:  BC lo hi
MOV AL, imm8:   B0 n
MOV BL, imm8:   B3 n

ADD AX, BX:     01 D8   (ModRM D8 = mod=11 reg=BX(3) rm=AX(0))
ADD AX, imm16:  05 lo hi
ADD AL, imm8:   04 n
ADC AX, BX:     11 D8
SUB AX, BX:     29 D8
SUB AX, imm16:  2D lo hi
SBB AX, BX:     19 D8
INC AX:         40
DEC AX:         48
NEG AX:         F7 D8   (F7 /3, ModRM D8 = mod=11 reg=3 rm=AX(0))
CMP AX, BX:     3B C3   (d=1: AX - BX)

AND AX, BX:     21 D8
OR  AX, BX:     09 D8
XOR AX, BX:     31 D8
NOT AX:         F7 D0   (F7 /2, ModRM D0 = mod=11 reg=2 rm=AX)
TEST AX, BX:    85 C3

SHL AX, 1:      D1 E0   (D1 /4, ModRM E0 = mod=11 reg=4 rm=AX)
SHR AX, 1:      D1 E8   (D1 /5)
SAR AX, 1:      D1 F8   (D1 /7)
ROL AX, 1:      D1 C0   (D1 /0)
ROR AX, 1:      D1 C8   (D1 /1)

PUSH AX:        50
POP  BX:        5B
PUSHF:          9C
POPF:           9D

JMP short:      EB disp
JZ/JE:          74 disp
JNZ/JNE:        75 disp
JB/JC:          72 disp
JL:             7C disp
JGE:            7D disp
LOOP:           E2 disp
JCXZ:           E3 disp

HLT:            F4
NOP:            90
CLC:            F8
STC:            F9
CMC:            F5
CLD:            FC
STD:            FD
CBW:            98
CWD:            99

IN  AL, n:      E4 n
OUT n,  AL:     E6 n
"""

from __future__ import annotations

from intel_8086_simulator import X86Simulator

# ── Helpers ───────────────────────────────────────────────────────────────────


def run(*bytes_: int) -> object:
    """Run a program from raw bytes, append HLT, return final state."""
    prog = bytes(bytes_) + bytes([0xF4])
    return X86Simulator().execute(prog).final_state


def run_prog(prog: bytes) -> object:
    """Run a program bytes object (must include HLT), return final state."""
    return X86Simulator().execute(prog).final_state


def w16(val: int) -> tuple[int, int]:
    """Split a 16-bit value into (lo, hi) bytes."""
    return val & 0xFF, (val >> 8) & 0xFF


def mov_ax(val: int) -> list[int]:
    lo, hi = w16(val)
    return [0xB8, lo, hi]  # MOV AX, val


def mov_bx(val: int) -> list[int]:
    lo, hi = w16(val)
    return [0xBB, lo, hi]


def mov_cx(val: int) -> list[int]:
    lo, hi = w16(val)
    return [0xB9, lo, hi]


def mov_dx(val: int) -> list[int]:
    lo, hi = w16(val)
    return [0xBA, lo, hi]


def mov_sp(val: int) -> list[int]:
    lo, hi = w16(val)
    return [0xBC, lo, hi]


# ── MOV ───────────────────────────────────────────────────────────────────────


class TestMOV:
    def test_mov_ax_imm16(self):
        s = run(*mov_ax(0x1234))
        assert s.ax == 0x1234

    def test_mov_bx_imm16(self):
        s = run(*mov_bx(0xABCD))
        assert s.bx == 0xABCD

    def test_mov_cx_imm16(self):
        s = run(*mov_cx(500))
        assert s.cx == 500

    def test_mov_dx_imm16(self):
        s = run(*mov_dx(0xFFFF))
        assert s.dx == 0xFFFF

    def test_mov_reg_to_reg(self):
        # MOV AX,0x55; MOV BX,AX  (8B C3 = MOV AX,BX → d=1: AX←BX; but here BX←AX)
        # MOV BX, AX: opcode 89 (MOV r/m,reg w=1), ModRM D8 (mod=11 reg=AX rm=BX)
        s = run(*mov_ax(0x55), 0x89, 0xC3)   # MOV BX, AX
        assert s.bx == 0x55
        assert s.ax == 0x55  # ax unchanged

    def test_mov_al_imm8(self):
        s = run(0xB0, 0x42)   # MOV AL, 0x42
        assert s.al == 0x42

    def test_mov_ah_imm8(self):
        s = run(0xB4, 0x99)   # MOV AH, 0x99
        assert s.ah == 0x99

    def test_mov_memory_immediate(self):
        # MOV word ptr [0x0100], 0x1234  →  C7 06 00 01 34 12
        # After execution, memory[0x100] = 0x34, memory[0x101] = 0x12
        sim = X86Simulator()
        sim.reset()
        prog = bytes([0xC7, 0x06, 0x00, 0x01, 0x34, 0x12, 0xF4])
        result = sim.execute(prog)
        assert result.final_state.memory[0x100] == 0x34
        assert result.final_state.memory[0x101] == 0x12

    def test_mov_ax_from_memory(self):
        # MOV AX, [0x0200]  →  A1 00 02
        sim = X86Simulator()
        sim.reset()
        sim._mem[0x200] = 0xAB
        sim._mem[0x201] = 0xCD
        sim.load(bytes([0xA1, 0x00, 0x02, 0xF4]))
        sim.step(); sim.step()
        s = sim.get_state()
        assert s.ax == 0xCDAB

    def test_mov_memory_from_ax(self):
        # MOV [0x0300], AX  →  A3 00 03
        sim = X86Simulator()
        sim.reset()
        sim._ax = 0x1234
        sim.load(bytes([0xA3, 0x00, 0x03, 0xF4]))
        sim.step()
        assert sim._mem[0x300] == 0x34
        assert sim._mem[0x301] == 0x12

    def test_xchg_ax_bx(self):
        # XCHG AX, BX  →  93
        s = run(*mov_ax(0x11), *mov_bx(0x22), 0x93)
        assert s.ax == 0x22
        assert s.bx == 0x11


# ── ADD / ADC ─────────────────────────────────────────────────────────────────


class TestADD:
    def test_add_reg_reg(self):
        s = run(*mov_ax(10), *mov_bx(20), 0x01, 0xD8)  # ADD AX, BX
        assert s.ax == 30

    def test_add_sets_zf_when_zero(self):
        s = run(*mov_ax(0), *mov_bx(0), 0x01, 0xD8)
        assert s.zf is True
        assert s.ax == 0

    def test_add_sets_sf_on_negative_result(self):
        # 0x7FFF + 1 = 0x8000 (signed: -32768)
        s = run(*mov_ax(0x7FFF), *mov_bx(1), 0x01, 0xD8)
        assert s.sf is True
        assert s.of is True   # signed overflow!

    def test_add_sets_cf_on_unsigned_overflow(self):
        # 0xFFFF + 1 = 0x10000, CF=1
        s = run(*mov_ax(0xFFFF), *mov_bx(1), 0x01, 0xD8)
        assert s.cf is True
        assert s.ax == 0  # wrapped

    def test_add_ax_imm16(self):
        s = run(*mov_ax(100), 0x05, 0x37, 0x00)  # ADD AX, 55
        assert s.ax == 155

    def test_add_al_imm8(self):
        s = run(0xB0, 0x0A, 0x04, 0x05)  # MOV AL,10; ADD AL,5
        assert s.al == 15

    def test_adc_includes_carry(self):
        # Set CF=1 via STC, then ADC AX, BX (should add BX + 1)
        s = run(*mov_ax(10), *mov_bx(20), 0xF9, 0x11, 0xD8)  # STC; ADC AX,BX
        assert s.ax == 31  # 10 + 20 + 1

    def test_adc_without_carry(self):
        s = run(*mov_ax(10), *mov_bx(20), 0xF8, 0x11, 0xD8)  # CLC; ADC AX,BX
        assert s.ax == 30


# ── SUB / SBB / NEG ──────────────────────────────────────────────────────────


class TestSUB:
    def test_sub_basic(self):
        s = run(*mov_ax(50), *mov_bx(20), 0x29, 0xD8)  # SUB AX, BX
        assert s.ax == 30

    def test_sub_sets_zf_when_equal(self):
        s = run(*mov_ax(42), *mov_bx(42), 0x29, 0xD8)
        assert s.zf is True
        assert s.ax == 0

    def test_sub_sets_cf_on_borrow(self):
        s = run(*mov_ax(5), *mov_bx(10), 0x29, 0xD8)  # 5 - 10 = -5, borrow!
        assert s.cf is True
        assert s.ax == 0xFFFB  # 65531

    def test_sub_ax_imm16(self):
        s = run(*mov_ax(100), 0x2D, 0x1E, 0x00)  # SUB AX, 30
        assert s.ax == 70

    def test_sbb_subtracts_borrow(self):
        # Set CF=1, then SBB AX, BX
        s = run(*mov_ax(30), *mov_bx(10), 0xF9, 0x19, 0xD8)  # STC; SBB AX,BX
        assert s.ax == 19  # 30 - 10 - 1 = 19

    def test_neg_negates_value(self):
        s = run(*mov_ax(5), 0xF7, 0xD8)  # NEG AX
        assert s.ax == 0xFFFB  # -5 in two's complement
        assert s.cf is True  # CF=1 since operand != 0

    def test_neg_zero_clears_cf(self):
        s = run(*mov_ax(0), 0xF7, 0xD8)  # NEG 0 → 0; CF=0
        assert s.ax == 0
        assert s.cf is False

    def test_neg_min_int(self):
        s = run(*mov_ax(0x8000), 0xF7, 0xD8)  # NEG -32768 = -32768 (overflow)
        assert s.ax == 0x8000  # unchanged (wraps)
        assert s.of is True


# ── INC / DEC ─────────────────────────────────────────────────────────────────


class TestINCDEC:
    def test_inc_ax(self):
        s = run(*mov_ax(41), 0x40)  # INC AX
        assert s.ax == 42

    def test_inc_wraps_at_ffff(self):
        s = run(*mov_ax(0xFFFF), 0x40)
        assert s.ax == 0

    def test_inc_does_not_affect_cf(self):
        # CF should remain 0 even after INC wraps
        s = run(*mov_ax(0xFFFF), 0xF8, 0x40)  # CLC; INC AX
        assert s.cf is False

    def test_inc_sets_of_at_max_positive(self):
        # INC 0x7FFF → 0x8000 (signed overflow)
        s = run(*mov_ax(0x7FFF), 0x40)
        assert s.of is True

    def test_dec_bx(self):
        s = run(*mov_bx(10), 0x4B)  # DEC BX
        assert s.bx == 9

    def test_dec_does_not_affect_cf(self):
        s = run(*mov_ax(0), 0xF9, 0x48)  # STC; DEC AX
        assert s.cf is True   # CF preserved by DEC

    def test_dec_sets_zf_on_zero(self):
        s = run(*mov_ax(1), 0x48)   # DEC AX → 0
        assert s.zf is True

    def test_dec_sets_of_at_min_negative(self):
        # DEC 0x8000 → 0x7FFF (signed overflow: -32768 - 1 = +32767?)
        s = run(*mov_ax(0x8000), 0x48)
        assert s.of is True


# ── CMP / TEST ────────────────────────────────────────────────────────────────


class TestCMPTEST:
    def test_cmp_equal_sets_zf(self):
        s = run(*mov_ax(42), *mov_bx(42), 0x3B, 0xC3)  # CMP AX, BX
        assert s.zf is True
        assert s.ax == 42   # CMP does not modify AX

    def test_cmp_ax_greater_clears_zf_and_cf(self):
        s = run(*mov_ax(10), *mov_bx(5), 0x3B, 0xC3)
        assert s.zf is False
        assert s.cf is False

    def test_cmp_ax_less_sets_cf(self):
        s = run(*mov_ax(5), *mov_bx(10), 0x3B, 0xC3)
        assert s.cf is True  # borrow

    def test_cmp_imm_equal(self):
        s = run(*mov_ax(100), 0x3D, 0x64, 0x00)  # CMP AX, 100
        assert s.zf is True

    def test_test_does_not_modify_ax(self):
        s = run(*mov_ax(0xFF), *mov_bx(0x01), 0x85, 0xC3)  # TEST AX, BX
        assert s.ax == 0xFF  # unchanged
        assert s.zf is False   # 0xFF & 0x01 = 1 ≠ 0

    def test_test_sets_zf_on_zero_result(self):
        s = run(*mov_ax(0xF0), *mov_bx(0x0F), 0x85, 0xC3)  # TEST AX, BX
        assert s.zf is True   # 0xF0 & 0x0F = 0


# ── Logical operations ────────────────────────────────────────────────────────


class TestLogical:
    def test_and_basic(self):
        s = run(*mov_ax(0xFF), *mov_bx(0x0F), 0x21, 0xD8)  # AND AX, BX
        assert s.ax == 0x0F

    def test_and_clears_cf_of(self):
        s = run(*mov_ax(0xFF), *mov_bx(0x0F), 0xF9, 0x21, 0xD8)  # STC; AND AX,BX
        assert s.cf is False
        assert s.of is False

    def test_and_al_imm(self):
        s = run(*mov_ax(0xFF), 0x24, 0x0F)  # AND AL, 0x0F
        assert s.al == 0x0F

    def test_or_basic(self):
        s = run(*mov_ax(0xF0), *mov_bx(0x0F), 0x09, 0xD8)  # OR AX, BX
        assert s.ax == 0xFF

    def test_or_al_imm(self):
        s = run(*mov_ax(0x30), 0x0C, 0x05)  # OR AL, 5
        assert s.al == 0x35

    def test_xor_basic(self):
        s = run(*mov_ax(0xFF), *mov_bx(0xFF), 0x31, 0xD8)  # XOR AX, BX
        assert s.ax == 0
        assert s.zf is True

    def test_xor_ax_with_self_zeroes(self):
        # XOR AX, AX  →  31 C0 (mod=11 reg=AX(0) rm=AX(0) = 0xC0)
        s = run(*mov_ax(0x1234), 0x31, 0xC0)
        assert s.ax == 0

    def test_xor_ax_imm(self):
        s = run(*mov_ax(0x00FF), 0x35, 0xFF, 0xFF)  # XOR AX, 0xFFFF
        assert s.ax == 0xFF00

    def test_not_basic(self):
        s = run(*mov_ax(0xFF00), 0xF7, 0xD0)  # NOT AX (F7 /2, ModRM D0)
        assert s.ax == 0x00FF

    def test_not_does_not_affect_flags(self):
        # NOT should not change CF, ZF, etc.
        s = run(*mov_ax(0xFFFF), 0xF8, 0xF7, 0xD0)  # CLC; NOT AX
        assert s.cf is False  # preserved
        assert s.ax == 0


# ── Shifts and Rotates ────────────────────────────────────────────────────────


class TestShifts:
    def test_shl_ax_1(self):
        s = run(*mov_ax(1), 0xD1, 0xE0)  # SHL AX, 1
        assert s.ax == 2

    def test_shl_shifts_out_cf(self):
        s = run(*mov_ax(0x8000), 0xD1, 0xE0)  # SHL 0x8000 by 1 → CF=1
        assert s.cf is True
        assert s.ax == 0

    def test_shr_ax_1(self):
        s = run(*mov_ax(8), 0xD1, 0xE8)  # SHR AX, 1
        assert s.ax == 4

    def test_shr_logical_fill_with_0(self):
        s = run(*mov_ax(0x8000), 0xD1, 0xE8)  # SHR 0x8000, 1 → 0x4000
        assert s.ax == 0x4000
        assert s.sf is False

    def test_sar_ax_1_preserves_sign(self):
        # SAR 0x8000, 1 → 0xC000 (arithmetic shift, sign-fills)
        s = run(*mov_ax(0x8000), 0xD1, 0xF8)  # SAR AX, 1
        assert s.ax == 0xC000
        assert s.sf is True

    def test_shl_by_cl(self):
        # SHL AX, CL  (D3 E0)
        s = run(*mov_ax(1), *mov_cx(3), 0xD3, 0xE0)
        assert s.ax == 8

    def test_shr_by_cl(self):
        s = run(*mov_ax(16), *mov_cx(2), 0xD3, 0xE8)
        assert s.ax == 4

    def test_rol_ax_1(self):
        # ROL 0x8000, 1 → 0x0001, CF=1
        s = run(*mov_ax(0x8000), 0xD1, 0xC0)  # ROL AX, 1
        assert s.ax == 1
        assert s.cf is True

    def test_ror_ax_1(self):
        # ROR 1, 1 → 0x8000, CF=1
        s = run(*mov_ax(1), 0xD1, 0xC8)  # ROR AX, 1
        assert s.ax == 0x8000
        assert s.cf is True

    def test_rcl_ax_1(self):
        # RCL with CF=1: 0x0001 RCL 1 → 0x0003, CF=0
        s = run(*mov_ax(1), 0xF9, 0xD1, 0xD0)  # STC; RCL AX, 1
        assert s.ax == 3
        assert s.cf is False

    def test_rcr_ax_1(self):
        # RCR AX,1 with AX=0, CF=1 → AX=0x8000, CF=0.
        # CF enters as bit15; old bit0 exits as new CF.
        s = run(*mov_ax(0), 0xF9, 0xD1, 0xD8)  # STC; RCR AX, 1
        assert s.ax == 0x8000
        assert s.cf is False


# ── MUL / IMUL / DIV / IDIV ──────────────────────────────────────────────────


class TestMulDiv:
    def test_mul_word(self):
        # MUL BX: DX:AX ← AX × BX
        s = run(*mov_ax(100), *mov_bx(200), 0xF7, 0xE3)  # MUL BX (F7 /4 ModRM=E3)
        assert s.ax == 20000
        assert s.dx == 0  # fits in 16 bits

    def test_mul_overflow_into_dx(self):
        s = run(*mov_ax(0x1000), *mov_bx(0x1000), 0xF7, 0xE3)
        # 0x1000 × 0x1000 = 0x1000000 → DX:AX = 0x0100:0x0000
        assert s.dx == 0x0100
        assert s.ax == 0x0000
        assert s.cf is True  # CF=1 since DX≠0

    def test_imul_signed(self):
        # IMUL: AX(signed) × BX(signed) → DX:AX
        # -1 × 5 = -5 = 0xFFFF:0xFFFB
        s = run(*mov_ax(0xFFFF), *mov_bx(5), 0xF7, 0xEB)  # IMUL BX (F7 /5)
        assert s.ax_signed == -5

    def test_div_word(self):
        # DIV BX: AX ← DX:AX / BX, DX ← remainder
        # 20 / 3 = 6 r 2
        s = run(*mov_ax(20), *mov_bx(3), 0xF7, 0xF3)  # DIV BX (F7 /6)
        assert s.ax == 6
        assert s.dx == 2

    def test_idiv_signed(self):
        # IDIV: signed division
        # -20 / 3 → quotient -6 in AL, remainder -2 in AH
        # CWD sign-extends AX=-20 (0xFFEC) into DX:AX before the divide.
        s = run(*mov_ax(0xFFEC), *mov_bx(3), 0x99, 0xF7, 0xFB)  # CWD; IDIV BX
        # quotient in AX (16-bit result), ax_signed = -6
        assert s.ax_signed == -6


# ── PUSH / POP ────────────────────────────────────────────────────────────────


class TestPushPop:
    def test_push_pop_roundtrip(self):
        # MOV SP, 0x1000; MOV AX, 0xDEAD; PUSH AX; POP BX
        s = run(*mov_sp(0x1000), *mov_ax(0xDEAD), 0x50, 0x5B)
        assert s.bx == 0xDEAD

    def test_push_decrements_sp(self):
        sim = X86Simulator()
        sim.reset()
        sim._sp = 0x1000
        sim.load(bytes([0x50, 0xF4]))  # PUSH AX; HLT
        sim.step(); sim.step()
        assert sim._sp == 0x0FFE  # SP - 2

    def test_pop_increments_sp(self):
        sim = X86Simulator()
        sim.reset()
        sim._sp = 0x1000
        sim._mem[0x1000] = 0x34  # value on stack
        sim._mem[0x1001] = 0x12
        sim.load(bytes([0x5B, 0xF4]))  # POP BX; HLT
        sim.step(); sim.step()
        assert sim._sp == 0x1002
        assert sim._bx == 0x1234

    def test_pushf_popf(self):
        # Set some flags, PUSHF, clear flags, POPF, check restored
        s = run(0xF9, 0x9C, 0xF8, 0x9D)  # STC; PUSHF; CLC; POPF
        assert s.cf is True  # CF restored from FLAGS on stack


# ── Control flow ──────────────────────────────────────────────────────────────


class TestControlFlow:
    def test_jmp_short_forward(self):
        # JMP +2 skips next 2 bytes (MOV BX,1), executes HLT
        prog = bytes([
            0xEB, 0x03,          # JMP +3
            0xBB, 0x01, 0x00,   # MOV BX, 1  (skipped)
            0xF4,                # HLT
        ])
        s = run_prog(prog)
        assert s.bx == 0   # MOV BX, 1 was skipped

    def test_jmp_short_backward(self):
        # Test loop exit: MOV CX,2; body; DEC CX; JNZ body; HLT
        # JNZ at offset 5: IP after = 7; target = 3 (INC AX); disp = 3-7 = -4 = 0xFC
        prog = bytes([
            0xB9, 0x02, 0x00,   # 0: MOV CX, 2
            0x40,                # 3: INC AX
            0x49,                # 4: DEC CX
            0x75, 0xFC,          # 5: JNZ -4 (back to offset 3)
            0xF4,                # 7: HLT
        ])
        s = run_prog(prog)
        assert s.ax == 2   # INC AX ran twice

    def test_jz_taken_when_zf(self):
        # CMP AX, AX; JZ +2 (skip MOV BX, 1)
        prog = bytes([
            0x3B, 0xC0,          # CMP AX, AX  (ZF=1)
            0x74, 0x03,          # JZ +3
            0xBB, 0x01, 0x00,   # MOV BX, 1  (skipped)
            0xF4,                # HLT
        ])
        s = run_prog(prog)
        assert s.bx == 0

    def test_jz_not_taken_when_not_zf(self):
        prog = bytes([
            *mov_ax(1), *mov_bx(2),
            0x3B, 0xC3,          # CMP AX, BX  (ZF=0)
            0x74, 0x01,          # JZ +1  (not taken)
            0xF4,                # HLT
        ])
        s = run_prog(prog)
        assert s.halted

    def test_jnz_taken_when_not_zf(self):
        prog = bytes([
            *mov_ax(1),
            0x3D, 0x00, 0x00,   # CMP AX, 0  (ZF=0)
            0x75, 0x01,          # JNZ +1  (taken)
            0x40,                # INC AX  (skipped)
            0xF4,
        ])
        s = run_prog(prog)
        assert s.ax == 1  # INC was skipped

    def test_jb_taken_when_cf(self):
        prog = bytes([
            *mov_ax(5), *mov_bx(10),
            0x3B, 0xC3,          # CMP AX, BX  (CF=1 since 5<10)
            0x72, 0x03,          # JB +3  (taken)
            *mov_ax(0xFFFF),     # not reached (3 bytes)
            0xF4,
        ])
        s = run_prog(prog)
        assert s.ax == 5  # MOV AX,0xFFFF skipped

    def test_jl_taken_when_sf_ne_of(self):
        # -1 < 0 → JL taken
        prog = bytes([
            *mov_ax(0xFFFF),     # -1 signed
            0x3D, 0x00, 0x00,   # CMP AX, 0  (SF≠OF → JL taken)
            0x7C, 0x03,          # JL +3
            *mov_ax(0),          # skipped
            0xF4,
        ])
        s = run_prog(prog)
        assert s.ax == 0xFFFF

    def test_call_ret(self):
        # CALL routine; routine does MOV BX,42; RET
        # MOV SP at 0, CALL at 3 (IP after=6), target=7 (MOV BX,42), disp=7-6=1
        prog = bytes([
            0xBC, 0x00, 0x10,   # 0: MOV SP, 0x1000
            0xE8, 0x01, 0x00,   # 3: CALL +1 (to offset 7)
            0xF4,                # 6: HLT  (return here)
            0xBB, 0x2A, 0x00,   # 7: MOV BX, 42
            0xC3,                # 10: RET
        ])
        s = run_prog(prog)
        assert s.bx == 42

    def test_loop_decrements_cx(self):
        prog = bytes([
            *mov_cx(3),
            0xE2, 0xFE,   # LOOP -2 (back to itself; exits when CX reaches 0)
            0xF4,
        ])
        s = run_prog(prog)
        assert s.cx == 0

    def test_jcxz_taken_when_cx_zero(self):
        prog = bytes([
            *mov_cx(0),
            0xE3, 0x01,   # JCXZ +1 (skip NOP)
            0x90,          # NOP (skipped)
            *mov_bx(1),   # BX=1 (reached)
            0xF4,
        ])
        s = run_prog(prog)
        assert s.bx == 1

    def test_jcxz_not_taken_when_cx_nonzero(self):
        prog = bytes([
            *mov_cx(1),
            0xE3, 0x03,   # JCXZ +3  (not taken)
            *mov_bx(1),   # BX=1 (reached)
            0xF4,
        ])
        s = run_prog(prog)
        assert s.bx == 1


# ── Miscellaneous ─────────────────────────────────────────────────────────────


class TestMisc:
    def test_nop(self):
        s = run(0x90)
        assert s.ax == 0  # NOP changes nothing

    def test_clc_stc_cmc(self):
        s = run(0xF9)  # STC
        assert s.cf is True
        s2 = run(0xF9, 0xF8)  # STC; CLC
        assert s2.cf is False
        s3 = run(0xF8, 0xF5)  # CLC; CMC
        assert s3.cf is True

    def test_cld_std(self):
        s = run(0xFD)  # STD
        assert s.df is True
        s2 = run(0xFD, 0xFC)  # STD; CLD
        assert s2.df is False

    def test_cli_sti(self):
        s = run(0xFB)  # STI
        assert s.if_ is True
        s2 = run(0xFB, 0xFA)  # STI; CLI
        assert s2.if_ is False

    def test_cbw_positive(self):
        s = run(0xB0, 0x42, 0x98)  # MOV AL,0x42; CBW
        assert s.ax == 0x0042  # sign extended (positive)

    def test_cbw_negative(self):
        s = run(0xB0, 0x80, 0x98)  # MOV AL,0x80; CBW
        assert s.ax == 0xFF80   # sign extended (negative)

    def test_cwd_positive(self):
        s = run(*mov_ax(0x1234), 0x99)  # CWD
        assert s.dx == 0  # positive AX → DX=0

    def test_cwd_negative(self):
        s = run(*mov_ax(0x8000), 0x99)  # CWD
        assert s.dx == 0xFFFF  # negative AX → DX=0xFFFF

    def test_lahf(self):
        # STC; LAHF → AH has CF set
        s = run(0xF9, 0x9F)  # STC; LAHF
        assert (s.ah & 1) == 1   # bit 0 of AH = CF

    def test_sahf(self):
        # MOV AH, 0x40 (ZF bit); SAHF → ZF set
        s = run(0xB4, 0x40, 0x9E)  # MOV AH, 0x40; SAHF
        assert s.zf is True  # bit 6 of AH = ZF

    def test_in_out_byte(self):
        # execute() resets state, so set ports after reset, then step manually.
        sim = X86Simulator()
        sim.reset()
        sim._input_ports[0x10] = 0xAB
        sim.load(bytes([0xE4, 0x10, 0xE6, 0x20, 0xF4]))  # IN AL,0x10; OUT 0x20,AL; HLT
        while not sim._halted:
            sim.step()
        state = sim.get_state()
        assert state.al == 0xAB
        assert state.output_ports[0x20] == 0xAB

    def test_in_dx_out_dx(self):
        # Set port before stepping (not before execute, which would reset it).
        sim = X86Simulator()
        sim.reset()
        sim._input_ports[5] = 0x77
        prog = bytes([
            0xBA, 0x05, 0x00,   # MOV DX, 5
            0xEC,               # IN AL, DX
            0xBA, 0x0A, 0x00,   # MOV DX, 10
            0xEE,               # OUT DX, AL
            0xF4,
        ])
        sim.load(prog)
        while not sim._halted:
            sim.step()
        state = sim.get_state()
        assert state.al == 0x77
        assert state.output_ports[10] == 0x77

    def test_lea(self):
        # LEA AX, [BX+SI] where BX=0x100, SI=0x50 → AX=0x150
        # LEA AX,[BX+SI] = 8D 00 (mod=00 reg=AX(0) rm=BX+SI(0))
        s = run(*mov_bx(0x100), *mov_ax(0), 0xBE, 0x50, 0x00,  # MOV SI, 0x50
                0x8D, 0x00)   # LEA AX, [BX+SI]
        assert s.ax == 0x150

    def test_xlat(self):
        # Build a table at DS:BX; load via XLAT
        sim = X86Simulator()
        sim.reset()
        sim._bx = 0x200          # table base
        sim._mem[0x205] = 0x99   # table[5] = 0x99
        sim._ax = 5               # AL = index
        sim.load(bytes([0xD7, 0xF4]))  # XLAT; HLT
        sim.step(); sim.step()
        assert sim._ax & 0xFF == 0x99


# ── String operations ─────────────────────────────────────────────────────────


class TestStringOps:
    def test_stos_byte(self):
        # STOS byte: [ES:DI] ← AL; DI += 1
        sim = X86Simulator()
        sim.reset()
        sim._ax = 0x42  # AL = 0x42
        sim._di = 0x100
        sim.load(bytes([0xAA, 0xF4]))  # STOSB; HLT
        sim.step(); sim.step()
        assert sim._mem[0x100] == 0x42  # ES=0, so physical=0x100
        assert sim._di == 0x101

    def test_stos_word(self):
        sim = X86Simulator()
        sim.reset()
        sim._ax = 0x1234
        sim._di = 0x100
        sim.load(bytes([0xAB, 0xF4]))  # STOSW; HLT
        sim.step(); sim.step()
        assert sim._mem[0x100] == 0x34
        assert sim._mem[0x101] == 0x12
        assert sim._di == 0x102

    def test_rep_stos_fills_memory(self):
        # REP STOSB fills CX bytes with AL
        sim = X86Simulator()
        sim.reset()
        sim._ax = 0x55  # AL
        sim._cx = 5
        sim._di = 0x300
        sim.load(bytes([0xF3, 0xAA, 0xF4]))  # REP STOSB; HLT
        sim.step(); sim.step()
        for i in range(5):
            assert sim._mem[0x300 + i] == 0x55
        assert sim._cx == 0
        assert sim._di == 0x305

    def test_lods_byte(self):
        sim = X86Simulator()
        sim.reset()
        sim._mem[0x100] = 0xAB
        sim._si = 0x100
        sim.load(bytes([0xAC, 0xF4]))  # LODSB; HLT
        sim.step(); sim.step()
        assert (sim._ax & 0xFF) == 0xAB
        assert sim._si == 0x101

    def test_movs_byte(self):
        sim = X86Simulator()
        sim.reset()
        sim._mem[0x100] = 0xCC  # source at DS:SI=0x100
        sim._si = 0x100
        sim._di = 0x200           # dest at ES:DI=0x200
        sim.load(bytes([0xA4, 0xF4]))  # MOVSB; HLT
        sim.step(); sim.step()
        assert sim._mem[0x200] == 0xCC
        assert sim._si == 0x101
        assert sim._di == 0x201

    def test_stos_backward_with_df(self):
        # STD; STOSB (DF=1 → DI decrements)
        sim = X86Simulator()
        sim.reset()
        sim._ax = 0x77
        sim._di = 0x105
        sim.load(bytes([0xFD, 0xAA, 0xF4]))  # STD; STOSB; HLT
        sim.step(); sim.step(); sim.step()
        assert sim._mem[0x105] == 0x77
        assert sim._di == 0x104  # decremented


# ── Flag accuracy ─────────────────────────────────────────────────────────────


class TestFlags:
    def test_parity_even_sets_pf(self):
        # ADD AL, 0  → result=0xFF (all 1s, even count)
        s = run(0xB0, 0xFF, 0x04, 0x00)  # MOV AL,0xFF; ADD AL,0
        assert s.pf is True   # 0xFF has 8 ones — even

    def test_parity_odd_clears_pf(self):
        s = run(0xB0, 0x01, 0x04, 0x00)  # MOV AL,1; ADD AL,0
        assert s.pf is False   # 0x01 has 1 one — odd

    def test_af_set_on_nibble_carry(self):
        # 0x0F + 0x01 = 0x10 → AF=1
        s = run(0xB0, 0x0F, 0x04, 0x01)  # MOV AL,0x0F; ADD AL,1
        assert s.af is True

    def test_of_cleared_by_and(self):
        s = run(*mov_ax(0x7FFF), *mov_bx(1), 0x01, 0xD8,  # ADD AX,BX → OF=1
                0x21, 0xD8)  # AND AX,BX → OF=0
        assert s.of is False

    def test_flags_property_packs_correctly(self):
        # After STC: CF=1 → flags bit 0 = 1; bit 1 always 1
        s = run(0xF9)  # STC
        assert s.flags & 1 == 1  # CF bit
        assert s.flags & 2 == 2  # always-1 bit

    def test_ax_signed_property(self):
        s = run(*mov_ax(0x8000))
        assert s.ax_signed == -32768

    def test_al_signed_property(self):
        s = run(0xB0, 0x80)  # MOV AL, 0x80
        assert s.al_signed == -128
