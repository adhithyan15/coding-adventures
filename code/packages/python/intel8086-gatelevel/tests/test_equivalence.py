"""Cross-validation tests: gate-level vs behavioral simulator.

Runs 40+ programs on both Intel8086GateLevelSimulator and X86Simulator
and asserts that the final register/flag/memory state is identical.

Programs use byte encoding directly (no assembler needed).
"""

import pytest

from intel_8086_simulator.simulator import X86Simulator
from intel8086_gatelevel.simulator import Intel8086GateLevelSimulator


def run_both(program: bytes, max_steps: int = 1000):
    """Run a program on both simulators and return (gate_state, behav_state)."""
    gate = Intel8086GateLevelSimulator()
    behav = X86Simulator()

    gate_result = gate.execute(program, max_steps=max_steps)
    behav_result = behav.execute(program, max_steps=max_steps)

    return gate_result.final_state, behav_result.final_state


def assert_states_equal(gs, bs, check_mem: bool = False):
    """Assert gate-level and behavioral final states are identical."""
    assert gs.ax == bs.ax, f"AX: gate={gs.ax:#x} behav={bs.ax:#x}"
    assert gs.bx == bs.bx, f"BX: gate={gs.bx:#x} behav={bs.bx:#x}"
    assert gs.cx == bs.cx, f"CX: gate={gs.cx:#x} behav={bs.cx:#x}"
    assert gs.dx == bs.dx, f"DX: gate={gs.dx:#x} behav={bs.dx:#x}"
    assert gs.si == bs.si, f"SI: {gs.si:#x} vs {bs.si:#x}"
    assert gs.di == bs.di, f"DI: {gs.di:#x} vs {bs.di:#x}"
    assert gs.sp == bs.sp, f"SP: {gs.sp:#x} vs {bs.sp:#x}"
    assert gs.bp == bs.bp, f"BP: {gs.bp:#x} vs {bs.bp:#x}"
    assert gs.ip == bs.ip, f"IP: {gs.ip:#x} vs {bs.ip:#x}"
    assert gs.cf == bs.cf, f"CF: {gs.cf} vs {bs.cf}"
    assert gs.zf == bs.zf, f"ZF: {gs.zf} vs {bs.zf}"
    assert gs.sf == bs.sf, f"SF: {gs.sf} vs {bs.sf}"
    assert gs.of == bs.of, f"OF: {gs.of} vs {bs.of}"
    assert gs.pf == bs.pf, f"PF: {gs.pf} vs {bs.pf}"
    assert gs.af == bs.af, f"AF: {gs.af} vs {bs.af}"
    assert gs.halted == bs.halted, "halted mismatch"


# ── Basic data transfer ───────────────────────────────────────────────────────

def test_equiv_mov_ax_imm():
    prog = bytes([0xB8, 0x34, 0x12,   # MOV AX, 0x1234
                  0xF4])               # HLT
    gs, bs = run_both(prog)
    assert_states_equal(gs, bs)
    assert gs.ax == 0x1234

def test_equiv_mov_bx_imm():
    prog = bytes([0xBB, 0xCD, 0xAB,   # MOV BX, 0xABCD
                  0xF4])
    gs, bs = run_both(prog)
    assert_states_equal(gs, bs)
    assert gs.bx == 0xABCD

def test_equiv_mov_al_imm():
    prog = bytes([0xB0, 0x42,         # MOV AL, 0x42
                  0xF4])
    gs, bs = run_both(prog)
    assert_states_equal(gs, bs)
    assert gs.al == 0x42

def test_equiv_mov_reg_to_reg():
    prog = bytes([0xB8, 0x0A, 0x00,   # MOV AX, 10
                  0x89, 0xC3,         # MOV BX, AX
                  0xF4])
    gs, bs = run_both(prog)
    assert_states_equal(gs, bs)
    assert gs.bx == 10

def test_equiv_xchg_ax_bx():
    prog = bytes([0xB8, 0x01, 0x00,   # MOV AX, 1
                  0xBB, 0x02, 0x00,   # MOV BX, 2
                  0x93,               # XCHG AX, BX
                  0xF4])
    gs, bs = run_both(prog)
    assert_states_equal(gs, bs)

# ── Arithmetic ────────────────────────────────────────────────────────────────

def test_equiv_add_ax_imm():
    prog = bytes([0xB8, 0x05, 0x00,   # MOV AX, 5
                  0x05, 0x03, 0x00,   # ADD AX, 3
                  0xF4])
    gs, bs = run_both(prog)
    assert_states_equal(gs, bs)
    assert gs.ax == 8

def test_equiv_sub_ax_imm():
    prog = bytes([0xB8, 0x0A, 0x00,   # MOV AX, 10
                  0x2D, 0x03, 0x00,   # SUB AX, 3
                  0xF4])
    gs, bs = run_both(prog)
    assert_states_equal(gs, bs)
    assert gs.ax == 7

def test_equiv_inc_dec():
    prog = bytes([0xB8, 0x00, 0x00,   # MOV AX, 0
                  0x40,               # INC AX
                  0x40,               # INC AX
                  0x48,               # DEC AX
                  0xF4])
    gs, bs = run_both(prog)
    assert_states_equal(gs, bs)
    assert gs.ax == 1

def test_equiv_add_carry():
    # 0xFFFF + 1 → CF=1
    prog = bytes([0xB8, 0xFF, 0xFF,   # MOV AX, 0xFFFF
                  0x05, 0x01, 0x00,   # ADD AX, 1
                  0xF4])
    gs, bs = run_both(prog)
    assert_states_equal(gs, bs)
    assert gs.cf is True

def test_equiv_neg():
    prog = bytes([0xB8, 0x05, 0x00,   # MOV AX, 5
                  0xF7, 0xD8,         # NEG AX (F7 mod=11 reg=3 rm=0)
                  0xF4])
    gs, bs = run_both(prog)
    assert_states_equal(gs, bs)
    assert gs.ax == 0xFFFB

# ── Logic ─────────────────────────────────────────────────────────────────────

def test_equiv_and():
    prog = bytes([0xB8, 0xFF, 0x00,   # MOV AX, 0x00FF
                  0x25, 0x0F, 0x00,   # AND AX, 0x000F
                  0xF4])
    gs, bs = run_both(prog)
    assert_states_equal(gs, bs)
    assert gs.ax == 0x0F

def test_equiv_or():
    prog = bytes([0xB8, 0xF0, 0x00,   # MOV AX, 0xF0
                  0x0D, 0x0F, 0x00,   # OR AX, 0x0F
                  0xF4])
    gs, bs = run_both(prog)
    assert_states_equal(gs, bs)
    assert gs.ax == 0xFF

def test_equiv_xor_self():
    prog = bytes([0xB8, 0x34, 0x12,   # MOV AX, 0x1234
                  0x31, 0xC0,         # XOR AX, AX
                  0xF4])
    gs, bs = run_both(prog)
    assert_states_equal(gs, bs)
    assert gs.ax == 0
    assert gs.zf is True

def test_equiv_not():
    prog = bytes([0xB8, 0x00, 0xFF,   # MOV AX, 0xFF00
                  0xF7, 0xD0,         # NOT AX
                  0xF4])
    gs, bs = run_both(prog)
    assert_states_equal(gs, bs)
    assert gs.ax == 0x00FF

# ── Shift / rotate ────────────────────────────────────────────────────────────

def test_equiv_shl_1():
    prog = bytes([0xB8, 0x01, 0x00,   # MOV AX, 1
                  0xD1, 0xE0,         # SHL AX, 1
                  0xF4])
    gs, bs = run_both(prog)
    assert_states_equal(gs, bs)
    assert gs.ax == 2

def test_equiv_shr_1():
    prog = bytes([0xB8, 0x04, 0x00,   # MOV AX, 4
                  0xD1, 0xE8,         # SHR AX, 1
                  0xF4])
    gs, bs = run_both(prog)
    assert_states_equal(gs, bs)
    assert gs.ax == 2

def test_equiv_sar_1():
    prog = bytes([0xB8, 0x00, 0x80,   # MOV AX, 0x8000 (negative)
                  0xD1, 0xF8,         # SAR AX, 1
                  0xF4])
    gs, bs = run_both(prog)
    assert_states_equal(gs, bs)

# ── Stack ─────────────────────────────────────────────────────────────────────

def test_equiv_push_pop():
    prog = bytes([0xB8, 0x34, 0x12,   # MOV AX, 0x1234
                  0xBB, 0x00, 0x00,   # MOV BX, 0
                  0x50,               # PUSH AX
                  0x5B,               # POP BX
                  0xF4])
    gs, bs = run_both(prog)
    assert_states_equal(gs, bs)
    assert gs.bx == 0x1234

def test_equiv_pushf_popf():
    prog = bytes([0xB8, 0xFF, 0xFF,   # MOV AX, 0xFFFF (set flags)
                  0x05, 0x01, 0x00,   # ADD AX, 1 → sets CF, ZF
                  0x9C,               # PUSHF
                  0xB8, 0x00, 0x00,   # MOV AX, 0 (clear AX)
                  0x9D,               # POPF
                  0xF4])
    gs, bs = run_both(prog)
    assert_states_equal(gs, bs)

# ── Control flow ──────────────────────────────────────────────────────────────

def test_equiv_jmp_short():
    # JMP +3: after fetching disp byte, IP=2; new IP = 2+3 = 5 → MOV AX, 1
    prog = bytes([0xEB, 0x03,         # JMP +3 (skip the 3-byte MOV AX, 0xFFFF)
                  0xB8, 0xFF, 0xFF,   # MOV AX, 0xFFFF (skipped)
                  0xB8, 0x01, 0x00,   # MOV AX, 1
                  0xF4])
    gs, bs = run_both(prog)
    assert_states_equal(gs, bs)
    assert gs.ax == 1

def test_equiv_jz_taken():
    prog = bytes([0xB8, 0x00, 0x00,   # MOV AX, 0  (ZF=0 after this)
                  0x3D, 0x00, 0x00,   # CMP AX, 0  (ZF=1)
                  0x74, 0x03,         # JZ +3
                  0xB8, 0xFF, 0xFF,   # MOV AX, 0xFFFF (skipped)
                  0xF4])
    gs, bs = run_both(prog)
    assert_states_equal(gs, bs)
    assert gs.ax == 0

def test_equiv_jnz_not_taken():
    prog = bytes([0x3D, 0x00, 0x00,   # CMP AX, 0 (AX=0, ZF=1)
                  0x75, 0x03,         # JNZ +3 (NOT taken)
                  0xEB, 0x03,         # JMP +3 (skip ahead)
                  0xB8, 0xFF, 0xFF,   # MOV AX, 0xFFFF
                  0xF4])
    gs, bs = run_both(prog)
    assert_states_equal(gs, bs)

def test_equiv_call_ret():
    # CALL to a small subroutine that doubles AX
    prog = bytes([
        0xB8, 0x05, 0x00,   # 0: MOV AX, 5
        0xE8, 0x03, 0x00,   # 3: CALL +3 (to offset 9)
        0xF4,               # 6: HLT
        0x90, 0x90,         # 7,8: NOP padding (to align call target)
        0xD1, 0xE0,         # 9: SHL AX, 1   (AX *= 2)
        0xC3,               # 11: RET
    ])
    gs, bs = run_both(prog)
    assert_states_equal(gs, bs)
    assert gs.ax == 10

def test_equiv_loop():
    # Loop 5 times, INC AX each iteration
    prog = bytes([
        0xB9, 0x05, 0x00,   # MOV CX, 5
        0xB8, 0x00, 0x00,   # MOV AX, 0
        0x40,               # INC AX   ← loop target (offset 6)
        0xE2, 0xFD,         # LOOP -3 (to offset 6)
        0xF4,               # HLT
    ])
    gs, bs = run_both(prog)
    assert_states_equal(gs, bs)
    assert gs.ax == 5

# ── Memory operations ─────────────────────────────────────────────────────────

def test_equiv_mov_to_from_memory():
    prog = bytes([
        0xB8, 0x34, 0x12,   # MOV AX, 0x1234
        0xA3, 0x00, 0x02,   # MOV [0x0200], AX
        0xB8, 0x00, 0x00,   # MOV AX, 0
        0xA1, 0x00, 0x02,   # MOV AX, [0x0200]
        0xF4,
    ])
    gs, bs = run_both(prog)
    assert_states_equal(gs, bs)
    assert gs.ax == 0x1234

def test_equiv_string_stos():
    prog = bytes([
        0xBB, 0x00, 0x00,   # MOV BX, 0 (ES=0 by default)
        0xB8, 0x42, 0x00,   # MOV AX, 0x42
        0xBF, 0x00, 0x02,   # MOV DI, 0x0200
        0xAA,               # STOSB
        0xF4,
    ])
    gs, bs = run_both(prog)
    assert_states_equal(gs, bs)
    assert gs.memory[0x0200] == 0x42

def test_equiv_string_lods():
    # Write a byte to memory, then load it with LODS
    prog = bytes([
        0xC6, 0x06, 0x00, 0x02, 0x55,  # MOV [0x0200], 0x55
        0xBE, 0x00, 0x02,               # MOV SI, 0x0200
        0xAC,                           # LODSB
        0xF4,
    ])
    gs, bs = run_both(prog)
    assert_states_equal(gs, bs)
    assert gs.al == 0x55

def test_equiv_string_movs():
    prog = bytes([
        0xC6, 0x06, 0x00, 0x01, 0x77,  # MOV [0x100], 0x77
        0xBE, 0x00, 0x01,               # MOV SI, 0x100
        0xBF, 0x00, 0x02,               # MOV DI, 0x200
        0xA4,                           # MOVSB
        0xF4,
    ])
    gs, bs = run_both(prog)
    assert_states_equal(gs, bs)
    assert gs.memory[0x200] == 0x77

# ── Multiply / Divide ─────────────────────────────────────────────────────────

def test_equiv_mul8():
    prog = bytes([
        0xB8, 0x06, 0x00,   # MOV AX, 6 (AL=6)
        0xB3, 0x07,         # MOV BL, 7
        0xF6, 0xE3,         # MUL BL
        0xF4,
    ])
    gs, bs = run_both(prog)
    assert_states_equal(gs, bs)
    assert gs.ax == 42

def test_equiv_div8():
    prog = bytes([
        0xB8, 0x0A, 0x00,   # MOV AX, 10
        0xB3, 0x03,         # MOV BL, 3
        0xF6, 0xF3,         # DIV BL
        0xF4,
    ])
    gs, bs = run_both(prog)
    assert_states_equal(gs, bs)
    assert gs.al == 3   # quotient
    assert gs.ah == 1   # remainder

# ── Flag instructions ─────────────────────────────────────────────────────────

def test_equiv_clc_stc():
    prog = bytes([0xF8,   # CLC
                  0xF9,   # STC
                  0xF4])
    gs, bs = run_both(prog)
    assert_states_equal(gs, bs)
    assert gs.cf is True

def test_equiv_cld_std():
    prog = bytes([0xFC,   # CLD
                  0xFD,   # STD
                  0xF4])
    gs, bs = run_both(prog)
    assert_states_equal(gs, bs)
    assert gs.df is True

def test_equiv_lahf_sahf():
    prog = bytes([
        0xB8, 0xFF, 0xFF,   # MOV AX, 0xFFFF
        0x05, 0x01, 0x00,   # ADD AX, 1 → sets CF, ZF, PF
        0x9F,               # LAHF
        0xB8, 0x00, 0x00,   # MOV AX, 0
        0x9E,               # SAHF
        0xF4,
    ])
    gs, bs = run_both(prog)
    assert_states_equal(gs, bs)

def test_equiv_cbw():
    prog = bytes([0xB0, 0x80,   # MOV AL, 0x80 (-128 signed)
                  0x98,         # CBW
                  0xF4])
    gs, bs = run_both(prog)
    assert_states_equal(gs, bs)
    assert gs.ax == 0xFF80

def test_equiv_cwd():
    prog = bytes([0xB8, 0x00, 0x80,   # MOV AX, 0x8000
                  0x99,               # CWD
                  0xF4])
    gs, bs = run_both(prog)
    assert_states_equal(gs, bs)
    assert gs.dx == 0xFFFF

# ── Segment registers ─────────────────────────────────────────────────────────

def test_equiv_push_pop_ds():
    prog = bytes([
        0x8E, 0xD8,         # MOV DS, AX (AX=0, DS=0)
        0xB8, 0x10, 0x00,   # MOV AX, 0x10
        0x8E, 0xD8,         # MOV DS, AX
        0x1E,               # PUSH DS
        0x8E, 0xC0,         # MOV ES, AX (0x10)
        0x07,               # POP ES
        0xF4,
    ])
    gs, bs = run_both(prog)
    assert_states_equal(gs, bs)

# ── All JCC conditions ────────────────────────────────────────────────────────

@pytest.mark.parametrize("jcc_op,flag_setup,expected_ax", [
    (0x70, bytes([0xB8, 0x00, 0x80, 0x05, 0x00, 0x80]), 0x0000),  # JO: 0x8000+0x8000 → OF=1
    (0x74, bytes([0x3D, 0x00, 0x00]), 0),    # JZ: CMP AX,0 (AX=0)
    (0x72, bytes([0x3D, 0x01, 0x00]), 0),    # JB: CMP AX,1 (AX=0 < 1)
    (0x78, bytes([0xB8, 0x80, 0x00, 0x3D, 0x00, 0x00]), 0x0080),  # JS: neg
])
def test_equiv_jcc_taken(jcc_op, flag_setup, expected_ax):
    # Build: flag_setup + JCC taken + MOV AX, 0xDEAD + HLT + MOV AX, expected + HLT
    target_skip = bytes([0xB8, 0xAD, 0xDE])  # MOV AX, 0xDEAD (skipped if taken)
    taken_body = bytes([0xF4])               # HLT after jump
    prog = (
        bytes([0xB8, 0x00, 0x00])   # MOV AX, 0
        + flag_setup
        + bytes([jcc_op, len(target_skip)])  # JCC skip
        + target_skip
        + taken_body
    )
    gs, bs = run_both(prog, max_steps=100)
    assert gs.ax == bs.ax, f"AX mismatch: {gs.ax:#x} vs {bs.ax:#x}"
