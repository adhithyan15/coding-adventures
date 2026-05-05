"""SIM00 protocol compliance tests for SPARCSimulator.

Verifies that SPARCSimulator implements the Simulator[SPARCState] protocol
correctly: reset(), load(), step(), execute(), get_state() all behave as
specified.
"""

from __future__ import annotations

import struct

import pytest
from simulator_protocol import ExecutionResult, Simulator, StepTrace

from sparc_v8_simulator import SPARCSimulator, SPARCState

# ── Encoding helpers ──────────────────────────────────────────────────────────

def w32(v: int) -> bytes:
    """Pack a 32-bit SPARC instruction as 4 big-endian bytes."""
    return struct.pack(">I", v & 0xFFFF_FFFF)


# HALT = ta 0  (op=2, rd=8/cond-always, op3=0x3A/Ticc, rs1=0, i=1, simm13=0)
HALT = w32(0x91D0_2000)

# NOP = SETHI 0, %g0  (op=0, rd=0, op2=4, imm22=0) = 0x01000000
NOP = w32(0x0100_0000)


def sethi(rd: int, imm22: int) -> bytes:
    """SETHI imm22, rd  — op=0, op2=4."""
    return w32((0x00 << 30) | (rd << 25) | (0x4 << 22) | (imm22 & 0x3FFFFF))


def alu_reg(op3: int, rd: int, rs1: int, rs2: int) -> bytes:
    """Format 3 ALU with register operand (i=0)."""
    return w32((0x2 << 30) | (rd << 25) | (op3 << 19) | (rs1 << 14) | rs2)


def alu_imm(op3: int, rd: int, rs1: int, simm13: int) -> bytes:
    """Format 3 ALU with immediate operand (i=1)."""
    return w32((0x2 << 30) | (rd << 25) | (op3 << 19) | (rs1 << 14) | (1 << 13) | (simm13 & 0x1FFF))


def add(rd: int, rs1: int, simm13: int) -> bytes:
    """ADD rd, rs1, simm13  — op3=0x00."""
    return alu_imm(0x00, rd, rs1, simm13)


# ── Protocol compliance ───────────────────────────────────────────────────────

class TestProtocolCompliance:
    """SPARCSimulator must satisfy Simulator[SPARCState] structurally."""

    def test_isinstance_simulator(self):
        sim = SPARCSimulator()
        assert isinstance(sim, Simulator)

    def test_has_all_protocol_methods(self):
        sim = SPARCSimulator()
        assert callable(sim.reset)
        assert callable(sim.load)
        assert callable(sim.step)
        assert callable(sim.execute)
        assert callable(sim.get_state)

    def test_execute_returns_execution_result(self):
        sim = SPARCSimulator()
        result = sim.execute(HALT)
        assert isinstance(result, ExecutionResult)

    def test_step_returns_step_trace(self):
        sim = SPARCSimulator()
        sim.load(NOP + HALT)
        trace = sim.step()
        assert isinstance(trace, StepTrace)

    def test_get_state_returns_sparc_state(self):
        sim = SPARCSimulator()
        state = sim.get_state()
        assert isinstance(state, SPARCState)


# ── Reset ─────────────────────────────────────────────────────────────────────

class TestReset:
    """reset() must return CPU to well-defined initial state."""

    def test_reset_pc_is_zero(self):
        sim = SPARCSimulator()
        sim._pc = 0x1234
        sim.reset()
        assert sim._pc == 0x0000

    def test_reset_npc_is_4(self):
        sim = SPARCSimulator()
        sim._npc = 0x5678
        sim.reset()
        assert sim._npc == 0x0004

    def test_reset_all_regs_zero(self):
        sim = SPARCSimulator()
        sim._regs[8] = 0xDEAD
        sim._regs[20] = 0xBEEF
        sim.reset()
        assert all(r == 0 for r in sim._regs)

    def test_reset_cwp_is_zero(self):
        sim = SPARCSimulator()
        sim._cwp = 2
        sim.reset()
        assert sim._cwp == 0

    def test_reset_condition_codes_cleared(self):
        sim = SPARCSimulator()
        sim._psr_n = True
        sim._psr_z = True
        sim._psr_v = True
        sim._psr_c = True
        sim.reset()
        assert not sim._psr_n
        assert not sim._psr_z
        assert not sim._psr_v
        assert not sim._psr_c

    def test_reset_y_zero(self):
        sim = SPARCSimulator()
        sim._y = 0xFFFF
        sim.reset()
        assert sim._y == 0

    def test_reset_memory_zeroed(self):
        sim = SPARCSimulator()
        sim._mem[0x100] = 0xAB
        sim.reset()
        assert sim._mem[0x100] == 0x00

    def test_reset_clears_halted(self):
        sim = SPARCSimulator()
        sim._halted = True
        sim.reset()
        assert not sim._halted

    def test_reset_via_get_state(self):
        sim = SPARCSimulator()
        state = sim.get_state()
        assert state.pc == 0x0000
        assert state.npc == 0x0004
        assert all(r == 0 for r in state.regs)
        assert state.cwp == 0
        assert not state.psr_n
        assert not state.psr_z
        assert not state.psr_v
        assert not state.psr_c
        assert state.y == 0
        assert not state.halted


# ── Load ──────────────────────────────────────────────────────────────────────

class TestLoad:
    """load() must reset and copy bytes to memory at 0x0000."""

    def test_load_places_bytes_at_0(self):
        sim = SPARCSimulator()
        prog = bytes([0x91, 0xD0, 0x20, 0x00])
        sim.load(prog)
        assert sim._mem[0] == 0x91
        assert sim._mem[1] == 0xD0
        assert sim._mem[2] == 0x20
        assert sim._mem[3] == 0x00

    def test_load_sets_pc_to_zero(self):
        sim = SPARCSimulator()
        sim.load(HALT)
        assert sim._pc == 0x0000

    def test_load_raises_on_overflow(self):
        sim = SPARCSimulator()
        with pytest.raises(ValueError):
            sim.load(bytes(65537))

    def test_load_resets_before_loading(self):
        sim = SPARCSimulator()
        sim._regs[8] = 0xAB
        sim.load(HALT)
        assert sim._regs[8] == 0x00

    def test_load_exactly_64k_ok(self):
        sim = SPARCSimulator()
        sim.load(bytes(65536))   # should not raise


# ── Execute ───────────────────────────────────────────────────────────────────

class TestExecute:
    """execute() must run to HALT and populate ExecutionResult correctly."""

    def test_execute_halt_immediately(self):
        sim = SPARCSimulator()
        result = sim.execute(HALT)
        assert result.ok
        assert result.halted
        assert result.error is None
        assert result.steps == 1

    def test_execute_returns_traces(self):
        sim = SPARCSimulator()
        result = sim.execute(HALT)
        assert len(result.traces) == 1
        assert result.traces[0].mnemonic == "HALT"

    def test_execute_max_steps_exceeded(self):
        # BA 0 (branch always to self) → infinite loop
        # BA: op=0, op2=2, cond=8 (always), disp22=0 → target = PC + 0 = PC
        loop = w32((0x0 << 30) | (0x8 << 25) | (0x2 << 22) | 0)
        result = SPARCSimulator().execute(loop + HALT, max_steps=10)
        assert not result.ok
        assert result.steps == 10
        assert "max_steps" in result.error

    def test_execute_sets_final_state(self):
        sim = SPARCSimulator()
        result = sim.execute(HALT)
        assert isinstance(result.final_state, SPARCState)
        assert result.final_state.halted

    def test_execute_resets_before_running(self):
        sim = SPARCSimulator()
        sim._regs[8] = 0xDE
        result = sim.execute(HALT)
        assert result.final_state.regs[8] == 0

    def test_execute_nop_then_halt(self):
        result = SPARCSimulator().execute(NOP + HALT)
        assert result.ok
        assert result.steps == 2
        assert result.traces[0].mnemonic == "NOP"
        assert result.traces[1].mnemonic == "HALT"


# ── GetState ──────────────────────────────────────────────────────────────────

class TestGetState:
    """get_state() must return an immutable snapshot."""

    def test_get_state_is_frozen(self):
        import dataclasses
        sim = SPARCSimulator()
        state = sim.get_state()
        with pytest.raises((dataclasses.FrozenInstanceError, TypeError)):
            state.pc = 0xFF   # type: ignore[misc]

    def test_get_state_snapshot_not_mutated_by_step(self):
        sim = SPARCSimulator()
        sim.load(NOP + HALT)
        state_before = sim.get_state()
        sim.step()
        state_after = sim.get_state()
        assert state_before.pc == 0x0000
        assert state_after.pc == 0x0004

    def test_get_state_regs_is_tuple(self):
        sim = SPARCSimulator()
        state = sim.get_state()
        assert isinstance(state.regs, tuple)
        assert len(state.regs) == 56  # 8 globals + 3*16 windowed

    def test_get_state_memory_is_tuple(self):
        sim = SPARCSimulator()
        state = sim.get_state()
        assert isinstance(state.memory, tuple)
        assert len(state.memory) == 65536

    def test_get_state_g0_always_zero(self):
        sim = SPARCSimulator()
        state = sim.get_state()
        assert state.regs[0] == 0   # global physical register 0


# ── Step ──────────────────────────────────────────────────────────────────────

class TestStep:
    """step() must advance PC by 4 and return correct StepTrace."""

    def test_step_nop_advances_pc_by_4(self):
        sim = SPARCSimulator()
        sim.load(NOP + HALT)
        assert sim._pc == 0x0000
        sim.step()
        assert sim._pc == 0x0004

    def test_step_on_halted_cpu_is_noop(self):
        sim = SPARCSimulator()
        sim.load(HALT)
        sim.step()    # executes HALT
        assert sim._halted
        pc = sim._pc
        trace = sim.step()   # should be no-op
        assert sim._pc == pc
        assert trace.mnemonic == "HALT"

    def test_step_trace_pc_before_after(self):
        sim = SPARCSimulator()
        sim.load(NOP + HALT)
        trace = sim.step()
        assert trace.pc_before == 0x0000
        assert trace.pc_after  == 0x0004

    def test_step_4byte_instruction_advances_pc_by_4(self):
        sim = SPARCSimulator()
        # ADD %g1, %g0, 42  (i=1, simm13=42)
        sim.load(add(1, 0, 42) + HALT)
        sim.step()
        assert sim._pc == 0x0004
