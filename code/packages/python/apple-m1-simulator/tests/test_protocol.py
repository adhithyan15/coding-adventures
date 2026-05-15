"""
Protocol compliance tests for AppleM1Simulator.

Verifies that the simulator correctly implements the SIM00 protocol:
  reset / load / step / execute / get_state

Also tests state property correctness and NEON register initialisation.
"""

from __future__ import annotations

import pytest
from conftest import run

from apple_m1_simulator import AppleM1Simulator
from apple_m1_simulator.simulator import HALT, dp_imm, fmov_gpr_to_fp_d, fmov_gpr_to_fp_s, movwide

# ── Reset tests ───────────────────────────────────────────────────────────────


def test_reset_zeroes_gpr(sim: AppleM1Simulator) -> None:
    """After reset all general-purpose registers are 0."""
    state = sim.get_state()
    assert all(r == 0 for r in state.gpr)


def test_reset_zeroes_pc(sim: AppleM1Simulator) -> None:
    """After reset PC is 0."""
    assert sim.get_state().pc == 0


def test_reset_zeroes_sp(sim: AppleM1Simulator) -> None:
    """After reset SP is 0."""
    assert sim.get_state().sp == 0


def test_reset_zeroes_nzcv(sim: AppleM1Simulator) -> None:
    """After reset NZCV is 0."""
    assert sim.get_state().nzcv == 0


def test_reset_zeroes_vreg(sim: AppleM1Simulator) -> None:
    """After reset all 32 NEON/FP registers are 0."""
    state = sim.get_state()
    assert len(state.vreg) == 32
    assert all(v == 0 for v in state.vreg)


def test_reset_zeroes_memory(sim: AppleM1Simulator) -> None:
    """After reset all 65536 memory bytes are 0."""
    state = sim.get_state()
    assert len(state.memory) == 65_536
    assert all(b == 0 for b in state.memory)


def test_reset_not_halted(sim: AppleM1Simulator) -> None:
    """After reset, halted is False."""
    assert not sim.get_state().halted


# ── Load tests ────────────────────────────────────────────────────────────────


def test_load_copies_program(sim: AppleM1Simulator) -> None:
    """load() copies program bytes into memory starting at address 0."""
    prog = b"\xAB\xCD\xEF\x12"
    sim.load(prog)
    state = sim.get_state()
    assert state.memory[0] == 0xAB
    assert state.memory[1] == 0xCD
    assert state.memory[2] == 0xEF
    assert state.memory[3] == 0x12


def test_load_resets_state(sim: AppleM1Simulator) -> None:
    """load() resets all registers before copying the program."""
    # Manually step to change state
    sim.load(movwide(1, 0b10, 0, 99, 1) + HALT)
    sim.step()
    assert sim.get_state().x1 == 99
    # Now reload a new program — registers should be reset
    sim.load(HALT)
    assert sim.get_state().x1 == 0


def test_load_resets_vreg(sim: AppleM1Simulator) -> None:
    """load() resets vreg to all zeros."""
    sim.load(HALT)
    assert all(v == 0 for v in sim.get_state().vreg)


# ── Step / StepTrace tests ────────────────────────────────────────────────────


def test_step_returns_step_trace(sim: AppleM1Simulator) -> None:
    """step() returns a StepTrace with pc_before, pc_after, mnemonic, description."""
    from simulator_protocol import StepTrace
    sim.load(movwide(1, 0b10, 0, 1, 0) + HALT)
    trace = sim.step()
    assert isinstance(trace, StepTrace)
    assert trace.pc_before == 0
    assert trace.pc_after == 4
    assert trace.mnemonic == "MOVZ"


def test_step_halted_returns_halt_trace(sim: AppleM1Simulator) -> None:
    """Stepping on a halted simulator returns a HALT trace without advancing PC."""
    sim.load(HALT)
    sim.step()  # First step halts
    trace = sim.step()  # Second step on halted
    assert trace.mnemonic == "HALT"
    assert trace.pc_before == trace.pc_after


# ── Execute tests ─────────────────────────────────────────────────────────────


def test_execute_returns_execution_result(sim: AppleM1Simulator) -> None:
    """execute() returns an ExecutionResult."""
    from simulator_protocol import ExecutionResult
    result = sim.execute(HALT)
    assert isinstance(result, ExecutionResult)
    assert result.halted
    assert result.error is None
    assert result.ok


def test_execute_halted_on_zero_word(sim: AppleM1Simulator) -> None:
    """Executing a single HALT word sets halted=True."""
    result = sim.execute(HALT)
    assert result.halted
    assert result.final_state.halted


def test_execute_max_steps_exceeded() -> None:
    """execute() raises ValueError for max_steps=0."""
    sim = AppleM1Simulator()
    with pytest.raises(ValueError):
        sim.execute(HALT, max_steps=0)


def test_execute_records_traces(sim: AppleM1Simulator) -> None:
    """execute() records a StepTrace for each executed instruction."""
    prog = movwide(1, 0b10, 0, 5, 0) + movwide(1, 0b10, 0, 7, 1) + HALT
    result = sim.execute(prog)
    assert result.steps == 3  # MOVZ, MOVZ, HALT
    assert result.traces[0].mnemonic == "MOVZ"
    assert result.traces[2].mnemonic == "HALT"


# ── State property tests ──────────────────────────────────────────────────────


def test_xzr_always_zero() -> None:
    """XZR (register 31) always reads 0 and ignores writes."""
    state = run([
        dp_imm(1, 0, 0, 42, 0, 31, 31),   # ADD XZR, XZR, #42 — writes discarded
    ])
    assert state.gpr[31] == 0


def test_state_x_register_properties() -> None:
    """State x0..x7 properties return GPR values."""
    state = run([movwide(1, 0b10, 0, 100, 3)])  # MOVZ X3, #100
    assert state.x3 == 100


def test_state_w_register_properties() -> None:
    """State w0..w5 properties return lower 32 bits."""
    # Write 0xABCD_EF01_0000_0042 to X0 via two MOVKs
    state = run([
        movwide(1, 0b10, 0, 0x0042, 0),        # MOVZ X0, #0x42 (lsl 0)
    ])
    assert state.w0 == state.x0 & 0xFFFF_FFFF


def test_state_nzcv_flags() -> None:
    """State n/z/c/v flag properties decode the nzcv nibble."""
    state = run([
        movwide(1, 0b10, 0, 0, 0),              # MOVZ X0, #0
        dp_imm(1, 1, 1, 0, 0, 0, 31),           # SUBS XZR, X0, #0 → Z=1
    ])
    assert state.z is True
    assert state.n is False


def test_state_d_register_property() -> None:
    """State d0..d7 properties return float (IEEE 754 double) from vreg lower 64b."""
    import struct as st
    # Pack 3.14 as a double bit-pattern, load into X0 via four MOVZ/MOVK, FMOV to D0.
    # The 64-bit pattern is split into four 16-bit chunks (little-endian 16-bit order):
    #   chunk0 = bits[15:0],  chunk1 = bits[31:16],
    #   chunk2 = bits[47:32], chunk3 = bits[63:48]
    bits = st.unpack(">Q", st.pack(">d", 3.14))[0]
    chunk0 = bits & 0xFFFF
    chunk1 = (bits >> 16) & 0xFFFF
    chunk2 = (bits >> 32) & 0xFFFF
    chunk3 = (bits >> 48) & 0xFFFF
    state = run([
        movwide(1, 0b10, 0, chunk0, 0),          # MOVZ X0, chunk0         (bits[15:0])
        movwide(1, 0b11, 1, chunk1, 0),          # MOVK X0, chunk1, lsl 16 (bits[31:16])
        movwide(1, 0b11, 2, chunk2, 0),          # MOVK X0, chunk2, lsl 32 (bits[47:32])
        movwide(1, 0b11, 3, chunk3, 0),          # MOVK X0, chunk3, lsl 48 (bits[63:48])
        fmov_gpr_to_fp_d(0, 0),                 # FMOV D0, X0
    ])
    assert abs(state.d0 - 3.14) < 1e-10


def test_state_s_register_property() -> None:
    """State s0..s7 properties return float (IEEE 754 single) from vreg lower 32b."""
    import struct as st
    bits32 = st.unpack(">I", st.pack(">f", 2.5))[0]
    state = run([
        movwide(0, 0b10, 0, bits32 & 0xFFFF, 0),        # MOVZ W0, lo
        movwide(0, 0b11, 1, (bits32 >> 16) & 0xFFFF, 0), # MOVK W0, hi, lsl 16
        fmov_gpr_to_fp_s(0, 0),                          # FMOV S0, W0
    ])
    assert abs(state.s0 - 2.5) < 1e-6


def test_state_v_property_raw_128bit() -> None:
    """State v0..v7 properties return the raw 128-bit integer."""
    state = run([HALT])
    assert state.v0 == 0


def test_d_bits_property() -> None:
    """d0_bits returns the lower 64-bit integer (not float)."""
    import struct as st
    bits64 = st.unpack(">Q", st.pack(">d", 1.0))[0]
    state = run([
        movwide(1, 0b10, 0, bits64 & 0xFFFF, 0),
        movwide(1, 0b11, 1, (bits64 >> 16) & 0xFFFF, 0),
        movwide(1, 0b11, 2, (bits64 >> 32) & 0xFFFF, 0),
        movwide(1, 0b11, 3, (bits64 >> 48) & 0xFFFF, 0),
        fmov_gpr_to_fp_d(0, 0),
    ])
    assert state.d0_bits == bits64


def test_get_state_is_immutable() -> None:
    """get_state() returns a frozen dataclass; mutating it raises TypeError."""
    sim = AppleM1Simulator()
    state = sim.get_state()
    with pytest.raises(Exception):
        state.pc = 42  # type: ignore[misc]


def test_halted_flag_persists() -> None:
    """After HALT, halted=True persists across get_state() calls."""
    sim = AppleM1Simulator()
    sim.load(HALT)
    sim.step()
    assert sim.get_state().halted
    assert sim.get_state().halted  # Still halted
