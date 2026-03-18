"""Tests for the ARM simulator."""

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
