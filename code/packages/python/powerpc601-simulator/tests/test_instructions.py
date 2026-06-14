"""Per-instruction unit tests for the PowerPC 601 simulator."""

from powerpc601_simulator import (
    BI_EQ,
    BI_LT,
    BO_ALWAYS,
    BO_BDNZ,
    BO_BDZ,
    BO_FALSE,
    BO_TRUE,
    HALT,
    SPR_CTR,
    SPR_LR,
    PowerPC601Simulator,
    PowerPC601State,
    b_form,
    d_form,
    i_form,
    x_form,
    xfx_form,
    xl_form,
    xo_form,
)
from powerpc601_simulator.simulator import (
    PO_ADDI,
    PO_ADDIS,
    PO_ANDI_DOT,
    PO_ANDIS_DOT,
    PO_B,
    PO_BC,
    PO_BX,
    PO_CMPI,
    PO_CMPLI,
    PO_LBZ,
    PO_LBZU,
    PO_LHA,
    PO_LHZ,
    PO_LHZU,
    PO_LWZ,
    PO_LWZU,
    PO_ORI,
    PO_ORIS,
    PO_STB,
    PO_STBU,
    PO_STH,
    PO_STW,
    PO_STWU,
    PO_SUBFIC,
    PO_X31,
    PO_XORI,
    XO_ADD,
    XO_ADDC,
    XO_ADDE,
    XO_AND,
    XO_BCCTR,
    XO_BCLR,
    XO_CMP,
    XO_CMPL,
    XO_CNTLZW,
    XO_DIVW,
    XO_DIVWU,
    XO_MFCR,
    XO_MFSPR,
    XO_MTCRF,
    XO_MTSPR,
    XO_MULLW,
    XO_NAND,
    XO_NEG,
    XO_NOR,
    XO_OR,
    XO_SLW,
    XO_SRAW,
    XO_SRAWI,
    XO_SRW,
    XO_SUBF,
    XO_XOR,
)

# ── Helpers ────────────────────────────────────────────────────────────────────


def run(prog: bytes) -> PowerPC601State:
    """Execute a program and return the final state.  Asserts execution is ok."""
    sim = PowerPC601Simulator()
    result = sim.execute(prog)
    assert result.ok, f"execution failed: {result.error}"
    return result.final_state


def preset(sim: PowerPC601Simulator, *, gpr=None, lr=None, ctr=None, xer=None, cr=None):
    """Manually override registers in the simulator state (after load, before step)."""
    s = sim.get_state()
    new_gpr = list(s.gpr)
    if gpr:
        for idx, val in gpr.items():
            new_gpr[idx] = val & 0xFFFF_FFFF
    sim._state = PowerPC601State(  # noqa: SLF001
        cia=s.cia,
        gpr=tuple(new_gpr),
        lr=(lr & 0xFFFF_FFFF) if lr is not None else s.lr,
        ctr=(ctr & 0xFFFF_FFFF) if ctr is not None else s.ctr,
        xer=xer if xer is not None else s.xer,
        cr=cr if cr is not None else s.cr,
        memory=s.memory,
        halted=s.halted,
    )


def run_from_current(sim: PowerPC601Simulator, max_steps: int = 1000) -> tuple[PowerPC601State, str | None]:
    """Step the already-loaded simulator without re-loading (preserves preset registers)."""
    for _ in range(max_steps):
        if sim.get_state().halted:
            break
        trace = sim.step()
        if sim.get_state().halted:
            break
        if trace.mnemonic.startswith("ERROR:"):
            return sim.get_state(), trace.mnemonic
    return sim.get_state(), None


# ── addi ──────────────────────────────────────────────────────────────────────


def test_addi_positive():
    prog = d_form(PO_ADDI, 3, 0, 100) + HALT
    s = run(prog)
    assert s.r3 == 100


def test_addi_negative():
    prog = d_form(PO_ADDI, 4, 0, -1) + HALT
    s = run(prog)
    assert s.r4 == 0xFFFF_FFFF


def test_addi_rA_zero_means_zero():
    """addi rD, 0, val uses 0 as base, not GPR0's value."""
    sim = PowerPC601Simulator()
    prog = d_form(PO_ADDI, 3, 0, 99) + HALT
    sim.load(prog)
    preset(sim, gpr={0: 9999})  # set GPR0 to something
    s, _ = run_from_current(sim)
    assert s.r3 == 99  # still 0 + 99, not 9999 + 99


def test_addi_rA_nonzero_adds():
    sim = PowerPC601Simulator()
    prog = d_form(PO_ADDI, 5, 3, 10) + HALT
    sim.load(prog)
    preset(sim, gpr={3: 20})
    s, _ = run_from_current(sim)
    assert s.r5 == 30


def test_addi_wrap32():
    sim = PowerPC601Simulator()
    prog = d_form(PO_ADDI, 6, 6, 1) + HALT
    sim.load(prog)
    preset(sim, gpr={6: 0xFFFF_FFFF})
    s, _ = run_from_current(sim)
    assert s.r6 == 0


# ── addis ─────────────────────────────────────────────────────────────────────


def test_addis_load_upper():
    """lis r3, 1  =  addis r3, 0, 1  → r3 = 0x0001_0000"""
    prog = d_form(PO_ADDIS, 3, 0, 1) + HALT
    s = run(prog)
    assert s.r3 == 0x0001_0000


def test_addis_adds_shifted():
    sim = PowerPC601Simulator()
    prog = d_form(PO_ADDIS, 4, 3, 2) + HALT
    sim.load(prog)
    preset(sim, gpr={3: 0x0000_0001})
    s, _ = run_from_current(sim)
    assert s.r4 == 0x0002_0001


# ── subfic ────────────────────────────────────────────────────────────────────


def test_subfic_basic():
    sim = PowerPC601Simulator()
    prog = d_form(PO_SUBFIC, 5, 3, 10) + HALT
    sim.load(prog)
    preset(sim, gpr={3: 3})
    s, _ = run_from_current(sim)
    assert s.r5 == 7   # 10 - 3 = 7


def test_subfic_ca_set_when_no_borrow():
    from powerpc601_simulator import XER_CA
    sim = PowerPC601Simulator()
    prog = d_form(PO_SUBFIC, 5, 3, 10) + HALT
    sim.load(prog)
    preset(sim, gpr={3: 3})  # 10 >= 3 unsigned → no borrow → CA=1
    s, _ = run_from_current(sim)
    assert s.xer & XER_CA


def test_subfic_ca_clear_on_borrow():
    from powerpc601_simulator import XER_CA
    sim = PowerPC601Simulator()
    prog = d_form(PO_SUBFIC, 5, 3, 3) + HALT
    sim.load(prog)
    preset(sim, gpr={3: 10})  # 3 < 10 unsigned → borrow → CA=0
    s, _ = run_from_current(sim)
    assert not (s.xer & XER_CA)


# ── ori / oris / xori ─────────────────────────────────────────────────────────


def test_ori_basic():
    sim = PowerPC601Simulator()
    prog = d_form(PO_ORI, 5, 3, 0xF0) + HALT  # ori r3, r5, 0xF0 — rS=r5, rA=r3
    sim.load(prog)
    preset(sim, gpr={5: 0x0F})
    s, _ = run_from_current(sim)
    assert s.r3 == 0xFF


def test_oris_shifted():
    sim = PowerPC601Simulator()
    prog = d_form(PO_ORIS, 5, 4, 0x0001) + HALT  # oris r4, r5, 0x0001 — rS=r5, rA=r4
    sim.load(prog)
    preset(sim, gpr={5: 0xFFFF})
    s, _ = run_from_current(sim)
    assert s.r4 == 0x0001_FFFF


def test_xori_basic():
    sim = PowerPC601Simulator()
    prog = d_form(PO_XORI, 5, 3, 0xFF) + HALT  # xori r3, r5, 0xFF — rS=r5, rA=r3
    sim.load(prog)
    preset(sim, gpr={5: 0xAA})
    s, _ = run_from_current(sim)
    assert s.r3 == 0x55


# ── andi. / andis. ────────────────────────────────────────────────────────────


def test_andi_dot_zero_sets_cr0_eq():
    prog = d_form(PO_ANDI_DOT, 3, 4, 0xFF) + HALT  # andi. r4, r3, 0xFF — rS=r3=0 → r4=0
    s = run(prog)
    assert s.cr0_eq


def test_andi_dot_nonzero_sets_cr0_gt():
    sim = PowerPC601Simulator()
    prog = d_form(PO_ANDI_DOT, 3, 4, 0xFF) + HALT  # andi. r4, r3, 0xFF — rS=r3, rA=r4
    sim.load(prog)
    preset(sim, gpr={3: 0x0A})  # r3=rS=0x0A; r4 = 0x0A & 0xFF = 0x0A > 0 → GT
    s, _ = run_from_current(sim)
    assert s.cr0_gt


def test_andi_dot_negative_sets_cr0_lt():
    """andi. UIMM is zero-extended so bit 31 is always 0; use andis. to set bit 31."""
    sim = PowerPC601Simulator()
    # andis. r4, r3, 0x8000 → r4 = r3 & (0x8000 << 16) = 0x8000_0000 → CR0.LT
    prog = d_form(PO_ANDIS_DOT, 3, 4, 0x8000) + HALT  # rS=r3, rA=r4
    sim.load(prog)
    preset(sim, gpr={3: 0xFFFF_FFFF})
    s, _ = run_from_current(sim)
    assert s.cr0_lt


def test_andis_dot_updates_cr0():
    sim = PowerPC601Simulator()
    prog = d_form(PO_ANDIS_DOT, 3, 4, 0x0001) + HALT  # andis. r4, r3, 0x0001 — rS=r3, rA=r4
    sim.load(prog)
    preset(sim, gpr={3: 0x0001_0000})  # r4 = r3 & (0x0001 << 16) = 0x0001_0000 → GT
    s, _ = run_from_current(sim)
    assert s.r4 == 0x0001_0000
    assert s.cr0_gt


# ── cmpwi / cmplwi ───────────────────────────────────────────────────────────


def test_cmpwi_less_than():
    sim = PowerPC601Simulator()
    prog = d_form(PO_CMPI, 0 << 2, 3, 10) + HALT  # cmpwi cr0, r3, 10
    sim.load(prog)
    preset(sim, gpr={3: 5})
    s, _ = run_from_current(sim)
    assert s.cr0_lt


def test_cmpwi_greater_than():
    sim = PowerPC601Simulator()
    prog = d_form(PO_CMPI, 0 << 2, 3, 5) + HALT  # cmpwi cr0, r3, 5
    sim.load(prog)
    preset(sim, gpr={3: 10})
    s, _ = run_from_current(sim)
    assert s.cr0_gt


def test_cmpwi_equal():
    sim = PowerPC601Simulator()
    prog = d_form(PO_CMPI, 0 << 2, 3, 7) + HALT
    sim.load(prog)
    preset(sim, gpr={3: 7})
    s, _ = run_from_current(sim)
    assert s.cr0_eq


def test_cmpwi_signed():
    """cmpwi should treat -1 as less than 0."""
    sim = PowerPC601Simulator()
    prog = d_form(PO_CMPI, 0 << 2, 3, 0) + HALT
    sim.load(prog)
    preset(sim, gpr={3: 0xFFFF_FFFF})  # -1 signed
    s, _ = run_from_current(sim)
    assert s.cr0_lt


def test_cmplwi_unsigned():
    """cmplwi treats -1 (0xFFFF_FFFF) as greater than 0."""
    sim = PowerPC601Simulator()
    prog = d_form(PO_CMPLI, 0 << 2, 3, 0) + HALT
    sim.load(prog)
    preset(sim, gpr={3: 0xFFFF_FFFF})
    s, _ = run_from_current(sim)
    assert s.cr0_gt


def test_cmpwi_cr1():
    """Compare into CR1 (crfD=1)."""
    sim = PowerPC601Simulator()
    prog = d_form(PO_CMPI, 1 << 2, 3, 0) + HALT  # cmpwi cr1, r3, 0
    sim.load(prog)
    preset(sim, gpr={3: 0})
    s, _ = run_from_current(sim)
    # CR1.EQ should be set (bit 2 of CR1 nibble)
    assert (s.cr_field(1) >> 1) & 1  # EQ bit of CR1


# ── add / subf / neg / mullw / divw / divwu ──────────────────────────────────


def test_add():
    sim = PowerPC601Simulator()
    prog = xo_form(PO_X31, 5, 3, 4, 0, XO_ADD) + HALT
    sim.load(prog)
    preset(sim, gpr={3: 15, 4: 27})
    s, _ = run_from_current(sim)
    assert s.r5 == 42


def test_subf():
    """subf rD, rA, rB → rD = rB - rA."""
    sim = PowerPC601Simulator()
    prog = xo_form(PO_X31, 5, 3, 4, 0, XO_SUBF) + HALT
    sim.load(prog)
    preset(sim, gpr={3: 3, 4: 10})
    s, _ = run_from_current(sim)
    assert s.r5 == 7


def test_neg():
    sim = PowerPC601Simulator()
    prog = xo_form(PO_X31, 5, 3, 0, 0, XO_NEG) + HALT
    sim.load(prog)
    preset(sim, gpr={3: 5})
    s, _ = run_from_current(sim)
    assert s.r5 == 0xFFFF_FFFB  # -5 as unsigned 32-bit


def test_mullw():
    sim = PowerPC601Simulator()
    prog = xo_form(PO_X31, 5, 3, 4, 0, XO_MULLW) + HALT
    sim.load(prog)
    preset(sim, gpr={3: 6, 4: 7})
    s, _ = run_from_current(sim)
    assert s.r5 == 42


def test_divw():
    sim = PowerPC601Simulator()
    prog = xo_form(PO_X31, 5, 3, 4, 0, XO_DIVW) + HALT
    sim.load(prog)
    preset(sim, gpr={3: 20, 4: 4})
    s, _ = run_from_current(sim)
    assert s.r5 == 5


def test_divw_truncates_toward_zero():
    sim = PowerPC601Simulator()
    prog = xo_form(PO_X31, 5, 3, 4, 0, XO_DIVW) + HALT
    sim.load(prog)
    preset(sim, gpr={3: 0xFFFF_FFFD, 4: 2})  # -3 / 2 = -1 (truncated toward 0)
    s, _ = run_from_current(sim)
    assert s.r5 == 0xFFFF_FFFF  # -1 as 32-bit


def test_divwu():
    sim = PowerPC601Simulator()
    prog = xo_form(PO_X31, 5, 3, 4, 0, XO_DIVWU) + HALT
    sim.load(prog)
    preset(sim, gpr={3: 100, 4: 7})
    s, _ = run_from_current(sim)
    assert s.r5 == 14  # 100 // 7 = 14


def test_addc_sets_carry():
    from powerpc601_simulator import XER_CA
    sim = PowerPC601Simulator()
    prog = xo_form(PO_X31, 5, 3, 4, 0, XO_ADDC) + HALT
    sim.load(prog)
    preset(sim, gpr={3: 0xFFFF_FFFF, 4: 1})
    s, _ = run_from_current(sim)
    assert s.r5 == 0
    assert s.xer & XER_CA


def test_adde_uses_carry():
    from powerpc601_simulator import XER_CA
    sim = PowerPC601Simulator()
    prog = xo_form(PO_X31, 5, 3, 4, 0, XO_ADDE) + HALT
    sim.load(prog)
    preset(sim, gpr={3: 1, 4: 2}, xer=XER_CA)  # CA=1
    s, _ = run_from_current(sim)
    assert s.r5 == 4   # 1 + 2 + 1 = 4


# ── and / or / xor / nand / nor / cntlzw ────────────────────────────────────


def test_and():
    sim = PowerPC601Simulator()
    prog = x_form(PO_X31, 3, 5, 4, XO_AND) + HALT  # and r5, r3, r4
    sim.load(prog)
    preset(sim, gpr={3: 0xFF, 4: 0xF0})
    s, _ = run_from_current(sim)
    assert s.r5 == 0xF0


def test_or():
    sim = PowerPC601Simulator()
    prog = x_form(PO_X31, 3, 5, 4, XO_OR) + HALT  # or r5, r3, r4
    sim.load(prog)
    preset(sim, gpr={3: 0x0F, 4: 0xF0})
    s, _ = run_from_current(sim)
    assert s.r5 == 0xFF


def test_xor():
    sim = PowerPC601Simulator()
    prog = x_form(PO_X31, 3, 5, 4, XO_XOR) + HALT  # xor r5, r3, r4
    sim.load(prog)
    preset(sim, gpr={3: 0xFF, 4: 0x0F})
    s, _ = run_from_current(sim)
    assert s.r5 == 0xF0


def test_nand():
    sim = PowerPC601Simulator()
    prog = x_form(PO_X31, 3, 5, 4, XO_NAND) + HALT  # nand r5, r3, r4
    sim.load(prog)
    preset(sim, gpr={3: 0xFFFF_FFFF, 4: 0xFFFF_FFFF})
    s, _ = run_from_current(sim)
    assert s.r5 == 0   # ~(all_ones & all_ones) = 0


def test_nor():
    sim = PowerPC601Simulator()
    prog = x_form(PO_X31, 3, 5, 4, XO_NOR) + HALT  # nor r5, r3, r4
    sim.load(prog)
    preset(sim, gpr={3: 0x0000_FFFF, 4: 0xFFFF_0000})
    s, _ = run_from_current(sim)
    assert s.r5 == 0   # ~(0xFFFF_FFFF) masked to 32 = 0


def test_cntlzw_all_ones():
    sim = PowerPC601Simulator()
    prog = x_form(PO_X31, 3, 5, 0, XO_CNTLZW) + HALT  # cntlzw r5, r3
    sim.load(prog)
    preset(sim, gpr={3: 0xFFFF_FFFF})
    s, _ = run_from_current(sim)
    assert s.r5 == 0


def test_cntlzw_zero():
    sim = PowerPC601Simulator()
    prog = x_form(PO_X31, 3, 5, 0, XO_CNTLZW) + HALT
    sim.load(prog)
    preset(sim, gpr={3: 0})
    s, _ = run_from_current(sim)
    assert s.r5 == 32


def test_cntlzw_one():
    sim = PowerPC601Simulator()
    prog = x_form(PO_X31, 3, 5, 0, XO_CNTLZW) + HALT
    sim.load(prog)
    preset(sim, gpr={3: 1})
    s, _ = run_from_current(sim)
    assert s.r5 == 31


# ── slw / srw / sraw / srawi ─────────────────────────────────────────────────


def test_slw():
    sim = PowerPC601Simulator()
    prog = x_form(PO_X31, 3, 5, 4, XO_SLW) + HALT
    sim.load(prog)
    preset(sim, gpr={3: 1, 4: 4})  # 1 << 4 = 16
    s, _ = run_from_current(sim)
    assert s.r5 == 16


def test_slw_ge32_is_zero():
    sim = PowerPC601Simulator()
    prog = x_form(PO_X31, 3, 5, 4, XO_SLW) + HALT
    sim.load(prog)
    preset(sim, gpr={3: 0xFFFF_FFFF, 4: 32})
    s, _ = run_from_current(sim)
    assert s.r5 == 0


def test_srw():
    sim = PowerPC601Simulator()
    prog = x_form(PO_X31, 3, 5, 4, XO_SRW) + HALT
    sim.load(prog)
    preset(sim, gpr={3: 0x100, 4: 4})  # 256 >> 4 = 16
    s, _ = run_from_current(sim)
    assert s.r5 == 16


def test_srw_ge32_is_zero():
    sim = PowerPC601Simulator()
    prog = x_form(PO_X31, 3, 5, 4, XO_SRW) + HALT
    sim.load(prog)
    preset(sim, gpr={3: 0xFFFF_FFFF, 4: 32})
    s, _ = run_from_current(sim)
    assert s.r5 == 0


def test_sraw_sign_extends():
    """sraw propagates the sign bit."""
    sim = PowerPC601Simulator()
    prog = x_form(PO_X31, 3, 5, 4, XO_SRAW) + HALT
    sim.load(prog)
    preset(sim, gpr={3: 0x8000_0000, 4: 1})  # -2^31 >> 1
    s, _ = run_from_current(sim)
    assert s.r5 == 0xC000_0000  # -2^31 / 2 = -2^30


def test_srawi_basic():
    sim = PowerPC601Simulator()
    prog = x_form(PO_X31, 3, 5, 2, XO_SRAWI) + HALT  # srawi r5, r3, 2
    sim.load(prog)
    preset(sim, gpr={3: 20})
    s, _ = run_from_current(sim)
    assert s.r5 == 5


def test_sraw_ca_set_for_negative_with_shifted_bits():
    from powerpc601_simulator import XER_CA
    sim = PowerPC601Simulator()
    prog = x_form(PO_X31, 3, 5, 4, XO_SRAW) + HALT
    sim.load(prog)
    preset(sim, gpr={3: 0xFFFF_FFFF, 4: 1})  # -1 >> 1; loses the LSB which is 1
    s, _ = run_from_current(sim)
    assert s.xer & XER_CA  # negative and shifted out a 1-bit


# ── cmpw / cmplw (X-form) ────────────────────────────────────────────────────


def test_cmpw_lt():
    sim = PowerPC601Simulator()
    prog = x_form(PO_X31, 0, 3, 4, XO_CMP) + HALT  # cmpw cr0, r3, r4
    sim.load(prog)
    preset(sim, gpr={3: 1, 4: 2})
    s, _ = run_from_current(sim)
    assert s.cr0_lt


def test_cmpw_gt():
    sim = PowerPC601Simulator()
    prog = x_form(PO_X31, 0, 3, 4, XO_CMP) + HALT
    sim.load(prog)
    preset(sim, gpr={3: 10, 4: 5})
    s, _ = run_from_current(sim)
    assert s.cr0_gt


def test_cmplw_unsigned():
    """cmplw treats 0xFFFF_FFFF as greater than 0."""
    sim = PowerPC601Simulator()
    prog = x_form(PO_X31, 0, 3, 4, XO_CMPL) + HALT  # cmplw cr0, r3, r4
    sim.load(prog)
    preset(sim, gpr={3: 0xFFFF_FFFF, 4: 0})
    s, _ = run_from_current(sim)
    assert s.cr0_gt


# ── lwz / lwzu / lbz / lbzu / lhz / lhzu / lha ───────────────────────────────


def test_lwz_loads_word():
    sim = PowerPC601Simulator()
    prog = d_form(PO_LWZ, 3, 0, 0x200) + HALT
    sim.load(prog)
    # Manually write a value into memory at 0x200
    s0 = sim.get_state()
    mem = list(s0.memory)
    mem[0x200] = 0x12; mem[0x201] = 0x34; mem[0x202] = 0xAB; mem[0x203] = 0xCD
    sim._state = PowerPC601State(**{**s0.__dict__, "memory": tuple(mem)})  # type: ignore[arg-type]
    s, _ = run_from_current(sim)
    assert s.r3 == 0x1234_ABCD


def test_lwzu_updates_ra():
    sim = PowerPC601Simulator()
    prog = d_form(PO_LWZU, 3, 4, 4) + HALT  # lwzu r3, 4(r4) — EA = r4+4
    sim.load(prog)
    s0 = sim.get_state()
    mem = list(s0.memory)
    mem[0x100] = 0; mem[0x101] = 0; mem[0x102] = 0; mem[0x103] = 0x42
    sim._state = PowerPC601State(**{**s0.__dict__, "memory": tuple(mem)})  # type: ignore[arg-type]
    preset(sim, gpr={4: 0x100 - 4})  # so EA = 0x100
    s, _ = run_from_current(sim)
    assert s.r3 == 0x42
    assert s.r4 == 0x100


def test_lbz_zero_extends():
    sim = PowerPC601Simulator()
    prog = d_form(PO_LBZ, 3, 0, 0x10) + HALT
    sim.load(prog)
    s0 = sim.get_state()
    mem = list(s0.memory)
    mem[0x10] = 0xFF
    sim._state = PowerPC601State(**{**s0.__dict__, "memory": tuple(mem)})  # type: ignore[arg-type]
    s, _ = run_from_current(sim)
    assert s.r3 == 0xFF  # zero-extended


def test_lhz_loads_halfword():
    sim = PowerPC601Simulator()
    prog = d_form(PO_LHZ, 5, 0, 0x20) + HALT
    sim.load(prog)
    s0 = sim.get_state()
    mem = list(s0.memory)
    mem[0x20] = 0xBE; mem[0x21] = 0xEF
    sim._state = PowerPC601State(**{**s0.__dict__, "memory": tuple(mem)})  # type: ignore[arg-type]
    s, _ = run_from_current(sim)
    assert s.r5 == 0xBEEF


def test_lha_sign_extends():
    sim = PowerPC601Simulator()
    prog = d_form(PO_LHA, 5, 0, 0x20) + HALT
    sim.load(prog)
    s0 = sim.get_state()
    mem = list(s0.memory)
    mem[0x20] = 0xFF; mem[0x21] = 0xFF  # -1 as 16-bit
    sim._state = PowerPC601State(**{**s0.__dict__, "memory": tuple(mem)})  # type: ignore[arg-type]
    s, _ = run_from_current(sim)
    assert s.r5 == 0xFFFF_FFFF  # sign-extended to 32 bits


def test_lhzu_updates_ra():
    sim = PowerPC601Simulator()
    prog = d_form(PO_LHZU, 5, 4, 2) + HALT  # lhzu r5, 2(r4)
    sim.load(prog)
    s0 = sim.get_state()
    mem = list(s0.memory)
    mem[0x102] = 0x12; mem[0x103] = 0x34
    sim._state = PowerPC601State(**{**s0.__dict__, "memory": tuple(mem)})  # type: ignore[arg-type]
    preset(sim, gpr={4: 0x100})
    s, _ = run_from_current(sim)
    assert s.r5 == 0x1234
    assert s.r4 == 0x102


def test_lbzu_updates_ra():
    sim = PowerPC601Simulator()
    prog = d_form(PO_LBZU, 5, 4, 1) + HALT
    sim.load(prog)
    s0 = sim.get_state()
    mem = list(s0.memory)
    mem[0x11] = 0x77
    sim._state = PowerPC601State(**{**s0.__dict__, "memory": tuple(mem)})  # type: ignore[arg-type]
    preset(sim, gpr={4: 0x10})
    s, _ = run_from_current(sim)
    assert s.r5 == 0x77
    assert s.r4 == 0x11


# ── stw / stwu / stb / stbu / sth ────────────────────────────────────────────


def test_stw_stores_word():
    sim = PowerPC601Simulator()
    prog = d_form(PO_STW, 3, 0, 0x200) + HALT
    sim.load(prog)
    preset(sim, gpr={3: 0xDEAD_BEEF})
    s, _ = run_from_current(sim)
    assert s.memory[0x200] == 0xDE
    assert s.memory[0x201] == 0xAD
    assert s.memory[0x202] == 0xBE
    assert s.memory[0x203] == 0xEF


def test_stwu_updates_ra():
    sim = PowerPC601Simulator()
    prog = d_form(PO_STWU, 3, 4, 4) + HALT  # stwu r3, 4(r4)
    sim.load(prog)
    preset(sim, gpr={3: 0x1234_5678, 4: 0x100})
    s, _ = run_from_current(sim)
    assert s.memory[0x104] == 0x12
    assert s.r4 == 0x104


def test_stb_stores_byte():
    sim = PowerPC601Simulator()
    prog = d_form(PO_STB, 3, 0, 0x50) + HALT
    sim.load(prog)
    preset(sim, gpr={3: 0x1234_56AB})
    s, _ = run_from_current(sim)
    assert s.memory[0x50] == 0xAB  # low byte only


def test_stbu_updates_ra():
    sim = PowerPC601Simulator()
    prog = d_form(PO_STBU, 3, 4, 1) + HALT
    sim.load(prog)
    preset(sim, gpr={3: 0xCC, 4: 0x20})
    s, _ = run_from_current(sim)
    assert s.memory[0x21] == 0xCC
    assert s.r4 == 0x21


def test_sth_stores_halfword():
    sim = PowerPC601Simulator()
    prog = d_form(PO_STH, 3, 0, 0x60) + HALT
    sim.load(prog)
    preset(sim, gpr={3: 0xABCD})
    s, _ = run_from_current(sim)
    assert s.memory[0x60] == 0xAB
    assert s.memory[0x61] == 0xCD


# ── b / bl ────────────────────────────────────────────────────────────────────


def test_b_jumps():
    """b +8 skips one 4-byte instruction."""
    # Layout: [0] b +8  [4] addi r3,0,99  [8] HALT
    prog = (
        i_form(PO_B, 8)            # jump over next instruction
        + d_form(PO_ADDI, 3, 0, 99)  # should be skipped
        + HALT
    )
    s = run(prog)
    assert s.r3 == 0  # addi skipped


def test_bl_saves_lr():
    """bl saves CIA+4 to LR."""
    # Layout: [0] bl +8  [4] HALT  [8] addi r3,0,1  [12] HALT
    prog = (
        i_form(PO_B, 8, LK=1)      # bl to +8
        + HALT                       # [4] never reached in first pass
        + d_form(PO_ADDI, 3, 0, 1)  # [8]
        + HALT                       # [12]
    )
    s = run(prog)
    assert s.lr == 4    # LR = CIA+4 = 0+4


def test_b_backward():
    """Backward branch: b -4 loops until something stops."""
    # Use a counter: dec CTR via mtspr; then bdnz loops
    # Simpler: just verify that a negative branch target works by using bc
    # Layout: [0] addi r3,0,1  [4] b -4 ... would loop forever.
    # Instead test via execute result after max_steps
    prog = d_form(PO_ADDI, 0, 0, 0) + i_form(PO_B, -4)  # NOP then loop
    result = PowerPC601Simulator().execute(prog, max_steps=5)
    assert not result.ok  # should hit max_steps


# ── bc (conditional branch) ───────────────────────────────────────────────────


def test_bc_beq_taken():
    """beq branches when CR0.EQ=1."""
    sim = PowerPC601Simulator()
    # [0] cmpwi r3, 0  [4] beq +8  [8] addi r3,0,99  [12] HALT
    prog = (
        d_form(PO_CMPI, 0, 3, 0)         # cmpwi cr0, r3, 0
        + b_form(PO_BC, BO_TRUE, BI_EQ, 8)  # beq +8 (skip next)
        + d_form(PO_ADDI, 3, 0, 99)        # r3 = 99 (skipped)
        + HALT
    )
    sim.load(prog)
    preset(sim, gpr={3: 0})
    s, _ = run_from_current(sim)
    assert s.r3 == 0  # addi was skipped


def test_bc_beq_not_taken():
    sim = PowerPC601Simulator()
    prog = (
        d_form(PO_CMPI, 0, 3, 0)           # cmpwi cr0, r3, 0
        + b_form(PO_BC, BO_TRUE, BI_EQ, 8)   # beq +8
        + d_form(PO_ADDI, 3, 0, 99)          # r3 = 99 (executed when not taken)
        + HALT
    )
    sim.load(prog)
    preset(sim, gpr={3: 5})  # r3 != 0 → EQ not set → branch not taken
    s, _ = run_from_current(sim)
    assert s.r3 == 99


def test_bc_blt_taken():
    sim = PowerPC601Simulator()
    prog = (
        d_form(PO_CMPI, 0, 3, 10)            # cmpwi cr0, r3, 10
        + b_form(PO_BC, BO_TRUE, BI_LT, 8)    # blt +8
        + d_form(PO_ADDI, 3, 0, 99)           # skipped
        + HALT
    )
    sim.load(prog)
    preset(sim, gpr={3: 5})
    s, _ = run_from_current(sim)
    assert s.r3 == 5  # addi skipped


def test_bc_bne_taken():
    sim = PowerPC601Simulator()
    prog = (
        d_form(PO_CMPI, 0, 3, 10)             # cmpwi cr0, r3, 10
        + b_form(PO_BC, BO_FALSE, BI_EQ, 8)   # bne +8 (branch if EQ=0)
        + d_form(PO_ADDI, 3, 0, 99)           # skipped
        + HALT
    )
    sim.load(prog)
    preset(sim, gpr={3: 5})  # 5 != 10 → EQ=0 → bne taken
    s, _ = run_from_current(sim)
    assert s.r3 == 5


def test_bc_bdnz():
    """bdnz decrements CTR and branches while CTR != 0."""
    sim = PowerPC601Simulator()
    # [0] addi r3, r3, 1   [4] bdnz -4 (back to 0)  [8] HALT
    prog = (
        d_form(PO_ADDI, 3, 3, 1)          # r3 += 1
        + b_form(PO_BC, BO_BDNZ, 0, -4)   # bdnz back to [0]
        + HALT
    )
    sim.load(prog)
    preset(sim, gpr={3: 0}, ctr=5)  # loop 5 times
    s, _ = run_from_current(sim)
    assert s.r3 == 5
    assert s.ctr == 0


def test_bc_bdz():
    """bdz branches when CTR reaches 0."""
    sim = PowerPC601Simulator()
    # [0] bdz +8  [4] addi r3,0,99  [8] HALT
    prog = (
        b_form(PO_BC, BO_BDZ, 0, 8)      # bdz to [8] if CTR==0 after dec
        + d_form(PO_ADDI, 3, 0, 99)       # [4] skipped when CTR was 1
        + HALT                              # [8]
    )
    sim.load(prog)
    preset(sim, ctr=1)  # CTR=1 → after dec = 0 → branch taken
    s, _ = run_from_current(sim)
    assert s.r3 == 0  # addi skipped


# ── blr / bctr / bctrl ────────────────────────────────────────────────────────


def test_blr_branches_to_lr():
    sim = PowerPC601Simulator()
    # [0] blr  → jumps to LR
    # Set LR to point to [8]: [4] addi r3,0,99  [8] HALT
    prog = (
        xl_form(PO_BX, BO_ALWAYS, 0, 0, XO_BCLR)  # blr
        + d_form(PO_ADDI, 3, 0, 99)                  # [4] skipped
        + HALT                                         # [8]
    )
    sim.load(prog)
    preset(sim, lr=8)
    s, _ = run_from_current(sim)
    assert s.r3 == 0  # addi at [4] was skipped


def test_bctr_branches_to_ctr():
    sim = PowerPC601Simulator()
    prog = (
        xl_form(PO_BX, BO_ALWAYS, 0, 0, XO_BCCTR)  # bctr
        + d_form(PO_ADDI, 3, 0, 99)                   # [4] skipped
        + HALT                                          # [8]
    )
    sim.load(prog)
    preset(sim, ctr=8)
    s, _ = run_from_current(sim)
    assert s.r3 == 0  # skipped


def test_bctrl_saves_lr():
    sim = PowerPC601Simulator()
    prog = (
        xl_form(PO_BX, BO_ALWAYS, 0, 0, XO_BCCTR, lk=1)  # bctrl
        + HALT                                                # [4] (LR target)
        + HALT                                                # [8]
    )
    sim.load(prog)
    preset(sim, ctr=8)  # branches to [8]
    s, _ = run_from_current(sim)
    assert s.lr == 4  # CIA+4 saved to LR


# ── mfspr / mtspr / mfcr / mtcrf ────────────────────────────────────────────


def test_mtspr_lr_and_mfspr_lr():
    sim = PowerPC601Simulator()
    prog = (
        xfx_form(PO_X31, 3, SPR_LR, XO_MTSPR)  # mtspr LR, r3
        + xfx_form(PO_X31, 5, SPR_LR, XO_MFSPR)  # mfspr r5, LR
        + HALT
    )
    sim.load(prog)
    preset(sim, gpr={3: 0xDEAD_BEEF})
    s, _ = run_from_current(sim)
    assert s.lr == 0xDEAD_BEEF
    assert s.r5 == 0xDEAD_BEEF


def test_mtspr_ctr_and_mfspr_ctr():
    sim = PowerPC601Simulator()
    prog = (
        xfx_form(PO_X31, 3, SPR_CTR, XO_MTSPR)
        + xfx_form(PO_X31, 5, SPR_CTR, XO_MFSPR)
        + HALT
    )
    sim.load(prog)
    preset(sim, gpr={3: 42})
    s, _ = run_from_current(sim)
    assert s.ctr == 42
    assert s.r5 == 42


def test_mfcr_reads_cr():
    sim = PowerPC601Simulator()
    prog = (
        x_form(PO_X31, 3, 0, 0, XO_MFCR)  # mfcr r3
        + HALT
    )
    sim.load(prog)
    preset(sim, cr=0xABCD_EF01)
    s, _ = run_from_current(sim)
    assert s.r3 == 0xABCD_EF01


def test_mtcrf_updates_selected_fields():
    sim = PowerPC601Simulator()
    # FXM=0xFF updates all CR fields
    prog = (
        x_form(PO_X31, 3, 0xFF, 0, XO_MTCRF) + HALT  # mtcrf 0xFF, r3
    )
    # Wait — mtcrf encoding: [OPCD:6][rS:5][1:1][FXM:8][--:1][rB:5][XO:10][0:1]
    # FXM is at bits [20:12] = positions [19:12] in Python bit notation
    # Let me encode this properly using the raw instruction:
    # OPCD=31 at [31:26], rS at [25:21], bit20=1, FXM at [19:12], XO=144 at [10:1]
    fxm = 0xFF
    rs = 3
    raw = (PO_X31 << 26) | (rs << 21) | (1 << 20) | (fxm << 12) | (XO_MTCRF << 1)
    prog = raw.to_bytes(4, "big") + HALT
    sim.load(prog)
    preset(sim, gpr={3: 0x1234_5678})
    s, _ = run_from_current(sim)
    assert s.cr == 0x1234_5678
