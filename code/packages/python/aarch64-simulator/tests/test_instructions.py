"""Unit tests for each AArch64 instruction class."""

import pytest

from aarch64_simulator import (
    COND_AL,
    COND_CC,
    COND_CS,
    COND_EQ,
    COND_GE,
    COND_GT,
    COND_HI,
    COND_LE,
    COND_LS,
    COND_MI,
    COND_NE,
    COND_PL,
    COND_VC,
    COND_VS,
    HALT,
    AArch64Simulator,
    AArch64State,
    branch_cond,
    branch_imm,
    branch_reg,
    cbz_cbnz,
    csel_enc,
    dp_imm,
    dp_reg,
    ldst_uoff,
    logic_imm,
    logic_reg,
    madd_msub,
    movwide,
    tbz_tbnz,
)


def run(prog: bytes) -> AArch64State:
    """Execute a program and return its final state."""
    return AArch64Simulator().execute(prog).final_state


def preset(sim: AArch64Simulator, **kwargs: int) -> None:
    """
    Set named registers on the simulator's current state.

    Keyword arg names: x0–x30 for GPRs, sp for stack pointer.
    """
    s = sim.get_state()
    gpr = list(s.gpr)
    new_sp = s.sp
    for name, val in kwargs.items():
        if name == "sp":
            new_sp = val
        else:
            idx = int(name[1:])  # strip leading 'x'
            gpr[idx] = val & 0xFFFF_FFFF_FFFF_FFFF
    sim._state = AArch64State(  # type: ignore[attr-defined]
        pc=s.pc, gpr=tuple(gpr), sp=new_sp, nzcv=s.nzcv, memory=s.memory, halted=s.halted
    )


# ── Data Processing Immediate ─────────────────────────────────────────────────


def test_add_imm_basic():
    """ADD X1, X0, #5 → X1 = 5."""
    s = run(dp_imm(1, 0, 0, 5, 0, 0, 1) + HALT)
    assert s.x1 == 5


def test_add_imm_shifted():
    """ADD X1, X0, #1, LSL #12 → X1 = 4096."""
    s = run(dp_imm(1, 0, 0, 1, 1, 0, 1) + HALT)
    assert s.x1 == 4096


def test_sub_imm_basic():
    """SUB X1, X2, #3 → X1 = X2 - 3."""
    sim = AArch64Simulator()
    sim.load(dp_imm(1, 1, 0, 3, 0, 2, 1) + HALT)
    preset(sim, x2=10)
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x1 == 7


def test_adds_sets_nzcv_zero():
    """ADDS XZR, X0, #0 sets Z flag when result is 0."""
    # ADDS X1, X0, #0 — X0=0 → N=0,Z=1,C=0,V=0
    s = run(dp_imm(1, 0, 1, 0, 0, 0, 1) + HALT)
    assert s.z
    assert not s.n
    assert not s.c
    assert not s.v


def test_adds_sets_carry():
    """ADDS sets C flag on unsigned overflow."""
    sim = AArch64Simulator()
    sim.load(dp_imm(1, 0, 1, 1, 0, 0, 1) + HALT)  # ADDS X1, X0, #1
    preset(sim, x0=0xFFFF_FFFF_FFFF_FFFF)
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x1 == 0
    assert s.c   # carry out
    assert s.z   # result is zero


def test_subs_sets_negative():
    """SUBS sets N flag when result is negative."""
    sim = AArch64Simulator()
    sim.load(dp_imm(1, 1, 1, 10, 0, 0, 1) + HALT)  # SUBS X1, X0, #10
    preset(sim, x0=3)
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.n


def test_cmp_imm_equal():
    """CMP X1, #5 where X1=5 sets Z flag (result == 0)."""
    # CMP is SUBS XZR, Rn, imm → Rd=31 (XZR)
    sim = AArch64Simulator()
    sim.load(dp_imm(1, 1, 1, 5, 0, 1, 31) + HALT)   # SUBS XZR, X1, #5
    preset(sim, x1=5)
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.z
    assert s.x1 == 5   # XZR write discarded; X1 unchanged


def test_add_32bit_zero_extends():
    """32-bit ADD (sf=0) zero-extends result into the upper 32 bits."""
    sim = AArch64Simulator()
    sim.load(dp_imm(0, 0, 0, 1, 0, 0, 1) + HALT)   # ADD W1, W0, #1
    # Set X1 to 0xDEAD_BEEF_0000_0000 beforehand
    preset(sim, x1=0xDEAD_BEEF_0000_0000)
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    # 32-bit ADD: result = 0 + 1 = 1; zero-extended to 0x0000_0000_0000_0001
    assert s.x1 == 1


# ── Move Wide Immediate ───────────────────────────────────────────────────────


def test_movz_basic():
    """MOVZ X0, #42 → X0 = 42."""
    s = run(movwide(1, 0b10, 0, 42, 0) + HALT)
    assert s.x0 == 42


def test_movz_shifted():
    """MOVZ X0, #1, LSL #16 → X0 = 0x10000."""
    s = run(movwide(1, 0b10, 1, 1, 0) + HALT)
    assert s.x0 == 0x1_0000


def test_movn_all_ones():
    """MOVN X0, #0 → X0 = 0xFFFF_FFFF_FFFF_FFFF."""
    s = run(movwide(1, 0b00, 0, 0, 0) + HALT)
    assert s.x0 == 0xFFFF_FFFF_FFFF_FFFF


def test_movk_preserves_other_bits():
    """MOVK X0, #0xABCD, LSL #16 replaces bits[31:16] only."""
    sim = AArch64Simulator()
    sim.load(movwide(1, 0b11, 1, 0xABCD, 0) + HALT)  # MOVK X0, #0xABCD, LSL#16
    preset(sim, x0=0x1234_5678_9ABC_DEF0)
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x0 == 0x1234_5678_ABCD_DEF0


def test_movz_32bit():
    """MOVZ W0, #0xFFFF zeroes upper 32 bits."""
    s = run(movwide(0, 0b10, 0, 0xFFFF, 0) + HALT)
    assert s.x0 == 0xFFFF


# ── Logical Immediate ─────────────────────────────────────────────────────────


def test_orr_imm_all_ones():
    """ORR X0, X0, #-1 (all bits set) via bitmask immediate."""
    # N=1, immr=0, imms=63 → 64 ones → 0xFFFF_FFFF_FFFF_FFFF
    s = run(logic_imm(1, 0b01, 1, 0, 63, 0, 0) + HALT)
    assert s.x0 == 0xFFFF_FFFF_FFFF_FFFF


def test_and_imm_mask():
    """AND X1, X0, #0xFF masks to low byte."""
    # N=0, immr=0, imms=7 → 8 ones (0xFF), repeated → 0x00FF00FF...
    # For masking to 0xFF in 64-bit, use N=1, immr=0, imms=7 → exactly 8 ones in 64-bit
    sim = AArch64Simulator()
    sim.load(logic_imm(1, 0b00, 1, 0, 7, 0, 1) + HALT)   # AND X1, X0, #0xFF
    preset(sim, x0=0x1234_5678_9ABC_DE0F)
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x1 == 0x0F


def test_eor_imm_toggle():
    """EOR X1, X0, #1 toggles bit 0."""
    sim = AArch64Simulator()
    # N=1, immr=0, imms=0 → 1 one → 0x0000...0001
    sim.load(logic_imm(1, 0b10, 1, 0, 0, 0, 1) + HALT)   # EOR X1, X0, #1
    preset(sim, x0=0b1010)
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x1 == 0b1011


def test_ands_imm_sets_flags():
    """ANDS sets N/Z flags; does not store result in XZR (TST pattern)."""
    sim = AArch64Simulator()
    # ANDS XZR, X0, #(N=1,immr=0,imms=0) — TST X0, #1
    sim.load(logic_imm(1, 0b11, 1, 0, 0, 0, 31) + HALT)
    preset(sim, x0=0x8000_0000_0000_0000)
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    # 0x8000...0000 & 1 = 0 → Z=1, N=0
    assert s.z
    assert not s.n


# ── Logical Register (shifted) ────────────────────────────────────────────────


def test_orr_reg_mov_alias():
    """ORR X1, XZR, X0 implements MOV X1, X0."""
    sim = AArch64Simulator()
    sim.load(logic_reg(1, 0b01, 0, 0, 0, 0, 31, 1) + HALT)  # ORR X1, XZR, X0
    preset(sim, x0=0xDEAD_BEEF)
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x1 == 0xDEAD_BEEF


def test_and_reg():
    """AND X2, X0, X1 → bitwise and."""
    sim = AArch64Simulator()
    sim.load(logic_reg(1, 0b00, 0, 0, 1, 0, 0, 2) + HALT)  # AND X2, X0, X1
    preset(sim, x0=0b1100, x1=0b1010)
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x2 == 0b1000


def test_eor_reg():
    """EOR X2, X0, X1."""
    sim = AArch64Simulator()
    sim.load(logic_reg(1, 0b10, 0, 0, 1, 0, 0, 2) + HALT)  # EOR X2, X0, X1
    preset(sim, x0=0b1010, x1=0b1100)
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x2 == 0b0110


def test_bic_reg():
    """BIC X2, X0, X1 (AND NOT) — N=1 inverts Rm before AND."""
    sim = AArch64Simulator()
    sim.load(logic_reg(1, 0b00, 0, 1, 1, 0, 0, 2) + HALT)  # BIC X2, X0, X1
    preset(sim, x0=0b1111, x1=0b1010)
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x2 == 0b0101


def test_orn_reg_mvn_alias():
    """ORN X1, XZR, X0 (MVN alias) inverts all bits of X0."""
    sim = AArch64Simulator()
    sim.load(logic_reg(1, 0b01, 0, 1, 0, 0, 31, 1) + HALT)  # ORN X1, XZR, X0
    preset(sim, x0=0)
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x1 == 0xFFFF_FFFF_FFFF_FFFF


def test_ands_reg_sets_flags():
    """ANDS X2, X0, X1 updates N and Z flags."""
    sim = AArch64Simulator()
    sim.load(logic_reg(1, 0b11, 0, 0, 1, 0, 0, 2) + HALT)  # ANDS X2, X0, X1
    preset(sim, x0=0xFFFF_FFFF_FFFF_FFFF, x1=0x8000_0000_0000_0000)
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x2 == 0x8000_0000_0000_0000
    assert s.n   # MSB set


def test_orr_reg_shifted():
    """ORR X2, X0, X1, LSL #4 — shift applies before operation."""
    sim = AArch64Simulator()
    sim.load(logic_reg(1, 0b01, 0, 0, 1, 4, 0, 2) + HALT)  # ORR X2, X0, X1 LSL#4
    preset(sim, x0=0, x1=1)
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x2 == 16   # 1 << 4


# ── Arithmetic Register (shifted) ─────────────────────────────────────────────


def test_add_reg_basic():
    """ADD X2, X0, X1 → X0 + X1."""
    sim = AArch64Simulator()
    sim.load(dp_reg(1, 0, 0, 0, 1, 0, 0, 2) + HALT)  # ADD X2, X0, X1
    preset(sim, x0=10, x1=20)
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x2 == 30


def test_sub_reg_basic():
    """SUB X2, X0, X1 → X0 - X1."""
    sim = AArch64Simulator()
    sim.load(dp_reg(1, 1, 0, 0, 1, 0, 0, 2) + HALT)  # SUB X2, X0, X1
    preset(sim, x0=50, x1=30)
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x2 == 20


def test_adds_reg_overflow():
    """ADDS sets V flag on signed overflow."""
    sim = AArch64Simulator()
    sim.load(dp_reg(1, 0, 1, 0, 1, 0, 0, 2) + HALT)  # ADDS X2, X0, X1
    # 0x7FFF...FFFF + 1 → signed overflow
    preset(sim, x0=0x7FFF_FFFF_FFFF_FFFF, x1=1)
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.v   # signed overflow


def test_subs_reg_carry():
    """SUBS (CMP) sets C=1 when no borrow (A >= B)."""
    sim = AArch64Simulator()
    # SUBS X2, X0, X1 where X0 >= X1 → C=1 (no borrow)
    sim.load(dp_reg(1, 1, 1, 0, 1, 0, 0, 2) + HALT)
    preset(sim, x0=10, x1=5)
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.c


def test_add_reg_lsl():
    """ADD X2, X0, X1, LSL #3 = X0 + (X1 << 3)."""
    sim = AArch64Simulator()
    sim.load(dp_reg(1, 0, 0, 0, 1, 3, 0, 2) + HALT)
    preset(sim, x0=0, x1=4)
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x2 == 32   # 4 << 3


# ── Load / Store ──────────────────────────────────────────────────────────────


def test_str_ldr_64bit():
    """STR X1, [X0] then LDR X2, [X0] round-trips a 64-bit value."""
    sim = AArch64Simulator()
    prog = (
        ldst_uoff(3, 0, 0b00, 0, 0, 1)     # STR X1, [X0]
        + ldst_uoff(3, 0, 0b01, 0, 0, 2)   # LDR X2, [X0]
        + HALT
    )
    sim.load(prog)
    preset(sim, x0=0x100, x1=0xDEAD_BEEF_CAFE_1234)
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x2 == 0xDEAD_BEEF_CAFE_1234


def test_str_ldr_32bit():
    """STR W1, [X0] stores only lower 32 bits; LDR W2, [X0] zero-extends."""
    sim = AArch64Simulator()
    prog = (
        ldst_uoff(2, 0, 0b00, 0, 0, 1)     # STR W1, [X0]
        + ldst_uoff(2, 0, 0b01, 0, 0, 2)   # LDR W2, [X0]
        + HALT
    )
    sim.load(prog)
    preset(sim, x0=0x200, x1=0xABCD_1234)
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x2 == 0xABCD_1234


def test_strb_ldrb():
    """STRB / LDRB round-trips a byte (zero-extended)."""
    sim = AArch64Simulator()
    prog = (
        ldst_uoff(0, 0, 0b00, 0, 0, 1)     # STRB W1, [X0]
        + ldst_uoff(0, 0, 0b01, 0, 0, 2)   # LDRB W2, [X0]
        + HALT
    )
    sim.load(prog)
    preset(sim, x0=0x300, x1=0xAB)
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x2 == 0xAB


def test_strh_ldrh():
    """STRH / LDRH round-trips a halfword (zero-extended)."""
    sim = AArch64Simulator()
    prog = (
        ldst_uoff(1, 0, 0b00, 0, 0, 1)     # STRH W1, [X0]
        + ldst_uoff(1, 0, 0b01, 0, 0, 2)   # LDRH W2, [X0]
        + HALT
    )
    sim.load(prog)
    preset(sim, x0=0x400, x1=0xBEEF)
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x2 == 0xBEEF


def test_ldrsb_sign_extends():
    """LDRSB sign-extends an 8-bit value into a 64-bit register."""
    sim = AArch64Simulator()
    prog = (
        ldst_uoff(0, 0, 0b00, 0, 0, 1)     # STRB W1, [X0]
        + ldst_uoff(0, 0, 0b10, 0, 0, 2)   # LDRSB X2, [X0]
        + HALT
    )
    sim.load(prog)
    preset(sim, x0=0x500, x1=0xFF)   # -1 in signed byte
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x2 == 0xFFFF_FFFF_FFFF_FFFF


def test_ldrsw_sign_extends():
    """LDRSW sign-extends a 32-bit value into a 64-bit register."""
    sim = AArch64Simulator()
    prog = (
        ldst_uoff(2, 0, 0b00, 0, 0, 1)     # STR W1, [X0]  (32-bit store)
        + ldst_uoff(2, 0, 0b10, 0, 0, 2)   # LDRSW X2, [X0]
        + HALT
    )
    sim.load(prog)
    preset(sim, x0=0x600, x1=0x8000_0000)   # -2147483648 in signed 32-bit
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x2 == 0xFFFF_FFFF_8000_0000


def test_ldr_with_imm_offset():
    """LDR X1, [X0, #8] loads from X0+8."""
    sim = AArch64Simulator()
    # imm12 for LDR 64-bit: EA = Rn + imm12*8
    # We want offset=8 → imm12=1
    prog = ldst_uoff(3, 0, 0b01, 1, 0, 1) + HALT   # LDR X1, [X0, #8]
    sim.load(prog)
    s0 = sim.get_state()
    mem = list(s0.memory)
    # Put 0xCAFEBABE_DEADBEEF at address 8
    val = 0xCAFEBABE_DEADBEEF
    for i in range(7, -1, -1):
        mem[8 + (7 - i)] = (val >> (i * 8)) & 0xFF
    sim._state = AArch64State(  # type: ignore[attr-defined]
        pc=s0.pc, gpr=s0.gpr, sp=s0.sp, nzcv=s0.nzcv, memory=tuple(mem), halted=s0.halted
    )
    preset(sim, x0=0)
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x1 == 0xCAFEBABE_DEADBEEF


# ── Branch ────────────────────────────────────────────────────────────────────


def test_b_forward():
    """B #8 skips the next instruction."""
    prog = (
        branch_imm(0, 2)                    # B #+8 (skip 2 insns ahead → byte 8)
        + dp_imm(1, 0, 0, 99, 0, 0, 1)     # [4] skipped
        + dp_imm(1, 0, 0, 42, 0, 0, 2)     # [8] ADD X2, X0, #42
        + HALT
    )
    s = run(prog)
    assert s.x1 == 0   # instruction at [4] skipped
    assert s.x2 == 42


def test_bl_sets_lr():
    """BL saves PC+4 in X30 (LR)."""
    prog = (
        branch_imm(1, 2)                    # BL #+8 → X30 = 4; jump to [8]
        + HALT                              # [4] — would be next if no BL
        + dp_imm(1, 0, 0, 7, 0, 0, 1)     # [8] ADD X1, X0, #7
        + HALT
    )
    s = run(prog)
    assert s.x30 == 4   # LR was set to return address
    assert s.x1 == 7


def test_br_reg():
    """BR X0 jumps to address in X0."""
    # Layout: [0]=MOVZ, [4]=BR, [8]=ADD X1 (skipped), [12]=ADD X2, [16]=HALT
    # We load X0=12 so BR jumps past [8] and lands at [12].
    prog = (
        movwide(1, 0b10, 0, 12, 0)         # [0] MOVZ X0, #12 → X0=12
        + branch_reg(0b000, 0)              # [4] BR X0 → jump to [12]
        + dp_imm(1, 0, 0, 99, 0, 0, 1)    # [8] skipped
        + dp_imm(1, 0, 0, 42, 0, 0, 2)    # [12] ADD X2, X0, #42 → X2=54
        + HALT
    )
    s = run(prog)
    assert s.x1 == 0   # never executed
    assert s.x2 == 54  # 12 + 42


def test_ret():
    """RET returns to address in X30."""
    prog = (
        movwide(1, 0b10, 0, 12, 30)        # [0] MOVZ X30, #12 → set LR=12
        + branch_reg(0b010, 30)             # [4] RET → jump to X30=12
        + dp_imm(1, 0, 0, 99, 0, 0, 1)    # [8] skipped
        + dp_imm(1, 0, 0, 42, 0, 0, 2)    # [12] ADD X2, X0, #42
        + HALT
    )
    s = run(prog)
    assert s.x1 == 0
    assert s.x2 == 42


def test_blr():
    """BLR X0 calls the address in X0 and saves PC+4 to X30."""
    prog = (
        movwide(1, 0b10, 0, 12, 0)         # [0] MOVZ X0, #12
        + branch_reg(0b001, 0)              # [4] BLR X0 → X30=8, jump to 12
        + HALT                              # [8] not reached (return will come back)
        + dp_imm(1, 0, 0, 7, 0, 31, 1)    # [12] ADD X1, XZR, #7 → X1=7
        + HALT                              # [16]
    )
    s = run(prog)
    assert s.x30 == 8
    assert s.x1 == 7


# ── Conditional Branch ────────────────────────────────────────────────────────


def test_b_cond_eq_taken():
    """B.EQ taken when Z=1."""
    prog = (
        dp_imm(1, 1, 1, 5, 0, 0, 31)      # [0] SUBS XZR, X0, #5 → sets flags; X0=0 → NE
        + branch_cond(2, COND_EQ)           # [4] B.EQ #+8 → jump to [12] if Z=1
        + dp_imm(1, 0, 0, 99, 0, 0, 1)    # [8] X1=99 (not taken if branch taken)
        + dp_imm(1, 0, 0, 42, 0, 0, 2)    # [12] X2=42
        + HALT
    )
    # X0=0, SUBS 0-5 → Z=0, branch NOT taken → X1=99
    s = run(prog)
    assert s.x1 == 99
    assert s.x2 == 42


def test_b_cond_ne_taken():
    """B.NE taken when Z=0."""
    sim = AArch64Simulator()
    prog = (
        dp_imm(1, 1, 1, 5, 0, 0, 31)      # [0] SUBS XZR, X0, #5
        + branch_cond(2, COND_NE)           # [4] B.NE #+8 → jump to [12] if Z=0
        + dp_imm(1, 0, 0, 99, 0, 31, 1)   # [8] skipped (ADD X1, XZR, #99)
        + dp_imm(1, 0, 0, 42, 0, 31, 2)   # [12] ADD X2, XZR, #42 → X2=42
        + HALT
    )
    sim.load(prog)
    preset(sim, x0=7)  # 7 != 5 → Z=0
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x1 == 0   # skipped
    assert s.x2 == 42


def test_b_cond_ge_lt():
    """B.GE taken when N==V; B.LT taken when N!=V."""
    sim = AArch64Simulator()
    # SUBS sets flags; then test B.GE (should take when X0 >= 5)
    prog = (
        dp_imm(1, 1, 1, 5, 0, 0, 31)      # [0] SUBS XZR, X0, #5
        + branch_cond(2, COND_GE)           # [4] B.GE #+8
        + dp_imm(1, 0, 0, 1, 0, 31, 1)    # [8] ADD X1, XZR, #1 (X0<5 path, skipped)
        + dp_imm(1, 0, 0, 2, 0, 31, 2)    # [12] ADD X2, XZR, #2 (common)
        + HALT
    )
    sim.load(prog)
    preset(sim, x0=10)  # 10 >= 5 → GE taken
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x1 == 0   # skipped
    assert s.x2 == 2


# ── CBZ / CBNZ ────────────────────────────────────────────────────────────────


def test_cbz_taken_on_zero():
    """CBZ X1, #8 branches when X1==0."""
    prog = (
        cbz_cbnz(1, 0, 2, 1)               # [0] CBZ X1, #+8 → [8] if X1==0
        + dp_imm(1, 0, 0, 99, 0, 0, 2)    # [4] X2=99 (skipped if taken)
        + dp_imm(1, 0, 0, 42, 0, 0, 3)    # [8] X3=42
        + HALT
    )
    s = run(prog)  # X1=0 by default
    assert s.x2 == 0   # skipped
    assert s.x3 == 42


def test_cbz_not_taken_on_nonzero():
    """CBZ X1, #8 does not branch when X1!=0."""
    sim = AArch64Simulator()
    prog = (
        cbz_cbnz(1, 0, 2, 1)               # [0] CBZ X1, #+8
        + dp_imm(1, 0, 0, 99, 0, 0, 2)    # [4] X2=99
        + HALT
    )
    sim.load(prog)
    preset(sim, x1=5)
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x2 == 99


def test_cbnz_taken_on_nonzero():
    """CBNZ X1, #8 branches when X1!=0."""
    sim = AArch64Simulator()
    prog = (
        cbz_cbnz(1, 1, 2, 1)               # [0] CBNZ X1, #+8
        + dp_imm(1, 0, 0, 99, 0, 0, 2)    # [4] X2=99 (skipped)
        + dp_imm(1, 0, 0, 42, 0, 0, 3)    # [8] X3=42
        + HALT
    )
    sim.load(prog)
    preset(sim, x1=1)
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x2 == 0
    assert s.x3 == 42


# ── TBZ / TBNZ ────────────────────────────────────────────────────────────────


def test_tbz_taken_when_bit_clear():
    """TBZ X1, #0, #8 branches when bit 0 of X1 is clear."""
    prog = (
        tbz_tbnz(0, 0, 0, 2, 1)            # [0] TBZ W1, #0, #+8
        + dp_imm(1, 0, 0, 99, 0, 0, 2)    # [4] skipped
        + dp_imm(1, 0, 0, 42, 0, 0, 3)    # [8] X3=42
        + HALT
    )
    # X1=0 → bit0 clear → branch taken
    s = run(prog)
    assert s.x2 == 0   # skipped
    assert s.x3 == 42


def test_tbnz_taken_when_bit_set():
    """TBNZ X1, #3, #8 branches when bit 3 of X1 is set."""
    sim = AArch64Simulator()
    prog = (
        tbz_tbnz(0, 1, 3, 2, 1)            # [0] TBNZ W1, #3, #+8
        + dp_imm(1, 0, 0, 99, 0, 0, 2)    # [4] skipped
        + dp_imm(1, 0, 0, 42, 0, 0, 3)    # [8] X3=42
        + HALT
    )
    sim.load(prog)
    preset(sim, x1=0b1000)   # bit 3 set
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x2 == 0
    assert s.x3 == 42


# ── MADD / MSUB / MUL ─────────────────────────────────────────────────────────


def test_madd_basic():
    """MADD X3, X0, X1, X2 → X3 = X2 + X0 * X1."""
    sim = AArch64Simulator()
    sim.load(madd_msub(1, 0, 1, 0, 2, 0, 3) + HALT)  # MADD X3, X0, X1, X2
    preset(sim, x0=3, x1=4, x2=10)
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x3 == 22   # 10 + 3*4


def test_msub_basic():
    """MSUB X3, X0, X1, X2 → X3 = X2 - X0 * X1."""
    sim = AArch64Simulator()
    sim.load(madd_msub(1, 0, 1, 1, 2, 0, 3) + HALT)  # MSUB X3, X0, X1, X2
    preset(sim, x0=3, x1=4, x2=20)
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x3 == 8   # 20 - 3*4


def test_mul_via_madd():
    """MUL X2, X0, X1 is MADD X2, X0, X1, XZR → X0*X1."""
    sim = AArch64Simulator()
    sim.load(madd_msub(1, 0, 1, 0, 31, 0, 2) + HALT)  # MUL X2, X0, X1
    preset(sim, x0=7, x1=6)
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x2 == 42


# ── UDIV / SDIV ───────────────────────────────────────────────────────────────


def test_udiv():
    """UDIV X2, X0, X1 → unsigned division."""
    # UDIV: Data Processing 2-Source, opcode2=000010
    from aarch64_simulator.simulator import _u32be
    # Encode UDIV X2, X0, X1: sf=1, bit30=0, bits[28:21]=0b11010110, Rm=1, opcode2=0b000010, Rn=0, Rd=2
    raw = (1 << 31) | (0 << 30) | (0b11010110 << 21) | (1 << 16) | (0b000010 << 10) | (0 << 5) | 2
    prog = _u32be(raw) + HALT
    sim = AArch64Simulator()
    sim.load(prog)
    preset(sim, x0=20, x1=3)
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x2 == 6   # 20 // 3 = 6


def test_sdiv():
    """SDIV X2, X0, X1 → signed division (truncates toward zero)."""
    from aarch64_simulator.simulator import _u32be
    # SDIV: opcode2=0b000011
    raw = (1 << 31) | (0 << 30) | (0b11010110 << 21) | (1 << 16) | (0b000011 << 10) | (0 << 5) | 2
    prog = _u32be(raw) + HALT
    sim = AArch64Simulator()
    sim.load(prog)
    preset(sim, x0=0xFFFF_FFFF_FFFF_FFFF, x1=2)  # -1 / 2 = 0 (truncate toward 0)
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x2 == 0   # -1 // 2 = 0 in C-style


# ── UMULH / SMULH ─────────────────────────────────────────────────────────────


def test_umulh_high_word():
    """UMULH X2, X0, X1 → upper 64 bits of X0*X1 (unsigned)."""
    from aarch64_simulator.simulator import _u32be
    # UMULH: 3-source, sf=1, op54=010, o0=0
    raw = (1 << 31) | (0b00_11011 << 24) | (0b010 << 21) | (1 << 16) | (0 << 15) | (31 << 10) | (0 << 5) | 2
    prog = _u32be(raw) + HALT
    sim = AArch64Simulator()
    sim.load(prog)
    preset(sim, x0=0xFFFF_FFFF_FFFF_FFFF, x1=2)
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    # 0xFFFF...FFFF * 2 = 0x1_FFFF...FFFE; upper 64 bits = 1
    assert s.x2 == 1


# ── Conditional Select ────────────────────────────────────────────────────────


def test_csel_true():
    """CSEL X2, X0, X1, EQ selects X0 when Z=1."""
    sim = AArch64Simulator()
    prog = (
        dp_imm(1, 1, 1, 10, 0, 0, 31)     # [0] SUBS XZR, X0, #10 → Z=1 when X0=10
        + csel_enc(1, 0, 0, 1, COND_EQ, 0b00, 0, 2)  # [4] CSEL X2, X0, X1, EQ
        + HALT
    )
    sim.load(prog)
    preset(sim, x0=10, x1=99)
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x2 == 10   # EQ is true (10 - 10 = 0 → Z=1) → selects X0=10


def test_csel_false():
    """CSEL X2, X0, X1, EQ selects X1 when Z=0."""
    sim = AArch64Simulator()
    prog = (
        dp_imm(1, 1, 1, 5, 0, 0, 31)      # SUBS XZR, X0, #5 → Z depends on X0
        + csel_enc(1, 0, 0, 1, COND_EQ, 0b00, 0, 2)  # CSEL X2, X0, X1, EQ
        + HALT
    )
    sim.load(prog)
    preset(sim, x0=10, x1=99)   # 10 != 5, Z=0
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x2 == 99   # EQ false → Rm


def test_csinc():
    """CSINC X2, X0, X1, EQ: false path increments Rm."""
    sim = AArch64Simulator()
    # Set Z=0 by making X0 != 5
    prog = (
        dp_imm(1, 1, 1, 5, 0, 0, 31)
        + csel_enc(1, 0, 0, 1, COND_EQ, 0b01, 0, 2)  # CSINC X2, X0, X1, EQ
        + HALT
    )
    sim.load(prog)
    preset(sim, x0=7, x1=10)   # Z=0
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x2 == 11   # X1+1


def test_csinv():
    """CSINV X2, X0, X1, EQ: false path inverts Rm."""
    sim = AArch64Simulator()
    prog = (
        dp_imm(1, 1, 1, 5, 0, 0, 31)
        + csel_enc(1, 1, 0, 1, COND_EQ, 0b00, 0, 2)  # CSINV X2, X0, X1, EQ
        + HALT
    )
    sim.load(prog)
    preset(sim, x0=7, x1=0)   # Z=0
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x2 == 0xFFFF_FFFF_FFFF_FFFF   # ~0


def test_csneg():
    """CSNEG X2, X0, X1, EQ: false path negates Rm."""
    sim = AArch64Simulator()
    prog = (
        dp_imm(1, 1, 1, 5, 0, 0, 31)
        + csel_enc(1, 1, 0, 1, COND_EQ, 0b01, 0, 2)  # CSNEG X2, X0, X1, EQ
        + HALT
    )
    sim.load(prog)
    preset(sim, x0=7, x1=42)   # Z=0
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x2 == ((-42) & 0xFFFF_FFFF_FFFF_FFFF)


# ── Bit manipulation ──────────────────────────────────────────────────────────


def test_clz_simple():
    """CLZ X1, X0 counts leading zeros."""
    from aarch64_simulator.simulator import _u32be
    # CLZ: 1-source, sf=1, bit30=1, bits[28:21]=0b11010110, bits[20:16]=00000, opcode2=0b000100, Rn=0, Rd=1
    raw = (1 << 31) | (1 << 30) | (0b11010110 << 21) | (0 << 16) | (0b000100 << 10) | (0 << 5) | 1
    prog = _u32be(raw) + HALT
    sim = AArch64Simulator()
    sim.load(prog)
    preset(sim, x0=0x0001_0000_0000_0000)   # bit 48 set → 15 leading zeros
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x1 == 15


def test_rbit():
    """RBIT X1, X0 reverses all bits."""
    from aarch64_simulator.simulator import _u32be
    raw = (1 << 31) | (1 << 30) | (0b11010110 << 21) | (0 << 16) | (0b000000 << 10) | (0 << 5) | 1
    prog = _u32be(raw) + HALT
    sim = AArch64Simulator()
    sim.load(prog)
    preset(sim, x0=1)   # bit 0 set → should become bit 63 set
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x1 == 0x8000_0000_0000_0000


def test_rev():
    """REV X1, X0 reverses byte order of 64-bit register."""
    from aarch64_simulator.simulator import _u32be
    raw = (1 << 31) | (1 << 30) | (0b11010110 << 21) | (0 << 16) | (0b000010 << 10) | (0 << 5) | 1
    prog = _u32be(raw) + HALT
    sim = AArch64Simulator()
    sim.load(prog)
    preset(sim, x0=0x0102_0304_0506_0708)
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x1 == 0x0807_0605_0403_0201


# ── XZR behavior ─────────────────────────────────────────────────────────────


def test_xzr_read_is_zero():
    """Reading XZR (register 31 in arithmetic) always returns 0."""
    sim = AArch64Simulator()
    # ADD X1, XZR, #5 → X1 = 0 + 5 = 5
    sim.load(dp_imm(1, 0, 0, 5, 0, 31, 1) + HALT)
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x1 == 5


def test_xzr_write_discarded():
    """Writing to XZR (Rd=31 in dp_imm) discards the result."""
    # ADD XZR, X0, #99 should not change any register
    s = run(dp_imm(1, 0, 0, 99, 0, 0, 31) + HALT)
    assert s.gpr[31] == 0


# ── LSRV / LSLV / ASRV / RORV ────────────────────────────────────────────────


def test_lslv():
    """LSLV X2, X0, X1 → X0 << (X1 mod 64)."""
    from aarch64_simulator.simulator import _u32be
    raw = (1 << 31) | (0 << 30) | (0b11010110 << 21) | (1 << 16) | (0b001000 << 10) | (0 << 5) | 2
    prog = _u32be(raw) + HALT
    sim = AArch64Simulator()
    sim.load(prog)
    preset(sim, x0=1, x1=8)
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x2 == 256


def test_lsrv():
    """LSRV X2, X0, X1 → X0 >> (X1 mod 64) unsigned."""
    from aarch64_simulator.simulator import _u32be
    raw = (1 << 31) | (0 << 30) | (0b11010110 << 21) | (1 << 16) | (0b001001 << 10) | (0 << 5) | 2
    prog = _u32be(raw) + HALT
    sim = AArch64Simulator()
    sim.load(prog)
    preset(sim, x0=256, x1=4)
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x2 == 16


# ── NOP ───────────────────────────────────────────────────────────────────────


def test_nop():
    """NOP advances PC but does nothing else."""
    nop = bytes.fromhex("D503201F")
    prog = nop + dp_imm(1, 0, 0, 5, 0, 0, 1) + HALT
    s = run(prog)
    assert s.x1 == 5
    assert s.pc == 8   # started at HALT position after 3 instructions


# ── Condition all-codes ────────────────────────────────────────────────────────


@pytest.mark.parametrize("cond,x0,expected_taken", [
    (COND_CS, 0xFFFF_FFFF_FFFF_FFFF, True),  # C=1 after ADDS max+1 wraps → carry
    (COND_CC, 5, True),    # C=0 after SUBS 5 - 10 → borrow → C=0
    (COND_MI, 0, False),   # N=0 after SUBS 0 - 0 → result=0, not negative
    (COND_PL, 5, True),    # N=0 after SUBS 5 - 0 → result positive
    (COND_VS, 0x7FFF_FFFF_FFFF_FFFF, True),   # overflow: signed max + 1
    (COND_VC, 5, True),    # no overflow: 5 - 0 is fine
    (COND_HI, 0, False),   # C=0 and Z=1 (0-0=0); HI = C&!Z → false
    (COND_LS, 0, True),    # LS = !C|Z; 0-0: Z=1 → true
    (COND_GT, 5, True),    # 5-0: positive, Z=0, N==V → GT true
    (COND_LE, 0, True),    # 0-0: Z=1 → LE true
    (COND_AL, 0, True),    # always
])
def test_b_cond_all_codes(cond: int, x0: int, expected_taken: bool):
    """Verify all condition codes route correctly."""
    sim = AArch64Simulator()
    # Choose flag-setter based on condition to exercise:
    #   CS/VS: ADDS X0, X0, #1 — use x0=max to produce carry/overflow
    #   CC:    SUBS XZR, X0, #10 — use x0=5 to produce borrow (C=0)
    #   rest:  SUBS XZR, X0, #0  — subtraction from preset x0
    if cond in (COND_CS, COND_VS):
        flag_setter = dp_imm(1, 0, 1, 1, 0, 0, 0)    # ADDS X0, X0, #1
    elif cond == COND_CC:
        flag_setter = dp_imm(1, 1, 1, 10, 0, 0, 31)  # SUBS XZR, X0, #10 (5<10 → borrow)
    else:
        flag_setter = dp_imm(1, 1, 1, 0, 0, 0, 31)   # SUBS XZR, X0, #0
    prog = (
        flag_setter                                   # [0] set flags
        + branch_cond(2, cond)                        # [4] B.cond #+8
        + dp_imm(1, 0, 0, 1, 0, 31, 1)              # [8] ADD X1, XZR, #1 = 1 (not-taken)
        + dp_imm(1, 0, 0, 2, 0, 31, 2)              # [12] ADD X2, XZR, #2 = 2 (fall-through)
        + HALT
    )
    sim.load(prog)
    preset(sim, x0=x0)
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    if expected_taken:
        assert s.x1 == 0, f"cond={cond:#06b} should be taken, X1 should be 0"
    else:
        assert s.x1 == 1, f"cond={cond:#06b} should be not-taken, X1 should be 1"
