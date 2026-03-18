"""Tests for the CPU with a mock instruction set.

To test the CPU independently of any real ISA, we create a tiny mock
instruction set with just 3 instructions:

    0x01 XX: LOAD_IMM — load immediate value XX into register 0
    0x02 XX: ADD_IMM  — add immediate value XX to register 0
    0x00 00: HALT     — stop execution

Each instruction is 4 bytes (padded with zeros for alignment).
"""

import pytest

from cpu_simulator.cpu import CPU
from cpu_simulator.memory import Memory
from cpu_simulator.pipeline import DecodeResult, ExecuteResult
from cpu_simulator.registers import RegisterFile


# --- Mock ISA ---


class MockDecoder:
    """Decodes our tiny 3-instruction mock ISA."""

    def decode(self, raw_instruction: int, pc: int) -> DecodeResult:
        opcode = raw_instruction & 0xFF
        arg = (raw_instruction >> 8) & 0xFF

        if opcode == 0x00:
            return DecodeResult(mnemonic="HALT", fields={}, raw_instruction=raw_instruction)
        elif opcode == 0x01:
            return DecodeResult(mnemonic="LOAD_IMM", fields={"value": arg}, raw_instruction=raw_instruction)
        elif opcode == 0x02:
            return DecodeResult(mnemonic="ADD_IMM", fields={"value": arg}, raw_instruction=raw_instruction)
        else:
            return DecodeResult(mnemonic="UNKNOWN", fields={"opcode": opcode}, raw_instruction=raw_instruction)


class MockExecutor:
    """Executes our tiny 3-instruction mock ISA."""

    def execute(
        self,
        decoded: DecodeResult,
        registers: RegisterFile,
        memory: Memory,
        pc: int,
    ) -> ExecuteResult:
        if decoded.mnemonic == "HALT":
            return ExecuteResult(
                description="Halt execution",
                registers_changed={},
                memory_changed={},
                next_pc=pc,
                halted=True,
            )
        elif decoded.mnemonic == "LOAD_IMM":
            value = decoded.fields["value"]
            registers.write(0, value)
            return ExecuteResult(
                description=f"R0 = {value}",
                registers_changed={"R0": value},
                memory_changed={},
                next_pc=pc + 4,
            )
        elif decoded.mnemonic == "ADD_IMM":
            value = decoded.fields["value"]
            old = registers.read(0)
            new = old + value
            registers.write(0, new)
            return ExecuteResult(
                description=f"R0 = {old} + {value} = {new}",
                registers_changed={"R0": new},
                memory_changed={},
                next_pc=pc + 4,
            )
        else:
            return ExecuteResult(
                description="Unknown instruction",
                registers_changed={},
                memory_changed={},
                next_pc=pc + 4,
            )


# --- Helper ---


def _make_instruction(opcode: int, arg: int = 0) -> bytes:
    """Encode a mock instruction as 4 little-endian bytes."""
    value = opcode | (arg << 8)
    return value.to_bytes(4, byteorder="little")


def _make_cpu() -> CPU:
    """Create a CPU with our mock ISA."""
    return CPU(
        decoder=MockDecoder(),
        executor=MockExecutor(),
        num_registers=4,
        bit_width=32,
    )


# --- Tests ---


class TestCPUStep:
    def test_single_load(self) -> None:
        """LOAD_IMM 5 should set R0 = 5."""
        cpu = _make_cpu()
        program = _make_instruction(0x01, 5) + _make_instruction(0x00)
        cpu.load_program(program)

        trace = cpu.step()
        assert trace.decode.mnemonic == "LOAD_IMM"
        assert trace.execute.registers_changed == {"R0": 5}
        assert cpu.registers.read(0) == 5
        assert cpu.pc == 4

    def test_load_and_add(self) -> None:
        """LOAD_IMM 1, ADD_IMM 2 should give R0 = 3."""
        cpu = _make_cpu()
        program = (
            _make_instruction(0x01, 1)  # R0 = 1
            + _make_instruction(0x02, 2)  # R0 = 1 + 2 = 3
            + _make_instruction(0x00)  # HALT
        )
        cpu.load_program(program)

        trace1 = cpu.step()
        assert trace1.decode.mnemonic == "LOAD_IMM"
        assert cpu.registers.read(0) == 1

        trace2 = cpu.step()
        assert trace2.decode.mnemonic == "ADD_IMM"
        assert trace2.execute.description == "R0 = 1 + 2 = 3"
        assert cpu.registers.read(0) == 3

    def test_halt(self) -> None:
        cpu = _make_cpu()
        cpu.load_program(_make_instruction(0x00))
        trace = cpu.step()
        assert trace.execute.halted is True
        assert cpu.halted is True

    def test_step_after_halt_raises(self) -> None:
        cpu = _make_cpu()
        cpu.load_program(_make_instruction(0x00))
        cpu.step()  # Execute HALT
        with pytest.raises(RuntimeError, match="halted"):
            cpu.step()


class TestCPURun:
    def test_run_simple_program(self) -> None:
        """Run: LOAD 1, ADD 2, HALT → R0 should be 3."""
        cpu = _make_cpu()
        program = (
            _make_instruction(0x01, 1)
            + _make_instruction(0x02, 2)
            + _make_instruction(0x00)
        )
        cpu.load_program(program)

        traces = cpu.run()
        assert len(traces) == 3  # LOAD, ADD, HALT
        assert cpu.registers.read(0) == 3
        assert cpu.halted is True

    def test_run_max_steps(self) -> None:
        """Run with max_steps should stop even without HALT."""
        cpu = _make_cpu()
        # Infinite loop: just LOAD_IMM 1 repeated (no HALT)
        program = _make_instruction(0x01, 1) * 100
        cpu.load_program(program)

        traces = cpu.run(max_steps=5)
        assert len(traces) == 5
        assert cpu.halted is False


class TestPipelineTrace:
    def test_trace_has_all_stages(self) -> None:
        cpu = _make_cpu()
        cpu.load_program(_make_instruction(0x01, 42) + _make_instruction(0x00))
        trace = cpu.step()

        assert trace.fetch.pc == 0
        assert trace.fetch.raw_instruction != 0
        assert trace.decode.mnemonic == "LOAD_IMM"
        assert trace.execute.description == "R0 = 42"
        assert trace.cycle == 0

    def test_trace_format_pipeline(self) -> None:
        """The pipeline format should contain all three stage names."""
        cpu = _make_cpu()
        cpu.load_program(_make_instruction(0x01, 1) + _make_instruction(0x00))
        trace = cpu.step()
        output = trace.format_pipeline()
        assert "FETCH" in output
        assert "DECODE" in output
        assert "EXECUTE" in output
        assert "Cycle 0" in output

    def test_register_snapshot(self) -> None:
        cpu = _make_cpu()
        cpu.load_program(_make_instruction(0x01, 7) + _make_instruction(0x00))
        trace = cpu.step()
        assert trace.register_snapshot["R0"] == 7


class TestCPUState:
    def test_initial_state(self) -> None:
        cpu = _make_cpu()
        state = cpu.state
        assert state.pc == 0
        assert state.halted is False
        assert state.cycle == 0
        assert all(v == 0 for v in state.registers.values())

    def test_state_after_execution(self) -> None:
        cpu = _make_cpu()
        cpu.load_program(_make_instruction(0x01, 5) + _make_instruction(0x00))
        cpu.step()
        state = cpu.state
        assert state.pc == 4
        assert state.registers["R0"] == 5
        assert state.cycle == 1
