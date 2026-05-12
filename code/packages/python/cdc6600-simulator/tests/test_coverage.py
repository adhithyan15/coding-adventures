"""Edge-case and coverage tests for the CDC 6600 simulator."""

import pytest

from cdc6600_simulator import HALT, CDC6600Simulator, long_instr, short_instr
from cdc6600_simulator.simulator import (
    F_IBBP,
    F_IXXP,
    F_JMP,
    F_LDBI,
    F_LDXI,
    F_STX,
    F_TBX,
)
from cdc6600_simulator.state import MASK18, MASK60, MEMORY_WORDS


def preset(sim, *, x=None, a=None, b=None):
    s = sim.get_state()
    nx = list(s.x)
    na = list(s.a)
    nb = list(s.b)
    if x:
        for idx, val in x.items():
            nx[idx] = val & MASK60
    if a:
        for idx, val in a.items():
            na[idx] = val & MASK18
    if b:
        for idx, val in b.items():
            if idx != 0:
                nb[idx] = val & MASK18
    from cdc6600_simulator.state import CDC6600State
    sim._state = CDC6600State(
        p=s.p, x=tuple(nx), a=tuple(na), b=tuple(nb),
        memory=s.memory, halted=s.halted,
    )


# ── B0 hardwired zero ─────────────────────────────────────────────────────────

def test_b0_reads_as_zero():
    # LDBI B0, 99 should not change B0
    s_after = CDC6600Simulator().execute(
        long_instr(F_LDBI, 0, 0, 99) + HALT
    ).final_state
    assert s_after.b0 == 0


def test_ibbp_into_b0_ignored():
    # IBBP B0, B1, B2 — write to B0 should be silently discarded
    sim = CDC6600Simulator()
    prog = short_instr(F_IBBP, 0, 1, 2) + HALT
    sim.load(prog)
    preset(sim, b={1: 10, 2: 5})
    result = sim.execute(prog)
    assert result.final_state.b0 == 0


def test_tbx_into_b0_ignored():
    # TBX B0, X1 — write to B0 should be silently discarded
    sim = CDC6600Simulator()
    prog = short_instr(F_TBX, 0, 1, 0) + HALT
    sim.load(prog)
    preset(sim, x={1: 42})
    result = sim.execute(prog)
    assert result.final_state.b0 == 0


# ── HALT on all-zeros parcel ──────────────────────────────────────────────────

def test_halt_parcel_stops_execution():
    # An all-zeros program halts immediately at P=0
    sim = CDC6600Simulator()
    result = sim.execute(b"\x00" * 8)
    assert result.halted


def test_halt_after_instructions():
    # Explicit HALT after one instruction
    prog = long_instr(F_LDXI, 1, 0, 5) + HALT
    sim = CDC6600Simulator()
    result = sim.execute(prog)
    assert result.halted
    assert result.final_state.x1 == 5


# ── 60-bit arithmetic mask ────────────────────────────────────────────────────

def test_ixxp_stays_60bit():
    # MASK60 + 1 should wrap to 0 (not overflow into 61st bit)
    sim = CDC6600Simulator()
    prog = short_instr(F_IXXP, 3, 1, 2) + HALT
    sim.load(prog)
    preset(sim, x={1: MASK60, 2: 1})
    result = sim.execute(prog)
    assert result.final_state.x3 == 0
    # Ensure the result fits in 60 bits
    assert result.final_state.x3 <= MASK60


def test_ixxp_never_exceeds_mask60():
    sim = CDC6600Simulator()
    prog = short_instr(F_IXXP, 3, 1, 2) + HALT
    sim.load(prog)
    preset(sim, x={1: MASK60, 2: MASK60})
    result = sim.execute(prog)
    v = result.final_state.x3
    assert v <= MASK60


# ── A/B register 18-bit mask ──────────────────────────────────────────────────

def test_b_register_wraps_18bit():
    sim = CDC6600Simulator()
    prog = short_instr(F_IBBP, 1, 2, 3) + HALT
    sim.load(prog)
    preset(sim, b={2: MASK18, 3: 1})
    result = sim.execute(prog)
    assert result.final_state.b1 == 0


def test_ldai_max_18bit():
    sim = CDC6600Simulator()
    result = sim.execute(long_instr(F_LDXI, 0, 0, 0) + long_instr(F_LDBI, 1, 0, MASK18) + HALT)
    assert result.final_state.b1 == MASK18


# ── max_steps guard ───────────────────────────────────────────────────────────

def test_max_steps_exceeded_returns_error():
    # JMP 0 forever — must stop after max_steps
    sim = CDC6600Simulator()
    prog = long_instr(F_JMP, 0, 0, 0)   # jump to parcel 0
    result = sim.execute(prog, max_steps=5)
    assert not result.ok
    assert result.error is not None
    assert "max_steps" in result.error


def test_max_steps_steps_count():
    sim = CDC6600Simulator()
    prog = long_instr(F_JMP, 0, 0, 0)
    result = sim.execute(prog, max_steps=7)
    assert result.steps == 7


# ── Memory bounds ─────────────────────────────────────────────────────────────

def test_stx_out_of_bounds():
    from cdc6600_simulator.simulator import F_LDAI
    sim = CDC6600Simulator()
    prog = (
        long_instr(F_LDAI, 1, 0, MEMORY_WORDS) +   # A1 = 4096 (out of bounds)
        long_instr(F_STX, 1, 2, 0) +                # STX: mem[A1+0] → error
        HALT
    )
    result = sim.execute(prog)
    assert not result.ok


# ── State immutability ────────────────────────────────────────────────────────

def test_frozen_state_x_immutable():
    sim = CDC6600Simulator()
    s = sim.get_state()
    with pytest.raises((AttributeError, TypeError)):
        s.x = (99,) * 8  # type: ignore[misc]


def test_frozen_state_p_immutable():
    sim = CDC6600Simulator()
    s = sim.get_state()
    with pytest.raises((AttributeError, TypeError)):
        s.p = 999  # type: ignore[misc]


def test_snapshot_not_modified_by_step():
    sim = CDC6600Simulator()
    sim.load(long_instr(F_LDXI, 1, 0, 42) + HALT)
    snap = sim.get_state()
    x1_before = snap.x1
    sim.step()
    # snap is frozen; sim has advanced but snap holds the old value
    assert snap.x1 == x1_before
    assert sim.get_state().x1 == 42


# ── Parcel packing / memory word layout ──────────────────────────────────────

def test_load_packs_parcels_into_word():
    # Verify that the first word contains what we loaded
    sim = CDC6600Simulator()
    # Encode LDXI X1,5 (30-bit = 2 parcels) then two HALT parcels
    prog = long_instr(F_LDXI, 1, 0, 5) + HALT + HALT
    sim.load(prog)
    s = sim.get_state()
    # Word 0 should be non-zero (contains the LDXI instruction)
    assert s.memory[0] != 0


def test_load_second_word_is_halt():
    # Program exactly 8 bytes → word 0 used, word 1 should be zero
    sim = CDC6600Simulator()
    prog = long_instr(F_LDXI, 1, 0, 7) + HALT + HALT   # exactly 8 bytes
    sim.load(prog)
    s = sim.get_state()
    assert s.memory[1] == 0


# ── Already-halted step ───────────────────────────────────────────────────────

def test_step_when_already_halted():
    sim = CDC6600Simulator()
    sim.execute(HALT)
    trace = sim.step()
    assert "HALT" in trace.mnemonic


# ── Convenience properties ────────────────────────────────────────────────────

def test_state_convenience_x_properties():
    sim = CDC6600Simulator()
    prog = b"".join(
        long_instr(F_LDXI, i, 0, i * 10)
        for i in range(8)
    ) + HALT
    result = sim.execute(prog)
    s = result.final_state
    assert s.x0 == 0
    assert s.x1 == 10
    assert s.x7 == 70


def test_state_convenience_b_properties():
    sim = CDC6600Simulator()
    prog = b"".join(
        long_instr(F_LDBI, i, 0, i + 1) if i != 0 else b""
        for i in range(8)
    ) + HALT
    result = sim.execute(prog)
    s = result.final_state
    assert s.b0 == 0
    assert s.b1 == 2
    assert s.b7 == 8
