"""SIM00 protocol compliance tests for AlphaSimulator.

Verifies that AlphaSimulator implements the Simulator[AlphaState] protocol
correctly: reset(), load(), step(), execute(), get_state() all behave as
specified in the SIM00 contract.
"""

from __future__ import annotations

import struct

import pytest
from simulator_protocol import ExecutionResult, Simulator, StepTrace

from alpha_axp_simulator import AlphaSimulator, AlphaState

# ── Encoding helpers ──────────────────────────────────────────────────────────

def w32(v: int) -> bytes:
    """Pack a 32-bit Alpha instruction as 4 little-endian bytes."""
    return struct.pack("<I", v & 0xFFFF_FFFF)


# HALT = call_pal 0x0000 = 0x00000000
HALT = w32(0x0000_0000)

# NOP = BIS r31, r31, r31  (op=0x11, func=0x20, Ra=Rb=Rc=31)
# Encoding: (0x11 << 26) | (31 << 21) | (31 << 16) | (0x20 << 5) | 31
NOP = w32((0x11 << 26) | (31 << 21) | (31 << 16) | (0x20 << 5) | 31)

# MOV: BIS r31, imm8, r1 = r1 = imm8
def mov_i(rd: int, imm8: int) -> bytes:
    """BIS r31, imm8, rd  — load 8-bit immediate (the Alpha MOV idiom)."""
    # Operate i-format: op=0x11, Ra=r31, lit8=imm8, i=1, func=0x20 (BIS), rc=rd
    return w32((0x11 << 26) | (31 << 21) | ((imm8 & 0xFF) << 13) | (1 << 12) | (0x20 << 5) | rd)


# ── Protocol compliance ───────────────────────────────────────────────────────

class TestProtocolCompliance:
    """AlphaSimulator must satisfy Simulator[AlphaState] structurally."""

    def test_isinstance_simulator(self):
        sim = AlphaSimulator()
        assert isinstance(sim, Simulator)

    def test_has_all_protocol_methods(self):
        sim = AlphaSimulator()
        assert callable(sim.reset)
        assert callable(sim.load)
        assert callable(sim.step)
        assert callable(sim.execute)
        assert callable(sim.get_state)

    def test_execute_returns_execution_result(self):
        sim = AlphaSimulator()
        result = sim.execute(HALT)
        assert isinstance(result, ExecutionResult)

    def test_step_returns_step_trace(self):
        sim = AlphaSimulator()
        sim.load(NOP + HALT)
        trace = sim.step()
        assert isinstance(trace, StepTrace)

    def test_get_state_returns_alpha_state(self):
        sim = AlphaSimulator()
        state = sim.get_state()
        assert isinstance(state, AlphaState)


# ── Reset ─────────────────────────────────────────────────────────────────────

class TestReset:
    """reset() must return CPU to a well-defined initial state."""

    def test_reset_pc_is_zero(self):
        sim = AlphaSimulator()
        sim._pc = 0x1234
        sim.reset()
        assert sim._pc == 0x0000

    def test_reset_npc_is_4(self):
        sim = AlphaSimulator()
        sim._npc = 0x5678
        sim.reset()
        assert sim._npc == 0x0004

    def test_reset_all_regs_zero(self):
        sim = AlphaSimulator()
        sim._regs[5]  = 0xDEAD_BEEF
        sim._regs[28] = 0xCAFE_BABE
        sim.reset()
        assert all(r == 0 for r in sim._regs)

    def test_reset_memory_zeroed(self):
        sim = AlphaSimulator()
        sim._mem[0x100] = 0xAB
        sim._mem[0x200] = 0xCD
        sim.reset()
        assert sim._mem[0x100] == 0x00
        assert sim._mem[0x200] == 0x00

    def test_reset_clears_halted(self):
        sim = AlphaSimulator()
        sim._halted = True
        sim.reset()
        assert not sim._halted

    def test_reset_via_get_state(self):
        sim = AlphaSimulator()
        state = sim.get_state()
        assert state.pc == 0x0000
        assert state.npc == 0x0004
        assert all(r == 0 for r in state.regs)
        assert not state.halted


# ── Load ──────────────────────────────────────────────────────────────────────

class TestLoad:
    """load() must reset and copy bytes to memory at 0x0000."""

    def test_load_places_bytes_at_0(self):
        sim = AlphaSimulator()
        # HALT = 0x00000000 = [0x00, 0x00, 0x00, 0x00] (little-endian)
        prog = bytes([0x00, 0x00, 0x00, 0x00])
        sim.load(prog)
        assert sim._mem[0] == 0x00
        assert sim._mem[1] == 0x00
        assert sim._mem[2] == 0x00
        assert sim._mem[3] == 0x00

    def test_load_places_non_halt_bytes(self):
        sim = AlphaSimulator()
        # NOP instruction bytes
        nop_bytes = NOP
        sim.load(nop_bytes + HALT)
        for i, b in enumerate(nop_bytes):
            assert sim._mem[i] == b

    def test_load_sets_pc_to_zero(self):
        sim = AlphaSimulator()
        sim.load(HALT)
        assert sim._pc == 0x0000

    def test_load_raises_on_overflow(self):
        sim = AlphaSimulator()
        with pytest.raises(ValueError):
            sim.load(bytes(65537))

    def test_load_resets_before_loading(self):
        sim = AlphaSimulator()
        sim._regs[5] = 0xAB
        sim.load(HALT)
        assert sim._regs[5] == 0x00

    def test_load_exactly_64k_ok(self):
        sim = AlphaSimulator()
        sim.load(bytes(65536))   # should not raise


# ── Execute ───────────────────────────────────────────────────────────────────

class TestExecute:
    """execute() must run to HALT and populate ExecutionResult correctly."""

    def test_execute_halt_immediately(self):
        sim = AlphaSimulator()
        result = sim.execute(HALT)
        assert result.ok
        assert result.halted
        assert result.error is None
        assert result.steps == 1

    def test_execute_returns_traces(self):
        sim = AlphaSimulator()
        result = sim.execute(HALT)
        assert len(result.traces) == 1
        assert result.traces[0].mnemonic == "HALT"

    def test_execute_max_steps_exceeded(self):
        # BR r31, 0: branch always to itself (disp21=0 → target = PC+4+0 = PC+4... wait)
        # Actually disp21=0 gives target = (PC+4) + 0 = PC+4 = next instruction
        # For a self-loop: disp21 = -1 (sign-extend 21 bits → 0x1FFFFF)
        # target = (PC+4) + (-1*4) = PC
        loop = w32((0x30 << 26) | (31 << 21) | (0x1FFFFF))   # BR r31, -1
        result = AlphaSimulator().execute(loop + HALT, max_steps=10)
        assert not result.ok
        assert result.steps == 10
        assert "max_steps" in result.error

    def test_execute_sets_final_state(self):
        sim = AlphaSimulator()
        result = sim.execute(HALT)
        assert isinstance(result.final_state, AlphaState)
        assert result.final_state.halted

    def test_execute_resets_before_running(self):
        sim = AlphaSimulator()
        sim._regs[5] = 0xDE
        result = sim.execute(HALT)
        assert result.final_state.regs[5] == 0

    def test_execute_nop_then_halt(self):
        result = AlphaSimulator().execute(NOP + HALT)
        assert result.ok
        assert result.steps == 2
        assert result.traces[0].mnemonic == "BIS"
        assert result.traces[1].mnemonic == "HALT"


# ── GetState ──────────────────────────────────────────────────────────────────

class TestGetState:
    """get_state() must return an immutable snapshot."""

    def test_get_state_is_frozen(self):
        import dataclasses
        sim = AlphaSimulator()
        state = sim.get_state()
        with pytest.raises((dataclasses.FrozenInstanceError, TypeError)):
            state.pc = 0xFF   # type: ignore[misc]

    def test_get_state_snapshot_not_mutated_by_step(self):
        sim = AlphaSimulator()
        sim.load(NOP + HALT)
        state_before = sim.get_state()
        sim.step()
        state_after = sim.get_state()
        assert state_before.pc == 0x0000
        assert state_after.pc == 0x0004

    def test_get_state_regs_is_tuple(self):
        sim = AlphaSimulator()
        state = sim.get_state()
        assert isinstance(state.regs, tuple)
        assert len(state.regs) == 32

    def test_get_state_memory_is_tuple(self):
        sim = AlphaSimulator()
        state = sim.get_state()
        assert isinstance(state.memory, tuple)
        assert len(state.memory) == 65536

    def test_get_state_r31_always_zero(self):
        sim = AlphaSimulator()
        state = sim.get_state()
        assert state.regs[31] == 0


# ── Step ──────────────────────────────────────────────────────────────────────

class TestStep:
    """step() must advance PC by 4 and return correct StepTrace."""

    def test_step_nop_advances_pc_by_4(self):
        sim = AlphaSimulator()
        sim.load(NOP + HALT)
        assert sim._pc == 0x0000
        sim.step()
        assert sim._pc == 0x0004

    def test_step_on_halted_cpu_is_noop(self):
        sim = AlphaSimulator()
        sim.load(HALT)
        sim.step()    # executes HALT
        assert sim._halted
        pc = sim._pc
        trace = sim.step()   # should be no-op
        assert sim._pc == pc
        assert trace.mnemonic == "HALT"

    def test_step_trace_pc_before_after(self):
        sim = AlphaSimulator()
        sim.load(NOP + HALT)
        trace = sim.step()
        assert trace.pc_before == 0x0000
        assert trace.pc_after  == 0x0004

    def test_step_4byte_instruction_advances_pc_by_4(self):
        sim = AlphaSimulator()
        sim.load(mov_i(1, 42) + HALT)
        sim.step()
        assert sim._pc == 0x0004
