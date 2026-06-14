"""Tests for GE225CodeGenerator — CodeGenerator[IrProgram, CompileResult] (LANG20).

Covers: name, validate (valid + invalid IR), generate, protocol check, round-trip.
"""

from __future__ import annotations

import pytest

from codegen_core import CodeGenerator
from compiler_ir import IrImmediate, IrInstruction, IrLabel, IrOp, IrProgram, IrRegister
from ir_to_ge225_compiler import GE225CodeGenerator
from ir_to_ge225_compiler.codegen import CodeGenError, CompileResult


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _minimal_ir() -> IrProgram:
    """Minimal valid IrProgram: LOAD_IMM r0, 1; HALT."""
    prog = IrProgram(entry_label="_start")
    prog.add_instruction(IrInstruction(IrOp.LOAD_IMM, [IrRegister(0), IrImmediate(1)], id=0))
    prog.add_instruction(IrInstruction(IrOp.HALT, [], id=1))
    return prog


def _bad_opcode_ir() -> IrProgram:
    """IrProgram with an opcode unsupported by GE-225 (CALL is not in the V1 set)."""
    prog = IrProgram(entry_label="_start")
    prog.add_instruction(IrInstruction(IrOp.CALL, [IrLabel("foo")], id=0))
    prog.add_instruction(IrInstruction(IrOp.HALT, [], id=1))
    return prog


def _overflow_constant_ir() -> IrProgram:
    """IrProgram with a LOAD_IMM constant > GE-225 20-bit max (524 287)."""
    prog = IrProgram(entry_label="_start")
    prog.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(0), IrImmediate(1_000_000_000)], id=0)
    )
    prog.add_instruction(IrInstruction(IrOp.HALT, [], id=1))
    return prog


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------


class TestGE225CodeGenerator:

    def test_name(self) -> None:
        assert GE225CodeGenerator().name == "ge225"

    def test_satisfies_codegenerator_protocol(self) -> None:
        assert isinstance(GE225CodeGenerator(), CodeGenerator)

    def test_validate_valid_ir_returns_empty(self) -> None:
        gen = GE225CodeGenerator()
        assert gen.validate(_minimal_ir()) == []

    def test_validate_bad_opcode_returns_errors(self) -> None:
        gen = GE225CodeGenerator()
        errors = gen.validate(_bad_opcode_ir())
        assert len(errors) >= 1
        assert any("CALL" in e or "unsupported" in e.lower() for e in errors)

    def test_validate_overflow_constant_returns_errors(self) -> None:
        gen = GE225CodeGenerator()
        errors = gen.validate(_overflow_constant_ir())
        assert len(errors) >= 1
        assert any("overflow" in e.lower() or "range" in e.lower() or "bit" in e.lower() for e in errors)

    def test_validate_does_not_raise(self) -> None:
        """validate() must return a list, never raise."""
        gen = GE225CodeGenerator()
        result = gen.validate(_bad_opcode_ir())
        assert isinstance(result, list)

    def test_generate_valid_ir_returns_compile_result(self) -> None:
        gen = GE225CodeGenerator()
        result = gen.generate(_minimal_ir())
        assert isinstance(result, CompileResult)

    def test_generate_binary_is_bytes(self) -> None:
        gen = GE225CodeGenerator()
        result = gen.generate(_minimal_ir())
        assert isinstance(result.binary, bytes)

    def test_generate_binary_is_nonempty(self) -> None:
        gen = GE225CodeGenerator()
        result = gen.generate(_minimal_ir())
        assert len(result.binary) > 0

    def test_generate_binary_length_multiple_of_3(self) -> None:
        """GE-225 words are 3 bytes (20 bits, packed big-endian)."""
        gen = GE225CodeGenerator()
        result = gen.generate(_minimal_ir())
        assert len(result.binary) % 3 == 0

    def test_generate_has_halt_address(self) -> None:
        gen = GE225CodeGenerator()
        result = gen.generate(_minimal_ir())
        assert isinstance(result.halt_address, int)
        assert result.halt_address > 0

    def test_generate_has_data_base(self) -> None:
        gen = GE225CodeGenerator()
        result = gen.generate(_minimal_ir())
        assert isinstance(result.data_base, int)

    def test_generate_bad_ir_raises(self) -> None:
        gen = GE225CodeGenerator()
        with pytest.raises(CodeGenError):
            gen.generate(_bad_opcode_ir())

    def test_round_trip_validate_then_generate(self) -> None:
        gen = GE225CodeGenerator()
        ir = _minimal_ir()
        errors = gen.validate(ir)
        assert errors == []
        result = gen.generate(ir)
        assert result.binary is not None

    def test_exported_from_package(self) -> None:
        """GE225CodeGenerator must be importable from the top-level package."""
        import ir_to_ge225_compiler
        assert hasattr(ir_to_ge225_compiler, "GE225CodeGenerator")
