"""Test suite: SIM00 protocol compliance for BabySimulator.

Verifies that BabySimulator satisfies the Simulator[BabyState] contract:
  - Construction and initial state
  - reset() behaviour
  - load() byte→word conversion and origin parameter
  - step() single-cycle execution and StepTrace fields
  - execute() full run with halted/max_steps paths
  - get_state() snapshot isolation
"""

from __future__ import annotations

import dataclasses

import pytest
from simulator_protocol import ExecutionResult, Simulator, StepTrace

from manchester_baby_simulator import BabySimulator, BabyState

# ── Helpers ───────────────────────────────────────────────────────────────────

def w(value: int) -> bytes:
    """Encode a 32-bit word as 4 little-endian bytes."""
    return (value & 0xFFFFFFFF).to_bytes(4, "little")


# Instruction encodings
def jmp(s: int) -> int:
    return (0b000 << 13) | (s & 0x1F)


def jrp(s: int) -> int:
    return (0b001 << 13) | (s & 0x1F)


def ldn(s: int) -> int:
    return (0b010 << 13) | (s & 0x1F)


def sto(s: int) -> int:
    return (0b011 << 13) | (s & 0x1F)


def sub(s: int) -> int:
    return (0b100 << 13) | (s & 0x1F)


STP = 0b111 << 13


# ── Protocol structural check ─────────────────────────────────────────────────

class TestProtocolCompliance:
    """Verify BabySimulator satisfies the Simulator[BabyState] protocol."""

    def test_isinstance_simulator_protocol(self):
        sim = BabySimulator()
        assert isinstance(sim, Simulator)

    def test_has_required_methods(self):
        sim = BabySimulator()
        assert callable(sim.reset)
        assert callable(sim.load)
        assert callable(sim.step)
        assert callable(sim.execute)
        assert callable(sim.get_state)

    def test_execute_returns_execution_result(self):
        sim = BabySimulator()
        result = sim.execute(w(STP))
        assert isinstance(result, ExecutionResult)

    def test_get_state_returns_baby_state(self):
        sim = BabySimulator()
        state = sim.get_state()
        assert isinstance(state, BabyState)

    def test_step_returns_step_trace(self):
        sim = BabySimulator()
        sim.reset()
        sim.load(w(STP))
        trace = sim.step()
        assert isinstance(trace, StepTrace)


# ── Construction ──────────────────────────────────────────────────────────────

class TestConstruction:
    """Fresh simulator should be in well-defined initial state."""

    def test_initial_store_all_zeros(self):
        sim = BabySimulator()
        state = sim.get_state()
        assert len(state.store) == 32
        assert all(v == 0 for v in state.store)

    def test_initial_accumulator_zero(self):
        sim = BabySimulator()
        assert sim.get_state().accumulator == 0

    def test_initial_ci_is_31(self):
        # CI starts at 31 so the first step() increments to 0
        sim = BabySimulator()
        assert sim.get_state().ci == 31

    def test_initial_not_halted(self):
        sim = BabySimulator()
        assert sim.get_state().halted is False


# ── Reset ─────────────────────────────────────────────────────────────────────

class TestReset:
    """reset() must restore the machine to its initial power-on state."""

    def test_reset_clears_store(self):
        sim = BabySimulator()
        sim._store[5] = 0xDEADBEEF
        sim.reset()
        assert sim.get_state().store[5] == 0

    def test_reset_clears_accumulator(self):
        sim = BabySimulator()
        sim._acc = 0x12345678
        sim.reset()
        assert sim.get_state().accumulator == 0

    def test_reset_sets_ci_to_31(self):
        sim = BabySimulator()
        sim._ci = 15
        sim.reset()
        assert sim.get_state().ci == 31

    def test_reset_clears_halted(self):
        sim = BabySimulator()
        sim._halted = True
        sim.reset()
        assert sim.get_state().halted is False

    def test_reset_after_execute(self):
        sim = BabySimulator()
        sim.execute(w(STP))
        sim.reset()
        state = sim.get_state()
        assert state.accumulator == 0
        assert state.ci == 31
        assert state.halted is False
        assert all(v == 0 for v in state.store)


# ── Load ──────────────────────────────────────────────────────────────────────

class TestLoad:
    """load() must correctly decode little-endian words into store."""

    def test_load_single_word_little_endian(self):
        sim = BabySimulator()
        sim.reset()
        sim.load(bytes([0x78, 0x56, 0x34, 0x12]))   # 0x12345678
        assert sim._store[0] == 0x12345678

    def test_load_multiple_words(self):
        sim = BabySimulator()
        sim.reset()
        data = bytes([
            0x01, 0x00, 0x00, 0x00,   # word 0 = 1
            0x02, 0x00, 0x00, 0x00,   # word 1 = 2
            0x03, 0x00, 0x00, 0x00,   # word 2 = 3
        ])
        sim.load(data)
        assert sim._store[0] == 1
        assert sim._store[1] == 2
        assert sim._store[2] == 3

    def test_load_origin_nonzero(self):
        sim = BabySimulator()
        sim.reset()
        sim.load(w(0xCAFEBABE), origin=10)
        assert sim._store[10] == 0xCAFEBABE
        assert sim._store[0] == 0     # unaffected

    def test_load_does_not_touch_other_words(self):
        sim = BabySimulator()
        sim.reset()
        sim._store[7] = 999
        sim.load(w(42), origin=3)
        assert sim._store[7] == 999   # untouched
        assert sim._store[3] == 42

    def test_load_value_42(self):
        # 42 = 0x0000002A
        sim = BabySimulator()
        sim.reset()
        sim.load(bytes([0x2A, 0x00, 0x00, 0x00]))
        assert sim._store[0] == 42

    def test_load_max_word(self):
        sim = BabySimulator()
        sim.reset()
        sim.load(bytes([0xFF, 0xFF, 0xFF, 0xFF]))
        assert sim._store[0] == 0xFFFFFFFF

    def test_load_ignores_incomplete_last_word(self):
        # 5 bytes: one complete word + 1 leftover byte (ignored)
        sim = BabySimulator()
        sim.reset()
        sim.load(bytes([0x01, 0x00, 0x00, 0x00, 0xFF]))
        assert sim._store[0] == 1
        assert sim._store[1] == 0     # second word not written

    def test_load_stops_at_store_boundary(self):
        # 40 words = 160 bytes; only 32 fit
        sim = BabySimulator()
        sim.reset()
        big = bytes([0xAA, 0x00, 0x00, 0x00] * 40)
        sim.load(big, origin=0)
        # All 32 lines should be 0xAA and no IndexError raised
        assert all(sim._store[i] == 0xAA for i in range(32))


# ── Step ──────────────────────────────────────────────────────────────────────

class TestStep:
    """step() must pre-increment CI, fetch, execute, and return a StepTrace."""

    def test_step_increments_ci_from_31_to_0(self):
        sim = BabySimulator()
        sim.reset()
        sim.load(w(STP))    # STP at line 0
        sim.step()
        assert sim._ci == 0

    def test_step_trace_pc_before_is_old_ci(self):
        sim = BabySimulator()
        sim.reset()
        sim.load(w(STP))
        trace = sim.step()
        assert trace.pc_before == 31    # CI was 31 before the step

    def test_step_trace_pc_after_is_new_ci(self):
        sim = BabySimulator()
        sim.reset()
        sim.load(w(STP))
        trace = sim.step()
        assert trace.pc_after == 0      # CI incremented to 0

    def test_step_trace_mnemonic_stp(self):
        sim = BabySimulator()
        sim.reset()
        sim.load(w(STP))
        trace = sim.step()
        assert trace.mnemonic == "STP"

    def test_step_trace_mnemonic_ldn(self):
        sim = BabySimulator()
        sim.reset()
        sim.load(w(ldn(0)) + w(STP))
        trace = sim.step()
        assert trace.mnemonic == "LDN 0"

    def test_step_raises_on_halted(self):
        sim = BabySimulator()
        sim.reset()
        sim.load(w(STP))
        sim.step()   # executes STP → halted=True
        with pytest.raises(RuntimeError):
            sim.step()

    def test_step_trace_description_contains_line(self):
        sim = BabySimulator()
        sim.reset()
        sim.load(w(STP))
        trace = sim.step()
        assert "line" in trace.description.lower()


# ── Execute ───────────────────────────────────────────────────────────────────

class TestExecute:
    """execute() must reset, load, run, and return correct ExecutionResult."""

    def test_execute_stp_immediately_halts(self):
        sim = BabySimulator()
        result = sim.execute(w(STP))
        assert result.halted is True
        assert result.steps == 1
        assert result.ok is True
        assert result.error is None

    def test_execute_returns_final_state(self):
        # LDN 0 then STP: A = −42
        prog = w(42) + w(ldn(0)) + w(STP)
        sim = BabySimulator()
        result = sim.execute(prog)
        assert result.final_state.acc_signed == -42

    def test_execute_trace_length_matches_steps(self):
        prog = w(42) + w(ldn(0)) + w(sto(1)) + w(STP)
        sim = BabySimulator()
        result = sim.execute(prog)
        assert len(result.traces) == result.steps

    def test_execute_max_steps_exceeded(self):
        # Tight infinite loop using JRP −1 displacement:
        # Line 0: JRP 1.  Store[1] = 0xFFFFFFFF (= −1 signed).
        # After pre-increment CI=0, JRP makes CI = 0 + (−1) = −1 = 31 (mod 32).
        # Next pre-increment: CI=0 again.  Loop forever.
        store = [0] * 32
        store[0] = jrp(1)
        store[1] = 0xFFFFFFFF    # signed displacement −1
        prog = b"".join(w(v) for v in store)
        sim = BabySimulator()
        result = sim.execute(prog, max_steps=50)
        assert result.halted is False
        assert result.steps == 50
        assert result.error is not None
        assert "max_steps" in result.error

    def test_execute_resets_before_run(self):
        # First run sets accumulator; second run (STP only) should reset it
        prog = w(99) + w(ldn(0)) + w(STP)
        sim = BabySimulator()
        sim.execute(prog)
        result2 = sim.execute(w(STP))
        assert result2.final_state.accumulator == 0

    def test_execute_ok_property(self):
        sim = BabySimulator()
        result = sim.execute(w(STP))
        assert result.ok is True

    def test_execute_not_ok_on_max_steps(self):
        store = [0] * 32
        store[0] = jrp(1)
        store[1] = 0xFFFFFFFF
        prog = b"".join(w(v) for v in store)
        sim = BabySimulator()
        result = sim.execute(prog, max_steps=10)
        assert result.ok is False


# ── get_state snapshot isolation ─────────────────────────────────────────────

class TestGetState:
    """get_state() must return an immutable snapshot, not a mutable view."""

    def test_snapshot_is_frozen(self):
        sim = BabySimulator()
        state = sim.get_state()
        assert dataclasses.is_dataclass(state)
        with pytest.raises(Exception):
            state.accumulator = 99  # type: ignore[misc]

    def test_snapshot_store_is_tuple(self):
        sim = BabySimulator()
        state = sim.get_state()
        assert isinstance(state.store, tuple)
        assert len(state.store) == 32

    def test_snapshot_not_aliased_to_internal_store(self):
        sim = BabySimulator()
        state_before = sim.get_state()
        sim._store[0] = 0xDEADBEEF
        assert state_before.store[0] == 0      # snapshot unchanged
        state_after = sim.get_state()
        assert state_after.store[0] == 0xDEADBEEF

    def test_two_snapshots_independent(self):
        sim = BabySimulator()
        s1 = sim.get_state()
        sim._acc = 42
        s2 = sim.get_state()
        assert s1.accumulator == 0
        assert s2.accumulator == 42
