"""Edge-case and coverage tests for the PowerPC 601 simulator."""

from powerpc601_simulator import (
    HALT,
    PowerPC601Simulator,
    PowerPC601State,
    d_form,
    i_form,
    xfx_form,
    xo_form,
)
from powerpc601_simulator.simulator import (
    PO_ADDI,
    PO_B,
    PO_CMPI,
    PO_LWZ,
    PO_STW,
    PO_X31,
    XO_ADD,
    XO_DIVW,
    XO_DIVWU,
    XO_MFSPR,
    XO_NEG,
)

# ── 32-bit masking ────────────────────────────────────────────────────────────


def test_add_wraps_32bit():
    sim = PowerPC601Simulator()
    prog = xo_form(PO_X31, 5, 3, 4, 0, XO_ADD) + HALT
    sim.load(prog)
    s0 = sim.get_state()
    gpr = list(s0.gpr)
    gpr[3] = 0xFFFF_FFFF
    gpr[4] = 1
    sim._state = PowerPC601State(**{**s0.__dict__, "gpr": tuple(gpr)})  # type: ignore[arg-type]
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.r5 == 0  # wraps to 0


def test_neg_of_zero():
    prog = xo_form(PO_X31, 5, 3, 0, 0, XO_NEG) + HALT
    result = PowerPC601Simulator().execute(prog)
    assert result.final_state.r5 == 0  # -0 = 0 in two's complement


def test_neg_min_int():
    """Negating the minimum 32-bit signed value overflows back to itself."""
    sim = PowerPC601Simulator()
    prog = xo_form(PO_X31, 5, 3, 0, 0, XO_NEG) + HALT
    sim.load(prog)
    s0 = sim.get_state()
    gpr = list(s0.gpr)
    gpr[3] = 0x8000_0000
    sim._state = PowerPC601State(**{**s0.__dict__, "gpr": tuple(gpr)})  # type: ignore[arg-type]
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.r5 == 0x8000_0000  # -INT_MIN = INT_MIN (overflow)


# ── Memory big-endian ─────────────────────────────────────────────────────────


def test_stw_big_endian_byte_order():
    """stw writes MSB first."""
    sim = PowerPC601Simulator()
    prog = d_form(PO_STW, 3, 0, 0x300) + HALT
    sim.load(prog)
    s0 = sim.get_state()
    gpr = list(s0.gpr)
    gpr[3] = 0x01_02_03_04
    sim._state = PowerPC601State(**{**s0.__dict__, "gpr": tuple(gpr)})  # type: ignore[arg-type]
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.memory[0x300] == 0x01
    assert s.memory[0x301] == 0x02
    assert s.memory[0x302] == 0x03
    assert s.memory[0x303] == 0x04


def test_lwz_big_endian_byte_order():
    """lwz reads MSB-first big-endian."""
    sim = PowerPC601Simulator()
    prog = d_form(PO_LWZ, 3, 0, 0x300) + HALT
    sim.load(prog)
    s0 = sim.get_state()
    mem = list(s0.memory)
    mem[0x300] = 0xDE; mem[0x301] = 0xAD; mem[0x302] = 0xBE; mem[0x303] = 0xEF
    sim._state = PowerPC601State(**{**s0.__dict__, "memory": tuple(mem)})  # type: ignore[arg-type]
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.r3 == 0xDEAD_BEEF


# ── HALT semantics ────────────────────────────────────────────────────────────


def test_halt_on_all_zeros():
    """0x00000000 halts immediately."""
    sim = PowerPC601Simulator()
    result = sim.execute(HALT)
    assert result.halted
    assert result.final_state.cia == 0


def test_step_on_halted_stays():
    """Stepping when already halted returns HALT trace without advancing CIA."""
    sim = PowerPC601Simulator()
    sim.execute(HALT)
    trace = sim.step()
    assert "HALT" in trace.mnemonic
    assert trace.pc_before == trace.pc_after


# ── Unknown opcode ────────────────────────────────────────────────────────────


def test_unknown_opcode_halts():
    """An unrecognized opcode causes an ERROR trace and halts."""
    bad = bytes([0b00000101, 0, 0, 0])  # primary opcode 1 is not implemented
    sim = PowerPC601Simulator()
    result = sim.execute(bad)
    assert not result.ok


# ── GPR0 base-register-zero rule ─────────────────────────────────────────────


def test_gpr0_as_base_is_zero_for_lwz():
    """lwz rD, d(r0) uses 0 as base, not GPR0's value."""
    sim = PowerPC601Simulator()
    prog = d_form(PO_LWZ, 5, 0, 0x100) + HALT
    sim.load(prog)
    s0 = sim.get_state()
    gpr = list(s0.gpr)
    gpr[0] = 0xDEAD  # would corrupt EA if GPR0 were used
    mem = list(s0.memory)
    mem[0x100] = 0; mem[0x101] = 0; mem[0x102] = 0; mem[0x103] = 0x42
    sim._state = PowerPC601State(**{**s0.__dict__, "gpr": tuple(gpr), "memory": tuple(mem)})  # type: ignore[arg-type]
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.r5 == 0x42  # loaded from address 0x100, not 0xDEAD+0x100


def test_gpr0_as_source_for_add_is_real():
    """GPR0 participates normally in arithmetic (only address calc is special)."""
    sim = PowerPC601Simulator()
    prog = xo_form(PO_X31, 5, 0, 3, 0, XO_ADD) + HALT  # add r5, r0, r3
    sim.load(prog)
    s0 = sim.get_state()
    gpr = list(s0.gpr)
    gpr[0] = 10
    gpr[3] = 20
    sim._state = PowerPC601State(**{**s0.__dict__, "gpr": tuple(gpr)})  # type: ignore[arg-type]
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.r5 == 30  # GPR0 used at face value in arithmetic


# ── State immutability ────────────────────────────────────────────────────────


def test_state_gpr_immutable():
    sim = PowerPC601Simulator()
    s = sim.get_state()
    raised = False
    try:
        s.gpr = (0,) * 32  # type: ignore[misc]
    except (AttributeError, TypeError):
        raised = True
    assert raised


def test_snapshot_independence():
    """get_state() returns a separate frozen snapshot."""
    sim = PowerPC601Simulator()
    sim.load(d_form(PO_ADDI, 3, 0, 1) + HALT)
    snap1 = sim.get_state()
    sim.step()
    snap2 = sim.get_state()
    assert snap1.cia == 0
    assert snap2.cia == 4


# ── Carry flag details ────────────────────────────────────────────────────────


def test_addi_does_not_set_carry():
    """addi never sets XER[CA]."""
    from powerpc601_simulator import XER_CA
    result = PowerPC601Simulator().execute(d_form(PO_ADDI, 3, 0, -1) + HALT)
    assert not (result.final_state.xer & XER_CA)


# ── CR field isolation ────────────────────────────────────────────────────────


def test_cmpwi_only_updates_target_field():
    """cmpwi cr1 should not touch CR0."""
    sim = PowerPC601Simulator()
    prog = (
        d_form(PO_CMPI, 0, 3, 5)     # cmpwi cr0, r3, 5  → sets CR0
        + d_form(PO_CMPI, 1 << 2, 4, 10)  # cmpwi cr1, r4, 10
        + HALT
    )
    sim.load(prog)
    s0 = sim.get_state()
    gpr = list(s0.gpr)
    gpr[3] = 3   # cr0: lt
    gpr[4] = 10  # cr1: eq
    sim._state = PowerPC601State(**{**s0.__dict__, "gpr": tuple(gpr)})  # type: ignore[arg-type]
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.cr0_lt                   # CR0 still lt
    assert (s.cr_field(1) >> 1) & 1   # CR1.EQ set


# ── divw / divwu by zero ─────────────────────────────────────────────────────


def test_divw_by_zero_returns_zero():
    sim = PowerPC601Simulator()
    prog = xo_form(PO_X31, 5, 3, 4, 0, XO_DIVW) + HALT
    sim.load(prog)
    s0 = sim.get_state()
    gpr = list(s0.gpr)
    gpr[3] = 100; gpr[4] = 0
    sim._state = PowerPC601State(**{**s0.__dict__, "gpr": tuple(gpr)})  # type: ignore[arg-type]
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.r5 == 0  # implementation defined; returns 0


def test_divwu_by_zero_returns_zero():
    sim = PowerPC601Simulator()
    prog = xo_form(PO_X31, 5, 3, 4, 0, XO_DIVWU) + HALT
    sim.load(prog)
    s0 = sim.get_state()
    gpr = list(s0.gpr)
    gpr[3] = 100; gpr[4] = 0
    sim._state = PowerPC601State(**{**s0.__dict__, "gpr": tuple(gpr)})  # type: ignore[arg-type]
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.r5 == 0


# ── mfspr unknown SPR reads as 0 ─────────────────────────────────────────────


def test_mfspr_unknown_spr_is_zero():
    sim = PowerPC601Simulator()
    prog = xfx_form(PO_X31, 5, 99, XO_MFSPR) + HALT  # SPR 99 is not simulated
    result = sim.execute(prog)
    assert result.final_state.r5 == 0


# ── bc with absolute address ──────────────────────────────────────────────────


def test_b_absolute():
    """b absolute: AA=1 targets an absolute address."""
    # Place a HALT at absolute address 8 and use b with AA=1
    sim = PowerPC601Simulator()
    prog = (
        i_form(PO_B, 8, AA=1)         # b 8 (absolute) → jump to byte 8
        + d_form(PO_ADDI, 3, 0, 99)   # [4] skipped
        + HALT                          # [8]
    )
    result = sim.execute(prog)
    assert result.final_state.r3 == 0


# ── Large immediate sign extension ───────────────────────────────────────────


def test_addi_sign_extends_negative():
    """addi with -1 (0xFFFF) should give 0xFFFF_FFFF."""
    s = PowerPC601Simulator().execute(d_form(PO_ADDI, 3, 0, 0xFFFF) + HALT).final_state
    assert s.r3 == 0xFFFF_FFFF


# ── execute clears previous state ─────────────────────────────────────────────


def test_execute_resets_between_calls():
    sim = PowerPC601Simulator()
    sim.execute(d_form(PO_ADDI, 3, 0, 42) + HALT)
    assert sim.get_state().r3 == 42
    sim.execute(HALT)
    assert sim.get_state().r3 == 0  # reset between calls
