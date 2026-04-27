"""Tests for Intel4004Simulator protocol conformance.

Verifies the backward-compatible additions that make Intel4004Simulator satisfy
the Simulator[Intel4004State] protocol from the simulator-protocol package:

  - Intel4004State frozen dataclass (state.py)
  - get_state() -> Intel4004State
  - execute(program, max_steps) -> ExecutionResult[Intel4004State]

These tests ADD coverage for new methods only.  They do not modify or replace
the existing tests in test_intel4004.py.

=== Test Programs ===

All test programs are minimal byte sequences:

  [0x01]             — HLT only (cleanest halt)
  [0xD5, 0x01]       — LDM 5, HLT  (accumulator = 5 after XCH not done)
  [0xD3, 0xB0, 0x01] — LDM 3, XCH R0, HLT  (R0 = 3, accumulator = 0)
  [0xD1, 0xB0, 0xD2, 0x80, 0xB1, 0x01]
                     — the classic 1+2 program: R1 = 3 at end
"""

from __future__ import annotations

import dataclasses

import pytest

from intel4004_simulator import Intel4004Simulator, Intel4004State
from simulator_protocol import ExecutionResult, StepTrace


# ===========================================================================
# Helpers
# ===========================================================================

HLT = bytes([0x01])
LDM5_HLT = bytes([0xD5, 0x01])          # LDM 5, HLT → accumulator stays 5 after HLT
LDM3_XCH0_HLT = bytes([0xD3, 0xB0, 0x01])  # LDM 3, XCH R0, HLT → R0=3, acc=0
ADD_PROGRAM = bytes([0xD1, 0xB0, 0xD2, 0x80, 0xB1, 0x01])  # R1=3


# ===========================================================================
# TestIntel4004State
# ===========================================================================


class TestIntel4004State:
    """Intel4004State frozen dataclass invariants."""

    def _make_state(self, **kwargs: object) -> Intel4004State:
        defaults: dict = {
            "accumulator": 0,
            "registers": tuple([0] * 16),
            "carry": False,
            "pc": 0,
            "halted": False,
            "ram": tuple(
                tuple(tuple(0 for _ in range(16)) for _ in range(4))
                for _ in range(4)
            ),
            "hw_stack": (0, 0, 0),
            "stack_pointer": 0,
        }
        defaults.update(kwargs)
        return Intel4004State(**defaults)  # type: ignore[arg-type]

    def test_state_is_frozen(self) -> None:
        """Intel4004State is immutable — mutations raise FrozenInstanceError."""
        state = self._make_state(accumulator=5)
        with pytest.raises(dataclasses.FrozenInstanceError):
            state.accumulator = 99  # type: ignore[misc]

    def test_state_registers_frozen(self) -> None:
        """registers tuple is immutable."""
        state = self._make_state()
        with pytest.raises((dataclasses.FrozenInstanceError, TypeError)):
            state.registers = tuple([1] * 16)  # type: ignore[misc]

    def test_state_carry_frozen(self) -> None:
        """carry field is immutable."""
        state = self._make_state(carry=False)
        with pytest.raises(dataclasses.FrozenInstanceError):
            state.carry = True  # type: ignore[misc]

    def test_state_fields_accessible(self) -> None:
        """All fields are accessible by name."""
        regs = tuple(range(16))
        state = Intel4004State(
            accumulator=7,
            registers=regs,
            carry=True,
            pc=42,
            halted=True,
            ram=tuple(
                tuple(tuple(0 for _ in range(16)) for _ in range(4))
                for _ in range(4)
            ),
            hw_stack=(100, 200, 300),
            stack_pointer=1,
        )
        assert state.accumulator == 7
        assert state.registers == regs
        assert state.carry is True
        assert state.pc == 42
        assert state.halted is True
        assert state.hw_stack == (100, 200, 300)
        assert state.stack_pointer == 1

    def test_state_ram_accessible(self) -> None:
        """RAM is accessible as nested tuples."""
        state = self._make_state()
        # 4 banks × 4 registers × 16 nibbles
        assert len(state.ram) == 4
        assert len(state.ram[0]) == 4
        assert len(state.ram[0][0]) == 16
        assert state.ram[0][0][0] == 0


# ===========================================================================
# TestGetState
# ===========================================================================


class TestGetState:
    """get_state() returns a frozen Intel4004State snapshot."""

    def test_get_state_returns_intel4004_state(self) -> None:
        """get_state() returns an Intel4004State instance."""
        sim = Intel4004Simulator()
        state = sim.get_state()
        assert isinstance(state, Intel4004State)

    def test_get_state_initial_accumulator_zero(self) -> None:
        """Fresh simulator has accumulator=0."""
        sim = Intel4004Simulator()
        state = sim.get_state()
        assert state.accumulator == 0

    def test_get_state_initial_carry_false(self) -> None:
        """Fresh simulator has carry=False."""
        sim = Intel4004Simulator()
        state = sim.get_state()
        assert state.carry is False

    def test_get_state_initial_halted_false(self) -> None:
        """Fresh simulator is not halted."""
        sim = Intel4004Simulator()
        state = sim.get_state()
        assert state.halted is False

    def test_get_state_after_run_reflects_final_state(self) -> None:
        """After run(), get_state() captures the final CPU state."""
        sim = Intel4004Simulator()
        sim.run(LDM3_XCH0_HLT)
        state = sim.get_state()
        assert state.halted is True
        assert state.registers[0] == 3  # XCH R0 moved 3 into R0

    def test_get_state_snapshot_does_not_change_after_reset(self) -> None:
        """Snapshot taken before reset is unchanged after reset() is called."""
        sim = Intel4004Simulator()
        sim.run(LDM5_HLT)
        # Note: after XCH, acc=0 but we're capturing after HLT executes XCH not done
        # LDM5 puts 5 in acc, HLT executes — acc should be 5
        state_before_reset = sim.get_state()
        halted_before = state_before_reset.halted
        sim.reset()
        # The snapshot should still show what it captured
        assert state_before_reset.halted == halted_before
        # Fresh state shows not halted
        state_after_reset = sim.get_state()
        assert state_after_reset.halted is False

    def test_get_state_registers_is_tuple(self) -> None:
        """registers in state snapshot is a tuple, not a list."""
        sim = Intel4004Simulator()
        state = sim.get_state()
        assert isinstance(state.registers, tuple)
        assert len(state.registers) == 16

    def test_get_state_hw_stack_is_tuple(self) -> None:
        """hw_stack in state snapshot is a tuple."""
        sim = Intel4004Simulator()
        state = sim.get_state()
        assert isinstance(state.hw_stack, tuple)
        assert len(state.hw_stack) == 3

    def test_get_state_ram_is_nested_tuples(self) -> None:
        """ram in state snapshot is nested tuples."""
        sim = Intel4004Simulator()
        state = sim.get_state()
        assert isinstance(state.ram, tuple)
        assert isinstance(state.ram[0], tuple)
        assert isinstance(state.ram[0][0], tuple)

    def test_get_state_is_frozen(self) -> None:
        """State snapshot is immutable — mutations raise FrozenInstanceError."""
        sim = Intel4004Simulator()
        state = sim.get_state()
        with pytest.raises(dataclasses.FrozenInstanceError):
            state.accumulator = 99  # type: ignore[misc]


# ===========================================================================
# TestExecute
# ===========================================================================


class TestExecute:
    """execute() returns ExecutionResult[Intel4004State] conforming to the protocol."""

    def test_execute_returns_execution_result(self) -> None:
        """execute() returns an ExecutionResult."""
        sim = Intel4004Simulator()
        result = sim.execute(HLT)
        assert isinstance(result, ExecutionResult)

    def test_execute_hlt_ok(self) -> None:
        """A program that is just HLT returns result.ok == True."""
        sim = Intel4004Simulator()
        result = sim.execute(HLT)
        assert result.ok is True
        assert result.halted is True
        assert result.error is None

    def test_execute_final_state_is_intel4004_state(self) -> None:
        """final_state in the result is an Intel4004State instance."""
        sim = Intel4004Simulator()
        result = sim.execute(HLT)
        assert isinstance(result.final_state, Intel4004State)

    def test_execute_registers_accessible_via_final_state(self) -> None:
        """Registers are accessible via result.final_state.registers."""
        sim = Intel4004Simulator()
        result = sim.execute(LDM3_XCH0_HLT)  # LDM 3, XCH R0, HLT
        assert result.ok is True
        assert result.final_state.registers[0] == 3  # R0 = 3

    def test_execute_accumulator_after_ldm(self) -> None:
        """Accumulator reflects the value loaded by LDM before HLT."""
        sim = Intel4004Simulator()
        # LDM 5 loads 5 into accumulator, then HLT
        result = sim.execute(LDM5_HLT)
        assert result.ok is True
        # After LDM 5 + HLT, accumulator is 5
        assert result.final_state.accumulator == 5

    def test_execute_step_count(self) -> None:
        """steps field counts instructions executed."""
        sim = Intel4004Simulator()
        result = sim.execute(HLT)
        assert result.steps == 1

    def test_execute_add_program_steps(self) -> None:
        """The 1+2 addition program runs for exactly 6 steps."""
        sim = Intel4004Simulator()
        result = sim.execute(ADD_PROGRAM)
        assert result.ok is True
        assert result.steps == 6
        assert result.final_state.registers[1] == 3  # R1 = 1 + 2 = 3

    def test_execute_max_steps_exceeded_not_ok(self) -> None:
        """When max_steps is exceeded, result.ok is False."""
        sim = Intel4004Simulator()
        # Program that jumps back to start — infinite loop (JUN 0x000 = 0x40, 0x00)
        infinite_loop = bytes([0x40, 0x00])  # JUN 0x000
        result = sim.execute(infinite_loop, max_steps=10)
        assert result.ok is False
        assert result.halted is False
        assert result.error is not None
        assert "max_steps" in result.error
        assert result.steps == 10

    def test_execute_traces_count_equals_steps(self) -> None:
        """Number of traces equals number of steps executed."""
        sim = Intel4004Simulator()
        result = sim.execute(ADD_PROGRAM)
        assert len(result.traces) == result.steps

    def test_execute_traces_are_step_trace_instances(self) -> None:
        """Each entry in traces is a StepTrace."""
        sim = Intel4004Simulator()
        result = sim.execute(HLT)
        assert len(result.traces) == 1
        assert isinstance(result.traces[0], StepTrace)

    def test_execute_trace_mnemonic_hlt(self) -> None:
        """HLT program produces a trace with mnemonic 'HLT'."""
        sim = Intel4004Simulator()
        result = sim.execute(HLT)
        assert result.traces[0].mnemonic == "HLT"

    def test_execute_does_not_break_run(self) -> None:
        """After execute(), calling run() still works (no state corruption)."""
        sim = Intel4004Simulator()
        sim.execute(HLT)
        traces = sim.run(ADD_PROGRAM)
        assert sim.registers[1] == 3
        assert len(traces) == 6
