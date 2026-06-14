"""SIM00 protocol conformance tests for M68KSimulator.

Verifies that M68KSimulator satisfies the ``Simulator[M68KState]`` protocol
from ``simulator_protocol``:

  • reset() zeroes state and halted flag
  • load() copies bytes to memory at LOAD_ADDR
  • step() returns StepTrace with correct structure
  • execute() returns ExecutionResult with correct structure
  • get_state() returns immutable M68KState snapshot
  • Protocol isinstance check passes (runtime_checkable)
"""

from __future__ import annotations

import struct
import unittest

from simulator_protocol import ExecutionResult, Simulator, StepTrace

from motorola_68000_simulator import M68KSimulator, M68KState

_LOAD  = 0x001000
_SP    = 0x00F000


def _w(v: int) -> bytes: return struct.pack(">H", v & 0xFFFF)
def _l(v: int) -> bytes: return struct.pack(">I", v & 0xFFFFFFFF)
def _stop() -> bytes:    return _w(0x4E4F)  # TRAP #15 — halt without touching SR


class TestProtocolConformance(unittest.TestCase):
    """M68KSimulator implements Simulator[M68KState]."""

    def setUp(self):
        self.sim = M68KSimulator()

    def test_isinstance_protocol(self):
        assert isinstance(self.sim, Simulator)

    def test_reset_clears_registers(self):
        # Dirty up registers
        self.sim._d = [0xDEAD] * 8
        self.sim._a = [0xBEEF] * 8
        self.sim._a[7] = 0xDEAD
        self.sim._pc = 0x9999
        self.sim._sr = 0xABCD
        self.sim._halted = True
        self.sim.reset()
        for i in range(8):
            assert self.sim._d[i] == 0
        for i in range(7):
            assert self.sim._a[i] == 0
        assert self.sim._a[7] == _SP
        assert self.sim._pc == _LOAD
        assert self.sim._sr == 0x2700
        assert not self.sim._halted

    def test_reset_zeroes_memory(self):
        self.sim._mem[0x2000] = 0xFF
        self.sim.reset()
        assert self.sim._mem[0x2000] == 0

    def test_load_writes_to_correct_address(self):
        self.sim.reset()
        prog = bytes([0xAA, 0xBB, 0xCC, 0xDD])
        self.sim.load(prog)
        assert self.sim._mem[_LOAD    ] == 0xAA
        assert self.sim._mem[_LOAD + 1] == 0xBB
        assert self.sim._mem[_LOAD + 2] == 0xCC
        assert self.sim._mem[_LOAD + 3] == 0xDD

    def test_load_does_not_reset_pc(self):
        self.sim.reset()
        self.sim._pc = 0x9000
        self.sim.load(bytes([0x4E, 0x71]))
        assert self.sim._pc == 0x9000   # load() does NOT reset PC

    def test_step_returns_step_trace(self):
        self.sim.reset()
        prog = _w(0x4E71) + _stop()   # NOP then STOP
        self.sim.load(prog)
        trace = self.sim.step()
        assert isinstance(trace, StepTrace)
        assert trace.pc_before == _LOAD
        assert trace.pc_after  == _LOAD + 2   # NOP is 2 bytes
        assert isinstance(trace.mnemonic, str)
        assert len(trace.mnemonic) > 0
        assert isinstance(trace.description, str)

    def test_step_raises_when_halted(self):
        self.sim.reset()
        self.sim.load(_stop())
        self.sim.step()   # execute STOP
        assert self.sim._halted
        with self.assertRaises(RuntimeError):
            self.sim.step()

    def test_execute_returns_execution_result(self):
        self.sim.reset()
        result = self.sim.execute(_w(0x7005) + _stop())
        assert isinstance(result, ExecutionResult)
        assert isinstance(result.final_state, M68KState)
        assert isinstance(result.traces, list)
        assert result.halted is True
        assert result.error is None
        assert result.ok is True

    def test_execute_counts_steps(self):
        prog = _w(0x4E71) + _w(0x4E71) + _w(0x4E71) + _stop()
        result = self.sim.execute(prog)
        assert result.steps == 4   # 3 NOPs + STOP

    def test_execute_returns_final_state(self):
        prog = _w(0x702A) + _stop()   # MOVEQ #42, D0
        result = self.sim.execute(prog)
        assert result.final_state.d0 == 42

    def test_execute_max_steps_exceeded(self):
        # BRA #-2 is an infinite loop: 0x60FE (BRA with disp8 = -2)
        prog = bytes([0x60, 0xFE])   # BRA #-2 (loops forever)
        result = self.sim.execute(prog, max_steps=100)
        assert not result.ok
        assert result.halted is False
        assert "max_steps" in (result.error or "")

    def test_execute_resets_before_running(self):
        prog = _w(0x7005) + _stop()
        result1 = self.sim.execute(prog)
        assert result1.final_state.d0 == 5
        # Run again: should get same result (reset happens in execute)
        result2 = self.sim.execute(prog)
        assert result2.final_state.d0 == 5

    def test_get_state_returns_frozen_snapshot(self):
        self.sim.reset()
        state = self.sim.get_state()
        assert isinstance(state, M68KState)
        # Verify immutability
        with self.assertRaises(Exception):
            state.d0 = 99  # type: ignore[misc]  # frozen dataclass

    def test_get_state_snapshot_not_affected_by_later_changes(self):
        self.sim.reset()
        self.sim._d[0] = 10
        state_before = self.sim.get_state()
        self.sim._d[0] = 99
        # snapshot should still show 10
        assert state_before.d0 == 10
        assert self.sim._d[0] == 99


class TestM68KStateProperties(unittest.TestCase):
    """Verify M68KState properties and accessors."""

    def _make_state(self, **kwargs) -> M68KState:
        defaults = dict(
            d0=0, d1=0, d2=0, d3=0, d4=0, d5=0, d6=0, d7=0,
            a0=0, a1=0, a2=0, a3=0, a4=0, a5=0, a6=0, a7=_SP,
            pc=_LOAD, sr=0x2700, halted=False,
            memory=tuple([0] * (16 * 1024 * 1024)),
        )
        defaults.update(kwargs)
        return M68KState(**defaults)

    def test_ccr_flags_extracted_from_sr(self):
        # SR with all CCR bits set: 0x2700 | 0x1F = 0x271F
        state = self._make_state(sr=0x271F)
        assert state.x is True
        assert state.n is True
        assert state.z is True
        assert state.v is True
        assert state.c is True

    def test_ccr_flags_clear(self):
        state = self._make_state(sr=0x2700)
        assert state.x is False
        assert state.n is False
        assert state.z is False
        assert state.v is False
        assert state.c is False

    def test_z_flag_only(self):
        state = self._make_state(sr=0x2704)   # Z=1
        assert state.z is True
        assert state.c is False
        assert state.n is False

    def test_d_tuple(self):
        state = self._make_state(d0=1, d1=2, d2=3, d3=4, d4=5, d5=6, d6=7, d7=8)
        assert state.d == (1, 2, 3, 4, 5, 6, 7, 8)

    def test_a_tuple(self):
        state = self._make_state(a0=10, a1=20, a7=_SP)
        assert state.a[0] == 10
        assert state.a[1] == 20
        assert state.a[7] == _SP

    def test_d_signed(self):
        state = self._make_state(d0=0x8000_0000)
        assert state.d_signed(0) == -2147483648

    def test_d_word_signed(self):
        state = self._make_state(d0=0xFFFF_8000)
        assert state.d_word_signed(0) == -32768

    def test_d_byte_signed(self):
        state = self._make_state(d0=0xFF)
        assert state.d_byte_signed(0) == -1

    def test_halted_field(self):
        state = self._make_state(halted=True)
        assert state.halted is True


class TestExecutionTrace(unittest.TestCase):
    """Verify that traces are captured correctly."""

    def test_traces_populated(self):
        sim = M68KSimulator()
        prog = _w(0x4E71) + _w(0x4E71) + _stop()
        result = sim.execute(prog)
        assert len(result.traces) == 3   # 2 NOPs + STOP

    def test_trace_pc_chain(self):
        sim = M68KSimulator()
        prog = _w(0x4E71) + _w(0x4E71) + _stop()
        result = sim.execute(prog)
        assert result.traces[0].pc_before == _LOAD
        assert result.traces[0].pc_after  == _LOAD + 2
        assert result.traces[1].pc_before == _LOAD + 2
        assert result.traces[1].pc_after  == _LOAD + 4

    def test_trace_mnemonic_nop(self):
        sim = M68KSimulator()
        prog = _w(0x4E71) + _stop()
        result = sim.execute(prog)
        assert result.traces[0].mnemonic == "NOP"

    def test_trace_description_format(self):
        sim = M68KSimulator()
        prog = _w(0x4E71) + _stop()
        result = sim.execute(prog)
        desc = result.traces[0].description
        assert "NOP" in desc
        assert "0x001000" in desc.lower() or "0x1000" in desc.lower()


class TestLoadAddress(unittest.TestCase):
    """Verify programs load at 0x001000 and PC starts there."""

    def test_load_address(self):
        sim = M68KSimulator()
        sim.reset()
        prog = bytes([0x70, 0x07])   # MOVEQ #7, D0
        sim.load(prog)
        assert sim._mem[0x001000] == 0x70
        assert sim._mem[0x001001] == 0x07
        assert sim._pc == 0x001000

    def test_initial_sp(self):
        sim = M68KSimulator()
        sim.reset()
        assert sim._a[7] == 0x00F000

    def test_initial_sr(self):
        sim = M68KSimulator()
        sim.reset()
        assert sim._sr == 0x2700   # supervisor mode, IMask=7


if __name__ == "__main__":
    unittest.main()
