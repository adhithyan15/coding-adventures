"""Tests for the GenericISA instruction set implementation."""

from __future__ import annotations

import pytest

from gpu_core.generic_isa import GenericISA
from gpu_core.memory import LocalMemory
from gpu_core.opcodes import (
    beq,
    blt,
    bne,
    fabs,
    fadd,
    ffma,
    fmul,
    fneg,
    fsub,
    halt,
    jmp,
    limm,
    load,
    mov,
    nop,
    store,
)
from gpu_core.protocols import InstructionSet
from gpu_core.registers import FPRegisterFile


@pytest.fixture()
def isa() -> GenericISA:
    return GenericISA()


@pytest.fixture()
def regs() -> FPRegisterFile:
    return FPRegisterFile()


@pytest.fixture()
def mem() -> LocalMemory:
    return LocalMemory()


class TestProtocolCompliance:
    """Verify GenericISA satisfies the InstructionSet protocol."""

    def test_is_instruction_set(self) -> None:
        assert isinstance(GenericISA(), InstructionSet)

    def test_has_name(self) -> None:
        assert GenericISA().name == "Generic"


class TestArithmetic:
    """Test floating-point arithmetic instructions."""

    def test_fadd(
        self, isa: GenericISA, regs: FPRegisterFile, mem: LocalMemory
    ) -> None:
        regs.write_float(0, 1.0)
        regs.write_float(1, 2.0)
        result = isa.execute(fadd(2, 0, 1), regs, mem)
        assert regs.read_float(2) == 3.0
        assert result.registers_changed == {"R2": 3.0}

    def test_fadd_negative(
        self, isa: GenericISA, regs: FPRegisterFile, mem: LocalMemory
    ) -> None:
        regs.write_float(0, 1.0)
        regs.write_float(1, -3.0)
        isa.execute(fadd(2, 0, 1), regs, mem)
        assert regs.read_float(2) == -2.0

    def test_fsub(
        self, isa: GenericISA, regs: FPRegisterFile, mem: LocalMemory
    ) -> None:
        regs.write_float(0, 5.0)
        regs.write_float(1, 3.0)
        isa.execute(fsub(2, 0, 1), regs, mem)
        assert regs.read_float(2) == 2.0

    def test_fmul(
        self, isa: GenericISA, regs: FPRegisterFile, mem: LocalMemory
    ) -> None:
        regs.write_float(0, 3.0)
        regs.write_float(1, 4.0)
        isa.execute(fmul(2, 0, 1), regs, mem)
        assert regs.read_float(2) == 12.0

    def test_fmul_by_zero(
        self, isa: GenericISA, regs: FPRegisterFile, mem: LocalMemory
    ) -> None:
        regs.write_float(0, 42.0)
        regs.write_float(1, 0.0)
        isa.execute(fmul(2, 0, 1), regs, mem)
        assert regs.read_float(2) == 0.0

    def test_ffma(
        self, isa: GenericISA, regs: FPRegisterFile, mem: LocalMemory
    ) -> None:
        """FMA: Rd = Rs1 * Rs2 + Rs3 = 2.0 * 3.0 + 1.0 = 7.0."""
        regs.write_float(0, 2.0)
        regs.write_float(1, 3.0)
        regs.write_float(2, 1.0)
        result = isa.execute(ffma(3, 0, 1, 2), regs, mem)
        assert regs.read_float(3) == 7.0
        assert "R3" in (result.registers_changed or {})

    def test_fneg(
        self, isa: GenericISA, regs: FPRegisterFile, mem: LocalMemory
    ) -> None:
        regs.write_float(0, 5.0)
        isa.execute(fneg(1, 0), regs, mem)
        assert regs.read_float(1) == -5.0

    def test_fneg_double(
        self, isa: GenericISA, regs: FPRegisterFile, mem: LocalMemory
    ) -> None:
        """Negating twice returns to original."""
        regs.write_float(0, 3.0)
        isa.execute(fneg(1, 0), regs, mem)
        isa.execute(fneg(2, 1), regs, mem)
        assert regs.read_float(2) == 3.0

    def test_fabs_positive(
        self, isa: GenericISA, regs: FPRegisterFile, mem: LocalMemory
    ) -> None:
        regs.write_float(0, 5.0)
        isa.execute(fabs(1, 0), regs, mem)
        assert regs.read_float(1) == 5.0

    def test_fabs_negative(
        self, isa: GenericISA, regs: FPRegisterFile, mem: LocalMemory
    ) -> None:
        regs.write_float(0, -5.0)
        isa.execute(fabs(1, 0), regs, mem)
        assert regs.read_float(1) == 5.0


class TestMemory:
    """Test memory load/store instructions."""

    def test_store_and_load(
        self, isa: GenericISA, regs: FPRegisterFile, mem: LocalMemory
    ) -> None:
        """Store a value then load it back."""
        regs.write_float(0, 0.0)  # base address = 0
        regs.write_float(1, 3.14)  # value to store
        isa.execute(store(0, 1, 0.0), regs, mem)
        isa.execute(load(2, 0, 0.0), regs, mem)
        assert regs.read_float(2) == pytest.approx(3.14, rel=1e-5)

    def test_store_with_offset(
        self, isa: GenericISA, regs: FPRegisterFile, mem: LocalMemory
    ) -> None:
        """Store with a non-zero offset."""
        regs.write_float(0, 0.0)  # base = 0
        regs.write_float(1, 42.0)
        isa.execute(store(0, 1, 8.0), regs, mem)  # store at address 8
        isa.execute(load(2, 0, 8.0), regs, mem)  # load from address 8
        assert regs.read_float(2) == 42.0

    def test_store_result_description(
        self, isa: GenericISA, regs: FPRegisterFile, mem: LocalMemory
    ) -> None:
        """Store returns memory_changed in result."""
        regs.write_float(0, 0.0)
        regs.write_float(1, 5.0)
        result = isa.execute(store(0, 1, 0.0), regs, mem)
        assert result.memory_changed is not None
        assert 0 in result.memory_changed

    def test_load_result_description(
        self, isa: GenericISA, regs: FPRegisterFile, mem: LocalMemory
    ) -> None:
        """Load returns registers_changed in result."""
        mem.store_python_float(0, 7.0)
        regs.write_float(0, 0.0)
        result = isa.execute(load(1, 0, 0.0), regs, mem)
        assert result.registers_changed is not None
        assert "R1" in result.registers_changed


class TestDataMovement:
    """Test MOV and LIMM instructions."""

    def test_mov(
        self, isa: GenericISA, regs: FPRegisterFile, mem: LocalMemory
    ) -> None:
        regs.write_float(0, 42.0)
        isa.execute(mov(1, 0), regs, mem)
        assert regs.read_float(1) == 42.0

    def test_limm(
        self, isa: GenericISA, regs: FPRegisterFile, mem: LocalMemory
    ) -> None:
        isa.execute(limm(0, 3.14), regs, mem)
        assert abs(regs.read_float(0) - 3.14) < 0.01

    def test_limm_negative(
        self, isa: GenericISA, regs: FPRegisterFile, mem: LocalMemory
    ) -> None:
        isa.execute(limm(0, -99.0), regs, mem)
        assert regs.read_float(0) == -99.0

    def test_limm_zero(
        self, isa: GenericISA, regs: FPRegisterFile, mem: LocalMemory
    ) -> None:
        isa.execute(limm(0, 0.0), regs, mem)
        assert regs.read_float(0) == 0.0


class TestControlFlow:
    """Test branch and jump instructions."""

    def test_beq_taken(
        self, isa: GenericISA, regs: FPRegisterFile, mem: LocalMemory
    ) -> None:
        """BEQ branches when registers are equal."""
        regs.write_float(0, 5.0)
        regs.write_float(1, 5.0)
        result = isa.execute(beq(0, 1, 3), regs, mem)
        assert result.next_pc_offset == 3

    def test_beq_not_taken(
        self, isa: GenericISA, regs: FPRegisterFile, mem: LocalMemory
    ) -> None:
        """BEQ falls through when registers differ."""
        regs.write_float(0, 5.0)
        regs.write_float(1, 3.0)
        result = isa.execute(beq(0, 1, 3), regs, mem)
        assert result.next_pc_offset == 1

    def test_blt_taken(
        self, isa: GenericISA, regs: FPRegisterFile, mem: LocalMemory
    ) -> None:
        """BLT branches when Rs1 < Rs2."""
        regs.write_float(0, 2.0)
        regs.write_float(1, 5.0)
        result = isa.execute(blt(0, 1, 4), regs, mem)
        assert result.next_pc_offset == 4

    def test_blt_not_taken(
        self, isa: GenericISA, regs: FPRegisterFile, mem: LocalMemory
    ) -> None:
        """BLT falls through when Rs1 >= Rs2."""
        regs.write_float(0, 5.0)
        regs.write_float(1, 2.0)
        result = isa.execute(blt(0, 1, 4), regs, mem)
        assert result.next_pc_offset == 1

    def test_bne_taken(
        self, isa: GenericISA, regs: FPRegisterFile, mem: LocalMemory
    ) -> None:
        """BNE branches when registers differ."""
        regs.write_float(0, 1.0)
        regs.write_float(1, 2.0)
        result = isa.execute(bne(0, 1, 2), regs, mem)
        assert result.next_pc_offset == 2

    def test_bne_not_taken(
        self, isa: GenericISA, regs: FPRegisterFile, mem: LocalMemory
    ) -> None:
        """BNE falls through when registers are equal."""
        regs.write_float(0, 5.0)
        regs.write_float(1, 5.0)
        result = isa.execute(bne(0, 1, 2), regs, mem)
        assert result.next_pc_offset == 1

    def test_jmp(
        self, isa: GenericISA, regs: FPRegisterFile, mem: LocalMemory
    ) -> None:
        """JMP sets absolute PC."""
        result = isa.execute(jmp(10), regs, mem)
        assert result.next_pc_offset == 10
        assert result.absolute_jump is True

    def test_nop(
        self, isa: GenericISA, regs: FPRegisterFile, mem: LocalMemory
    ) -> None:
        """NOP does nothing but advance PC."""
        result = isa.execute(nop(), regs, mem)
        assert result.next_pc_offset == 1
        assert result.halted is False

    def test_halt(
        self, isa: GenericISA, regs: FPRegisterFile, mem: LocalMemory
    ) -> None:
        """HALT sets the halted flag."""
        result = isa.execute(halt(), regs, mem)
        assert result.halted is True


class TestDescriptions:
    """Test that execute results include readable descriptions."""

    def test_fadd_description(
        self, isa: GenericISA, regs: FPRegisterFile, mem: LocalMemory
    ) -> None:
        regs.write_float(0, 1.0)
        regs.write_float(1, 2.0)
        result = isa.execute(fadd(2, 0, 1), regs, mem)
        assert "1.0" in result.description
        assert "2.0" in result.description
        assert "3.0" in result.description

    def test_ffma_description(
        self, isa: GenericISA, regs: FPRegisterFile, mem: LocalMemory
    ) -> None:
        regs.write_float(0, 2.0)
        regs.write_float(1, 3.0)
        regs.write_float(2, 1.0)
        result = isa.execute(ffma(3, 0, 1, 2), regs, mem)
        assert "7.0" in result.description

    def test_branch_description_taken(
        self, isa: GenericISA, regs: FPRegisterFile, mem: LocalMemory
    ) -> None:
        regs.write_float(0, 5.0)
        regs.write_float(1, 5.0)
        result = isa.execute(beq(0, 1, 3), regs, mem)
        assert "branch" in result.description.lower()

    def test_branch_description_not_taken(
        self, isa: GenericISA, regs: FPRegisterFile, mem: LocalMemory
    ) -> None:
        regs.write_float(0, 1.0)
        regs.write_float(1, 2.0)
        result = isa.execute(beq(0, 1, 3), regs, mem)
        assert "fall through" in result.description.lower()
