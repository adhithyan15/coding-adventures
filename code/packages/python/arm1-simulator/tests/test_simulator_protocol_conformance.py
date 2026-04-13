"""Conformance tests for ARM1's simulator-protocol implementation.

These tests verify that ``ARM1`` correctly satisfies the
``Simulator[ARM1State]`` protocol — specifically:

    * ``get_state()`` returns an ``ARM1State`` with correct field values.
    * ``ARM1State`` is truly frozen (immutable).
    * ``execute()`` returns a well-formed ``ExecutionResult[ARM1State]``.
    * A simple halt program runs to completion cleanly.
    * Programs that exceed ``max_steps`` produce a descriptive error.
    * Register values are accessible through ``ExecutionResult.final_state``.

These tests are intentionally thin and focused — the heavy behavioural
correctness testing lives in ``test_arm1_simulator.py``.  Here we only care
that the protocol surface is wired up correctly.

Building a minimal test program
--------------------------------
The ARM1 has no assembler in this package, so we use the encoding helpers
(``encode_mov_imm``, ``encode_halt``, etc.) already exposed by the
``arm1_simulator`` module.  Programs are assembled with ``struct.pack``::

    import struct
    code = b"".join(struct.pack("<I", w) for w in [encode_mov_imm(...), encode_halt()])

"""

from __future__ import annotations

import dataclasses
import struct

import pytest

from arm1_simulator import (
    ARM1,
    COND_AL,
    COND_NE,
    MASK_32,
    MODE_SVC,
    OP_SUB,
    encode_alu_reg,
    encode_halt,
    encode_mov_imm,
)
from arm1_simulator.state import ARM1State
from simulator_protocol import ExecutionResult, StepTrace


# ---------------------------------------------------------------------------
# Helper
# ---------------------------------------------------------------------------


def assemble(*instruction_words: int) -> bytes:
    """Pack a sequence of 32-bit instruction words into a byte string.

    ARM encodes instructions in little-endian word order.

    Parameters
    ----------
    *instruction_words:
        One or more 32-bit integers produced by the ``encode_*`` helpers.

    Returns
    -------
    bytes:
        The corresponding machine-code bytes ready to pass to
        ``cpu.execute()`` or ``cpu.load_program()``.
    """
    return b"".join(struct.pack("<I", w & MASK_32) for w in instruction_words)


# ---------------------------------------------------------------------------
# get_state() tests
# ---------------------------------------------------------------------------


class TestGetStateReturnsARM1State:
    """get_state() must return an ARM1State instance."""

    def test_get_state_returns_arm1_state(self) -> None:
        """The value returned by get_state() must be an ARM1State."""
        cpu = ARM1(1024)
        state = cpu.get_state()
        assert isinstance(state, ARM1State), (
            f"Expected ARM1State, got {type(state).__name__}"
        )

    def test_get_state_pc_matches_cpu_pc(self) -> None:
        """state.pc must equal cpu.pc at the time of capture."""
        cpu = ARM1(1024)
        assert cpu.get_state().pc == cpu.pc

    def test_get_state_mode_is_svc_at_power_on(self) -> None:
        """The ARM1 boots into Supervisor (SVC) mode."""
        cpu = ARM1(256)
        state = cpu.get_state()
        assert state.mode == MODE_SVC

    def test_get_state_halted_false_initially(self) -> None:
        """A freshly-created CPU must not be halted."""
        cpu = ARM1(256)
        assert cpu.get_state().halted is False

    def test_get_state_registers_tuple_length(self) -> None:
        """registers must have exactly 16 entries (R0–R15)."""
        cpu = ARM1(256)
        state = cpu.get_state()
        assert len(state.registers) == 16

    def test_get_state_banked_fiq_length(self) -> None:
        """banked_fiq must have exactly 7 entries."""
        cpu = ARM1(256)
        assert len(cpu.get_state().banked_fiq) == 7

    def test_get_state_banked_irq_length(self) -> None:
        """banked_irq must have exactly 2 entries."""
        cpu = ARM1(256)
        assert len(cpu.get_state().banked_irq) == 2

    def test_get_state_banked_svc_length(self) -> None:
        """banked_svc must have exactly 2 entries."""
        cpu = ARM1(256)
        assert len(cpu.get_state().banked_svc) == 2

    def test_get_state_memory_is_bytes(self) -> None:
        """state.memory must be immutable bytes, not a bytearray."""
        cpu = ARM1(256)
        state = cpu.get_state()
        assert isinstance(state.memory, bytes)

    def test_get_state_memory_snapshot_is_independent(self) -> None:
        """Writing to cpu.memory after get_state() must not affect the snapshot."""
        cpu = ARM1(256)
        state = cpu.get_state()
        cpu.write_byte(0, 0xAB)
        # The snapshot must still show 0 at address 0
        assert state.memory[0] == 0

    def test_get_state_flags_all_false_at_power_on(self) -> None:
        """All condition flags are clear after reset."""
        cpu = ARM1(256)
        state = cpu.get_state()
        assert state.flags_n is False
        assert state.flags_z is False
        assert state.flags_c is False
        assert state.flags_v is False


# ---------------------------------------------------------------------------
# ARM1State frozen tests
# ---------------------------------------------------------------------------


class TestGetStateIsFrozen:
    """ARM1State must be immutable (frozen dataclass)."""

    def test_get_state_is_frozen(self) -> None:
        """Assigning to any field must raise FrozenInstanceError."""
        cpu = ARM1(256)
        state = cpu.get_state()
        with pytest.raises(dataclasses.FrozenInstanceError):
            state.pc = 99  # type: ignore[misc]

    def test_registers_tuple_is_immutable(self) -> None:
        """The registers field is a tuple, so item assignment is forbidden."""
        cpu = ARM1(256)
        state = cpu.get_state()
        with pytest.raises(TypeError):
            state.registers[0] = 42  # type: ignore[index]

    def test_banked_fiq_is_tuple(self) -> None:
        """banked_fiq is a tuple (not a list)."""
        cpu = ARM1(256)
        assert isinstance(cpu.get_state().banked_fiq, tuple)


# ---------------------------------------------------------------------------
# execute() return-type tests
# ---------------------------------------------------------------------------


class TestExecuteReturnsExecutionResult:
    """execute() must return an ExecutionResult[ARM1State]."""

    def test_execute_returns_execution_result(self) -> None:
        """Return type must be ExecutionResult."""
        cpu = ARM1(1024)
        code = assemble(encode_halt())
        result = cpu.execute(code)
        assert isinstance(result, ExecutionResult)

    def test_execute_final_state_is_arm1_state(self) -> None:
        """final_state inside ExecutionResult must be an ARM1State."""
        cpu = ARM1(1024)
        code = assemble(encode_halt())
        result = cpu.execute(code)
        assert isinstance(result.final_state, ARM1State)

    def test_execute_traces_are_step_trace_instances(self) -> None:
        """Every entry in result.traces must be a StepTrace."""
        cpu = ARM1(1024)
        code = assemble(encode_halt())
        result = cpu.execute(code)
        for trace in result.traces:
            assert isinstance(trace, StepTrace)


# ---------------------------------------------------------------------------
# Simple halt program
# ---------------------------------------------------------------------------


class TestExecuteSimpleHaltProgram:
    """A one-instruction program consisting of only HALT must succeed cleanly."""

    def test_execute_simple_halt_program(self) -> None:
        """HALT-only program: halted=True, ok=True, error=None."""
        cpu = ARM1(1024)
        code = assemble(encode_halt())
        result = cpu.execute(code)

        assert result.halted is True
        assert result.ok is True
        assert result.error is None

    def test_execute_halt_produces_one_trace(self) -> None:
        """A single HALT instruction produces exactly one StepTrace."""
        cpu = ARM1(1024)
        code = assemble(encode_halt())
        result = cpu.execute(code)
        assert result.steps == 1
        assert len(result.traces) == 1

    def test_execute_halt_trace_pc_before_is_zero(self) -> None:
        """The HALT instruction is at address 0, so pc_before must be 0."""
        cpu = ARM1(1024)
        code = assemble(encode_halt())
        result = cpu.execute(code)
        assert result.traces[0].pc_before == 0

    def test_execute_mnemonic_not_empty(self) -> None:
        """Every StepTrace must have a non-empty mnemonic."""
        cpu = ARM1(1024)
        code = assemble(encode_halt())
        result = cpu.execute(code)
        assert result.traces[0].mnemonic != ""

    def test_execute_description_not_empty(self) -> None:
        """Every StepTrace must have a non-empty description."""
        cpu = ARM1(1024)
        code = assemble(encode_halt())
        result = cpu.execute(code)
        assert result.traces[0].description != ""


# ---------------------------------------------------------------------------
# max_steps exceeded
# ---------------------------------------------------------------------------


class TestExecuteMaxStepsExceeded:
    """When the program runs longer than max_steps, the error field is set."""

    def test_execute_max_steps_exceeded(self) -> None:
        """A program that never halts must produce an error after max_steps."""
        cpu = ARM1(1024)
        # Unconditional branch to itself: infinite loop
        branch_self = (COND_AL << 28) | 0x0A000000 | ((-2 >> 2) & 0x00FFFFFF)
        code = assemble(branch_self)
        result = cpu.execute(code, max_steps=10)

        assert result.halted is False
        assert result.ok is False
        assert result.error is not None
        assert "max_steps" in result.error

    def test_execute_max_steps_step_count(self) -> None:
        """Step count must equal the requested max_steps limit."""
        cpu = ARM1(1024)
        branch_self = (COND_AL << 28) | 0x0A000000 | ((-2 >> 2) & 0x00FFFFFF)
        code = assemble(branch_self)
        result = cpu.execute(code, max_steps=5)

        assert result.steps == 5


# ---------------------------------------------------------------------------
# Register values accessible through final_state
# ---------------------------------------------------------------------------


class TestExecuteRegistersAccessible:
    """After execute(), final_state.registers must reflect the computation."""

    def test_execute_registers_accessible(self) -> None:
        """MOV R0, #42 followed by HALT: final_state.registers[0] == 42."""
        cpu = ARM1(1024)
        code = assemble(
            encode_mov_imm(COND_AL, 0, 42),
            encode_halt(),
        )
        result = cpu.execute(code)

        assert result.ok, f"Program failed: {result.error}"
        assert result.final_state.registers[0] == 42

    def test_execute_multi_register_program(self) -> None:
        """Compute 1 + 2 = 3 and verify all three destination registers."""
        cpu = ARM1(1024)
        code = assemble(
            encode_mov_imm(COND_AL, 0, 1),
            encode_mov_imm(COND_AL, 1, 2),
            encode_alu_reg(COND_AL, 0x4, 0, 2, 0, 1),  # ADD R2, R0, R1
            encode_halt(),
        )
        result = cpu.execute(code)

        assert result.ok, f"Program failed: {result.error}"
        assert result.final_state.registers[0] == 1
        assert result.final_state.registers[1] == 2
        assert result.final_state.registers[2] == 3

    def test_execute_final_state_halted_true(self) -> None:
        """final_state.halted must be True after a clean halt."""
        cpu = ARM1(1024)
        code = assemble(encode_halt())
        result = cpu.execute(code)
        assert result.final_state.halted is True

    def test_execute_resets_before_running(self) -> None:
        """execute() must reset state between calls so prior runs don't leak."""
        cpu = ARM1(1024)
        # First run: set R0 = 99
        code_a = assemble(encode_mov_imm(COND_AL, 0, 99), encode_halt())
        cpu.execute(code_a)

        # Second run: only HALT — R0 must be 0, not 99
        code_b = assemble(encode_halt())
        result = cpu.execute(code_b)
        assert result.final_state.registers[0] == 0
