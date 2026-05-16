"""test_equivalence.py — Cross-validate gate-level vs behavioral simulator.

Each test encodes a small PPC program, runs it on both simulators, and
compares the final register/memory state.

Programs encoded as big-endian 32-bit words using the encoding helpers.
"""

from __future__ import annotations

import struct

from powerpc601_simulator import PowerPC601Simulator as BehavioralSim

from powerpc601_gatelevel import PowerPC601GateLevelSimulator as GateSim

# ── Encoding helpers ──────────────────────────────────────────────────────────

def w(word: int) -> bytes:
    """Pack a 32-bit big-endian instruction word as bytes."""
    return struct.pack(">I", word)


HALT = b"\x00\x00\x00\x00"


def d_form(op: int, rd: int, ra: int, imm: int) -> bytes:
    return w((op << 26) | (rd << 21) | (ra << 16) | (imm & 0xFFFF))


def xo_form(op: int, rd: int, ra: int, rb: int, oe: int, xo: int, rc: int = 0) -> bytes:
    return w((op << 26) | (rd << 21) | (ra << 16) | (rb << 11) | (oe << 10) | (xo << 1) | rc)


def x_form(op: int, rs: int, ra: int, rb: int, xo: int, rc: int = 0) -> bytes:
    return w((op << 26) | (rs << 21) | (ra << 16) | (rb << 11) | (xo << 1) | rc)


def i_form(op: int, li: int, aa: int = 0, lk: int = 0) -> bytes:
    LI = (li >> 2) & 0xFFFFFF
    return w((op << 26) | (LI << 2) | (aa << 1) | lk)


def b_form(op: int, bo: int, bi: int, bd: int, aa: int = 0, lk: int = 0) -> bytes:
    BD = (bd >> 2) & 0x3FFF
    return w((op << 26) | (bo << 21) | (bi << 16) | (BD << 2) | (aa << 1) | lk)


def xfx_form(op: int, rs: int, spr: int, xo: int) -> bytes:
    spr_enc = ((spr & 0x1F) << 5) | ((spr >> 5) & 0x1F)
    return w((op << 26) | (rs << 21) | (spr_enc << 11) | (xo << 1))


def xl_form(op: int, bo: int, bi: int, bh: int, xo: int, lk: int = 0) -> bytes:
    return w((op << 26) | (bo << 21) | (bi << 16) | (bh << 11) | (xo << 1) | lk)


def run_both(prog: bytes, max_steps: int = 500):
    """Run prog on both simulators. Return (gate_state, behavioral_state)."""
    bsim = BehavioralSim()
    b_result = bsim.execute(prog, max_steps=max_steps)

    gsim = GateSim()
    g_result = gsim.execute(prog, max_steps=max_steps)

    return g_result.final_state, b_result.final_state


def assert_gprs_equal(gs, bs, regs=None):
    """Assert that the specified GPRs match between gate and behavioral."""
    if regs is None:
        regs = range(32)
    for r in regs:
        assert gs.gpr[r] == bs.gpr[r], f"r{r}: gate=0x{gs.gpr[r]:08X} vs behavioral=0x{bs.gpr[r]:08X}"


# ── Test 1: Arithmetic ADDI / ADD / SUBF ──────────────────────────────────────

def test_arithmetic_addi_add_subf():
    """Cross-validate: ADDI, ADD, SUBF."""
    # li r3, 10 (addi r3, 0, 10)
    # li r4, 20 (addi r4, 0, 20)
    # add r5, r3, r4   → r5 = 30
    # subf r6, r3, r4  → r6 = r4 - r3 = 10
    prog = (
        d_form(14, 3, 0, 10)          # addi r3, 0, 10
        + d_form(14, 4, 0, 20)        # addi r4, 0, 20
        + xo_form(31, 5, 3, 4, 0, 266)  # add r5, r3, r4
        + xo_form(31, 6, 3, 4, 0, 40)   # subf r6, r3, r4
        + HALT
    )
    gs, bs = run_both(prog)
    assert_gprs_equal(gs, bs, [3, 4, 5, 6])
    assert gs.gpr[5] == 30
    assert gs.gpr[6] == 10


# ── Test 2: Logic — AND / OR / XOR ───────────────────────────────────────────

def test_logic_and_or_xor():
    """Cross-validate: AND, OR, XOR on known bit patterns.

    Note: ORC (xo=412) is not implemented in the behavioral simulator, so
    cross-validation only covers the three instructions below.
    """
    # r3 = 0xAAAAAAAA, r4 = 0x55555555
    prog = (
        d_form(15, 3, 0, 0xAAAA)            # addis r3, 0, 0xAAAA (upper)
        + d_form(24, 3, 3, 0xAAAA)           # ori r3, r3, 0xAAAA (lower)
        + d_form(15, 4, 0, 0x5555)           # addis r4, 0, 0x5555
        + d_form(24, 4, 4, 0x5555)           # ori r4, r4, 0x5555
        + x_form(31, 3, 5, 4, 28)            # and r5, r3, r4  → 0
        + x_form(31, 3, 6, 4, 444)           # or r6, r3, r4   → 0xFFFFFFFF
        + x_form(31, 3, 7, 4, 316)           # xor r7, r3, r4  → 0xFFFFFFFF
        + HALT
    )
    gs, bs = run_both(prog)
    assert_gprs_equal(gs, bs, [3, 4, 5, 6, 7])
    assert gs.gpr[5] == 0  # AND of alternating patterns
    assert gs.gpr[6] == 0xFFFFFFFF
    assert gs.gpr[7] == 0xFFFFFFFF


# ── Test 3: Shifts — SLW / SRW / SRAW ────────────────────────────────────────

def test_shifts_slw_srw_sraw():
    """Cross-validate: SLW, SRW, SRAW by various amounts."""
    # r3 = 0x80000001 (has both MSB and LSB set)
    # r8 = 1 (shift amount)
    prog = (
        d_form(15, 3, 0, 0x8000)     # addis r3, 0, 0x8000 → 0x80000000
        + d_form(24, 3, 3, 1)         # ori r3, r3, 1 → 0x80000001
        + d_form(14, 8, 0, 1)         # addi r8, 0, 1 (shift amount)
        + x_form(31, 3, 4, 8, 24)     # slw r4, r3, r8  → 0x00000002 (MSB lost)
        + x_form(31, 3, 5, 8, 536)    # srw r5, r3, r8  → 0x40000000
        + x_form(31, 3, 6, 8, 792)    # sraw r6, r3, r8 → arithmetic: 0xC0000000
        + HALT
    )
    gs, bs = run_both(prog)
    assert_gprs_equal(gs, bs, [3, 4, 5, 6])


# ── Test 4: Rotate — RLWINM (gate-level only) ─────────────────────────────────

def test_rotate_rlwinm():
    """Gate-level RLWINM: rotate left and AND with mask.

    RLWINM is not implemented in the behavioral reference simulator, so
    this test validates the gate-level result directly against the known-
    correct answer rather than cross-comparing both simulators.

    r3 = 0x12345678; rlwinm r4, r3, 8, 0, 23
      rotl(0x12345678, 8) = 0x34567812
      mask(MB=0, ME=23)   = 0xFFFFFF00   (bits 31..8 set in PPC notation)
      result              = 0x34567800
    """
    from powerpc601_gatelevel import PowerPC601GateLevelSimulator

    prog = (
        d_form(15, 3, 0, 0x1234)     # addis r3, 0, 0x1234
        + d_form(24, 3, 3, 0x5678)   # ori r3, r3, 0x5678
        # rlwinm: op=21, rs=3, ra=4, sh=8, mb=0, me=23
        + w((21 << 26) | (3 << 21) | (4 << 16) | (8 << 11) | (0 << 6) | (23 << 1))
        + HALT
    )
    gsim = PowerPC601GateLevelSimulator()
    g_result = gsim.execute(prog, max_steps=500)
    gs = g_result.final_state
    assert gs.gpr[3] == 0x12345678
    assert gs.gpr[4] == 0x34567800


# ── Test 5: MULLW / DIVWU ─────────────────────────────────────────────────────

def test_multiply_and_divide():
    """Cross-validate: MULLW, DIVWU."""
    prog = (
        d_form(14, 3, 0, 6)          # addi r3, 0, 6
        + d_form(14, 4, 0, 7)        # addi r4, 0, 7
        + xo_form(31, 5, 3, 4, 0, 235)  # mullw r5, r3, r4 → 42
        + d_form(14, 6, 0, 100)      # addi r6, 0, 100
        + d_form(14, 7, 0, 7)        # addi r7, 0, 7
        + xo_form(31, 8, 6, 7, 0, 459)  # divwu r8, r6, r7 → 14
        + HALT
    )
    gs, bs = run_both(prog)
    assert_gprs_equal(gs, bs, [3, 4, 5, 6, 7, 8])
    assert gs.gpr[5] == 42
    assert gs.gpr[8] == 14


# ── Test 6: CMPI + branch ─────────────────────────────────────────────────────

def test_compare_and_branch():
    """Cross-validate: CMPI + conditional branch (skip if not equal)."""
    # r3 = 5; cmpi cr0, r3, 5 → EQ; bc BO_FALSE/BI_EQ → skip next; addi r4, 0, 99
    # If branch NOT taken (EQ), r4 should stay 0
    # If branch TAKEN (because not EQ), skip addi r4, 0, 99
    # Since r3=5 == 5, EQ=1, BO_FALSE(16) branches if CR[BI]=0, so NOT taken
    prog = (
        d_form(14, 3, 0, 5)          # addi r3, 0, 5
        + d_form(11, 0, 3, 5)        # cmpi cr0, r3, 5 → EQ=1
        # BO=16 (false/0): branch if CR[BI]=0; BI=2 (EQ bit); EQ=1 so NOT taken
        + b_form(16, 16, 2, 8)       # bc BO_FALSE, BI_EQ, +8 (skip 2 instructions)
        + d_form(14, 4, 0, 99)       # addi r4, 0, 99 (NOT skipped since branch not taken)
        + HALT
    )
    gs, bs = run_both(prog)
    assert_gprs_equal(gs, bs, [3, 4])


# ── Test 7: BL / BLR subroutine call ──────────────────────────────────────────

def test_bl_bclr_subroutine():
    """Cross-validate: BL (branch-and-link) then BLR (branch to LR)."""
    # Program layout:
    # 0x00: bl +8   (jump to 0x08, save return addr 0x04 in LR)
    # 0x04: addi r4, 0, 99    (return point: shouldn't execute before return)
    # 0x08: addi r3, 0, 42    (subroutine body)
    # 0x0C: blr               (return to LR = 0x04)
    # 0x10: HALT               (after subroutine return, execute this)
    #
    # But after blr returns to 0x04, we'd execute addi r4 then fall to 0x08 again...
    # Let's rearrange:
    # 0x00: li r3, 0          (r3 = 0)
    # 0x04: bl +0x10          (jump to 0x14, LR = 0x08)
    # 0x08: addi r5, 0, 77    (executed after return)
    # 0x0C: HALT
    # 0x10: addi r3, 0, 42    (subroutine)
    # 0x14: addi r4, 0, 11    (subroutine cont'd)
    # 0x18: blr               (return)
    prog = (
        d_form(14, 3, 0, 0)          # 0x00: addi r3, 0, 0
        + i_form(18, 0x10, lk=1)     # 0x04: bl +0x10 → jump to 0x14
        + d_form(14, 5, 0, 77)       # 0x08: addi r5, 0, 77 (after return)
        + HALT                         # 0x0C: halt
        + d_form(14, 3, 0, 42)       # 0x10: addi r3, 0, 42 (subroutine)
        + d_form(14, 4, 0, 11)       # 0x14: addi r4, 0, 11
        + xl_form(19, 20, 0, 0, 16)  # 0x18: blr (bclr BO_ALWAYS, 0)
    )
    gs, bs = run_both(prog)
    assert gs.gpr[3] == bs.gpr[3]  # 42
    assert gs.gpr[4] == bs.gpr[4]  # 11
    assert gs.gpr[5] == bs.gpr[5]  # 77


# ── Test 8: Memory load/store ──────────────────────────────────────────────────

def test_memory_load_store():
    """Cross-validate: STW, LWZ round-trip."""
    # Store 0xDEAD to mem[0x1000], load it back
    prog = (
        d_form(15, 3, 0, 0xDEAD)    # addis r3, 0, 0xDEAD → 0xDEAD0000 (wrong)
        + d_form(14, 3, 0, 0xBEEF)  # nope, build with ORI
    )
    # Build 0x1000 in r5:
    prog = (
        d_form(14, 3, 0, 0x1234)    # addi r3, 0, 0x1234 (value to store, small)
        + d_form(15, 5, 0, 0x1)     # addis r5, 0, 1 → 0x00010000 (address)
        + d_form(36, 3, 5, 0)       # stw r3, 0(r5)
        + d_form(14, 3, 0, 0)       # clear r3
        + d_form(32, 6, 5, 0)       # lwz r6, 0(r5)
        + HALT
    )
    gs, bs = run_both(prog)
    assert gs.gpr[6] == bs.gpr[6]
    assert gs.gpr[6] == 0x1234


# ── Test 9: CNTLZW ────────────────────────────────────────────────────────────

def test_cntlzw():
    """Cross-validate: CNTLZW on known values."""
    # r3 = 0 → 32 leading zeros
    # r4 = 1 → 31 leading zeros
    # r5 = 0x80000000 → 0 leading zeros
    prog = (
        d_form(14, 3, 0, 0)                      # addi r3, 0, 0
        + d_form(14, 4, 0, 1)                    # addi r4, 0, 1
        + d_form(15, 5, 0, -0x8000 & 0xFFFF)     # addis r5, 0, 0x8000
        + x_form(31, 3, 6, 0, 26)               # cntlzw r6, r3
        + x_form(31, 4, 7, 0, 26)               # cntlzw r7, r4
        + x_form(31, 5, 8, 0, 26)               # cntlzw r8, r5
        + HALT
    )
    gs, bs = run_both(prog)
    assert gs.gpr[6] == bs.gpr[6]  # 32
    assert gs.gpr[7] == bs.gpr[7]  # 31
    assert gs.gpr[8] == bs.gpr[8]  # 0


# ── Test 10: XER carry through ADDC/ADDE ─────────────────────────────────────

def test_xer_carry_addc_adde():
    """Cross-validate: ADDC sets XER[CA], ADDE uses it."""
    # r3 = 0xFFFFFFFF, r4 = 1 → addc r5 = 0, CA=1
    # adde r6, r3, r4 (with CA=1) → r3 + r4 + 1 = 0 + 1 = 1
    prog = (
        d_form(14, 3, 0, -1)            # addi r3, 0, -1 → 0xFFFFFFFF
        + d_form(14, 4, 0, 1)           # addi r4, 0, 1
        + xo_form(31, 5, 3, 4, 0, 10)   # addc r5, r3, r4 → 0, CA=1
        + xo_form(31, 6, 3, 4, 0, 138)  # adde r6, r3, r4 → +CA=1 = 1
        + HALT
    )
    gs, bs = run_both(prog)
    assert_gprs_equal(gs, bs, [3, 4, 5, 6])
    assert gs.xer == bs.xer
