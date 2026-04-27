"""Conformance tests for RiscVSimulator's simulator-protocol implementation.

These tests verify that ``RiscVSimulator`` correctly satisfies the
``Simulator[RiscVState]`` protocol — specifically:

    * ``get_state()`` returns a ``RiscVState`` with correct field values.
    * ``RiscVState`` is truly frozen (immutable).
    * ``execute()`` returns a well-formed ``ExecutionResult[RiscVState]``.
    * A simple ecall-halt program runs to completion cleanly.
    * Programs that exceed ``max_steps`` produce a descriptive error.
    * Register values are accessible through ``ExecutionResult.final_state``.

These tests are intentionally thin and focused — the heavy behavioural
correctness testing lives in ``test_riscv.py``.  Here we only care that the
protocol surface is wired up correctly.

RISC-V halt convention
-----------------------
Unlike the ARM1 (which has a dedicated HALT opcode), RISC-V halts via
``ecall`` with ``mtvec == 0``.  When the CSR file's mtvec holds 0, the
executor interprets ``ecall`` as a halt signal and sets ``cpu.halted = True``.

This is the default state of a fresh ``RiscVSimulator`` — mtvec is 0, so the
first ``ecall`` halts execution.  All simple test programs end with
``encode_ecall()`` as their halt instruction.
"""

from __future__ import annotations

import dataclasses

import pytest

from riscv_simulator import RiscVSimulator, RiscVState
from riscv_simulator.encoding import (
    assemble,
    encode_addi,
    encode_add,
    encode_ecall,
)
from simulator_protocol import ExecutionResult, StepTrace


# ---------------------------------------------------------------------------
# get_state() tests
# ---------------------------------------------------------------------------


class TestGetStateReturnsRiscVState:
    """get_state() must return a RiscVState instance."""

    def test_get_state_returns_riscv_state(self) -> None:
        """The value returned by get_state() must be a RiscVState."""
        sim = RiscVSimulator(1024)
        state = sim.get_state()
        assert isinstance(state, RiscVState), (
            f"Expected RiscVState, got {type(state).__name__}"
        )

    def test_get_state_pc_is_zero_initially(self) -> None:
        """PC must be 0 before any program is loaded or run."""
        sim = RiscVSimulator(1024)
        assert sim.get_state().pc == 0

    def test_get_state_registers_tuple_length(self) -> None:
        """registers must have exactly 32 entries (x0–x31)."""
        sim = RiscVSimulator(1024)
        state = sim.get_state()
        assert len(state.registers) == 32

    def test_get_state_x0_is_always_zero(self) -> None:
        """x0 is hardwired to 0 in RISC-V — must always read as 0."""
        sim = RiscVSimulator(1024)
        state = sim.get_state()
        assert state.registers[0] == 0

    def test_get_state_halted_false_initially(self) -> None:
        """A freshly-created simulator must not be halted."""
        sim = RiscVSimulator(1024)
        assert sim.get_state().halted is False

    def test_get_state_csr_mtvec_zero_initially(self) -> None:
        """mtvec starts at 0 (meaning ecall acts as a halt)."""
        sim = RiscVSimulator(1024)
        assert sim.get_state().csr_mtvec == 0

    def test_get_state_memory_is_bytes(self) -> None:
        """state.memory must be immutable bytes, not a bytearray."""
        sim = RiscVSimulator(1024)
        state = sim.get_state()
        assert isinstance(state.memory, bytes)

    def test_get_state_memory_length_matches_constructor(self) -> None:
        """state.memory length must match the memory_size passed to __init__."""
        sim = RiscVSimulator(4096)
        state = sim.get_state()
        assert len(state.memory) == 4096

    def test_get_state_memory_snapshot_is_independent(self) -> None:
        """Writing to the simulator's memory after get_state() must not
        affect the snapshot."""
        sim = RiscVSimulator(1024)
        state = sim.get_state()
        sim.cpu.memory.write_byte(0, 0xAB)
        # The snapshot must still show 0 at address 0
        assert state.memory[0] == 0

    def test_get_state_all_csrs_zero_initially(self) -> None:
        """All M-mode CSRs start at 0 after construction."""
        sim = RiscVSimulator(1024)
        state = sim.get_state()
        assert state.csr_mstatus == 0
        assert state.csr_mtvec == 0
        assert state.csr_mscratch == 0
        assert state.csr_mepc == 0
        assert state.csr_mcause == 0


# ---------------------------------------------------------------------------
# RiscVState frozen tests
# ---------------------------------------------------------------------------


class TestGetStateIsFrozen:
    """RiscVState must be immutable (frozen dataclass)."""

    def test_get_state_is_frozen(self) -> None:
        """Assigning to any field must raise FrozenInstanceError."""
        sim = RiscVSimulator(1024)
        state = sim.get_state()
        with pytest.raises(dataclasses.FrozenInstanceError):
            state.pc = 99  # type: ignore[misc]

    def test_registers_tuple_is_immutable(self) -> None:
        """The registers field is a tuple — item assignment is forbidden."""
        sim = RiscVSimulator(1024)
        state = sim.get_state()
        with pytest.raises(TypeError):
            state.registers[1] = 42  # type: ignore[index]

    def test_registers_is_tuple_not_list(self) -> None:
        """registers must be a tuple, not a list."""
        sim = RiscVSimulator(1024)
        assert isinstance(sim.get_state().registers, tuple)


# ---------------------------------------------------------------------------
# execute() return-type tests
# ---------------------------------------------------------------------------


class TestExecuteReturnsExecutionResult:
    """execute() must return an ExecutionResult[RiscVState]."""

    def test_execute_returns_execution_result(self) -> None:
        """Return type must be ExecutionResult."""
        sim = RiscVSimulator(1024)
        code = assemble([encode_ecall()])
        result = sim.execute(code)
        assert isinstance(result, ExecutionResult)

    def test_execute_final_state_is_riscv_state(self) -> None:
        """final_state inside ExecutionResult must be a RiscVState."""
        sim = RiscVSimulator(1024)
        code = assemble([encode_ecall()])
        result = sim.execute(code)
        assert isinstance(result.final_state, RiscVState)

    def test_execute_traces_are_step_trace_instances(self) -> None:
        """Every entry in result.traces must be a StepTrace."""
        sim = RiscVSimulator(1024)
        code = assemble([encode_ecall()])
        result = sim.execute(code)
        for trace in result.traces:
            assert isinstance(trace, StepTrace)


# ---------------------------------------------------------------------------
# Simple ecall-halt program
# ---------------------------------------------------------------------------


class TestExecuteSimpleHaltProgram:
    """A one-instruction ecall program must succeed cleanly."""

    def test_execute_simple_halt_program(self) -> None:
        """ecall-only program: halted=True, ok=True, error=None."""
        sim = RiscVSimulator(1024)
        code = assemble([encode_ecall()])
        result = sim.execute(code)

        assert result.halted is True
        assert result.ok is True
        assert result.error is None

    def test_execute_halt_produces_one_trace(self) -> None:
        """A single ecall instruction produces exactly one StepTrace."""
        sim = RiscVSimulator(1024)
        code = assemble([encode_ecall()])
        result = sim.execute(code)
        assert result.steps == 1
        assert len(result.traces) == 1

    def test_execute_halt_trace_pc_before_is_zero(self) -> None:
        """The ecall instruction is at address 0, so pc_before must be 0."""
        sim = RiscVSimulator(1024)
        code = assemble([encode_ecall()])
        result = sim.execute(code)
        assert result.traces[0].pc_before == 0

    def test_execute_mnemonic_not_empty(self) -> None:
        """Every StepTrace must have a non-empty mnemonic."""
        sim = RiscVSimulator(1024)
        code = assemble([encode_ecall()])
        result = sim.execute(code)
        assert result.traces[0].mnemonic != ""

    def test_execute_description_not_empty(self) -> None:
        """Every StepTrace must have a non-empty description."""
        sim = RiscVSimulator(1024)
        code = assemble([encode_ecall()])
        result = sim.execute(code)
        assert result.traces[0].description != ""


# ---------------------------------------------------------------------------
# max_steps exceeded
# ---------------------------------------------------------------------------


class TestExecuteMaxStepsExceeded:
    """When the program runs longer than max_steps, the error field is set."""

    def test_execute_max_steps_exceeded(self) -> None:
        """A program that never halts must produce an error after max_steps.

        We craft an infinite loop: addi x0, x0, 0 (NOP) repeated.  Since
        x0 is hardwired to 0, this does nothing and loops forever.  We use
        a small max_steps so the test is fast.
        """
        sim = RiscVSimulator(65536)
        # NOP loop: addi x0, x0, 0 repeated many times — never halts
        nop = encode_addi(0, 0, 0)
        # Fill with NOPs to avoid running off the end of the program
        code = assemble([nop] * 1000)
        result = sim.execute(code, max_steps=10)

        assert result.halted is False
        assert result.ok is False
        assert result.error is not None
        assert "max_steps" in result.error

    def test_execute_max_steps_step_count(self) -> None:
        """Step count must equal the requested max_steps limit."""
        sim = RiscVSimulator(65536)
        nop = encode_addi(0, 0, 0)
        code = assemble([nop] * 1000)
        result = sim.execute(code, max_steps=7)

        assert result.steps == 7


# ---------------------------------------------------------------------------
# Register values accessible through final_state
# ---------------------------------------------------------------------------


class TestExecuteRegistersAccessible:
    """After execute(), final_state.registers must reflect the computation."""

    def test_execute_registers_accessible(self) -> None:
        """addi x1, x0, 42 followed by ecall: final_state.registers[1] == 42."""
        sim = RiscVSimulator(65536)
        code = assemble([
            encode_addi(1, 0, 42),
            encode_ecall(),
        ])
        result = sim.execute(code)

        assert result.ok, f"Program failed: {result.error}"
        assert result.final_state.registers[1] == 42

    def test_execute_multi_register_program(self) -> None:
        """Compute 3 + 7 = 10 and verify the destination register."""
        sim = RiscVSimulator(65536)
        code = assemble([
            encode_addi(1, 0, 3),   # x1 = 3
            encode_addi(2, 0, 7),   # x2 = 7
            encode_add(3, 1, 2),    # x3 = x1 + x2 = 10
            encode_ecall(),
        ])
        result = sim.execute(code)

        assert result.ok, f"Program failed: {result.error}"
        assert result.final_state.registers[1] == 3
        assert result.final_state.registers[2] == 7
        assert result.final_state.registers[3] == 10

    def test_execute_x0_remains_zero_after_write_attempt(self) -> None:
        """x0 must always read as 0 even after an addi that targets it."""
        sim = RiscVSimulator(65536)
        code = assemble([
            encode_addi(0, 0, 99),  # Attempt to write 99 into x0 (silently ignored)
            encode_ecall(),
        ])
        result = sim.execute(code)

        assert result.ok, f"Program failed: {result.error}"
        assert result.final_state.registers[0] == 0

    def test_execute_final_state_halted_true(self) -> None:
        """final_state.halted must be True after a clean halt."""
        sim = RiscVSimulator(65536)
        code = assemble([encode_ecall()])
        result = sim.execute(code)
        assert result.final_state.halted is True

    def test_execute_resets_before_running(self) -> None:
        """execute() must reset state between calls so prior runs do not leak."""
        sim = RiscVSimulator(65536)
        # First run: set x1 = 99
        code_a = assemble([encode_addi(1, 0, 99), encode_ecall()])
        sim.execute(code_a)

        # Second run: only ecall — x1 must be 0, not 99
        code_b = assemble([encode_ecall()])
        result = sim.execute(code_b)
        assert result.final_state.registers[1] == 0
