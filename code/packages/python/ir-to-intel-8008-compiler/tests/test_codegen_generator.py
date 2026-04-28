"""Tests for Intel8008CodeGenerator — CodeGenerator[IrProgram, str] (LANG20).

Covers: name, validate (valid + invalid IR), generate, protocol check, round-trip.
"""

from __future__ import annotations

import pytest

from codegen_core import CodeGenerator
from compiler_ir import IrDataDecl, IrImmediate, IrInstruction, IrLabel, IrOp, IrProgram, IrRegister
from intel_8008_ir_validator import IrValidationError
from ir_to_intel_8008_compiler import Intel8008CodeGenerator


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _minimal_ir() -> IrProgram:
    """Minimal valid IrProgram for the 8008: LOAD_IMM r1, 42; HALT."""
    prog = IrProgram(entry_label="_start")
    prog.add_instruction(IrInstruction(IrOp.LOAD_IMM, [IrRegister(1), IrImmediate(42)], id=0))
    prog.add_instruction(IrInstruction(IrOp.HALT, [], id=1))
    return prog


def _too_many_registers_ir() -> IrProgram:
    """IrProgram with > 6 distinct virtual registers — fails 8008 register_count rule."""
    prog = IrProgram(entry_label="_start")
    # Use registers 0..7 (8 distinct > 6 limit on 8008)
    for i in range(8):
        prog.add_instruction(
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(i), IrImmediate(i)], id=i)
        )
    prog.add_instruction(IrInstruction(IrOp.HALT, [], id=8))
    return prog


def _unsupported_opcode_ir() -> IrProgram:
    """IrProgram with LOAD_WORD which is not in the 8008 supported set."""
    prog = IrProgram(entry_label="_start")
    # LOAD_WORD is not a valid 8008 IR opcode
    prog.add_instruction(
        IrInstruction(IrOp.LOAD_WORD, [IrRegister(1), IrRegister(0), IrImmediate(0)], id=0)
    )
    prog.add_instruction(IrInstruction(IrOp.HALT, [], id=1))
    return prog


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------


class TestIntel8008CodeGenerator:

    def test_name(self) -> None:
        assert Intel8008CodeGenerator().name == "intel8008"

    def test_satisfies_codegenerator_protocol(self) -> None:
        assert isinstance(Intel8008CodeGenerator(), CodeGenerator)

    def test_validate_valid_ir_returns_empty(self) -> None:
        gen = Intel8008CodeGenerator()
        assert gen.validate(_minimal_ir()) == []

    def test_validate_too_many_registers_returns_errors(self) -> None:
        gen = Intel8008CodeGenerator()
        errors = gen.validate(_too_many_registers_ir())
        assert len(errors) >= 1
        assert any("register" in e.lower() for e in errors)

    def test_validate_returns_list_of_strings(self) -> None:
        """validate() must return list[str], not list[IrValidationError]."""
        gen = Intel8008CodeGenerator()
        errors = gen.validate(_too_many_registers_ir())
        assert all(isinstance(e, str) for e in errors)

    def test_validate_does_not_raise(self) -> None:
        gen = Intel8008CodeGenerator()
        result = gen.validate(_too_many_registers_ir())
        assert isinstance(result, list)

    def test_generate_valid_ir_returns_string(self) -> None:
        gen = Intel8008CodeGenerator()
        result = gen.generate(_minimal_ir())
        assert isinstance(result, str)

    def test_generate_output_is_nonempty(self) -> None:
        gen = Intel8008CodeGenerator()
        result = gen.generate(_minimal_ir())
        assert len(result) > 0

    def test_generate_output_contains_org_directive(self) -> None:
        gen = Intel8008CodeGenerator()
        result = gen.generate(_minimal_ir())
        assert "ORG" in result

    def test_generate_output_contains_halt(self) -> None:
        gen = Intel8008CodeGenerator()
        result = gen.generate(_minimal_ir())
        assert "HLT" in result

    def test_generate_bad_ir_raises(self) -> None:
        gen = Intel8008CodeGenerator()
        with pytest.raises(IrValidationError):
            gen.generate(_too_many_registers_ir())

    def test_round_trip_validate_then_generate(self) -> None:
        gen = Intel8008CodeGenerator()
        ir = _minimal_ir()
        errors = gen.validate(ir)
        assert errors == []
        result = gen.generate(ir)
        assert isinstance(result, str)

    def test_exported_from_package(self) -> None:
        import ir_to_intel_8008_compiler
        assert hasattr(ir_to_intel_8008_compiler, "Intel8008CodeGenerator")
