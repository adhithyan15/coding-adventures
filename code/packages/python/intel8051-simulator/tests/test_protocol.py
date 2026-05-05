"""SIM00 protocol compliance tests for I8051Simulator.

Verifies that I8051Simulator implements the Simulator[I8051State] protocol
correctly: reset(), load(), step(), execute(), get_state() all behave as
specified.
"""

from __future__ import annotations

import pytest
from simulator_protocol import ExecutionResult, Simulator, StepTrace

from intel8051_simulator import I8051Simulator, I8051State
from intel8051_simulator.state import SFR_ACC, SFR_P0, SFR_P1, SFR_P2, SFR_P3, SFR_SP

# ── Encoding helpers ──────────────────────────────────────────────────────────

HALT = bytes([0xA5])   # HALT sentinel
NOP  = bytes([0x00])   # NOP


def MOV_A_IMM(v: int) -> bytes:
    """MOV A, #v — load immediate into accumulator."""
    return bytes([0x74, v])


# ── Protocol compliance ───────────────────────────────────────────────────────

class TestProtocolCompliance:
    """I8051Simulator must satisfy Simulator[I8051State] structurally."""

    def test_isinstance_simulator(self):
        sim = I8051Simulator()
        assert isinstance(sim, Simulator)

    def test_has_all_protocol_methods(self):
        sim = I8051Simulator()
        assert callable(sim.reset)
        assert callable(sim.load)
        assert callable(sim.step)
        assert callable(sim.execute)
        assert callable(sim.get_state)

    def test_execute_returns_execution_result(self):
        sim = I8051Simulator()
        result = sim.execute(HALT)
        assert isinstance(result, ExecutionResult)

    def test_step_returns_step_trace(self):
        sim = I8051Simulator()
        sim.load(NOP + HALT)
        trace = sim.step()
        assert isinstance(trace, StepTrace)

    def test_get_state_returns_i8051state(self):
        sim = I8051Simulator()
        state = sim.get_state()
        assert isinstance(state, I8051State)


# ── Reset ─────────────────────────────────────────────────────────────────────

class TestReset:
    """reset() must return CPU to well-defined initial state."""

    def test_reset_pc_is_zero(self):
        sim = I8051Simulator()
        sim._pc = 0x1234
        sim.reset()
        assert sim._pc == 0x0000

    def test_reset_sp_is_0x07(self):
        sim = I8051Simulator()
        sim._iram[SFR_SP] = 0xFF
        sim.reset()
        assert sim._iram[SFR_SP] == 0x07

    def test_reset_acc_is_zero(self):
        sim = I8051Simulator()
        sim._iram[SFR_ACC] = 0xAB
        sim.reset()
        assert sim._iram[SFR_ACC] == 0x00

    def test_reset_psw_is_zero(self):
        sim = I8051Simulator()
        sim._iram[0xD0] = 0xFF
        sim.reset()
        assert sim._iram[0xD0] == 0x00

    def test_reset_ports_are_0xff(self):
        sim = I8051Simulator()
        sim._iram[SFR_P0] = 0x00
        sim._iram[SFR_P1] = 0x00
        sim.reset()
        assert sim._iram[SFR_P0] == 0xFF
        assert sim._iram[SFR_P1] == 0xFF
        assert sim._iram[SFR_P2] == 0xFF
        assert sim._iram[SFR_P3] == 0xFF

    def test_reset_clears_halted(self):
        sim = I8051Simulator()
        sim._halted = True
        sim.reset()
        assert not sim._halted

    def test_reset_iram_zeroed(self):
        sim = I8051Simulator()
        sim._iram[0x30] = 0xAB
        sim.reset()
        assert sim._iram[0x30] == 0x00

    def test_reset_via_get_state(self):
        sim = I8051Simulator()
        state = sim.get_state()
        assert state.pc == 0x0000
        assert state.sp == 0x07
        assert state.psw == 0x00
        assert not state.halted


# ── Load ──────────────────────────────────────────────────────────────────────

class TestLoad:
    """load() must reset and copy bytes to code memory at 0x0000."""

    def test_load_places_bytes_at_0(self):
        sim = I8051Simulator()
        prog = bytes([0xAB, 0xCD, 0xEF])
        sim.load(prog)
        assert sim._code[0] == 0xAB
        assert sim._code[1] == 0xCD
        assert sim._code[2] == 0xEF

    def test_load_sets_pc_to_zero(self):
        sim = I8051Simulator()
        sim.load(HALT)
        assert sim._pc == 0x0000

    def test_load_raises_on_overflow(self):
        sim = I8051Simulator()
        with pytest.raises(ValueError):
            sim.load(bytes(65537))

    def test_load_resets_before_loading(self):
        sim = I8051Simulator()
        sim._iram[SFR_ACC] = 0xAB
        sim.load(HALT)
        assert sim._iram[SFR_ACC] == 0x00

    def test_load_exactly_64k_ok(self):
        sim = I8051Simulator()
        sim.load(bytes(65536))   # should not raise


# ── Execute ───────────────────────────────────────────────────────────────────

class TestExecute:
    """execute() must run to HALT and populate ExecutionResult correctly."""

    def test_execute_halt_immediately(self):
        sim = I8051Simulator()
        result = sim.execute(HALT)
        assert result.ok
        assert result.halted
        assert result.error is None
        assert result.steps == 1

    def test_execute_returns_traces(self):
        sim = I8051Simulator()
        result = sim.execute(HALT)
        assert len(result.traces) == 1
        assert result.traces[0].mnemonic == "HALT"

    def test_execute_max_steps_exceeded(self):
        # SJMP -2 loops forever (offset = 0xFE → signed = -2 → branch back to self)
        loop = bytes([0x80, 0xFE])   # SJMP rel=-2 (infinite loop)
        result = I8051Simulator().execute(loop + HALT, max_steps=10)
        assert not result.ok
        assert result.steps == 10
        assert "max_steps" in result.error

    def test_execute_sets_final_state(self):
        sim = I8051Simulator()
        result = sim.execute(HALT)
        assert isinstance(result.final_state, I8051State)
        assert result.final_state.halted

    def test_execute_resets_before_running(self):
        sim = I8051Simulator()
        sim._iram[SFR_ACC] = 0xDE
        result = sim.execute(HALT)
        assert result.final_state.acc == 0

    def test_execute_nop_then_halt(self):
        result = I8051Simulator().execute(NOP + HALT)
        assert result.ok
        assert result.steps == 2
        assert result.traces[0].mnemonic == "NOP"
        assert result.traces[1].mnemonic == "HALT"


# ── GetState ──────────────────────────────────────────────────────────────────

class TestGetState:
    """get_state() must return an immutable snapshot."""

    def test_get_state_is_frozen(self):
        import dataclasses
        sim = I8051Simulator()
        state = sim.get_state()
        with pytest.raises((dataclasses.FrozenInstanceError, TypeError)):
            state.pc = 0xFF   # type: ignore[misc]

    def test_get_state_snapshot_not_mutated_by_step(self):
        sim = I8051Simulator()
        sim.load(NOP + HALT)
        state_before = sim.get_state()
        sim.step()
        state_after = sim.get_state()
        assert state_before.pc == 0x0000
        assert state_after.pc == 0x0001

    def test_get_state_iram_is_tuple(self):
        sim = I8051Simulator()
        state = sim.get_state()
        assert isinstance(state.iram, tuple)
        assert len(state.iram) == 256

    def test_get_state_code_is_tuple(self):
        sim = I8051Simulator()
        state = sim.get_state()
        assert isinstance(state.code, tuple)
        assert len(state.code) == 65536

    def test_get_state_xdata_is_tuple(self):
        sim = I8051Simulator()
        state = sim.get_state()
        assert isinstance(state.xdata, tuple)
        assert len(state.xdata) == 65536


# ── Step ──────────────────────────────────────────────────────────────────────

class TestStep:
    """step() must advance PC and return correct StepTrace."""

    def test_step_nop_advances_pc_by_1(self):
        sim = I8051Simulator()
        sim.load(NOP + HALT)
        assert sim._pc == 0x0000
        sim.step()
        assert sim._pc == 0x0001

    def test_step_on_halted_cpu_is_noop(self):
        sim = I8051Simulator()
        sim.load(HALT)
        sim.step()    # executes HALT
        assert sim._halted
        pc = sim._pc
        trace = sim.step()   # should be no-op
        assert sim._pc == pc
        assert trace.mnemonic == "HALT"

    def test_step_trace_pc_before_after(self):
        sim = I8051Simulator()
        sim.load(NOP + HALT)
        trace = sim.step()
        assert trace.pc_before == 0x0000
        assert trace.pc_after  == 0x0001

    def test_step_2byte_instruction_advances_pc_by_2(self):
        sim = I8051Simulator()
        sim.load(bytes([0x74, 0x42]) + HALT)   # MOV A, #0x42
        sim.step()
        assert sim._pc == 0x0002
