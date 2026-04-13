"""Tests for the simulator-protocol package.

=== Test Organization ===

Tests are grouped by type:

  TestStepTrace          — StepTrace dataclass invariants (immutability, fields)
  TestExecutionResult    — ExecutionResult invariants (ok property, immutability, generics)
  TestSimulatorProtocol  — Structural subtyping (duck typing, no explicit inheritance)
  TestEndToEndLoop       — Simulating the compile → assemble → execute → assert cycle

=== Key Concepts ===

- Structural subtyping: a class satisfies Simulator[StateT] if it has the right
  methods, with no ``class Foo(Simulator[...])`` declaration needed.
- ``ExecutionResult.ok`` is True only when BOTH halted=True AND error=None.
- ``StepTrace`` and ``ExecutionResult`` are frozen — mutations raise FrozenInstanceError.
"""

from __future__ import annotations

import dataclasses
from dataclasses import dataclass
from typing import TYPE_CHECKING

import pytest

from simulator_protocol import ExecutionResult, Simulator, StepTrace

if TYPE_CHECKING:
    pass


# ===========================================================================
# Helpers — mock state and simulator for testing
# ===========================================================================


@dataclass(frozen=True)
class MockState:
    """Minimal CPU state for testing."""

    accumulator: int
    pc: int
    halted: bool


class MockSimulator:
    """A minimal simulator that satisfies Simulator[MockState] structurally.

    No explicit inheritance from Simulator — structural subtyping means this
    class "is a" Simulator just by having the right methods.
    """

    def __init__(self, halt_after: int = 1) -> None:
        self._acc = 0
        self._pc = 0
        self._halted = False
        self._program: bytes = b""
        self._halt_after = halt_after

    def load(self, program: bytes) -> None:
        self._program = program
        self._pc = 0
        self._halted = False
        self._acc = 0

    def step(self) -> StepTrace:
        if self._halted:
            raise RuntimeError("CPU is halted")
        pc_before = self._pc
        # Simple: first byte is "opcode", 0x01 = HLT, 0xD0|n = LDM n
        opcode = self._program[self._pc] if self._pc < len(self._program) else 0x01
        if opcode == 0x01:
            self._halted = True
            mnemonic = "HLT"
            self._pc += 1
        else:
            imm = opcode & 0x0F
            self._acc = imm
            mnemonic = f"LDM {imm}"
            self._pc += 1
        return StepTrace(
            pc_before=pc_before,
            pc_after=self._pc,
            mnemonic=mnemonic,
            description=f"{mnemonic} @ 0x{pc_before:03X}",
        )

    def execute(self, program: bytes, max_steps: int = 100_000) -> ExecutionResult[MockState]:
        self.reset()
        self.load(program)
        traces: list[StepTrace] = []
        steps = 0
        while not self._halted and steps < max_steps:
            trace = self.step()
            traces.append(trace)
            steps += 1
        return ExecutionResult(
            halted=self._halted,
            steps=steps,
            final_state=self.get_state(),
            error=None if self._halted else f"max_steps ({max_steps}) exceeded",
            traces=traces,
        )

    def get_state(self) -> MockState:
        return MockState(accumulator=self._acc, pc=self._pc, halted=self._halted)

    def reset(self) -> None:
        self._acc = 0
        self._pc = 0
        self._halted = False
        self._program = b""


# ===========================================================================
# TestStepTrace
# ===========================================================================


class TestStepTrace:
    """StepTrace invariants — immutability and field access."""

    def test_step_trace_fields(self) -> None:
        """All four fields are accessible."""
        trace = StepTrace(
            pc_before=0x000,
            pc_after=0x001,
            mnemonic="NOP",
            description="NOP @ 0x000",
        )
        assert trace.pc_before == 0x000
        assert trace.pc_after == 0x001
        assert trace.mnemonic == "NOP"
        assert trace.description == "NOP @ 0x000"

    def test_step_trace_is_frozen(self) -> None:
        """StepTrace is immutable — mutations raise FrozenInstanceError."""
        trace = StepTrace(pc_before=0, pc_after=1, mnemonic="NOP", description="NOP @ 0x000")
        with pytest.raises(dataclasses.FrozenInstanceError):
            trace.pc_before = 99  # type: ignore[misc]

    def test_step_trace_mnemonic_frozen(self) -> None:
        """Mnemonic field is also immutable."""
        trace = StepTrace(pc_before=0, pc_after=1, mnemonic="ADD R2", description="ADD R2 @ 0x000")
        with pytest.raises(dataclasses.FrozenInstanceError):
            trace.mnemonic = "NOP"  # type: ignore[misc]

    def test_step_trace_description_frozen(self) -> None:
        """Description field is also immutable."""
        trace = StepTrace(pc_before=0, pc_after=1, mnemonic="LDM 7", description="LDM 7 @ 0x003")
        with pytest.raises(dataclasses.FrozenInstanceError):
            trace.description = "changed"  # type: ignore[misc]

    def test_step_trace_pc_after_frozen(self) -> None:
        """pc_after field is also immutable."""
        trace = StepTrace(pc_before=0, pc_after=5, mnemonic="JUN 5", description="JUN 5 @ 0x000")
        with pytest.raises(dataclasses.FrozenInstanceError):
            trace.pc_after = 0  # type: ignore[misc]

    def test_step_trace_equality(self) -> None:
        """Two StepTraces with same values are equal."""
        t1 = StepTrace(pc_before=0, pc_after=1, mnemonic="NOP", description="NOP @ 0x000")
        t2 = StepTrace(pc_before=0, pc_after=1, mnemonic="NOP", description="NOP @ 0x000")
        assert t1 == t2

    def test_step_trace_inequality(self) -> None:
        """Two StepTraces with different mnemonics are not equal."""
        t1 = StepTrace(pc_before=0, pc_after=1, mnemonic="NOP", description="NOP @ 0x000")
        t2 = StepTrace(pc_before=0, pc_after=1, mnemonic="HLT", description="HLT @ 0x000")
        assert t1 != t2

    def test_step_trace_hashable(self) -> None:
        """Frozen dataclasses are hashable and can be used in sets."""
        trace = StepTrace(pc_before=0, pc_after=1, mnemonic="NOP", description="NOP @ 0x000")
        s = {trace}
        assert trace in s


# ===========================================================================
# TestExecutionResult
# ===========================================================================


class TestExecutionResult:
    """ExecutionResult invariants — ok property, immutability, generics."""

    def _make_trace(self) -> StepTrace:
        return StepTrace(pc_before=0, pc_after=1, mnemonic="HLT", description="HLT @ 0x000")

    def test_ok_true_when_halted_and_no_error(self) -> None:
        """ok is True when halted=True and error=None."""
        state = MockState(accumulator=5, pc=1, halted=True)
        result: ExecutionResult[MockState] = ExecutionResult(
            halted=True,
            steps=1,
            final_state=state,
            error=None,
            traces=[self._make_trace()],
        )
        assert result.ok is True

    def test_ok_false_when_not_halted(self) -> None:
        """ok is False when halted=False, even if error is None."""
        state = MockState(accumulator=0, pc=100, halted=False)
        result: ExecutionResult[MockState] = ExecutionResult(
            halted=False,
            steps=100_000,
            final_state=state,
            error="max_steps (100000) exceeded",
            traces=[],
        )
        assert result.ok is False

    def test_ok_false_when_error_is_set(self) -> None:
        """ok is False when error is a non-None string, even if halted=True."""
        state = MockState(accumulator=0, pc=0, halted=True)
        result: ExecutionResult[MockState] = ExecutionResult(
            halted=True,
            steps=5,
            final_state=state,
            error="illegal opcode 0xFF at 0x004",
            traces=[],
        )
        assert result.ok is False

    def test_ok_false_when_both_not_halted_and_error(self) -> None:
        """ok is False when both halted=False and error is set."""
        state = MockState(accumulator=0, pc=0, halted=False)
        result: ExecutionResult[MockState] = ExecutionResult(
            halted=False,
            steps=0,
            final_state=state,
            error="load error",
            traces=[],
        )
        assert result.ok is False

    def test_execution_result_is_frozen(self) -> None:
        """ExecutionResult is immutable — mutations raise FrozenInstanceError."""
        state = MockState(accumulator=0, pc=0, halted=True)
        result: ExecutionResult[MockState] = ExecutionResult(
            halted=True,
            steps=1,
            final_state=state,
            error=None,
            traces=[],
        )
        with pytest.raises(dataclasses.FrozenInstanceError):
            result.steps = 99  # type: ignore[misc]

    def test_execution_result_halted_frozen(self) -> None:
        """halted field is immutable."""
        state = MockState(accumulator=0, pc=0, halted=True)
        result: ExecutionResult[MockState] = ExecutionResult(
            halted=True, steps=1, final_state=state, error=None, traces=[]
        )
        with pytest.raises(dataclasses.FrozenInstanceError):
            result.halted = False  # type: ignore[misc]

    def test_execution_result_traces_accessible(self) -> None:
        """traces list is accessible and preserves order."""
        t1 = StepTrace(pc_before=0, pc_after=1, mnemonic="NOP", description="NOP @ 0x000")
        t2 = StepTrace(pc_before=1, pc_after=2, mnemonic="HLT", description="HLT @ 0x001")
        state = MockState(accumulator=0, pc=2, halted=True)
        result: ExecutionResult[MockState] = ExecutionResult(
            halted=True, steps=2, final_state=state, error=None, traces=[t1, t2]
        )
        assert len(result.traces) == 2
        assert result.traces[0].mnemonic == "NOP"
        assert result.traces[1].mnemonic == "HLT"

    def test_execution_result_final_state_accessible(self) -> None:
        """final_state is accessible and carries the state snapshot."""
        state = MockState(accumulator=7, pc=3, halted=True)
        result: ExecutionResult[MockState] = ExecutionResult(
            halted=True, steps=3, final_state=state, error=None, traces=[]
        )
        assert result.final_state.accumulator == 7
        assert result.final_state.pc == 3

    def test_execution_result_empty_traces_is_valid(self) -> None:
        """Empty trace list is valid — some architectures don't record traces."""
        state = MockState(accumulator=0, pc=0, halted=True)
        result: ExecutionResult[MockState] = ExecutionResult(
            halted=True, steps=0, final_state=state, error=None, traces=[]
        )
        assert result.traces == []
        assert result.ok is True

    def test_execution_result_steps_count(self) -> None:
        """steps field records the total instruction count."""
        state = MockState(accumulator=0, pc=42, halted=True)
        result: ExecutionResult[MockState] = ExecutionResult(
            halted=True, steps=42, final_state=state, error=None, traces=[]
        )
        assert result.steps == 42

    def test_execution_result_error_message_preserved(self) -> None:
        """error string is stored verbatim."""
        state = MockState(accumulator=0, pc=0, halted=False)
        msg = "max_steps (9999) exceeded"
        result: ExecutionResult[MockState] = ExecutionResult(
            halted=False, steps=9999, final_state=state, error=msg, traces=[]
        )
        assert result.error == msg


# ===========================================================================
# TestSimulatorProtocol
# ===========================================================================


class TestSimulatorProtocol:
    """Structural subtyping — a class implements Simulator[StateT] without
    explicit inheritance.  This is Python's duck typing with types."""

    def test_mock_simulator_satisfies_protocol(self) -> None:
        """MockSimulator satisfies Simulator[MockState] structurally.

        The runtime_checkable version of Protocol allows isinstance() checks.
        We use a duck-typing check instead: verify all required methods exist.
        """
        sim = MockSimulator()
        assert hasattr(sim, "load")
        assert hasattr(sim, "step")
        assert hasattr(sim, "execute")
        assert hasattr(sim, "get_state")
        assert hasattr(sim, "reset")

    def test_no_explicit_inheritance_needed(self) -> None:
        """MockSimulator does NOT inherit from Simulator, yet it works."""
        assert issubclass(MockSimulator, Simulator)  # type: ignore[arg-type]

    def test_duck_typing_with_type_annotation(self) -> None:
        """Assigning MockSimulator to a Simulator[MockState] variable works."""
        sim: Simulator[MockState] = MockSimulator()  # type: ignore[assignment]
        result = sim.execute(bytes([0x01]))  # HLT
        assert result.ok is True

    def test_execute_returns_execution_result(self) -> None:
        """execute() returns an ExecutionResult."""
        sim = MockSimulator()
        result = sim.execute(bytes([0x01]))
        assert isinstance(result, ExecutionResult)

    def test_get_state_returns_state(self) -> None:
        """get_state() returns the architecture state object."""
        sim = MockSimulator()
        sim.load(bytes([0x01]))
        state = sim.get_state()
        assert isinstance(state, MockState)

    def test_step_returns_step_trace(self) -> None:
        """step() returns a StepTrace."""
        sim = MockSimulator()
        sim.load(bytes([0xD7, 0x01]))  # LDM 7, HLT
        trace = sim.step()
        assert isinstance(trace, StepTrace)
        assert trace.mnemonic == "LDM 7"

    def test_reset_clears_state(self) -> None:
        """reset() returns simulator to initial state."""
        sim = MockSimulator()
        sim.load(bytes([0xD5, 0x01]))  # LDM 5, HLT
        sim.step()
        sim.reset()
        state = sim.get_state()
        assert state.accumulator == 0
        assert state.pc == 0
        assert state.halted is False


# ===========================================================================
# TestEndToEndLoop
# ===========================================================================


class TestEndToEndLoop:
    """Simulate the compile → assemble → execute → assert cycle."""

    def test_hlt_only_program_is_ok(self) -> None:
        """A program that is just HLT halts cleanly."""
        sim = MockSimulator()
        result = sim.execute(bytes([0x01]))  # HLT
        assert result.ok is True
        assert result.halted is True
        assert result.error is None
        assert result.steps == 1

    def test_ldm_then_hlt(self) -> None:
        """LDM 7 followed by HLT leaves accumulator at 7."""
        sim = MockSimulator()
        result = sim.execute(bytes([0xD7, 0x01]))  # LDM 7, HLT
        assert result.ok is True
        assert result.final_state.accumulator == 7
        assert result.steps == 2

    def test_max_steps_exceeded_not_ok(self) -> None:
        """A program that never halts returns ok=False after max_steps."""
        # Infinite loop: just LDM 1 repeating (no HLT)
        # We use max_steps=3 so the test is fast
        sim = MockSimulator()
        # Trick MockSimulator: program of all 0xD1 (no HLT byte)
        program = bytes([0xD1] * 100)
        result = sim.execute(program, max_steps=3)
        assert result.ok is False
        assert result.halted is False
        assert result.error is not None
        assert "max_steps" in result.error
        assert result.steps == 3

    def test_traces_match_steps(self) -> None:
        """Number of traces equals number of steps."""
        sim = MockSimulator()
        result = sim.execute(bytes([0xD1, 0xD2, 0x01]))  # LDM 1, LDM 2, HLT
        assert len(result.traces) == result.steps

    def test_trace_mnemonics_are_correct(self) -> None:
        """Trace mnemonics reflect instruction order."""
        sim = MockSimulator()
        result = sim.execute(bytes([0xD3, 0x01]))  # LDM 3, HLT
        assert result.traces[0].mnemonic == "LDM 3"
        assert result.traces[1].mnemonic == "HLT"

    def test_trace_pc_before_advances(self) -> None:
        """pc_before in traces increases monotonically for sequential code."""
        sim = MockSimulator()
        result = sim.execute(bytes([0xD1, 0xD2, 0x01]))
        pcs = [t.pc_before for t in result.traces]
        assert pcs == sorted(pcs)

    def test_final_state_is_frozen(self) -> None:
        """final_state is a frozen snapshot — mutations raise FrozenInstanceError."""
        sim = MockSimulator()
        result = sim.execute(bytes([0xD5, 0x01]))
        with pytest.raises(dataclasses.FrozenInstanceError):
            result.final_state.accumulator = 0  # type: ignore[misc]
