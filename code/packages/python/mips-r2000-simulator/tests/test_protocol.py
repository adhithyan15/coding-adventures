"""SIM00 protocol compliance tests for MIPSSimulator.

Verifies that MIPSSimulator implements the Simulator[MIPSState] protocol
correctly: reset(), load(), step(), execute(), get_state() all behave as
specified.
"""

from __future__ import annotations

import struct

import pytest
from simulator_protocol import ExecutionResult, Simulator, StepTrace

from mips_r2000_simulator import MIPSSimulator, MIPSState

# ── Encoding helpers ──────────────────────────────────────────────────────────

def w32(v: int) -> bytes:
    """Pack a 32-bit MIPS instruction as 4 big-endian bytes."""
    return struct.pack(">I", v & 0xFFFF_FFFF)


# HALT = SYSCALL  (op=0, funct=0x0C)
HALT = w32(0x0000_000C)

# NOP = SLL $zero, $zero, 0  (op=0, funct=0, shamt=0, rd=rt=rs=0)
NOP = w32(0x0000_0000)


def ADDIU_REG(rt: int, rs: int, imm: int) -> bytes:
    """ADDIU rt, rs, imm  — op=9, no overflow."""
    imm &= 0xFFFF
    return w32((0x09 << 26) | (rs << 21) | (rt << 16) | imm)


def ADDU(rd: int, rs: int, rt: int) -> bytes:
    """ADDU rd, rs, rt  — R-type, funct=0x21."""
    return w32((rs << 21) | (rt << 16) | (rd << 11) | 0x21)


# ── Protocol compliance ───────────────────────────────────────────────────────

class TestProtocolCompliance:
    """MIPSSimulator must satisfy Simulator[MIPSState] structurally."""

    def test_isinstance_simulator(self):
        sim = MIPSSimulator()
        assert isinstance(sim, Simulator)

    def test_has_all_protocol_methods(self):
        sim = MIPSSimulator()
        assert callable(sim.reset)
        assert callable(sim.load)
        assert callable(sim.step)
        assert callable(sim.execute)
        assert callable(sim.get_state)

    def test_execute_returns_execution_result(self):
        sim = MIPSSimulator()
        result = sim.execute(HALT)
        assert isinstance(result, ExecutionResult)

    def test_step_returns_step_trace(self):
        sim = MIPSSimulator()
        sim.load(NOP + HALT)
        trace = sim.step()
        assert isinstance(trace, StepTrace)

    def test_get_state_returns_mips_state(self):
        sim = MIPSSimulator()
        state = sim.get_state()
        assert isinstance(state, MIPSState)


# ── Reset ─────────────────────────────────────────────────────────────────────

class TestReset:
    """reset() must return CPU to well-defined initial state."""

    def test_reset_pc_is_zero(self):
        sim = MIPSSimulator()
        sim._pc = 0x1234
        sim.reset()
        assert sim._pc == 0x0000

    def test_reset_all_regs_zero(self):
        sim = MIPSSimulator()
        sim._regs[8] = 0xDEAD
        sim._regs[16] = 0xBEEF
        sim.reset()
        assert all(r == 0 for r in sim._regs)

    def test_reset_hi_lo_zero(self):
        sim = MIPSSimulator()
        sim._hi = 0xFFFF
        sim._lo = 0xFFFF
        sim.reset()
        assert sim._hi == 0
        assert sim._lo == 0

    def test_reset_memory_zeroed(self):
        sim = MIPSSimulator()
        sim._mem[0x100] = 0xAB
        sim.reset()
        assert sim._mem[0x100] == 0x00

    def test_reset_clears_halted(self):
        sim = MIPSSimulator()
        sim._halted = True
        sim.reset()
        assert not sim._halted

    def test_reset_via_get_state(self):
        sim = MIPSSimulator()
        state = sim.get_state()
        assert state.pc == 0x0000
        assert all(r == 0 for r in state.regs)
        assert state.hi == 0
        assert state.lo == 0
        assert not state.halted


# ── Load ──────────────────────────────────────────────────────────────────────

class TestLoad:
    """load() must reset and copy bytes to memory at 0x0000."""

    def test_load_places_bytes_at_0(self):
        sim = MIPSSimulator()
        prog = bytes([0xAB, 0xCD, 0xEF, 0x12])
        sim.load(prog)
        assert sim._mem[0] == 0xAB
        assert sim._mem[1] == 0xCD
        assert sim._mem[2] == 0xEF
        assert sim._mem[3] == 0x12

    def test_load_sets_pc_to_zero(self):
        sim = MIPSSimulator()
        sim.load(HALT)
        assert sim._pc == 0x0000

    def test_load_raises_on_overflow(self):
        sim = MIPSSimulator()
        with pytest.raises(ValueError):
            sim.load(bytes(65537))

    def test_load_resets_before_loading(self):
        sim = MIPSSimulator()
        sim._regs[8] = 0xAB
        sim.load(HALT)
        assert sim._regs[8] == 0x00

    def test_load_exactly_64k_ok(self):
        sim = MIPSSimulator()
        sim.load(bytes(65536))   # should not raise


# ── Execute ───────────────────────────────────────────────────────────────────

class TestExecute:
    """execute() must run to HALT and populate ExecutionResult correctly."""

    def test_execute_halt_immediately(self):
        sim = MIPSSimulator()
        result = sim.execute(HALT)
        assert result.ok
        assert result.halted
        assert result.error is None
        assert result.steps == 1

    def test_execute_returns_traces(self):
        sim = MIPSSimulator()
        result = sim.execute(HALT)
        assert len(result.traces) == 1
        assert result.traces[0].mnemonic == "HALT"

    def test_execute_max_steps_exceeded(self):
        # BEQ $zero, $zero, -1 → infinite loop (offset -1 in words = -4 bytes)
        # BEQ: op=4, rs=0, rt=0, imm=0xFFFF (-1 in 16-bit signed = -4 bytes)
        loop = w32((0x04 << 26) | 0xFFFF)  # BEQ $zero,$zero,-4
        result = MIPSSimulator().execute(loop + HALT, max_steps=10)
        assert not result.ok
        assert result.steps == 10
        assert "max_steps" in result.error

    def test_execute_sets_final_state(self):
        sim = MIPSSimulator()
        result = sim.execute(HALT)
        assert isinstance(result.final_state, MIPSState)
        assert result.final_state.halted

    def test_execute_resets_before_running(self):
        sim = MIPSSimulator()
        sim._regs[8] = 0xDE
        result = sim.execute(HALT)
        assert result.final_state.regs[8] == 0

    def test_execute_nop_then_halt(self):
        result = MIPSSimulator().execute(NOP + HALT)
        assert result.ok
        assert result.steps == 2
        assert result.traces[0].mnemonic == "NOP"
        assert result.traces[1].mnemonic == "HALT"


# ── GetState ──────────────────────────────────────────────────────────────────

class TestGetState:
    """get_state() must return an immutable snapshot."""

    def test_get_state_is_frozen(self):
        import dataclasses
        sim = MIPSSimulator()
        state = sim.get_state()
        with pytest.raises((dataclasses.FrozenInstanceError, TypeError)):
            state.pc = 0xFF   # type: ignore[misc]

    def test_get_state_snapshot_not_mutated_by_step(self):
        sim = MIPSSimulator()
        sim.load(NOP + HALT)
        state_before = sim.get_state()
        sim.step()
        state_after = sim.get_state()
        assert state_before.pc == 0x0000
        assert state_after.pc == 0x0004

    def test_get_state_regs_is_tuple(self):
        sim = MIPSSimulator()
        state = sim.get_state()
        assert isinstance(state.regs, tuple)
        assert len(state.regs) == 32

    def test_get_state_memory_is_tuple(self):
        sim = MIPSSimulator()
        state = sim.get_state()
        assert isinstance(state.memory, tuple)
        assert len(state.memory) == 65536

    def test_get_state_r0_always_zero(self):
        sim = MIPSSimulator()
        state = sim.get_state()
        assert state.regs[0] == 0


# ── Step ──────────────────────────────────────────────────────────────────────

class TestStep:
    """step() must advance PC by 4 and return correct StepTrace."""

    def test_step_nop_advances_pc_by_4(self):
        sim = MIPSSimulator()
        sim.load(NOP + HALT)
        assert sim._pc == 0x0000
        sim.step()
        assert sim._pc == 0x0004

    def test_step_on_halted_cpu_is_noop(self):
        sim = MIPSSimulator()
        sim.load(HALT)
        sim.step()    # executes HALT
        assert sim._halted
        pc = sim._pc
        trace = sim.step()   # should be no-op
        assert sim._pc == pc
        assert trace.mnemonic == "HALT"

    def test_step_trace_pc_before_after(self):
        sim = MIPSSimulator()
        sim.load(NOP + HALT)
        trace = sim.step()
        assert trace.pc_before == 0x0000
        assert trace.pc_after  == 0x0004

    def test_step_4byte_instruction_advances_pc_by_4(self):
        sim = MIPSSimulator()
        # ADDIU $t0, $zero, 0x42  — 4-byte I-type
        sim.load(ADDIU_REG(8, 0, 0x42) + HALT)
        sim.step()
        assert sim._pc == 0x0004
