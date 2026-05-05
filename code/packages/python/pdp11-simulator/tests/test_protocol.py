"""SIM00 protocol compliance tests for PDP11Simulator.

Verifies that PDP11Simulator implements the Simulator[PDP11State] protocol
correctly: reset(), load(), step(), execute(), get_state() all behave as
specified.
"""

from __future__ import annotations

import pytest
from simulator_protocol import ExecutionResult, Simulator, StepTrace

from pdp11_simulator import PDP11Simulator, PDP11State

# ── Encoding helpers ──────────────────────────────────────────────────────────

def _w(value: int) -> bytes:
    """Pack a 16-bit value as a little-endian word."""
    return bytes([value & 0xFF, (value >> 8) & 0xFF])

HALT = _w(0x0000)   # HALT instruction

# ── Protocol compliance ───────────────────────────────────────────────────────

class TestProtocolCompliance:
    """PDP11Simulator must satisfy Simulator[PDP11State] structurally."""

    def test_isinstance_simulator(self):
        sim = PDP11Simulator()
        assert isinstance(sim, Simulator)

    def test_has_all_protocol_methods(self):
        sim = PDP11Simulator()
        assert callable(sim.reset)
        assert callable(sim.load)
        assert callable(sim.step)
        assert callable(sim.execute)
        assert callable(sim.get_state)

    def test_execute_returns_execution_result(self):
        sim = PDP11Simulator()
        result = sim.execute(HALT)
        assert isinstance(result, ExecutionResult)

    def test_step_returns_step_trace(self):
        sim = PDP11Simulator()
        sim.load(HALT)
        trace = sim.step()
        assert isinstance(trace, StepTrace)

    def test_get_state_returns_pdp11state(self):
        sim = PDP11Simulator()
        state = sim.get_state()
        assert isinstance(state, PDP11State)


class TestReset:
    """reset() must return CPU to well-defined initial state."""

    def test_reset_clears_registers(self):
        sim = PDP11Simulator()
        sim._r[0] = 0xABCD
        sim._r[3] = 0x1234
        sim.reset()
        assert sim._r[0] == 0
        assert sim._r[3] == 0

    def test_reset_sets_sp(self):
        sim = PDP11Simulator()
        sim.reset()
        assert sim._r[6] == 0xF000  # SP = R6

    def test_reset_sets_pc(self):
        sim = PDP11Simulator()
        sim.reset()
        assert sim._r[7] == 0x1000  # PC = R7

    def test_reset_clears_psw(self):
        sim = PDP11Simulator()
        sim._psw = 0xFF
        sim.reset()
        assert sim._psw == 0

    def test_reset_clears_memory(self):
        sim = PDP11Simulator()
        sim._mem[0x1000] = 0xAB
        sim.reset()
        assert sim._mem[0x1000] == 0

    def test_reset_clears_halted(self):
        sim = PDP11Simulator()
        sim._halted = True
        sim.reset()
        assert not sim._halted

    def test_reset_state_via_get_state(self):
        sim = PDP11Simulator()
        state = sim.get_state()
        assert state.r[6] == 0xF000
        assert state.r[7] == 0x1000
        assert state.psw == 0
        assert not state.halted


class TestLoad:
    """load() must reset and copy the program to 0x1000."""

    def test_load_places_bytes_at_0x1000(self):
        sim = PDP11Simulator()
        prog = b'\xAB\xCD\xEF'
        sim.load(prog)
        assert sim._mem[0x1000] == 0xAB
        assert sim._mem[0x1001] == 0xCD
        assert sim._mem[0x1002] == 0xEF

    def test_load_sets_pc_to_0x1000(self):
        sim = PDP11Simulator()
        sim.load(b'\x00\x00')
        assert sim._r[7] == 0x1000

    def test_load_raises_on_overflow(self):
        sim = PDP11Simulator()
        with pytest.raises(ValueError):
            sim.load(bytes(65536))   # too large

    def test_load_returns_state_reflecting_program(self):
        sim = PDP11Simulator()
        sim.load(HALT)
        assert sim._mem[0x1000] == 0x00
        assert sim._mem[0x1001] == 0x00


class TestExecute:
    """execute() must run to HALT and populate ExecutionResult correctly."""

    def test_execute_halt_immediately(self):
        sim = PDP11Simulator()
        result = sim.execute(HALT)
        assert result.ok
        assert result.halted
        assert result.error is None
        assert result.steps == 1

    def test_execute_returns_traces(self):
        sim = PDP11Simulator()
        result = sim.execute(HALT)
        assert len(result.traces) == 1
        assert result.traces[0].mnemonic == "HALT"

    def test_execute_max_steps_exceeded(self):
        sim = PDP11Simulator()
        # BR 0 = infinite loop: opcode 0x01, offset 0xFF (-1 word → PC -= 2 → same instr)
        # Wait: BR offset=-1 means branch to PC - 2, which is the BR itself
        # Encoding: 0x01FE (opcode=0x01, offset=0xFE=-2? let's use 0xFF=-1)
        # EA = PC_after_fetch + 2*offset  where offset = sign_extend(0xFF, 8) = -1
        # PC after fetching BR word = 0x1002; EA = 0x1002 + 2*(-1) = 0x1000 → loops
        loop = _w(0x01FF)   # BR -1 (loops back to itself)
        result = sim.execute(loop + HALT, max_steps=10)
        assert not result.ok
        assert result.steps == 10
        assert "max_steps" in result.error

    def test_execute_sets_final_state(self):
        sim = PDP11Simulator()
        result = sim.execute(HALT)
        assert isinstance(result.final_state, PDP11State)
        assert result.final_state.halted

    def test_execute_resets_before_running(self):
        sim = PDP11Simulator()
        sim._r[0] = 0xDEAD
        result = sim.execute(HALT)
        assert result.final_state.r[0] == 0


class TestGetState:
    """get_state() must return an immutable snapshot."""

    def test_get_state_is_frozen(self):
        import dataclasses
        sim = PDP11Simulator()
        state = sim.get_state()
        with pytest.raises((dataclasses.FrozenInstanceError, TypeError)):
            state.psw = 0xFF  # type: ignore[misc]

    def test_get_state_snapshot_not_mutated_by_step(self):
        sim = PDP11Simulator()
        # NOP (0x00A0) then HALT
        sim.load(_w(0x00A0) + HALT)
        state_before = sim.get_state()
        sim.step()   # execute NOP
        state_after = sim.get_state()
        # The snapshot taken before step should be unchanged
        assert state_before.r[7] == 0x1000
        assert state_after.r[7] == 0x1002

    def test_get_state_memory_is_tuple(self):
        sim = PDP11Simulator()
        state = sim.get_state()
        assert isinstance(state.memory, tuple)
        assert len(state.memory) == 65536

    def test_get_state_registers_is_tuple(self):
        sim = PDP11Simulator()
        state = sim.get_state()
        assert isinstance(state.r, tuple)
        assert len(state.r) == 8


class TestStep:
    """step() must advance PC and return correct StepTrace."""

    def test_step_advances_pc_by_2(self):
        sim = PDP11Simulator()
        sim.load(_w(0x00A0) + HALT)   # NOP, HALT
        assert sim._r[7] == 0x1000
        sim.step()
        assert sim._r[7] == 0x1002

    def test_step_on_halted_cpu_is_noop(self):
        sim = PDP11Simulator()
        sim.load(HALT)
        sim.step()     # executes HALT
        assert sim._halted
        pc = sim._r[7]
        trace = sim.step()   # should be no-op
        assert sim._r[7] == pc
        assert trace.mnemonic == "HALT"

    def test_step_trace_pc_before_after(self):
        sim = PDP11Simulator()
        sim.load(_w(0x00A0) + HALT)
        trace = sim.step()
        assert trace.pc_before == 0x1000
        assert trace.pc_after  == 0x1002
