"""Tests for opcodes and instruction construction."""

from __future__ import annotations

from gpu_core.opcodes import (
    Instruction,
    Opcode,
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


class TestOpcode:
    """Test the Opcode enum."""

    def test_all_opcodes_exist(self) -> None:
        """Verify all 16 opcodes are defined."""
        assert len(Opcode) == 16

    def test_opcode_values(self) -> None:
        """Opcode values are lowercase strings."""
        assert Opcode.FADD.value == "fadd"
        assert Opcode.HALT.value == "halt"


class TestInstruction:
    """Test the Instruction dataclass."""

    def test_frozen(self) -> None:
        """Instructions are immutable."""
        inst = Instruction(Opcode.FADD, rd=0, rs1=1, rs2=2)
        with __import__("pytest").raises(AttributeError):
            inst.rd = 5  # type: ignore[misc]

    def test_defaults(self) -> None:
        """Unspecified fields default to 0."""
        inst = Instruction(Opcode.NOP)
        assert inst.rd == 0
        assert inst.rs1 == 0
        assert inst.rs2 == 0
        assert inst.rs3 == 0
        assert inst.immediate == 0.0

    def test_repr_fadd(self) -> None:
        """FADD repr shows assembly-like syntax."""
        inst = fadd(2, 0, 1)
        assert repr(inst) == "FADD R2, R0, R1"

    def test_repr_ffma(self) -> None:
        """FFMA repr shows all four register operands."""
        inst = ffma(3, 0, 1, 2)
        assert repr(inst) == "FFMA R3, R0, R1, R2"

    def test_repr_limm(self) -> None:
        """LIMM repr shows the immediate value."""
        inst = limm(0, 3.14)
        assert "3.14" in repr(inst)

    def test_repr_load(self) -> None:
        """LOAD repr shows memory access syntax."""
        inst = load(0, 1, 4.0)
        assert "LOAD" in repr(inst)
        assert "[R1+" in repr(inst)

    def test_repr_store(self) -> None:
        """STORE repr shows memory access syntax."""
        inst = store(1, 2, 8.0)
        assert "STORE" in repr(inst)

    def test_repr_beq(self) -> None:
        """BEQ repr shows branch offset."""
        inst = beq(0, 1, 3)
        assert "BEQ" in repr(inst)
        assert "+3" in repr(inst)

    def test_repr_beq_negative(self) -> None:
        """BEQ with negative offset shows minus sign."""
        inst = beq(0, 1, -2)
        assert "-2" in repr(inst)

    def test_repr_halt(self) -> None:
        """HALT repr is simple."""
        assert repr(halt()) == "HALT"

    def test_repr_nop(self) -> None:
        """NOP repr is simple."""
        assert repr(nop()) == "NOP"

    def test_repr_jmp(self) -> None:
        """JMP repr shows target."""
        inst = jmp(5)
        assert "JMP" in repr(inst)
        assert "5" in repr(inst)


class TestHelperConstructors:
    """Test the convenience helper functions."""

    def test_fadd(self) -> None:
        inst = fadd(2, 0, 1)
        assert inst.opcode == Opcode.FADD
        assert inst.rd == 2
        assert inst.rs1 == 0
        assert inst.rs2 == 1

    def test_fsub(self) -> None:
        inst = fsub(2, 0, 1)
        assert inst.opcode == Opcode.FSUB

    def test_fmul(self) -> None:
        inst = fmul(2, 0, 1)
        assert inst.opcode == Opcode.FMUL

    def test_ffma(self) -> None:
        inst = ffma(3, 0, 1, 2)
        assert inst.opcode == Opcode.FFMA
        assert inst.rs3 == 2

    def test_fneg(self) -> None:
        inst = fneg(1, 0)
        assert inst.opcode == Opcode.FNEG
        assert inst.rd == 1
        assert inst.rs1 == 0

    def test_fabs(self) -> None:
        inst = fabs(1, 0)
        assert inst.opcode == Opcode.FABS

    def test_load(self) -> None:
        inst = load(0, 1, 4.0)
        assert inst.opcode == Opcode.LOAD
        assert inst.rd == 0
        assert inst.rs1 == 1
        assert inst.immediate == 4.0

    def test_load_default_offset(self) -> None:
        inst = load(0, 1)
        assert inst.immediate == 0.0

    def test_store(self) -> None:
        inst = store(1, 2, 8.0)
        assert inst.opcode == Opcode.STORE
        assert inst.rs1 == 1
        assert inst.rs2 == 2
        assert inst.immediate == 8.0

    def test_mov(self) -> None:
        inst = mov(1, 0)
        assert inst.opcode == Opcode.MOV

    def test_limm(self) -> None:
        inst = limm(0, 3.14)
        assert inst.opcode == Opcode.LIMM
        assert inst.immediate == 3.14

    def test_beq(self) -> None:
        inst = beq(0, 1, 3)
        assert inst.opcode == Opcode.BEQ
        assert inst.immediate == 3.0

    def test_blt(self) -> None:
        inst = blt(0, 1, -2)
        assert inst.opcode == Opcode.BLT
        assert inst.immediate == -2.0

    def test_bne(self) -> None:
        inst = bne(0, 1, 5)
        assert inst.opcode == Opcode.BNE

    def test_jmp(self) -> None:
        inst = jmp(10)
        assert inst.opcode == Opcode.JMP
        assert inst.immediate == 10.0

    def test_nop(self) -> None:
        assert nop().opcode == Opcode.NOP

    def test_halt(self) -> None:
        assert halt().opcode == Opcode.HALT
