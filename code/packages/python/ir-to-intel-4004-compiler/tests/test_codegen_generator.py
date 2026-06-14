"""Tests for Intel4004CodeGenerator — CodeGenerator[IrProgram, str] (LANG20).

Covers: name, validate (valid + invalid IR), generate, protocol check, round-trip.
"""

from __future__ import annotations

import pytest

from codegen_core import CodeGenerator
from compiler_ir import IrDataDecl, IrImmediate, IrInstruction, IrLabel, IrOp, IrProgram, IrRegister
from intel_4004_ir_validator import IrValidationError
from ir_to_intel_4004_compiler import Intel4004CodeGenerator


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _minimal_ir() -> IrProgram:
    """Minimal valid IrProgram for the 4004: LOAD_IMM r2, 5; HALT."""
    prog = IrProgram(entry_label="_start")
    prog.add_instruction(IrInstruction(IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(5)], id=0))
    prog.add_instruction(IrInstruction(IrOp.HALT, [], id=1))
    return prog


def _too_many_registers_ir() -> IrProgram:
    """IrProgram that uses > 12 distinct virtual registers — fails register_count rule."""
    prog = IrProgram(entry_label="_start")
    # Use registers 0..13 (14 distinct registers > 12 limit)
    for i in range(14):
        prog.add_instruction(
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(i), IrImmediate(i)], id=i)
        )
    prog.add_instruction(IrInstruction(IrOp.HALT, [], id=14))
    return prog


def _too_much_ram_ir() -> IrProgram:
    """IrProgram with > 160 bytes of static RAM — fails static_ram rule."""
    prog = IrProgram(entry_label="_start")
    # Declare 170 bytes of static data (> 160-byte GE-225 RAM limit on 4004)
    prog.add_data(IrDataDecl(label="big", size=170))
    prog.add_instruction(IrInstruction(IrOp.HALT, [], id=0))
    return prog


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------


class TestIntel4004CodeGenerator:

    def test_name(self) -> None:
        assert Intel4004CodeGenerator().name == "intel4004"

    def test_satisfies_codegenerator_protocol(self) -> None:
        assert isinstance(Intel4004CodeGenerator(), CodeGenerator)

    def test_validate_valid_ir_returns_empty(self) -> None:
        gen = Intel4004CodeGenerator()
        assert gen.validate(_minimal_ir()) == []

    def test_validate_too_many_registers_returns_errors(self) -> None:
        gen = Intel4004CodeGenerator()
        errors = gen.validate(_too_many_registers_ir())
        assert len(errors) >= 1
        assert any("register" in e.lower() for e in errors)

    def test_validate_too_much_ram_returns_errors(self) -> None:
        gen = Intel4004CodeGenerator()
        errors = gen.validate(_too_much_ram_ir())
        assert len(errors) >= 1
        assert any("ram" in e.lower() or "160" in e for e in errors)

    def test_validate_returns_list_of_strings(self) -> None:
        """validate() must return list[str], not list[IrValidationError]."""
        gen = Intel4004CodeGenerator()
        errors = gen.validate(_too_many_registers_ir())
        assert all(isinstance(e, str) for e in errors)

    def test_validate_does_not_raise(self) -> None:
        gen = Intel4004CodeGenerator()
        result = gen.validate(_too_many_registers_ir())
        assert isinstance(result, list)

    def test_generate_valid_ir_returns_string(self) -> None:
        gen = Intel4004CodeGenerator()
        result = gen.generate(_minimal_ir())
        assert isinstance(result, str)

    def test_generate_output_is_nonempty(self) -> None:
        gen = Intel4004CodeGenerator()
        result = gen.generate(_minimal_ir())
        assert len(result) > 0

    def test_generate_output_contains_org_directive(self) -> None:
        gen = Intel4004CodeGenerator()
        result = gen.generate(_minimal_ir())
        assert "ORG" in result

    def test_generate_output_contains_halt(self) -> None:
        gen = Intel4004CodeGenerator()
        result = gen.generate(_minimal_ir())
        assert "HLT" in result

    def test_generate_bad_ir_raises(self) -> None:
        gen = Intel4004CodeGenerator()
        with pytest.raises(IrValidationError):
            gen.generate(_too_many_registers_ir())

    def test_round_trip_validate_then_generate(self) -> None:
        gen = Intel4004CodeGenerator()
        ir = _minimal_ir()
        errors = gen.validate(ir)
        assert errors == []
        result = gen.generate(ir)
        assert isinstance(result, str)

    def test_exported_from_package(self) -> None:
        import ir_to_intel_4004_compiler
        assert hasattr(ir_to_intel_4004_compiler, "Intel4004CodeGenerator")
