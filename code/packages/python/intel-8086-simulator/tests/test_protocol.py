"""Test suite: SIM00 protocol compliance for X86Simulator.

Verifies that X86Simulator satisfies the Simulator[X86State] contract:
  - Construction and initial state
  - reset() behaviour
  - load() byte-writing and origin parameter
  - step() single-cycle execution and StepTrace fields
  - execute() full run with halted/max_steps paths
  - get_state() snapshot isolation
"""

from __future__ import annotations

import dataclasses

import pytest
from simulator_protocol import ExecutionResult, Simulator, StepTrace

from intel_8086_simulator import X86Simulator, X86State

# ── Helpers ───────────────────────────────────────────────────────────────────


def hlt() -> bytes:
    """HLT instruction."""
    return bytes([0xF4])


def mov_ax(val: int) -> bytes:
    """MOV AX, imm16."""
    return bytes([0xB8, val & 0xFF, (val >> 8) & 0xFF])


# ── Protocol structural check ─────────────────────────────────────────────────


class TestProtocolCompliance:
    """Verify X86Simulator satisfies the Simulator[X86State] protocol."""

    def test_isinstance_simulator_protocol(self):
        sim = X86Simulator()
        assert isinstance(sim, Simulator)

    def test_has_required_methods(self):
        sim = X86Simulator()
        assert callable(sim.reset)
        assert callable(sim.load)
        assert callable(sim.step)
        assert callable(sim.execute)
        assert callable(sim.get_state)

    def test_execute_returns_execution_result(self):
        sim = X86Simulator()
        result = sim.execute(hlt())
        assert isinstance(result, ExecutionResult)

    def test_get_state_returns_x86state(self):
        sim = X86Simulator()
        state = sim.get_state()
        assert isinstance(state, X86State)

    def test_step_returns_step_trace(self):
        sim = X86Simulator()
        sim.reset()
        sim.load(hlt())
        trace = sim.step()
        assert isinstance(trace, StepTrace)


# ── Construction ──────────────────────────────────────────────────────────────


class TestConstruction:
    """Fresh simulator should be in well-defined initial state."""

    def test_initial_registers_zero(self):
        sim = X86Simulator()
        state = sim.get_state()
        assert state.ax == 0
        assert state.bx == 0
        assert state.cx == 0
        assert state.dx == 0
        assert state.si == 0
        assert state.di == 0
        assert state.sp == 0
        assert state.bp == 0

    def test_initial_segment_registers_zero(self):
        sim = X86Simulator()
        state = sim.get_state()
        assert state.cs == 0
        assert state.ds == 0
        assert state.ss == 0
        assert state.es == 0

    def test_initial_ip_zero(self):
        sim = X86Simulator()
        assert sim.get_state().ip == 0

    def test_initial_flags_all_false(self):
        sim = X86Simulator()
        s = sim.get_state()
        assert s.cf is False
        assert s.pf is False
        assert s.af is False
        assert s.zf is False
        assert s.sf is False
        assert s.tf is False
        assert s.if_ is False
        assert s.df is False
        assert s.of is False

    def test_initial_not_halted(self):
        sim = X86Simulator()
        assert sim.get_state().halted is False

    def test_initial_memory_all_zeros(self):
        sim = X86Simulator()
        state = sim.get_state()
        assert len(state.memory) == 1_048_576
        assert all(b == 0 for b in state.memory)

    def test_initial_ports_all_zeros(self):
        sim = X86Simulator()
        state = sim.get_state()
        assert len(state.input_ports) == 256
        assert len(state.output_ports) == 256
        assert all(b == 0 for b in state.input_ports)
        assert all(b == 0 for b in state.output_ports)


# ── Reset ─────────────────────────────────────────────────────────────────────


class TestReset:
    """reset() must restore the machine to its initial power-on state."""

    def test_reset_clears_ax(self):
        sim = X86Simulator()
        sim._ax = 0x1234
        sim.reset()
        assert sim.get_state().ax == 0

    def test_reset_clears_all_gp_registers(self):
        sim = X86Simulator()
        sim._ax = 1; sim._bx = 2; sim._cx = 3; sim._dx = 4
        sim._si = 5; sim._di = 6; sim._sp = 7; sim._bp = 8
        sim.reset()
        s = sim.get_state()
        assert s.ax == s.bx == s.cx == s.dx == 0
        assert s.si == s.di == s.sp == s.bp == 0

    def test_reset_clears_segment_registers(self):
        sim = X86Simulator()
        sim._cs = 0x1000; sim._ds = 0x2000
        sim.reset()
        s = sim.get_state()
        assert s.cs == s.ds == s.ss == s.es == 0

    def test_reset_clears_ip(self):
        sim = X86Simulator()
        sim._ip = 0x200
        sim.reset()
        assert sim.get_state().ip == 0

    def test_reset_clears_flags(self):
        sim = X86Simulator()
        sim._cf = True; sim._zf = True; sim._of = True
        sim.reset()
        s = sim.get_state()
        assert s.cf is False
        assert s.zf is False
        assert s.of is False

    def test_reset_clears_halted(self):
        sim = X86Simulator()
        sim._halted = True
        sim.reset()
        assert sim.get_state().halted is False

    def test_reset_clears_memory(self):
        sim = X86Simulator()
        sim._mem[42] = 0xFF
        sim.reset()
        assert sim.get_state().memory[42] == 0

    def test_reset_after_execute(self):
        sim = X86Simulator()
        sim.execute(mov_ax(0x1234) + hlt())
        sim.reset()
        s = sim.get_state()
        assert s.ax == 0
        assert s.ip == 0
        assert s.halted is False
        assert s.cf is False


# ── Load ──────────────────────────────────────────────────────────────────────


class TestLoad:
    """load() must write bytes into physical memory at the given origin."""

    def test_load_writes_bytes_at_zero(self):
        sim = X86Simulator()
        sim.reset()
        sim.load(bytes([0xAA, 0xBB, 0xCC]))
        s = sim.get_state()
        assert s.memory[0] == 0xAA
        assert s.memory[1] == 0xBB
        assert s.memory[2] == 0xCC

    def test_load_origin_nonzero(self):
        sim = X86Simulator()
        sim.reset()
        sim.load(bytes([0x42]), origin=0x100)
        assert sim.get_state().memory[0x100] == 0x42
        assert sim.get_state().memory[0] == 0  # unaffected

    def test_load_does_not_reset_registers(self):
        sim = X86Simulator()
        sim._ax = 0x5678
        sim.load(hlt())
        assert sim._ax == 0x5678  # unchanged

    def test_load_stops_at_memory_boundary(self):
        sim = X86Simulator()
        sim.reset()
        # Writing 4 bytes starting at 0xFFFFE (last 2 bytes of 1MB)
        sim.load(bytes([0x01, 0x02, 0x03, 0x04]), origin=0xFFFFE)
        assert sim.get_state().memory[0xFFFFE] == 0x01
        assert sim.get_state().memory[0xFFFFF] == 0x02
        # Bytes 03/04 should be silently dropped (beyond 1MB)

    def test_load_does_not_touch_other_memory(self):
        sim = X86Simulator()
        sim.reset()
        sim._mem[500] = 0xDD
        sim.load(bytes([0x11]), origin=100)
        assert sim._mem[500] == 0xDD  # untouched
        assert sim._mem[100] == 0x11


# ── Step ──────────────────────────────────────────────────────────────────────


class TestStep:
    """step() must execute one instruction and return a valid StepTrace."""

    def test_step_hlt_halts(self):
        sim = X86Simulator()
        sim.reset()
        sim.load(hlt())
        sim.step()
        assert sim._halted is True

    def test_step_trace_pc_before(self):
        sim = X86Simulator()
        sim.reset()
        sim.load(hlt())
        trace = sim.step()
        assert trace.pc_before == 0  # IP was 0 when step started

    def test_step_trace_pc_after_hlt(self):
        sim = X86Simulator()
        sim.reset()
        sim.load(hlt())
        trace = sim.step()
        assert trace.pc_after == 1  # IP advanced past HLT

    def test_step_trace_mnemonic_hlt(self):
        sim = X86Simulator()
        sim.reset()
        sim.load(hlt())
        trace = sim.step()
        assert trace.mnemonic == "HLT"

    def test_step_trace_mnemonic_nop(self):
        sim = X86Simulator()
        sim.reset()
        sim.load(bytes([0x90, 0xF4]))  # NOP; HLT
        trace = sim.step()
        assert trace.mnemonic == "NOP"

    def test_step_trace_description_contains_ip(self):
        sim = X86Simulator()
        sim.reset()
        sim.load(hlt())
        trace = sim.step()
        # description should mention the IP/CS info
        assert "0000" in trace.description

    def test_step_raises_on_halted(self):
        sim = X86Simulator()
        sim.reset()
        sim.load(hlt())
        sim.step()  # halts
        with pytest.raises(RuntimeError):
            sim.step()

    def test_step_advances_ip_past_multibyte_instruction(self):
        sim = X86Simulator()
        sim.reset()
        sim.load(mov_ax(0) + hlt())  # 3-byte MOV + 1-byte HLT
        trace = sim.step()
        assert trace.pc_after == 3  # advanced past 3-byte MOV AX, imm16


# ── Execute ───────────────────────────────────────────────────────────────────


class TestExecute:
    """execute() must reset, load, run, and return correct ExecutionResult."""

    def test_execute_hlt_immediately_halts(self):
        sim = X86Simulator()
        result = sim.execute(hlt())
        assert result.halted is True
        assert result.steps == 1
        assert result.ok is True
        assert result.error is None

    def test_execute_returns_final_state(self):
        sim = X86Simulator()
        result = sim.execute(mov_ax(0x1234) + hlt())
        assert result.final_state.ax == 0x1234

    def test_execute_trace_length_matches_steps(self):
        prog = mov_ax(5) + bytes([0x40]) + hlt()  # MOV AX,5; INC AX; HLT
        sim = X86Simulator()
        result = sim.execute(prog)
        assert len(result.traces) == result.steps

    def test_execute_max_steps_exceeded(self):
        # Infinite loop: JMP short -2 (EB FE)
        prog = bytes([0xEB, 0xFE])  # JMP $ (jumps to itself)
        sim = X86Simulator()
        result = sim.execute(prog, max_steps=50)
        assert result.halted is False
        assert result.steps == 50
        assert result.error is not None
        assert "max_steps" in result.error

    def test_execute_resets_before_run(self):
        sim = X86Simulator()
        sim.execute(mov_ax(0x1234) + hlt())
        result2 = sim.execute(hlt())
        assert result2.final_state.ax == 0  # reset cleared AX

    def test_execute_ok_property(self):
        sim = X86Simulator()
        result = sim.execute(hlt())
        assert result.ok is True

    def test_execute_not_ok_on_max_steps(self):
        sim = X86Simulator()
        result = sim.execute(bytes([0xEB, 0xFE]), max_steps=10)
        assert result.ok is False


# ── get_state snapshot isolation ─────────────────────────────────────────────


class TestGetState:
    """get_state() must return an immutable snapshot, not a mutable view."""

    def test_snapshot_is_frozen(self):
        sim = X86Simulator()
        state = sim.get_state()
        assert dataclasses.is_dataclass(state)
        with pytest.raises(Exception):
            state.ax = 99  # type: ignore[misc]

    def test_snapshot_memory_is_tuple(self):
        sim = X86Simulator()
        state = sim.get_state()
        assert isinstance(state.memory, tuple)
        assert len(state.memory) == 1_048_576

    def test_snapshot_not_aliased_to_internal_memory(self):
        sim = X86Simulator()
        state_before = sim.get_state()
        sim._mem[0] = 0xFF
        assert state_before.memory[0] == 0  # snapshot unchanged
        state_after = sim.get_state()
        assert state_after.memory[0] == 0xFF

    def test_two_snapshots_independent(self):
        sim = X86Simulator()
        s1 = sim.get_state()
        sim._ax = 42
        s2 = sim.get_state()
        assert s1.ax == 0
        assert s2.ax == 42

    def test_snapshot_ports_are_tuples(self):
        sim = X86Simulator()
        state = sim.get_state()
        assert isinstance(state.input_ports, tuple)
        assert isinstance(state.output_ports, tuple)
        assert len(state.input_ports) == 256
        assert len(state.output_ports) == 256
