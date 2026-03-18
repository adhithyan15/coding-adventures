"""Tests for the RISC-V RV32I simulator."""

import pytest

from riscv_simulator.simulator import (
    RiscVDecoder,
    RiscVSimulator,
    assemble,
    encode_add,
    encode_addi,
    encode_ecall,
)


class TestEncoding:
    """Verify instruction encoding matches known RISC-V binary values."""

    def test_encode_addi_x1_x0_1(self) -> None:
        """addi x1, x0, 1 should encode to 0x00100093."""
        assert encode_addi(1, 0, 1) == 0x00100093

    def test_encode_addi_x2_x0_2(self) -> None:
        """addi x2, x0, 2 should encode to 0x00200113."""
        assert encode_addi(2, 0, 2) == 0x00200113

    def test_encode_add_x3_x1_x2(self) -> None:
        """add x3, x1, x2 should encode to 0x002081B3."""
        assert encode_add(3, 1, 2) == 0x002081B3

    def test_encode_ecall(self) -> None:
        """ecall should encode to 0x00000073."""
        assert encode_ecall() == 0x00000073


class TestDecoder:
    """Verify the decoder correctly extracts fields from binary instructions."""

    def test_decode_addi(self) -> None:
        decoder = RiscVDecoder()
        result = decoder.decode(0x00100093, pc=0)
        assert result.mnemonic == "addi"
        assert result.fields["rd"] == 1
        assert result.fields["rs1"] == 0
        assert result.fields["imm"] == 1

    def test_decode_add(self) -> None:
        decoder = RiscVDecoder()
        result = decoder.decode(0x002081B3, pc=0)
        assert result.mnemonic == "add"
        assert result.fields["rd"] == 3
        assert result.fields["rs1"] == 1
        assert result.fields["rs2"] == 2

    def test_decode_ecall(self) -> None:
        decoder = RiscVDecoder()
        result = decoder.decode(0x00000073, pc=0)
        assert result.mnemonic == "ecall"

    def test_decode_negative_immediate(self) -> None:
        """addi x1, x0, -1 should have imm = -1 (sign-extended)."""
        decoder = RiscVDecoder()
        instr = encode_addi(1, 0, -1)
        result = decoder.decode(instr, pc=0)
        assert result.fields["imm"] == -1


class TestRiscVSimulator:
    """End-to-end tests running actual RISC-V programs."""

    def test_x_equals_1_plus_2(self) -> None:
        """The target program: x = 1 + 2 → x3 should be 3.

        Program:
            addi x1, x0, 1    # x1 = 1
            addi x2, x0, 2    # x2 = 2
            add  x3, x1, x2   # x3 = x1 + x2 = 3
            ecall              # halt
        """
        sim = RiscVSimulator()
        program = assemble([
            encode_addi(1, 0, 1),
            encode_addi(2, 0, 2),
            encode_add(3, 1, 2),
            encode_ecall(),
        ])
        traces = sim.run(program)

        assert len(traces) == 4
        assert sim.cpu.registers.read(1) == 1
        assert sim.cpu.registers.read(2) == 2
        assert sim.cpu.registers.read(3) == 3
        assert sim.cpu.halted is True

    def test_x0_stays_zero(self) -> None:
        """Writing to x0 should be ignored — x0 is always 0."""
        sim = RiscVSimulator()
        program = assemble([
            encode_addi(0, 0, 42),  # Try to write 42 to x0
            encode_ecall(),
        ])
        sim.run(program)
        assert sim.cpu.registers.read(0) == 0

    def test_pipeline_trace_visible(self) -> None:
        """Each step should produce a visible pipeline trace."""
        sim = RiscVSimulator()
        program = assemble([
            encode_addi(1, 0, 7),
            encode_ecall(),
        ])
        sim.cpu.load_program(program)
        trace = sim.step()

        assert trace.fetch.pc == 0
        assert trace.decode.mnemonic == "addi"
        assert trace.decode.fields["imm"] == 7
        assert "x1" in trace.execute.registers_changed
        assert trace.execute.registers_changed["x1"] == 7

    def test_pipeline_format(self) -> None:
        """The pipeline format should show all three stages."""
        sim = RiscVSimulator()
        program = assemble([encode_addi(1, 0, 1), encode_ecall()])
        sim.cpu.load_program(program)
        trace = sim.step()
        output = trace.format_pipeline()
        assert "FETCH" in output
        assert "DECODE" in output
        assert "EXECUTE" in output
        assert "addi" in output

    def test_add_large_numbers(self) -> None:
        """Add 100 + 200 = 300."""
        sim = RiscVSimulator()
        program = assemble([
            encode_addi(1, 0, 100),
            encode_addi(2, 0, 200),
            encode_add(3, 1, 2),
            encode_ecall(),
        ])
        sim.run(program)
        assert sim.cpu.registers.read(3) == 300

    def test_negative_immediate(self) -> None:
        """addi x1, x0, -5 should set x1 to -5 (as unsigned 0xFFFFFFFB)."""
        sim = RiscVSimulator()
        program = assemble([
            encode_addi(1, 0, -5),
            encode_ecall(),
        ])
        sim.run(program)
        # In 32-bit unsigned, -5 = 0xFFFFFFFB = 4294967291
        assert sim.cpu.registers.read(1) == 0xFFFFFFFB
