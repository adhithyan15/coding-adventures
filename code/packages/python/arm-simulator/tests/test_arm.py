"""Tests for the ARM simulator.

Test organization:
    - TestEncoding          : Instruction encoding correctness
    - TestDecoder           : Decoder field extraction
    - TestARMSimulator      : End-to-end programs via run()
    - TestSimulatorProtocol : simulator-protocol conformance (get_state, execute)
"""

from __future__ import annotations

import pytest

from arm_simulator.simulator import (
    ARMDecoder,
    ARMSimulator,
    assemble,
    encode_add,
    encode_hlt,
    encode_mov_imm,
    encode_sub,
)


class TestEncoding:
    """Verify instruction encoding matches known ARM binary values."""

    def test_encode_mov_r0_1(self) -> None:
        """MOV R0, #1 should encode to 0xE3A00001.

        Breakdown: cond=1110 00 I=1 opcode=1101 S=0 Rn=0000 Rd=0000 imm=00000001
        """
        assert encode_mov_imm(0, 1) == 0xE3A00001

    def test_encode_mov_r1_2(self) -> None:
        """MOV R1, #2 should encode to 0xE3A01002."""
        assert encode_mov_imm(1, 2) == 0xE3A01002

    def test_encode_add_r2_r0_r1(self) -> None:
        """ADD R2, R0, R1 should encode to 0xE0802001.

        Breakdown: cond=1110 00 I=0 opcode=0100 S=0 Rn=0000 Rd=0010 Rm=0001
        """
        assert encode_add(2, 0, 1) == 0xE0802001

    def test_encode_sub_r2_r0_r1(self) -> None:
        """SUB R2, R0, R1 should encode to 0xE0402001.

        Breakdown: cond=1110 00 I=0 opcode=0010 S=0 Rn=0000 Rd=0010 Rm=0001
        """
        assert encode_sub(2, 0, 1) == 0xE0402001

    def test_encode_hlt(self) -> None:
        """HLT should encode to 0xFFFFFFFF."""
        assert encode_hlt() == 0xFFFFFFFF


class TestDecoder:
    """Verify the decoder correctly extracts fields from binary instructions."""

    def test_decode_mov_imm(self) -> None:
        """MOV R0, #1 should decode with rd=0 and imm=1."""
        decoder = ARMDecoder()
        result = decoder.decode(0xE3A00001, pc=0)
        assert result.mnemonic == "mov"
        assert result.fields["rd"] == 0
        assert result.fields["imm"] == 1
        assert result.fields["i_bit"] == 1
        assert result.fields["opcode"] == 0b1101

    def test_decode_add_reg(self) -> None:
        """ADD R2, R0, R1 should decode with rd=2, rn=0, rm=1."""
        decoder = ARMDecoder()
        result = decoder.decode(0xE0802001, pc=0)
        assert result.mnemonic == "add"
        assert result.fields["rd"] == 2
        assert result.fields["rn"] == 0
        assert result.fields["rm"] == 1
        assert result.fields["i_bit"] == 0

    def test_decode_sub_reg(self) -> None:
        """SUB R2, R0, R1 should decode with rd=2, rn=0, rm=1."""
        decoder = ARMDecoder()
        result = decoder.decode(0xE0402001, pc=0)
        assert result.mnemonic == "sub"
        assert result.fields["rd"] == 2
        assert result.fields["rn"] == 0
        assert result.fields["rm"] == 1

    def test_decode_hlt(self) -> None:
        """HLT (0xFFFFFFFF) should decode to mnemonic 'hlt'."""
        decoder = ARMDecoder()
        result = decoder.decode(0xFFFFFFFF, pc=0)
        assert result.mnemonic == "hlt"

    def test_decode_condition_field(self) -> None:
        """All normal instructions should have condition code AL (0b1110)."""
        decoder = ARMDecoder()
        result = decoder.decode(encode_mov_imm(0, 42), pc=0)
        assert result.fields["cond"] == 0b1110


class TestARMSimulator:
    """End-to-end tests running actual ARM programs."""

    def test_x_equals_1_plus_2(self) -> None:
        """The target program: x = 1 + 2 -> R2 should be 3.

        Program:
            MOV R0, #1       ; R0 = 1
            MOV R1, #2       ; R1 = 2
            ADD R2, R0, R1   ; R2 = R0 + R1 = 3
            HLT              ; halt
        """
        sim = ARMSimulator()
        program = assemble([
            encode_mov_imm(0, 1),
            encode_mov_imm(1, 2),
            encode_add(2, 0, 1),
            encode_hlt(),
        ])
        traces = sim.run(program)

        assert len(traces) == 4
        assert sim.cpu.registers.read(0) == 1
        assert sim.cpu.registers.read(1) == 2
        assert sim.cpu.registers.read(2) == 3
        assert sim.cpu.halted is True

    def test_subtraction(self) -> None:
        """SUB R2, R0, R1 with R0=10, R1=3 should give R2=7."""
        sim = ARMSimulator()
        program = assemble([
            encode_mov_imm(0, 10),
            encode_mov_imm(1, 3),
            encode_sub(2, 0, 1),
            encode_hlt(),
        ])
        sim.run(program)
        assert sim.cpu.registers.read(2) == 7

    def test_add_large_numbers(self) -> None:
        """ADD 100 + 200 = 300."""
        sim = ARMSimulator()
        program = assemble([
            encode_mov_imm(0, 100),
            encode_mov_imm(1, 200),
            encode_add(2, 0, 1),
            encode_hlt(),
        ])
        sim.run(program)
        assert sim.cpu.registers.read(2) == 300

    def test_pipeline_trace_visible(self) -> None:
        """Each step should produce a visible pipeline trace."""
        sim = ARMSimulator()
        program = assemble([
            encode_mov_imm(0, 7),
            encode_hlt(),
        ])
        sim.cpu.load_program(program)
        trace = sim.step()

        assert trace.fetch.pc == 0
        assert trace.decode.mnemonic == "mov"
        assert trace.decode.fields["imm"] == 7
        assert "R0" in trace.execute.registers_changed
        assert trace.execute.registers_changed["R0"] == 7

    def test_pipeline_format(self) -> None:
        """The pipeline format should show all three stages."""
        sim = ARMSimulator()
        program = assemble([encode_mov_imm(0, 1), encode_hlt()])
        sim.cpu.load_program(program)
        trace = sim.step()
        output = trace.format_pipeline()
        assert "FETCH" in output
        assert "DECODE" in output
        assert "EXECUTE" in output
        assert "mov" in output

    def test_sixteen_registers(self) -> None:
        """ARM should have 16 registers available."""
        sim = ARMSimulator()
        assert sim.cpu.registers.num_registers == 16


# ---------------------------------------------------------------------------
# simulator-protocol conformance tests
# ---------------------------------------------------------------------------


class TestSimulatorProtocol:
    """Verify ARMSimulator conforms to the Simulator[ARMState] protocol.

    These tests exercise ``get_state()``, ``execute()``, ``load()``, and
    ``reset()`` — the four new methods added for protocol conformance.
    """

    def test_get_state_returns_arm_state(self) -> None:
        """get_state() must return an ARMState instance."""
        from arm_simulator import ARMState

        sim = ARMSimulator()
        state = sim.get_state()
        assert isinstance(state, ARMState)

    def test_get_state_is_frozen(self) -> None:
        """ARMState must be immutable — assignment raises FrozenInstanceError."""
        import dataclasses

        sim = ARMSimulator()
        state = sim.get_state()
        with pytest.raises(dataclasses.FrozenInstanceError):
            state.pc = 99  # type: ignore[misc]

    def test_get_state_initial_values(self) -> None:
        """A fresh simulator's state snapshot must have all-zero registers and PC=0."""
        sim = ARMSimulator()
        state = sim.get_state()
        assert state.pc == 0
        assert state.halted is False
        assert len(state.registers) == 16
        assert all(r == 0 for r in state.registers)
        assert len(state.flags) == 4
        assert len(state.memory) == 65536

    def test_get_state_reflects_current_registers(self) -> None:
        """After running a program, get_state() must reflect the final register values."""
        sim = ARMSimulator()
        program = assemble([
            encode_mov_imm(0, 42),   # R0 = 42
            encode_hlt(),
        ])
        sim.run(program)
        state = sim.get_state()
        assert state.registers[0] == 42
        assert state.halted is True

    def test_get_state_memory_is_immutable_snapshot(self) -> None:
        """Mutating the simulator's memory after get_state() must not affect the snapshot."""
        sim = ARMSimulator()
        state_before = sim.get_state()
        # Write directly to the CPU memory (simulating a mid-run mutation)
        sim.cpu.memory._data[0] = 0xAB
        state_after = sim.get_state()
        # state_before must be unchanged (bytes is immutable)
        assert state_before.memory[0] == 0x00
        # state_after reflects the mutation
        assert state_after.memory[0] == 0xAB

    def test_execute_returns_execution_result(self) -> None:
        """execute() must return an ExecutionResult with the expected attributes."""
        from simulator_protocol import ExecutionResult

        sim = ARMSimulator()
        result = sim.execute(assemble([encode_hlt()]))
        assert isinstance(result, ExecutionResult)
        assert hasattr(result, "halted")
        assert hasattr(result, "steps")
        assert hasattr(result, "final_state")
        assert hasattr(result, "error")
        assert hasattr(result, "traces")

    def test_execute_simple_program_halts(self) -> None:
        """A program ending in HLT must produce result.ok == True."""
        sim = ARMSimulator()
        program = assemble([
            encode_mov_imm(0, 1),    # R0 = 1
            encode_mov_imm(1, 2),    # R1 = 2
            encode_add(2, 0, 1),     # R2 = R0 + R1 = 3
            encode_hlt(),
        ])
        result = sim.execute(program)
        assert result.ok is True
        assert result.halted is True
        assert result.error is None
        assert result.final_state.registers[2] == 3

    def test_execute_max_steps_exceeded(self) -> None:
        """When max_steps is hit without HLT, result.ok must be False with an error."""
        sim = ARMSimulator()
        # A program that loops: MOV R0, #1 repeated many times, no HLT.
        # With max_steps=3 it will stop before naturally halting.
        many_movs = [encode_mov_imm(0, 1)] * 10
        program = assemble(many_movs + [encode_hlt()])
        result = sim.execute(program, max_steps=3)
        assert result.ok is False
        assert result.halted is False
        assert result.error is not None
        assert "max_steps" in result.error
        assert result.steps == 3

    def test_execute_final_state_accessible(self) -> None:
        """final_state on ExecutionResult must expose ARMState fields."""
        from arm_simulator import ARMState

        sim = ARMSimulator()
        program = assemble([
            encode_mov_imm(0, 10),   # R0 = 10
            encode_mov_imm(1, 5),    # R1 = 5
            encode_sub(2, 0, 1),     # R2 = R0 - R1 = 5
            encode_hlt(),
        ])
        result = sim.execute(program)
        state = result.final_state
        assert isinstance(state, ARMState)
        assert state.registers[0] == 10
        assert state.registers[1] == 5
        assert state.registers[2] == 5
        assert state.halted is True

    def test_execute_traces_match_steps(self) -> None:
        """The length of result.traces must equal result.steps."""
        sim = ARMSimulator()
        program = assemble([
            encode_mov_imm(0, 1),
            encode_mov_imm(1, 2),
            encode_hlt(),
        ])
        result = sim.execute(program)
        assert result.steps == 3
        assert len(result.traces) == 3

    def test_execute_trace_has_correct_structure(self) -> None:
        """Each StepTrace must have pc_before, pc_after, mnemonic, description."""
        from simulator_protocol import StepTrace

        sim = ARMSimulator()
        result = sim.execute(assemble([encode_hlt()]))
        assert len(result.traces) == 1
        trace = result.traces[0]
        assert isinstance(trace, StepTrace)
        assert trace.pc_before == 0
        assert trace.pc_after == 0  # HLT does not advance PC
        assert trace.mnemonic == "hlt"
        assert "0x" in trace.description

    def test_execute_resets_between_calls(self) -> None:
        """Calling execute() twice must produce independent results."""
        sim = ARMSimulator()
        r1 = sim.execute(assemble([encode_mov_imm(0, 7), encode_hlt()]))
        r2 = sim.execute(assemble([encode_mov_imm(0, 99), encode_hlt()]))
        assert r1.final_state.registers[0] == 7
        assert r2.final_state.registers[0] == 99

    def test_reset_clears_all_state(self) -> None:
        """reset() must zero all registers and PC."""
        sim = ARMSimulator()
        program = assemble([encode_mov_imm(3, 0xFF), encode_hlt()])
        sim.run(program)
        assert sim.cpu.registers.read(3) == 0xFF
        sim.reset()
        state = sim.get_state()
        assert state.registers[3] == 0
        assert state.pc == 0
        assert state.halted is False

    def test_load_sets_program_memory(self) -> None:
        """load() must place bytes into memory and allow manual execution."""
        sim = ARMSimulator()
        sim.reset()
        program = assemble([encode_mov_imm(0, 5), encode_hlt()])
        sim.load(program)
        # Manually run until halted
        while not sim.cpu.halted:
            sim.cpu.step()
        assert sim.cpu.registers.read(0) == 5
